//! Outbound notifications (2.0.76) - desktop and ntfy. marko's own request:
//! "chcem tiez nejake pushup notifikacie, najlepsie aj do mobilu, no len
//! tie najviac prioritne" (push notifications, ideally to mobile too, but
//! only the highest-priority ones).
//!
//! Deliberately reuses the exact same 4 categories AlertBell/
//! AttentionSection already track on the Dashboard (see `DashboardAlerts`
//! in models.rs and `compute_dashboard_alerts` in dashboard.rs) - no new
//! detection logic, no new scoring engine. "Only the highest-priority ones"
//! is enforced two ways: `notification_log` (013_notifications.sql) caps
//! every category at one notification per calendar day no matter how many
//! times the periodic check runs, and the "upcoming events" category
//! specifically only becomes push-worthy once the soonest such event is
//! within `NOTIFICATION_URGENT_EVENT_DAYS` - a 14-day-out event is worth
//! showing on the Dashboard bell (cheap, glanceable) but not worth
//! interrupting marko's phone for yet.
//!
//! 2.0.77 removed the email channel this feature shipped with in 2.0.76 at
//! marko's own request ("email zatial odstranme") - no `EmailChannelConfig`,
//! no `lettre` dependency, no SMTP anywhere in this app any more. If email
//! ever comes back, treat it as a fresh design rather than resurrecting
//! this: 2.0.76 asked for full SMTP credentials, exactly the kind of
//! per-person setup marko has since said he does not want.
//!
//! 2.0.77 also tried simplifying the mobile-push channel (then Pushover) to
//! "just a user key" by embedding this app's own application token at
//! build time - marko pushed back wanting it to need only ONE thing with
//! zero setup anywhere, not even a one-time app registration by him. That
//! is not something Pushover's API can ever do: every send needs BOTH a
//! user key (who receives it) AND an application token (which app is
//! sending) - no way to derive one from the other, by design, for any app
//! that uses Pushover. 2.0.78 replaced Pushover with **ntfy**
//! (<https://ntfy.sh>) instead, which actually satisfies "just one thing,
//! no signup anywhere": its public server accepts a plain HTTP POST to
//! `https://ntfy.sh/<topic>` with no authentication and no application-
//! level credential at all - the "topic" (any string the person makes up)
//! is the only identifier that exists on either side. A person installs the
//! free ntfy app, subscribes to their own made-up topic, and types that
//! same topic into Settings -> Notifications - nothing to register, nothing
//! for this app to embed. The trade-off, stated plainly for whoever reads
//! this next: since ntfy's public server has no authentication, the topic
//! name IS the entire access control - anyone who learns it can publish to
//! it (or read it), so Settings' own copy tells people to pick something
//! non-guessable, the same way a password would be. This is a materially
//! different trust model from Pushover's (a per-account user key plus a
//! per-app token), accepted here because it is what lets this feature ask
//! for only one thing, which marko explicitly said matters more to him than
//! Pushover's extra layer of protection.
//!
//! Credentials this app itself stores (just the ntfy topic now) are kept
//! the same way every other local secret already is - plain text in the
//! generic `app_settings` key/value table (see commands::settings and
//! google_auth.rs's own doc comment on why that's the accepted trust
//! boundary for this local, single-user desktop app). Never echoed back to
//! the frontend: `NotificationStatus` (models.rs) carries only a `*_set:
//! bool` presence flag, precedented by `GoogleSignInStatus`.
//!
//! Anything that touches the network runs `async fn` + `tauri::async_
//! runtime::spawn_blocking` - this codebase already hit and fixed the
//! alternative once (see google_auth.rs's own doc comment on the 2.0.12 ->
//! 2.0.13 fix to `start_google_sign_in`): a plain synchronous
//! `#[tauri::command]` runs on Tauri's single main thread, so a command
//! that blocks on real network I/O there freezes the whole app's UI until
//! it returns. Only `test_desktop_notification` stays plain `fn` - it's a
//! local OS call, no network.
//!
//! Testing follows the same split as google_sheets.rs/ai_categorize.rs/
//! fx.rs: the actual `.send()`/HTTP call stays thin and deliberately
//! untested (this sandbox cannot reach `ntfy.sh` - see fx.rs's own doc
//! comment on the same restriction), everything else (which categories are
//! due, what a notification says, how config merges) is a small pure
//! function with full unit-test coverage.

use crate::commands::dashboard::compute_dashboard_alerts;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    DashboardAlerts, NotificationConfig, NotificationConfigInput, NotificationStatus,
    NotificationTestResult, NtfyChannelConfig,
};
use chrono::{Local, NaiveDate};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use tauri::State;
use tauri_plugin_notification::NotificationExt;

const NOTIFICATION_CONFIG_KEY: &str = "notification_config";

// Mirrors Dashboard.tsx's/Pulls.tsx's own UPCOMING_WARNING_WINDOW_DAYS (the
// threshold that turns an upcoming-event row amber/red) - this backend-only
// check has no access to that frontend constant, so the value is duplicated
// here rather than shared. If that threshold is ever tuned, update both.
const NOTIFICATION_URGENT_EVENT_DAYS: i64 = 3;

/// The 4 things a notification can be about - exactly AlertBell's/
/// AttentionSection's own 4 categories (Dashboard.tsx), nothing new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NotificationCategory {
    UnpaidOrders,
    PendingSales,
    MissingListingPrice,
    UpcomingEvents,
}

impl NotificationCategory {
    /// The value stored in `notification_log.category` - a small fixed set
    /// of string keys this enum owns, not a foreign key to anything else.
    fn db_key(self) -> &'static str {
        match self {
            NotificationCategory::UnpaidOrders => "unpaid_orders",
            NotificationCategory::PendingSales => "pending_sales",
            NotificationCategory::MissingListingPrice => "missing_listing_price",
            NotificationCategory::UpcomingEvents => "upcoming_events",
        }
    }
}

// ---------------------------------------------------------------------------
// Pure logic - fully unit-tested, no I/O.
// ---------------------------------------------------------------------------

/// Which categories should notify right now: currently non-zero AND not
/// already sent today. `notification_log`'s own UNIQUE(category, sent_on)
/// constraint is the actual enforcement of "at most once per day" (see
/// `mark_sent`) - this check exists so a category already known to be sent
/// isn't even attempted again this run.
pub(crate) fn categories_due_for_notification(
    alerts: &DashboardAlerts,
    today: NaiveDate,
    already_sent_today: &HashSet<String>,
) -> Vec<NotificationCategory> {
    let mut due = Vec::new();
    let not_yet_sent = |category: NotificationCategory| !already_sent_today.contains(category.db_key());

    if alerts.unpaid_orders_count > 0 && not_yet_sent(NotificationCategory::UnpaidOrders) {
        due.push(NotificationCategory::UnpaidOrders);
    }
    if alerts.pending_sales_count > 0 && not_yet_sent(NotificationCategory::PendingSales) {
        due.push(NotificationCategory::PendingSales);
    }
    if alerts.missing_listing_price_count > 0 && not_yet_sent(NotificationCategory::MissingListingPrice) {
        due.push(NotificationCategory::MissingListingPrice);
    }

    // "Only the highest priority ones" (see module doc comment): the
    // soonest upcoming event must already be as urgent as the Dashboard's
    // own amber/red badge, not merely somewhere in the 14-day window the
    // Dashboard bell shows.
    let soonest_is_urgent = alerts
        .upcoming_events
        .first()
        .and_then(|ev| NaiveDate::parse_from_str(&ev.event_date, "%Y-%m-%d").ok())
        .is_some_and(|event_date| (event_date - today).num_days() <= NOTIFICATION_URGENT_EVENT_DAYS);
    if alerts.upcoming_events_count > 0 && soonest_is_urgent && not_yet_sent(NotificationCategory::UpcomingEvents) {
        due.push(NotificationCategory::UpcomingEvents);
    }

    due
}

/// One (title, body) pair per category - reuses the same labels
/// AttentionSection/AlertCard (Dashboard.tsx) already show, so a
/// notification and the Dashboard tile it corresponds to always read as the
/// same thing described twice, never two different vocabularies.
pub(crate) fn build_notification_message(
    category: NotificationCategory,
    alerts: &DashboardAlerts,
    today: NaiveDate,
) -> (String, String) {
    let plural = |n: i64| if n == 1 { "" } else { "s" };
    match category {
        NotificationCategory::UnpaidOrders => (
            "Unpaid payments".to_string(),
            format!(
                "{} order{} unpaid or only partially paid.",
                alerts.unpaid_orders_count,
                plural(alerts.unpaid_orders_count)
            ),
        ),
        NotificationCategory::PendingSales => {
            let amount = crate::money::format_cents(alerts.pending_sales_amount_cents);
            let currency = alerts.pending_sales_currency.as_deref().unwrap_or("mixed currencies");
            (
                "Pending sales".to_string(),
                format!(
                    "{} sale{} awaiting payment - {amount} {currency} not yet collected from buyers.",
                    alerts.pending_sales_count,
                    plural(alerts.pending_sales_count)
                ),
            )
        }
        NotificationCategory::MissingListingPrice => (
            "Missing listing price".to_string(),
            format!(
                "{} order{} have a ticket with no listing price set.",
                alerts.missing_listing_price_orders_count,
                plural(alerts.missing_listing_price_orders_count)
            ),
        ),
        NotificationCategory::UpcomingEvents => match alerts.upcoming_events.first() {
            Some(ev) => {
                let when = NaiveDate::parse_from_str(&ev.event_date, "%Y-%m-%d")
                    .ok()
                    .map(|d| (d - today).num_days())
                    .map(|n| match n {
                        n if n > 0 => format!("in {n} day{}", plural(n)),
                        0 => "today".to_string(),
                        n => format!("{} day{} overdue", -n, plural(-n)),
                    })
                    .unwrap_or_else(|| "soon".to_string());
                (
                    "Upcoming event".to_string(),
                    format!(
                        "{} is {when} - {} ticket{} still unsold.",
                        ev.name,
                        ev.relevant_inventory,
                        plural(ev.relevant_inventory)
                    ),
                )
            }
            // Defensive only - upcoming_events_count > 0 always means at
            // least one entry is present in practice (see DashboardAlerts's
            // own doc comment), never actually reached.
            None => (
                "Upcoming events".to_string(),
                format!(
                    "{} upcoming event{} with unsold inventory.",
                    alerts.upcoming_events_count,
                    plural(alerts.upcoming_events_count)
                ),
            ),
        },
    }
}

/// What Settings -> Notifications actually receives from the GET command -
/// never the raw secret value, only whether one is currently stored.
fn status_from_config(config: &NotificationConfig) -> NotificationStatus {
    NotificationStatus {
        desktop_enabled: config.desktop_enabled,
        ntfy_enabled: config.ntfy.enabled,
        ntfy_topic_set: !config.ntfy.topic.is_empty(),
    }
}

/// Merges a `NotificationConfigInput` (submitted by Settings) onto the
/// existing stored config. `desktop_enabled`/`ntfy_enabled` come from the
/// input unconditionally (Settings always submits its whole form); the
/// secret `ntfy_topic` is `Option<String>` and only overwrites when `Some` -
/// `None` means the user left that field blank, i.e. keep whatever is
/// already stored.
fn apply_input(existing: NotificationConfig, input: NotificationConfigInput) -> NotificationConfig {
    NotificationConfig {
        desktop_enabled: input.desktop_enabled,
        ntfy: NtfyChannelConfig {
            enabled: input.ntfy_enabled,
            topic: input.ntfy_topic.unwrap_or(existing.ntfy.topic),
        },
    }
}

// ---------------------------------------------------------------------------
// Storage - the config blob and the per-day dedup log.
// ---------------------------------------------------------------------------

/// Absent key -> a disabled, empty config (nothing configured yet, same as
/// a brand-new install). A present-but-corrupt blob is a real, surfaced
/// error, never silently discarded - same convention `sheets_sync::
/// load_connection` already established for its own stored JSON blob.
fn load_notification_config(conn: &Connection) -> AppResult<NotificationConfig> {
    match get_setting(conn, NOTIFICATION_CONFIG_KEY)? {
        None => Ok(NotificationConfig::default()),
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| AppError::Other(format!("stored notification settings are corrupt: {e}"))),
    }
}

fn save_notification_config(conn: &Connection, config: &NotificationConfig) -> AppResult<()> {
    let raw = serde_json::to_string(config)
        .map_err(|e| AppError::Other(format!("failed to serialize notification settings: {e}")))?;
    set_setting(conn, NOTIFICATION_CONFIG_KEY, &raw)
}

fn already_sent_today(conn: &Connection, today: NaiveDate) -> AppResult<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT category FROM notification_log WHERE sent_on = ?1")?;
    let rows = stmt.query_map(params![today.to_string()], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<HashSet<_>, _>>()?)
}

/// `INSERT OR IGNORE`: the table's own UNIQUE(category, sent_on) constraint
/// is the actual enforcement of "at most once per category per day" - a
/// constraint hit here (this run and an earlier one both decided the same
/// category was due) is a normal, silent no-op, never an error.
fn mark_sent(conn: &Connection, category: NotificationCategory, today: NaiveDate) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO notification_log (category, sent_on) VALUES (?1, ?2)",
        params![category.db_key(), today.to_string()],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Thin, deliberately-untested network/OS calls.
// ---------------------------------------------------------------------------

fn send_desktop_notification(app: &tauri::AppHandle, title: &str, body: &str) -> AppResult<()> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| AppError::Other(format!("could not show desktop notification: {e}")))
}

/// ntfy.sh's public server needs no authentication and no application-level
/// credential (see this module's top doc comment) - a plain POST to
/// `https://ntfy.sh/<topic>` with the message as the body and the title as
/// a header is the entire protocol. `utf8_percent_encode`/`NON_ALPHANUMERIC`
/// is the same URL-path-segment-encoding idiom google_sheets.rs already uses
/// for spreadsheet/file ids - defensive against a topic containing spaces,
/// diacritics, or other characters that would otherwise break the URL.
fn send_ntfy(cfg: &NtfyChannelConfig, title: &str, body: &str) -> AppResult<()> {
    let topic = cfg.topic.trim();
    if topic.is_empty() {
        return Err(AppError::Validation("no ntfy topic is set".to_string()));
    }
    let encoded_topic = utf8_percent_encode(topic, NON_ALPHANUMERIC);
    let url = format!("https://ntfy.sh/{encoded_topic}");

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .header("Title", title)
        .body(body.to_string())
        .send()
        .map_err(|e| AppError::External(format!("could not reach ntfy: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(AppError::External(format!("ntfy rejected the request ({status}): {text}")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_notification_status(state: State<AppState>) -> AppResult<NotificationStatus> {
    let conn = state.db.lock().unwrap();
    Ok(status_from_config(&load_notification_config(&conn)?))
}

#[tauri::command]
pub fn set_notification_config(state: State<AppState>, input: NotificationConfigInput) -> AppResult<NotificationStatus> {
    let conn = state.db.lock().unwrap();
    let existing = load_notification_config(&conn)?;
    let merged = apply_input(existing, input);
    save_notification_config(&conn, &merged)?;
    Ok(status_from_config(&merged))
}

/// Local OS call only, no network - stays plain `fn` (see module doc
/// comment for why the ntfy test command below can't).
#[tauri::command]
pub fn test_desktop_notification(app: tauri::AppHandle) -> AppResult<NotificationTestResult> {
    Ok(match send_desktop_notification(&app, "TIQR Manager", "This is a test notification.") {
        Ok(()) => NotificationTestResult { success: true, message: "Desktop notification sent.".to_string() },
        Err(e) => NotificationTestResult { success: false, message: e.to_string() },
    })
}

#[tauri::command]
pub async fn test_ntfy_notification(state: State<'_, AppState>) -> AppResult<NotificationTestResult> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_notification_config(&conn)?.ntfy
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        send_ntfy(&cfg, "TIQR Manager", "This is a test ntfy notification.")
    })
    .await;
    Ok(match result {
        Ok(Ok(())) => NotificationTestResult { success: true, message: "Test ntfy notification sent.".to_string() },
        Ok(Err(e)) => NotificationTestResult { success: false, message: e.to_string() },
        Err(e) => NotificationTestResult {
            success: false,
            message: format!("the test-ntfy task did not complete cleanly: {e}"),
        },
    })
}

/// The periodic check (Layout.tsx calls this every 30 minutes). `async fn` +
/// `spawn_blocking` for ntfy - see module doc comment. Silent on both
/// success and failure toward the caller (Layout.tsx's own `.catch(() =>
/// {})`) - same "no new alert noise" philosophy `DashboardAlerts` itself
/// already follows; a single channel failing never stops the others from
/// being tried, and never fails the whole run.
#[tauri::command]
pub async fn check_and_send_notifications(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let config = {
        let conn = state.db.lock().unwrap();
        load_notification_config(&conn)?
    };
    if !config.desktop_enabled && !config.ntfy.enabled {
        return Ok(());
    }

    let today = Local::now().date_naive();
    let (alerts, sent_today) = {
        let conn = state.db.lock().unwrap();
        let alerts = compute_dashboard_alerts(&conn, today)?;
        let sent_today = already_sent_today(&conn, today)?;
        (alerts, sent_today)
    };

    let due = categories_due_for_notification(&alerts, today, &sent_today);

    for category in due {
        let (title, body) = build_notification_message(category, &alerts, today);
        let mut any_success = false;

        if config.desktop_enabled && send_desktop_notification(&app, &title, &body).is_ok() {
            any_success = true;
        }
        if config.ntfy.enabled {
            let cfg = config.ntfy.clone();
            let (t, b) = (title.clone(), body.clone());
            let sent = tauri::async_runtime::spawn_blocking(move || send_ntfy(&cfg, &t, &b)).await;
            if matches!(sent, Ok(Ok(()))) {
                any_success = true;
            }
        }

        if any_success {
            let conn = state.db.lock().unwrap();
            mark_sent(&conn, category, today)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::models::UpcomingEventAlert;
    use chrono::Duration;

    fn sample_alerts() -> DashboardAlerts {
        DashboardAlerts {
            unpaid_orders_count: 0,
            missing_listing_price_count: 0,
            missing_listing_price_orders_count: 0,
            upcoming_events_count: 0,
            upcoming_events: Vec::new(),
            pending_sales_count: 0,
            pending_sales_amount_cents: 0,
            pending_sales_currency: Some("EUR".to_string()),
            // 2.0.79: not one of this module's own notification categories
            // (see this file's module doc comment) - added only because
            // DashboardAlerts now requires it.
            pulls_needing_transfer_count: 0,
        }
    }

    // ---- categories_due_for_notification -----------------------------

    #[test]
    fn nothing_is_due_when_every_category_is_at_zero() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let due = categories_due_for_notification(&sample_alerts(), today, &HashSet::new());
        assert!(due.is_empty());
    }

    #[test]
    fn unpaid_orders_is_due_when_non_zero_and_not_already_sent() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts { unpaid_orders_count: 3, ..sample_alerts() };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due, vec![NotificationCategory::UnpaidOrders]);
    }

    #[test]
    fn pending_sales_is_due_when_non_zero_and_not_already_sent() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts { pending_sales_count: 2, ..sample_alerts() };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due, vec![NotificationCategory::PendingSales]);
    }

    #[test]
    fn missing_listing_price_is_due_when_non_zero_and_not_already_sent() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts {
            missing_listing_price_count: 5,
            missing_listing_price_orders_count: 2,
            ..sample_alerts()
        };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due, vec![NotificationCategory::MissingListingPrice]);
    }

    #[test]
    fn a_category_already_sent_today_is_not_due_again() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts { unpaid_orders_count: 3, ..sample_alerts() };
        let mut sent = HashSet::new();
        sent.insert(NotificationCategory::UnpaidOrders.db_key().to_string());
        let due = categories_due_for_notification(&alerts, today, &sent);
        assert!(due.is_empty());
    }

    #[test]
    fn upcoming_events_is_not_due_when_the_soonest_one_is_outside_the_urgent_window() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let far = (today + Duration::days(10)).to_string(); // outside NOTIFICATION_URGENT_EVENT_DAYS (3)
        let alerts = DashboardAlerts {
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Far Event".to_string(),
                event_date: far,
                relevant_inventory: 4,
            }],
            ..sample_alerts()
        };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert!(due.is_empty(), "an event further out than the urgent window must not push yet, even though the Dashboard bell already shows it");
    }

    #[test]
    fn upcoming_events_is_due_exactly_at_the_urgent_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let boundary = (today + Duration::days(NOTIFICATION_URGENT_EVENT_DAYS)).to_string();
        let alerts = DashboardAlerts {
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Boundary Event".to_string(),
                event_date: boundary,
                relevant_inventory: 2,
            }],
            ..sample_alerts()
        };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due, vec![NotificationCategory::UpcomingEvents]);
    }

    #[test]
    fn upcoming_events_is_due_when_overdue() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let overdue = (today - Duration::days(2)).to_string();
        let alerts = DashboardAlerts {
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Overdue Event".to_string(),
                event_date: overdue,
                relevant_inventory: 1,
            }],
            ..sample_alerts()
        };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due, vec![NotificationCategory::UpcomingEvents]);
    }

    #[test]
    fn all_four_categories_can_be_due_at_once() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let soon = (today + Duration::days(1)).to_string();
        let alerts = DashboardAlerts {
            unpaid_orders_count: 1,
            pending_sales_count: 1,
            missing_listing_price_count: 1,
            missing_listing_price_orders_count: 1,
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Soon Event".to_string(),
                event_date: soon,
                relevant_inventory: 1,
            }],
            ..sample_alerts()
        };
        let due = categories_due_for_notification(&alerts, today, &HashSet::new());
        assert_eq!(due.len(), 4);
    }

    // ---- build_notification_message -----------------------------------

    #[test]
    fn every_category_produces_a_non_empty_title_and_body() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts {
            unpaid_orders_count: 2,
            pending_sales_count: 1,
            pending_sales_amount_cents: 1500,
            missing_listing_price_count: 3,
            missing_listing_price_orders_count: 1,
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Some Concert".to_string(),
                event_date: today.to_string(),
                relevant_inventory: 4,
            }],
            ..sample_alerts()
        };
        for category in [
            NotificationCategory::UnpaidOrders,
            NotificationCategory::PendingSales,
            NotificationCategory::MissingListingPrice,
            NotificationCategory::UpcomingEvents,
        ] {
            let (title, body) = build_notification_message(category, &alerts, today);
            assert!(!title.is_empty());
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn pending_sales_message_includes_the_formatted_amount_and_currency() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts {
            pending_sales_count: 1,
            pending_sales_amount_cents: 1234,
            pending_sales_currency: Some("EUR".to_string()),
            ..sample_alerts()
        };
        let (_, body) = build_notification_message(NotificationCategory::PendingSales, &alerts, today);
        assert!(body.contains("12.34"));
        assert!(body.contains("EUR"));
    }

    #[test]
    fn upcoming_event_message_says_today_when_due_today() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let alerts = DashboardAlerts {
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Today Event".to_string(),
                event_date: today.to_string(),
                relevant_inventory: 2,
            }],
            ..sample_alerts()
        };
        let (_, body) = build_notification_message(NotificationCategory::UpcomingEvents, &alerts, today);
        assert!(body.contains("today"));
        assert!(body.contains("Today Event"));
    }

    #[test]
    fn upcoming_event_message_says_overdue_when_in_the_past() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let overdue = (today - Duration::days(1)).to_string();
        let alerts = DashboardAlerts {
            upcoming_events_count: 1,
            upcoming_events: vec![UpcomingEventAlert {
                id: 1,
                name: "Overdue Event".to_string(),
                event_date: overdue,
                relevant_inventory: 1,
            }],
            ..sample_alerts()
        };
        let (_, body) = build_notification_message(NotificationCategory::UpcomingEvents, &alerts, today);
        assert!(body.contains("overdue"));
    }

    // ---- status_from_config / apply_input -------------------------------

    #[test]
    fn status_never_carries_the_actual_secret_value() {
        let config = NotificationConfig {
            desktop_enabled: true,
            ntfy: NtfyChannelConfig { enabled: true, topic: "my-very-secret-topic".to_string() },
        };
        let status = status_from_config(&config);
        assert!(status.ntfy_topic_set);
        // The debug representation of the status must never contain the
        // actual secret text - only a boolean about its presence.
        let dump = format!("{status:?}");
        assert!(!dump.contains("my-very-secret-topic"));
    }

    #[test]
    fn a_none_secret_in_the_input_keeps_the_existing_stored_secret() {
        let existing = NotificationConfig {
            ntfy: NtfyChannelConfig { topic: "old-topic".to_string(), ..Default::default() },
            ..Default::default()
        };
        let input = NotificationConfigInput { desktop_enabled: true, ntfy_enabled: true, ntfy_topic: None };
        let merged = apply_input(existing, input);
        assert_eq!(merged.ntfy.topic, "old-topic");
        // Non-secret fields still come from the input.
        assert!(merged.desktop_enabled);
        assert!(merged.ntfy.enabled);
    }

    #[test]
    fn a_some_secret_in_the_input_overwrites_the_existing_stored_secret() {
        let existing = NotificationConfig {
            ntfy: NtfyChannelConfig { topic: "old-topic".to_string(), ..Default::default() },
            ..Default::default()
        };
        let input = NotificationConfigInput {
            desktop_enabled: false,
            ntfy_enabled: false,
            ntfy_topic: Some("new-topic".to_string()),
        };
        let merged = apply_input(existing, input);
        assert_eq!(merged.ntfy.topic, "new-topic");
    }

    // ---- config storage round-trip (app_settings) -----------------------

    #[test]
    fn a_freshly_created_database_has_no_notification_config_yet() {
        let conn = test_conn();
        let config = load_notification_config(&conn).unwrap();
        assert!(!config.desktop_enabled);
        assert!(!config.ntfy.enabled);
    }

    #[test]
    fn saving_then_loading_the_config_round_trips_every_field() {
        let conn = test_conn();
        let config = NotificationConfig {
            desktop_enabled: true,
            ntfy: NtfyChannelConfig { enabled: true, topic: "tiqr-marko-alerts".to_string() },
        };
        save_notification_config(&conn, &config).unwrap();
        let loaded = load_notification_config(&conn).unwrap();
        assert_eq!(loaded.desktop_enabled, config.desktop_enabled);
        assert_eq!(loaded.ntfy.topic, config.ntfy.topic);
    }

    #[test]
    fn a_config_stored_by_an_older_shape_with_leftover_email_and_pushover_fields_still_loads() {
        // 2.0.77 dropped the `email` field NotificationConfig used to
        // carry; 2.0.78 dropped `pushover` (replaced by `ntfy`).
        // `#[serde(default)]` must tolerate both leftover keys rather than
        // treating an existing install's stored config as corrupt.
        let conn = test_conn();
        let old_shape = r#"{"desktopEnabled":true,"email":{"enabled":true,"smtpHost":"smtp.example.com"},"pushover":{"enabled":true,"userKey":"uk"},"ntfy":{"enabled":true,"topic":"my-topic"}}"#;
        set_setting(&conn, NOTIFICATION_CONFIG_KEY, old_shape).unwrap();
        let loaded = load_notification_config(&conn).unwrap();
        assert!(loaded.desktop_enabled);
        assert!(loaded.ntfy.enabled);
        assert_eq!(loaded.ntfy.topic, "my-topic");
    }

    #[test]
    fn a_corrupt_stored_blob_is_a_real_error_not_silently_reset_to_defaults() {
        let conn = test_conn();
        set_setting(&conn, NOTIFICATION_CONFIG_KEY, "not valid json at all").unwrap();
        assert!(load_notification_config(&conn).is_err());
    }

    #[test]
    fn get_and_set_notification_config_round_trip_through_the_real_commands_shape() {
        // Exercises the same load -> merge -> save -> load path the real
        // get_notification_status/set_notification_config commands use,
        // just against the plain Connection directly (no Tauri State needed
        // here, same "impl function" testability convention as everywhere
        // else in this codebase).
        let conn = test_conn();
        let input = NotificationConfigInput {
            desktop_enabled: true,
            ntfy_enabled: true,
            ntfy_topic: Some("secret-topic".to_string()),
        };
        let existing = load_notification_config(&conn).unwrap();
        let merged = apply_input(existing, input);
        save_notification_config(&conn, &merged).unwrap();

        let reloaded = load_notification_config(&conn).unwrap();
        let status = status_from_config(&reloaded);
        assert!(status.desktop_enabled);
        assert!(status.ntfy_topic_set);
    }

    // ---- notification_log dedup (013_notifications.sql) -----------------

    #[test]
    fn nothing_is_marked_sent_on_a_fresh_database() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        assert!(already_sent_today(&conn, today).unwrap().is_empty());
    }

    #[test]
    fn marking_a_category_sent_makes_it_show_up_as_already_sent_today() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        mark_sent(&conn, NotificationCategory::UnpaidOrders, today).unwrap();
        let sent = already_sent_today(&conn, today).unwrap();
        assert!(sent.contains(NotificationCategory::UnpaidOrders.db_key()));
        assert!(!sent.contains(NotificationCategory::PendingSales.db_key()));
    }

    #[test]
    fn marking_the_same_category_sent_twice_the_same_day_does_not_error() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        mark_sent(&conn, NotificationCategory::UnpaidOrders, today).unwrap();
        // The UNIQUE(category, sent_on) constraint would reject a plain
        // INSERT here - INSERT OR IGNORE must swallow that as a normal,
        // silent no-op, not surface it as an AppError.
        mark_sent(&conn, NotificationCategory::UnpaidOrders, today).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_log WHERE category = 'unpaid_orders'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "must still be exactly one row, not two");
    }

    #[test]
    fn a_category_sent_on_a_different_day_is_not_already_sent_today() {
        let conn = test_conn();
        let yesterday = NaiveDate::from_ymd_opt(2026, 8, 25).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        mark_sent(&conn, NotificationCategory::UnpaidOrders, yesterday).unwrap();
        assert!(already_sent_today(&conn, today).unwrap().is_empty());
    }

    // ---- send_ntfy ---------------------------------------------------------

    #[test]
    fn send_ntfy_fails_cleanly_with_no_topic_set() {
        let cfg = NtfyChannelConfig { enabled: true, topic: String::new() };
        let err = send_ntfy(&cfg, "title", "body").unwrap_err();
        assert!(err.to_string().contains("no ntfy topic is set"), "{err}");
    }

    #[test]
    fn send_ntfy_fails_cleanly_with_a_blank_topic() {
        let cfg = NtfyChannelConfig { enabled: true, topic: "   ".to_string() };
        let err = send_ntfy(&cfg, "title", "body").unwrap_err();
        assert!(err.to_string().contains("no ntfy topic is set"), "{err}");
    }
}
