//! Outbound notifications (2.0.76) - desktop, email, Pushover. marko's own
//! request: "chcem tiez nejake pushup notifikacie, najlepsie aj do mobilu,
//! no len tie najviac prioritne" (push notifications, ideally to mobile
//! too, but only the highest-priority ones).
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
//! Credentials (the SMTP password, the Pushover keys) are stored the same
//! way every other secret in this app already is - plain text in the
//! generic `app_settings` key/value table (see commands::settings and
//! google_auth.rs's own doc comment on why that's the accepted trust
//! boundary for this local, single-user desktop app). Never echoed back to
//! the frontend: `NotificationStatus` (models.rs) carries only `*_set: bool`
//! presence flags, precedented by `GoogleSignInStatus`.
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
//! fx.rs: the actual `.send()`/HTTP calls stay thin and deliberately
//! untested (this sandbox can reach neither `api.pushover.net` nor any real
//! SMTP host - see fx.rs's own doc comment on the same restriction),
//! everything else (which categories are due, what a notification says, how
//! config merges) is a small pure function with full unit-test coverage.

use crate::commands::dashboard::compute_dashboard_alerts;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    DashboardAlerts, EmailChannelConfig, NotificationConfig, NotificationConfigInput,
    NotificationStatus, NotificationTestResult, PushoverChannelConfig,
};
use chrono::{Local, NaiveDate};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
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
/// never the raw secret values, only whether one is currently stored.
fn status_from_config(config: &NotificationConfig) -> NotificationStatus {
    NotificationStatus {
        desktop_enabled: config.desktop_enabled,
        email_enabled: config.email.enabled,
        email_smtp_host: config.email.smtp_host.clone(),
        email_smtp_port: config.email.smtp_port,
        email_smtp_username: config.email.smtp_username.clone(),
        email_smtp_password_set: !config.email.smtp_password.is_empty(),
        email_from_address: config.email.from_address.clone(),
        email_to_address: config.email.to_address.clone(),
        pushover_enabled: config.pushover.enabled,
        pushover_user_key_set: !config.pushover.user_key.is_empty(),
        pushover_api_token_set: !config.pushover.api_token.is_empty(),
    }
}

/// Merges a `NotificationConfigInput` (submitted by Settings) onto the
/// existing stored config. Every non-secret field comes from the input
/// unconditionally (Settings always submits its whole form); every secret
/// field is `Option<String>` and only overwrites when `Some` - `None` means
/// the user left that field blank, i.e. keep whatever is already stored.
fn apply_input(existing: NotificationConfig, input: NotificationConfigInput) -> NotificationConfig {
    NotificationConfig {
        desktop_enabled: input.desktop_enabled,
        email: EmailChannelConfig {
            enabled: input.email_enabled,
            smtp_host: input.email_smtp_host,
            smtp_port: input.email_smtp_port,
            smtp_username: input.email_smtp_username,
            smtp_password: input.email_smtp_password.unwrap_or(existing.email.smtp_password),
            from_address: input.email_from_address,
            to_address: input.email_to_address,
        },
        pushover: PushoverChannelConfig {
            enabled: input.pushover_enabled,
            user_key: input.pushover_user_key.unwrap_or(existing.pushover.user_key),
            api_token: input.pushover_api_token.unwrap_or(existing.pushover.api_token),
        },
    }
}

/// Pure - turns a failed Pushover HTTP response into a human-readable
/// message. Same "describe_rejected_request" convention fx.rs already
/// established for a different external API: takes a plain `StatusCode` +
/// body string (never a real `reqwest::blocking::Response`), so it's
/// testable without a live network call.
fn describe_pushover_error(status: reqwest::StatusCode, body: &str) -> String {
    // Pushover's own error shape on failure: {"status":0,"errors":["..."]}
    let messages: Option<Vec<String>> = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("errors").cloned())
        .and_then(|errs| serde_json::from_value(errs).ok());
    match messages {
        Some(msgs) if !msgs.is_empty() => format!("Pushover rejected the request ({status}): {}", msgs.join(", ")),
        _ => format!("Pushover rejected the request ({status}): {body}"),
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

fn send_pushover(cfg: &PushoverChannelConfig, title: &str, body: &str) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://api.pushover.net/1/messages.json")
        .form(&[
            ("token", cfg.api_token.as_str()),
            ("user", cfg.user_key.as_str()),
            ("title", title),
            ("message", body),
        ])
        .send()
        .map_err(|e| AppError::External(format!("could not reach Pushover: {e}")))?;

    let status = resp.status();
    let text = resp
        .text()
        .map_err(|e| AppError::External(format!("could not read Pushover's response: {e}")))?;
    if !status.is_success() {
        return Err(AppError::External(describe_pushover_error(status, &text)));
    }
    Ok(())
}

fn send_email(cfg: &EmailChannelConfig, title: &str, body: &str) -> AppResult<()> {
    let email = Message::builder()
        .from(
            cfg.from_address
                .parse()
                .map_err(|e| AppError::Validation(format!("invalid 'from' email address: {e}")))?,
        )
        .to(cfg
            .to_address
            .parse()
            .map_err(|e| AppError::Validation(format!("invalid 'to' email address: {e}")))?)
        .subject(title)
        .body(body.to_string())
        .map_err(|e| AppError::Other(format!("failed to build the email: {e}")))?;

    let mailer = SmtpTransport::relay(&cfg.smtp_host)
        .map_err(|e| AppError::External(format!("could not configure the SMTP connection: {e}")))?
        .port(cfg.smtp_port)
        .credentials(Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone()))
        .build();

    mailer
        .send(&email)
        .map_err(|e| AppError::External(format!("could not send the email: {e}")))?;
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
/// comment for why the other two test commands below can't).
#[tauri::command]
pub fn test_desktop_notification(app: tauri::AppHandle) -> AppResult<NotificationTestResult> {
    Ok(match send_desktop_notification(&app, "TIQR Manager", "This is a test notification.") {
        Ok(()) => NotificationTestResult { success: true, message: "Desktop notification sent.".to_string() },
        Err(e) => NotificationTestResult { success: false, message: e.to_string() },
    })
}

#[tauri::command]
pub async fn test_email_notification(state: State<'_, AppState>) -> AppResult<NotificationTestResult> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_notification_config(&conn)?.email
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        send_email(&cfg, "TIQR Manager - test", "This is a test email from TIQR Manager.")
    })
    .await;
    Ok(match result {
        Ok(Ok(())) => NotificationTestResult { success: true, message: "Test email sent.".to_string() },
        Ok(Err(e)) => NotificationTestResult { success: false, message: e.to_string() },
        Err(e) => NotificationTestResult {
            success: false,
            message: format!("the test-email task did not complete cleanly: {e}"),
        },
    })
}

#[tauri::command]
pub async fn test_pushover_notification(state: State<'_, AppState>) -> AppResult<NotificationTestResult> {
    let cfg = {
        let conn = state.db.lock().unwrap();
        load_notification_config(&conn)?.pushover
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        send_pushover(&cfg, "TIQR Manager", "This is a test Pushover notification.")
    })
    .await;
    Ok(match result {
        Ok(Ok(())) => NotificationTestResult { success: true, message: "Test Pushover notification sent.".to_string() },
        Ok(Err(e)) => NotificationTestResult { success: false, message: e.to_string() },
        Err(e) => NotificationTestResult {
            success: false,
            message: format!("the test-pushover task did not complete cleanly: {e}"),
        },
    })
}

/// The periodic check (Layout.tsx calls this every 30 minutes). `async fn` +
/// `spawn_blocking` for email/Pushover - see module doc comment. Silent on
/// both success and failure toward the caller (Layout.tsx's own `.catch(()
/// => {})`) - same "no new alert noise" philosophy `DashboardAlerts` itself
/// already follows; a single channel failing never stops the others from
/// being tried, and never fails the whole run.
#[tauri::command]
pub async fn check_and_send_notifications(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let config = {
        let conn = state.db.lock().unwrap();
        load_notification_config(&conn)?
    };
    if !config.desktop_enabled && !config.email.enabled && !config.pushover.enabled {
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
        if config.email.enabled {
            let cfg = config.email.clone();
            let (t, b) = (title.clone(), body.clone());
            let sent = tauri::async_runtime::spawn_blocking(move || send_email(&cfg, &t, &b)).await;
            if matches!(sent, Ok(Ok(()))) {
                any_success = true;
            }
        }
        if config.pushover.enabled {
            let cfg = config.pushover.clone();
            let (t, b) = (title.clone(), body.clone());
            let sent = tauri::async_runtime::spawn_blocking(move || send_pushover(&cfg, &t, &b)).await;
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
    fn status_never_carries_the_actual_secret_values() {
        let config = NotificationConfig {
            desktop_enabled: true,
            email: EmailChannelConfig {
                enabled: true,
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: 587,
                smtp_username: "marko@example.com".to_string(),
                smtp_password: "super-secret-password".to_string(),
                from_address: "marko@example.com".to_string(),
                to_address: "marko@example.com".to_string(),
            },
            pushover: PushoverChannelConfig {
                enabled: true,
                user_key: "u-key-value".to_string(),
                api_token: "a-token-value".to_string(),
            },
        };
        let status = status_from_config(&config);
        assert!(status.email_smtp_password_set);
        assert!(status.pushover_user_key_set);
        assert!(status.pushover_api_token_set);
        // The debug representation of the status must never contain any of
        // the actual secret text - only booleans about its presence.
        let dump = format!("{status:?}");
        assert!(!dump.contains("super-secret-password"));
        assert!(!dump.contains("u-key-value"));
        assert!(!dump.contains("a-token-value"));
    }

    #[test]
    fn a_none_secret_in_the_input_keeps_the_existing_stored_secret() {
        let existing = NotificationConfig {
            email: EmailChannelConfig { smtp_password: "old-password".to_string(), ..Default::default() },
            pushover: PushoverChannelConfig {
                user_key: "old-user-key".to_string(),
                api_token: "old-api-token".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let input = NotificationConfigInput {
            desktop_enabled: true,
            email_enabled: true,
            email_smtp_host: "smtp.example.com".to_string(),
            email_smtp_port: 587,
            email_smtp_username: "marko".to_string(),
            email_smtp_password: None,
            email_from_address: "marko@example.com".to_string(),
            email_to_address: "marko@example.com".to_string(),
            pushover_enabled: true,
            pushover_user_key: None,
            pushover_api_token: None,
        };
        let merged = apply_input(existing, input);
        assert_eq!(merged.email.smtp_password, "old-password");
        assert_eq!(merged.pushover.user_key, "old-user-key");
        assert_eq!(merged.pushover.api_token, "old-api-token");
        // Non-secret fields still come from the input.
        assert_eq!(merged.email.smtp_host, "smtp.example.com");
        assert!(merged.desktop_enabled);
    }

    #[test]
    fn a_some_secret_in_the_input_overwrites_the_existing_stored_secret() {
        let existing = NotificationConfig {
            email: EmailChannelConfig { smtp_password: "old-password".to_string(), ..Default::default() },
            ..Default::default()
        };
        let input = NotificationConfigInput {
            desktop_enabled: false,
            email_enabled: false,
            email_smtp_host: String::new(),
            email_smtp_port: 0,
            email_smtp_username: String::new(),
            email_smtp_password: Some("new-password".to_string()),
            email_from_address: String::new(),
            email_to_address: String::new(),
            pushover_enabled: false,
            pushover_user_key: None,
            pushover_api_token: None,
        };
        let merged = apply_input(existing, input);
        assert_eq!(merged.email.smtp_password, "new-password");
    }

    // ---- config storage round-trip (app_settings) -----------------------

    #[test]
    fn a_freshly_created_database_has_no_notification_config_yet() {
        let conn = test_conn();
        let config = load_notification_config(&conn).unwrap();
        assert!(!config.desktop_enabled);
        assert!(!config.email.enabled);
        assert!(!config.pushover.enabled);
    }

    #[test]
    fn saving_then_loading_the_config_round_trips_every_field() {
        let conn = test_conn();
        let config = NotificationConfig {
            desktop_enabled: true,
            email: EmailChannelConfig {
                enabled: true,
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: 465,
                smtp_username: "marko".to_string(),
                smtp_password: "p@ss".to_string(),
                from_address: "a@example.com".to_string(),
                to_address: "b@example.com".to_string(),
            },
            pushover: PushoverChannelConfig {
                enabled: true,
                user_key: "uk".to_string(),
                api_token: "at".to_string(),
            },
        };
        save_notification_config(&conn, &config).unwrap();
        let loaded = load_notification_config(&conn).unwrap();
        assert_eq!(loaded.desktop_enabled, config.desktop_enabled);
        assert_eq!(loaded.email.smtp_host, config.email.smtp_host);
        assert_eq!(loaded.email.smtp_port, config.email.smtp_port);
        assert_eq!(loaded.pushover.user_key, config.pushover.user_key);
        assert_eq!(loaded.pushover.api_token, config.pushover.api_token);
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
            email_enabled: false,
            email_smtp_host: "smtp.example.com".to_string(),
            email_smtp_port: 587,
            email_smtp_username: String::new(),
            email_smtp_password: Some("secret".to_string()),
            email_from_address: String::new(),
            email_to_address: String::new(),
            pushover_enabled: false,
            pushover_user_key: None,
            pushover_api_token: None,
        };
        let existing = load_notification_config(&conn).unwrap();
        let merged = apply_input(existing, input);
        save_notification_config(&conn, &merged).unwrap();

        let reloaded = load_notification_config(&conn).unwrap();
        let status = status_from_config(&reloaded);
        assert!(status.desktop_enabled);
        assert!(status.email_smtp_password_set);
        assert_eq!(status.email_smtp_host, "smtp.example.com");
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

    // ---- describe_pushover_error -----------------------------------------

    #[test]
    fn describe_pushover_error_extracts_the_real_error_list() {
        let body = r#"{"user":"invalid","errors":["user identifier is invalid"],"status":0,"request":"abc"}"#;
        let msg = describe_pushover_error(reqwest::StatusCode::from_u16(400).unwrap(), body);
        assert!(msg.contains("user identifier is invalid"));
    }

    #[test]
    fn describe_pushover_error_falls_back_gracefully_on_a_non_json_body() {
        let msg = describe_pushover_error(reqwest::StatusCode::from_u16(500).unwrap(), "internal server error");
        assert!(msg.contains("500"));
        assert!(msg.contains("internal server error"));
    }
}
