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
    DashboardAlerts, DiscordChannelConfig, DiscordTestResult, NotificationConfig,
    NotificationConfigInput, NotificationStatus, NotificationTestResult, NtfyChannelConfig,
};
use chrono::{Local, NaiveDate, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
        discord_enabled: config.discord.enabled,
        discord_sales_set: !config.discord.sales.trim().is_empty(),
        discord_finance_set: !config.discord.finance.trim().is_empty(),
        discord_inventory_set: !config.discord.inventory.trim().is_empty(),
        discord_mail_set: !config.discord.mail.trim().is_empty(),
        discord_auto: config.discord.auto.clone(),
        discord_mail_options: config.discord.mail_options.clone(),
        discord_sale_options: config.discord.sale_options.clone(),
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
        // 2.100.0: `unwrap_or(existing)` on every flag - an omitted field
        // keeps what is stored. See NotificationConfigInput's doc comment
        // for the bug this fixes.
        desktop_enabled: input.desktop_enabled.unwrap_or(existing.desktop_enabled),
        ntfy: NtfyChannelConfig {
            enabled: input.ntfy_enabled.unwrap_or(existing.ntfy.enabled),
            topic: input.ntfy_topic.unwrap_or(existing.ntfy.topic),
        },
        discord: DiscordChannelConfig {
            enabled: input.discord_enabled.unwrap_or(existing.discord.enabled),
            sales: input.discord_sales.unwrap_or(existing.discord.sales),
            finance: input.discord_finance.unwrap_or(existing.discord.finance),
            inventory: input.discord_inventory.unwrap_or(existing.discord.inventory),
            mail: input.discord_mail.unwrap_or(existing.discord.mail),
            auto: input.discord_auto.unwrap_or(existing.discord.auto),
            mail_options: input.discord_mail_options.unwrap_or(existing.discord.mail_options),
            sale_options: input.discord_sale_options.unwrap_or(existing.discord.sale_options),
        },
    }
}

// ---------------------------------------------------------------------------
// Discord webhooks (2.99.0).
//
// There was no Discord integration in this app before this - the only prior
// mention anywhere was a comment saying Discord SIGN-IN had been deferred
// (firebase.ts). So this is the whole channel, not a button added to one.
//
// Three things about the shape of it are deliberate.
//
// 1. The frontend never sees a webhook URL and never sends one. It sends a
//    DESTINATION NAME ("sales"), and this module looks the URL up. A command
//    that accepted a URL from the frontend would be a general-purpose HTTP
//    POST proxy wearing a Discord label.
//
// 2. The stored URL is validated against Discord's own hosts before every
//    send, not only when it is saved. Validating at save time alone trusts
//    the database; this app's database is a file on disk that syncs between
//    two machines, so the check belongs at the point of use.
//
// 3. A timeout is NOT a failure. Discord may have accepted the message and
//    the response may simply not have come back. That case is reported as
//    `unknown` and is never retried automatically - an automatic retry on an
//    ambiguous send is how one test becomes two messages.
// ---------------------------------------------------------------------------

/// The three destinations, as an enum rather than a string, so an unknown
/// value is rejected at the boundary instead of reaching a lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DiscordDestination {
    Sales,
    Finance,
    Inventory,
    /// 2.113.0: important mail that is none of the three.
    Mail,
}

impl DiscordDestination {
    /// Rejects anything that is not one of the three. This is the check that
    /// stops the command being pointed at an arbitrary key.
    pub(crate) fn parse(raw: &str) -> AppResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "sales" => Ok(DiscordDestination::Sales),
            "finance" => Ok(DiscordDestination::Finance),
            "inventory" => Ok(DiscordDestination::Inventory),
            "mail" => Ok(DiscordDestination::Mail),
            other => Err(AppError::Validation(format!(
                "\"{other}\" is not a Discord destination - expected sales, finance, inventory or mail"
            ))),
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            DiscordDestination::Sales => "sales",
            DiscordDestination::Finance => "finance",
            DiscordDestination::Inventory => "inventory",
            DiscordDestination::Mail => "mail",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            DiscordDestination::Sales => "Sales",
            DiscordDestination::Finance => "Finance",
            DiscordDestination::Inventory => "Inventory",
            DiscordDestination::Mail => "Mail",
        }
    }

    pub(crate) fn url_in<'a>(self, cfg: &'a DiscordChannelConfig) -> &'a str {
        match self {
            DiscordDestination::Sales => &cfg.sales,
            DiscordDestination::Finance => &cfg.finance,
            DiscordDestination::Inventory => &cfg.inventory,
            DiscordDestination::Mail => &cfg.mail,
        }
    }
}

/// The only hosts this module will POST to.
///
/// Discord serves webhooks from `discord.com` and the older `discordapp.com`,
/// plus the `ptb.`/`canary.` release channels. Anything else is refused even
/// if it is sitting in the config - see note 2 in the module comment above.
const DISCORD_WEBHOOK_HOSTS: [&str; 4] = [
    "https://discord.com/api/webhooks/",
    "https://discordapp.com/api/webhooks/",
    "https://ptb.discord.com/api/webhooks/",
    "https://canary.discord.com/api/webhooks/",
];

/// `Ok` only for a URL that is unmistakably a Discord webhook. Pure, so the
/// tests can cover it without a network.
pub(crate) fn validate_webhook_url(url: &str) -> AppResult<()> {
    let url = url.trim();
    if url.is_empty() {
        return Err(AppError::Validation("no webhook is configured for this destination".to_string()));
    }
    if !DISCORD_WEBHOOK_HOSTS.iter().any(|h| url.starts_with(h)) {
        // The URL itself is NOT included in this message - it is a secret,
        // and an error string is one of the easiest places for one to leak.
        return Err(AppError::Validation(
            "the stored webhook is not a Discord webhook URL (it must start with https://discord.com/api/webhooks/)"
                .to_string(),
        ));
    }
    // A real webhook carries an id and a token after the prefix. Anything
    // shorter is a truncated paste, which is worth catching before a send.
    let tail = DISCORD_WEBHOOK_HOSTS
        .iter()
        .find_map(|h| url.strip_prefix(h))
        .unwrap_or("");
    if tail.len() < 20 || !tail.contains('/') {
        return Err(AppError::Validation(
            "the stored webhook URL looks incomplete - it should end with an id and a token".to_string(),
        ));
    }
    Ok(())
}

/// What one HTTP status from Discord MEANS, as a pure function.
///
/// Split out deliberately: it is the part of this feature with real branching
/// and it is the part worth testing exhaustively, and as a pure function it
/// can be tested without a network, without a mock server, and without a new
/// dependency. Returns `(status, error_code, message)`.
pub(crate) fn interpret_discord_status(code: u16) -> (&'static str, Option<&'static str>, String) {
    match code {
        200..=299 => ("delivered", None, "Discord accepted the message.".to_string()),
        400 => (
            "failed",
            Some("INVALID_PAYLOAD"),
            "Discord rejected the message as malformed. This is a bug in TIQR, not in your webhook.".to_string(),
        ),
        401 | 403 => (
            "failed",
            Some("WEBHOOK_UNAUTHORIZED"),
            "Discord refused the webhook's credentials. The webhook may have been regenerated - copy the URL again from the channel's settings.".to_string(),
        ),
        404 => (
            "failed",
            Some("WEBHOOK_NOT_FOUND"),
            "Discord says this webhook no longer exists. It was probably deleted in the channel's settings.".to_string(),
        ),
        429 => (
            "failed",
            Some("RATE_LIMITED"),
            "Discord is rate-limiting this webhook. Wait for the time shown and try again.".to_string(),
        ),
        500..=599 => (
            "failed",
            Some("DISCORD_UNAVAILABLE"),
            "Discord had a server error. This is temporary and nothing is wrong with your webhook.".to_string(),
        ),
        other => (
            "failed",
            Some("UNEXPECTED_STATUS"),
            format!("Discord answered with an unexpected status ({other})."),
        ),
    }
}

/// One id per attempt, embedded in the message that reaches the channel so a
/// row in the UI can be matched to a message in Discord. Built from the clock
/// plus a process counter rather than pulling in a uuid crate for it.
fn new_test_id() -> String {
    static COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut guard = COUNTER.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *guard = guard.wrapping_add(1);
    format!("{nanos:x}-{:x}", *guard)
}

/// Server-side cooldown, per destination.
///
/// The disabled button in the UI stops the obvious double-click; this stops
/// everything else, because the UI is not the trust boundary. In memory on
/// purpose - a cooldown that survived a restart would be stored state about
/// nothing, and restarting the app is not an abuse vector.
const TEST_COOLDOWN: Duration = Duration::from_secs(10);

/// The decision itself, split from the storage so it is testable without
/// touching the process-wide map (which every test would then share).
pub(crate) fn cooldown_from_previous(prev: Option<Instant>, now: Instant) -> Option<Duration> {
    let prev = prev?;
    let elapsed = now.saturating_duration_since(prev);
    if elapsed < TEST_COOLDOWN {
        Some(TEST_COOLDOWN - elapsed)
    } else {
        None
    }
}

fn cooldown_remaining(dest: DiscordDestination, now: Instant) -> Option<Duration> {
    static LAST: OnceLock<Mutex<HashMap<&'static str, Instant>>> = OnceLock::new();
    let mut guard = LAST.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    match cooldown_from_previous(guard.get(dest.key()).copied(), now) {
        Some(left) => Some(left),
        None => {
            // Only a permitted attempt resets the clock, so a rejected one
            // cannot push the window forward forever.
            guard.insert(dest.key(), now);
            None
        }
    }
}

/// The embed that reaches the channel. Carries the destination, the test id
/// and the timestamp, and NOTHING from the database - no sale, no order, no
/// figure. A test must not be able to leak business data into a chat channel.
///
/// 2.113.0: dressed like every other TIQR message (commands/discord.rs) -
/// the logo as the sender's picture, a top line, the brand in the footer -
/// so the first thing marko sees in a new channel is what the rest will
/// look like.
fn test_embed(dest: DiscordDestination, test_id: &str, tested_at: &str) -> serde_json::Value {
    use crate::commands::discord::{BRAND, LOGO_URL};
    serde_json::json!({
        "username": BRAND,
        "avatar_url": LOGO_URL,
        "allowed_mentions": { "parse": [] },
        "embeds": [{
            "author": { "name": "✅ Connected", "icon_url": LOGO_URL },
            "title": format!("{} channel connected", dest.label()),
            "description": "TIQR Manager can post here. This is a test notification - no records were changed.",
            "color": 0x8B5CF6,
            "fields": [
                { "name": "Destination", "value": dest.label(), "inline": true },
                { "name": "Status", "value": "Delivered", "inline": true },
                { "name": "Test ID", "value": test_id, "inline": true }
            ],
            "footer": { "text": BRAND, "icon_url": LOGO_URL },
            "timestamp": tested_at
        }]
    })
}

// ---------------------------------------------------------------------------
// Storage - the config blob and the per-day dedup log.
// ---------------------------------------------------------------------------

/// Absent key -> a disabled, empty config (nothing configured yet, same as
/// a brand-new install). A present-but-corrupt blob is a real, surfaced
/// error, never silently discarded - same convention `sheets_sync::
/// load_connection` already established for its own stored JSON blob.
pub(crate) fn load_notification_config(conn: &Connection) -> AppResult<NotificationConfig> {
    match get_setting(conn, NOTIFICATION_CONFIG_KEY)? {
        None => Ok(NotificationConfig::default()),
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| AppError::Other(format!("stored notification settings are corrupt: {e}"))),
    }
}

pub(crate) fn save_notification_config(conn: &Connection, config: &NotificationConfig) -> AppResult<()> {
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

/// 2.27.0: `pub(crate)` so the Price Checker's automatic scan run can tell
/// marko it has finished while he is in another window - that run exists
/// precisely so he does not have to sit and watch it. Nothing else about this
/// module's config or category gating is involved: the run's notification is
/// the direct result of a button he pressed, not a monitored condition.
pub(crate) fn send_desktop_notification(app: &tauri::AppHandle, title: &str, body: &str) -> AppResult<()> {
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

/// The blocking half of a test: one POST, one interpretation, no retries.
///
/// Returns `(status, error_code, message, retry_after_seconds)` so the caller
/// can build the result without this function knowing anything about Tauri.
fn post_discord_test(
    url: &str,
    dest: DiscordDestination,
    test_id: &str,
    tested_at: &str,
) -> (&'static str, Option<&'static str>, String, Option<u64>) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                "failed",
                Some("CLIENT_ERROR"),
                format!("could not build an HTTP client: {e}"),
                None,
            )
        }
    };

    let resp = client.post(url).json(&test_embed(dest, test_id, tested_at)).send();

    match resp {
        Ok(r) => {
            let code = r.status().as_u16();
            // Read the header, never the body. Discord also puts `retry_after`
            // in the JSON body, but a body is the one place a response could
            // carry something we do not want to surface, so it is not read.
            let retry_after = r
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|v| v.ceil() as u64);
            let (status, code_str, message) = interpret_discord_status(code);
            (status, code_str, message, if code == 429 { retry_after } else { None })
        }
        Err(e) if e.is_timeout() => (
            // NOT a failure. Discord may have taken the message.
            "unknown",
            Some("TIMEOUT"),
            "The request timed out before Discord answered. The message may or may not have arrived - check the channel before testing again."
                .to_string(),
            None,
        ),
        Err(e) if e.is_connect() || e.is_request() => (
            "unknown",
            Some("NETWORK"),
            "Could not reach Discord. Check your connection - the message may not have been sent at all."
                .to_string(),
            None,
        ),
        Err(_) => (
            "unknown",
            Some("UNKNOWN"),
            "The request did not complete and the outcome could not be determined.".to_string(),
            None,
        ),
    }
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
    // 2.113.0: a pasted webhook is checked as it is saved, so a typo is said
    // now rather than at the first message that never arrives. An empty
    // string removes the webhook and needs no check.
    for url in [&input.discord_sales, &input.discord_finance, &input.discord_inventory, &input.discord_mail]
        .into_iter()
        .flatten()
        .filter(|u| !u.trim().is_empty())
    {
        validate_webhook_url(url)?;
    }
    let conn = state.db.lock().unwrap();
    let existing = load_notification_config(&conn)?;
    let merged = apply_input(existing, input);
    save_notification_config(&conn, &merged)?;
    // 2.110.0: the same webhooks on the other computer (shared_settings.rs).
    crate::commands::shared_settings::touch(&conn, &crate::commands::shared_settings::NOTIFICATIONS)?;
    Ok(status_from_config(&merged))
}

/// Local OS call only, no network - stays plain `fn` (see module doc
/// comment for why the ntfy test command below can't).
#[tauri::command(async)]
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

/// 2.99.0: send one test message to ONE configured Discord destination.
///
/// Takes a destination NAME, never a URL - see the module comment above for
/// why. This is the only Discord-sending entry point the frontend has.
#[tauri::command]
pub async fn test_discord_webhook(
    state: State<'_, AppState>,
    destination: String,
) -> AppResult<DiscordTestResult> {
    let dest = DiscordDestination::parse(&destination)?;
    // Milliseconds and `Z`: the one shape Discord's embed timestamp takes.
    let tested_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let test_id = new_test_id();

    // Throttle BEFORE the lookup, so a flood of requests does not even read
    // the config. `?` is not used for this: a cooldown is an ordinary
    // outcome the UI shows in its own right, not an error.
    if let Some(left) = cooldown_remaining(dest, Instant::now()) {
        let secs = left.as_secs().max(1);
        return Ok(DiscordTestResult {
            success: false,
            destination: dest.key().to_string(),
            status: "failed".to_string(),
            error_code: Some("COOLDOWN".to_string()),
            message: format!("Wait {secs}s before testing {} again.", dest.label()),
            tested_at,
            test_id,
            retry_after_seconds: Some(secs),
        });
    }

    let url = {
        let conn = state.db.lock().unwrap();
        let cfg = load_notification_config(&conn)?.discord;
        dest.url_in(&cfg).trim().to_string()
    };

    // Validated at the point of use, not only at save time.
    if let Err(e) = validate_webhook_url(&url) {
        return Ok(DiscordTestResult {
            success: false,
            destination: dest.key().to_string(),
            status: "failed".to_string(),
            error_code: Some("WEBHOOK_NOT_CONFIGURED".to_string()),
            message: e.to_string(),
            tested_at,
            test_id,
            retry_after_seconds: None,
        });
    }

    let (dest_key, label) = (dest.key().to_string(), dest.label());
    let (id_for_task, at_for_task) = (test_id.clone(), tested_at.clone());
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        post_discord_test(&url, dest, &id_for_task, &at_for_task)
    })
    .await;

    let (status, code, message, retry_after) = match outcome {
        Ok(v) => v,
        // The task itself failed to run - that says nothing about whether
        // Discord got the message, so it is `unknown` like a timeout.
        Err(_) => (
            "unknown",
            Some("TASK_FAILED"),
            "The test did not finish cleanly, so delivery could not be confirmed.".to_string(),
            None,
        ),
    };

    Ok(DiscordTestResult {
        success: status == "delivered",
        destination: dest_key,
        status: status.to_string(),
        error_code: code.map(|c| c.to_string()),
        message: if status == "delivered" {
            format!("Test message delivered to {label}.")
        } else {
            message
        },
        tested_at,
        test_id,
        retry_after_seconds: retry_after,
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
            discord: DiscordChannelConfig::default(),
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
        let input = NotificationConfigInput {
            desktop_enabled: Some(true),
            ntfy_enabled: Some(true),
            ntfy_topic: None,
            discord_enabled: None,
            discord_sales: None,
            discord_finance: None,
            discord_inventory: None,
            discord_mail: None,
            discord_auto: None,
            discord_mail_options: None,
            discord_sale_options: None,
        };
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
            desktop_enabled: Some(false),
            ntfy_enabled: Some(false),
            ntfy_topic: Some("new-topic".to_string()),
            discord_enabled: None,
            discord_sales: None,
            discord_finance: None,
            discord_inventory: None,
            discord_mail: None,
            discord_auto: None,
            discord_mail_options: None,
            discord_sale_options: None,
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
            discord: DiscordChannelConfig::default(),
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
            desktop_enabled: Some(true),
            ntfy_enabled: Some(true),
            ntfy_topic: Some("secret-topic".to_string()),
            discord_enabled: None,
            discord_sales: None,
            discord_finance: None,
            discord_inventory: None,
            discord_mail: None,
            discord_auto: None,
            discord_mail_options: None,
            discord_sale_options: None,
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

    // =======================================================================
    // 2.99.0 - Discord test webhook.
    //
    // NOTE on `Duration`: this module imports `chrono::Duration`, which
    // shadows `std::time::Duration` from `super::*`. The cooldown tests
    // below therefore spell out `std::time::Duration` in full.
    //
    // No test here opens a socket. Every branch that depends on Discord's
    // answer is reached through `interpret_discord_status`, which takes a
    // status code and returns the verdict - so the whole decision table is
    // covered with no mock server and no new dependency.
    // =======================================================================

    const REAL_SHAPED_URL: &str =
        "https://discord.com/api/webhooks/123456789012345678/aBcDeFgHiJkLmNoPqRsTuVwXyZ-0123456789";

    // ---- DiscordDestination::parse ----------------------------------------

    #[test]
    fn parse_accepts_the_three_destinations() {
        assert_eq!(DiscordDestination::parse("sales").unwrap(), DiscordDestination::Sales);
        assert_eq!(DiscordDestination::parse("finance").unwrap(), DiscordDestination::Finance);
        assert_eq!(DiscordDestination::parse("inventory").unwrap(), DiscordDestination::Inventory);
    }

    #[test]
    fn parse_is_forgiving_about_case_and_padding() {
        assert_eq!(DiscordDestination::parse("  Sales  ").unwrap(), DiscordDestination::Sales);
        assert_eq!(DiscordDestination::parse("INVENTORY").unwrap(), DiscordDestination::Inventory);
    }

    #[test]
    fn parse_rejects_anything_else() {
        for bad in ["", "   ", "orders", "pulls", "sales2", "../sales", "SALES;finance"] {
            assert!(
                DiscordDestination::parse(bad).is_err(),
                "{bad:?} must not parse as a destination"
            );
        }
    }

    /// The point of the enum: a URL can never arrive from the frontend as a
    /// destination, so the command cannot be aimed at an arbitrary address.
    #[test]
    fn parse_rejects_a_url_so_the_command_cannot_be_used_as_a_proxy() {
        assert!(DiscordDestination::parse(REAL_SHAPED_URL).is_err());
        assert!(DiscordDestination::parse("https://example.com/collect").is_err());
        assert!(DiscordDestination::parse("http://127.0.0.1:8080/").is_err());
    }

    // ---- validate_webhook_url ---------------------------------------------

    #[test]
    fn a_real_shaped_discord_webhook_validates() {
        assert!(validate_webhook_url(REAL_SHAPED_URL).is_ok());
        assert!(validate_webhook_url(&format!("  {REAL_SHAPED_URL}  ")).is_ok());
    }

    #[test]
    fn every_discord_host_validates() {
        for host in DISCORD_WEBHOOK_HOSTS {
            let url = format!("{host}123456789012345678/aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789");
            assert!(validate_webhook_url(&url).is_ok(), "{host} must be accepted");
        }
    }

    #[test]
    fn an_empty_webhook_is_reported_as_not_configured() {
        let err = validate_webhook_url("").unwrap_err();
        assert!(err.to_string().contains("no webhook is configured"), "{err}");
        let err = validate_webhook_url("    ").unwrap_err();
        assert!(err.to_string().contains("no webhook is configured"), "{err}");
    }

    /// This is the proxy guard at the point of sending: even if a non-Discord
    /// URL got into the config, nothing is ever POSTed to it.
    #[test]
    fn a_non_discord_host_is_refused() {
        for bad in [
            "https://example.com/api/webhooks/1/tok",
            "http://discord.com/api/webhooks/1/tok",            // not https
            "https://discord.com.evil.test/api/webhooks/1/tok", // prefix trick
            "https://evil.test/?u=https://discord.com/api/webhooks/1/tok",
            "https://discord.com/api/channels/1/messages",      // Discord, wrong API
            "http://127.0.0.1:9999/",
            "file:///etc/passwd",
        ] {
            assert!(validate_webhook_url(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn a_truncated_webhook_is_refused() {
        assert!(validate_webhook_url("https://discord.com/api/webhooks/").is_err());
        assert!(validate_webhook_url("https://discord.com/api/webhooks/123").is_err());
        // Long enough, but no `/` - so no token half.
        assert!(validate_webhook_url(
            "https://discord.com/api/webhooks/1234567890123456789012345"
        )
        .is_err());
    }

    /// The whole reason these messages are written by hand: an error string
    /// is one of the easiest places for a secret to escape to.
    #[test]
    fn a_validation_error_never_quotes_the_url_or_its_token() {
        let secret_token = "aBcDeFgHiJkLmNoPqRsTuVwXyZ-0123456789";
        for bad in [
            format!("https://evil.test/api/webhooks/123456789012345678/{secret_token}"),
            format!("http://discord.com/api/webhooks/123456789012345678/{secret_token}"),
            "https://discord.com/api/webhooks/123".to_string(),
        ] {
            let msg = validate_webhook_url(&bad).unwrap_err().to_string();
            assert!(!msg.contains(secret_token), "token leaked into: {msg}");
            assert!(!msg.contains(&bad), "whole URL leaked into: {msg}");
            assert!(!msg.contains("evil.test"), "host leaked into: {msg}");
        }
    }

    // ---- interpret_discord_status -----------------------------------------

    #[test]
    fn a_2xx_is_delivered_and_carries_no_error_code() {
        for code in [200u16, 201, 204, 299] {
            let (status, err, _) = interpret_discord_status(code);
            assert_eq!(status, "delivered", "{code}");
            assert!(err.is_none(), "{code} must not carry an error code");
        }
    }

    #[test]
    fn every_documented_failure_maps_to_its_own_code() {
        let cases = [
            (400u16, "INVALID_PAYLOAD"),
            (401, "WEBHOOK_UNAUTHORIZED"),
            (403, "WEBHOOK_UNAUTHORIZED"),
            (404, "WEBHOOK_NOT_FOUND"),
            (429, "RATE_LIMITED"),
            (500, "DISCORD_UNAVAILABLE"),
            (502, "DISCORD_UNAVAILABLE"),
            (503, "DISCORD_UNAVAILABLE"),
        ];
        for (code, expected) in cases {
            let (status, err, msg) = interpret_discord_status(code);
            assert_eq!(status, "failed", "{code}");
            assert_eq!(err, Some(expected), "{code}");
            assert!(!msg.is_empty(), "{code} must carry a human message");
        }
    }

    #[test]
    fn an_undocumented_status_is_still_handled() {
        let (status, err, msg) = interpret_discord_status(418);
        assert_eq!(status, "failed");
        assert_eq!(err, Some("UNEXPECTED_STATUS"));
        assert!(msg.contains("418"), "{msg}");
    }

    /// Nothing may be reported as delivered except a 2xx. This is the
    /// "do not claim delivery without confirmation" rule, as a test.
    #[test]
    fn no_status_outside_2xx_is_ever_reported_as_delivered() {
        for code in (100u16..=599).chain([600, 999]) {
            let (status, _, _) = interpret_discord_status(code);
            if (200..=299).contains(&code) {
                assert_eq!(status, "delivered", "{code}");
            } else {
                assert_ne!(status, "delivered", "{code} must not count as delivered");
            }
        }
    }

    // ---- cooldown ---------------------------------------------------------

    #[test]
    fn a_first_attempt_has_no_cooldown() {
        assert!(cooldown_from_previous(None, Instant::now()).is_none());
    }

    #[test]
    fn a_second_attempt_inside_the_window_is_held_off() {
        let now = Instant::now();
        let prev = now - std::time::Duration::from_secs(3);
        let left = cooldown_from_previous(Some(prev), now).expect("must be held off");
        assert!(left.as_secs() >= 6 && left.as_secs() <= 7, "{left:?}");
    }

    #[test]
    fn an_attempt_after_the_window_is_allowed_again() {
        let now = Instant::now();
        assert!(cooldown_from_previous(Some(now - TEST_COOLDOWN), now).is_none());
        assert!(cooldown_from_previous(Some(now - std::time::Duration::from_secs(60)), now).is_none());
    }

    /// A clock that appears to go backwards must not produce a huge wait.
    #[test]
    fn a_future_timestamp_does_not_produce_a_nonsense_wait() {
        let now = Instant::now();
        let left = cooldown_from_previous(Some(now + std::time::Duration::from_secs(5)), now);
        assert!(left.is_some_and(|d| d <= TEST_COOLDOWN), "{left:?}");
    }

    /// The real wired-up function, through the process-wide map. Uses
    /// Inventory and is the only test that touches that destination.
    #[test]
    fn the_stored_cooldown_blocks_an_immediate_repeat() {
        let t0 = Instant::now();
        assert!(cooldown_remaining(DiscordDestination::Inventory, t0).is_none());
        assert!(cooldown_remaining(DiscordDestination::Inventory, t0).is_some());
        let later = t0 + TEST_COOLDOWN + std::time::Duration::from_secs(1);
        assert!(cooldown_remaining(DiscordDestination::Inventory, later).is_none());
    }

    // ---- test_embed -------------------------------------------------------

    #[test]
    fn the_test_embed_says_what_it_is() {
        let body = test_embed(DiscordDestination::Sales, "abc-1", "2026-10-17T10:00:00Z").to_string();
        assert!(body.contains("TIQR Manager"), "{body}");
        assert!(body.contains("Sales"), "{body}");
        assert!(body.contains("abc-1"), "{body}");
        assert!(body.contains("2026-10-17T10:00:00Z"), "{body}");
        assert!(
            body.contains("test notification"),
            "the message must say it is a test: {body}"
        );
    }

    /// The message that reaches a chat channel must carry nothing from this
    /// app's records. The embed is built from three arguments and constants,
    /// so this asserts what it is NOT allowed to look like.
    #[test]
    fn the_test_embed_carries_nothing_from_the_database() {
        let body = test_embed(DiscordDestination::Finance, "id-1", "2026-10-17T10:00:00Z")
            .to_string()
            .to_lowercase();
        for forbidden in [
            "eur", "usd", "gbp", "czk", // money
            "@",                        // any address
            "customer", "buyer", "order #", "invoice",
            "cents", "profit", "revenue", "payout", "balance",
            "ticket", "listing", "event", "venue", "batch",
            "token", "secret", "api key", "password", "cookie",
            "discord.com/api/webhooks",
        ] {
            assert!(
                !body.contains(forbidden),
                "{forbidden:?} must not appear in a test message: {body}"
            );
        }
    }

    /// Two tests a moment apart must be distinguishable in the channel.
    #[test]
    fn each_test_gets_its_own_id() {
        let a = new_test_id();
        let b = new_test_id();
        assert_ne!(a, b);
        assert!(!a.is_empty() && !b.is_empty());
    }

    // ---- status reporting -------------------------------------------------

    /// `NotificationStatus` is what the Settings screen reads. It must say
    /// WHETHER a webhook is set, never what it is.
    #[test]
    fn the_status_reports_only_whether_each_webhook_is_set() {
        let cfg = NotificationConfig {
            desktop_enabled: false,
            ntfy: NtfyChannelConfig::default(),
            discord: DiscordChannelConfig {
                enabled: true,
                sales: REAL_SHAPED_URL.to_string(),
                finance: String::new(),
                inventory: "   ".to_string(),
                mail: String::new(),
                auto: Default::default(),
                mail_options: Default::default(),
                sale_options: Default::default(),
            },
        };
        let status = status_from_config(&cfg);
        assert!(status.discord_enabled);
        assert!(status.discord_sales_set);
        assert!(!status.discord_finance_set, "an empty webhook is not set");
        assert!(!status.discord_inventory_set, "a blank webhook is not set");

        let serialised = serde_json::to_string(&status).unwrap();
        assert!(
            !serialised.contains("discord.com"),
            "the status sent to the frontend must not contain a URL: {serialised}"
        );
        assert!(
            !serialised.contains("aBcDeFgHiJkLmNoPqRsTuVwXyZ"),
            "the status sent to the frontend must not contain a token: {serialised}"
        );
    }

    // ---- apply_input ------------------------------------------------------

    fn cfg_with(sales: &str, finance: &str, inventory: &str) -> NotificationConfig {
        NotificationConfig {
            desktop_enabled: false,
            ntfy: NtfyChannelConfig::default(),
            discord: DiscordChannelConfig {
                enabled: false,
                sales: sales.to_string(),
                finance: finance.to_string(),
                inventory: inventory.to_string(),
                mail: String::new(),
                auto: Default::default(),
                mail_options: Default::default(),
                sale_options: Default::default(),
            },
        }
    }

    fn input_with(
        sales: Option<&str>,
        finance: Option<&str>,
        inventory: Option<&str>,
    ) -> NotificationConfigInput {
        // Mirrors what DiscordCard actually sends: the discord fields and
        // nothing else, so desktop/ntfy are untouched.
        NotificationConfigInput {
            desktop_enabled: None,
            ntfy_enabled: None,
            ntfy_topic: None,
            discord_enabled: Some(true),
            discord_sales: sales.map(str::to_string),
            discord_finance: finance.map(str::to_string),
            discord_inventory: inventory.map(str::to_string),
            discord_mail: None,
            discord_auto: None,
            discord_mail_options: None,
            discord_sale_options: None,
        }
    }

    /// THE REGRESSION TEST for what CI caught in 2.100.0.
    ///
    /// Two Settings cards write this one config. Saving from the Discord
    /// card must not switch desktop notifications or ntfy off, and saving
    /// from the Notifications card must not switch Discord off. While the
    /// flags were plain `bool` this was impossible to express - every save
    /// carried a value for every flag.
    #[test]
    fn saving_discord_settings_does_not_switch_desktop_or_ntfy_off() {
        let existing = NotificationConfig {
            desktop_enabled: true,
            ntfy: NtfyChannelConfig { enabled: true, topic: "keep-me".to_string() },
            discord: DiscordChannelConfig::default(),
        };
        let after = apply_input(existing, input_with(Some(REAL_SHAPED_URL), None, None));
        assert!(after.desktop_enabled, "desktop must survive a Discord save");
        assert!(after.ntfy.enabled, "ntfy must survive a Discord save");
        assert_eq!(after.ntfy.topic, "keep-me");
        assert!(after.discord.enabled);
    }

    #[test]
    fn saving_notification_settings_does_not_switch_discord_off() {
        let existing = NotificationConfig {
            desktop_enabled: false,
            ntfy: NtfyChannelConfig::default(),
            discord: DiscordChannelConfig {
                enabled: true,
                sales: REAL_SHAPED_URL.to_string(),
                finance: String::new(),
                inventory: String::new(),
                mail: String::new(),
                auto: Default::default(),
                mail_options: Default::default(),
                sale_options: Default::default(),
            },
        };
        // Exactly what NotificationsCard sends: its own three fields only.
        let input = NotificationConfigInput {
            desktop_enabled: Some(true),
            ntfy_enabled: Some(true),
            ntfy_topic: Some("new-topic".to_string()),
            discord_enabled: None,
            discord_sales: None,
            discord_finance: None,
            discord_inventory: None,
            discord_mail: None,
            discord_auto: None,
            discord_mail_options: None,
            discord_sale_options: None,
        };
        let after = apply_input(existing, input);
        assert!(after.discord.enabled, "Discord must survive a Notifications save");
        assert_eq!(after.discord.sales, REAL_SHAPED_URL, "the webhook must survive too");
        assert!(after.desktop_enabled);
        assert_eq!(after.ntfy.topic, "new-topic");
    }

    /// An input that says nothing at all changes nothing at all.
    #[test]
    fn an_entirely_empty_input_leaves_every_field_exactly_as_it_was() {
        let existing = NotificationConfig {
            desktop_enabled: true,
            ntfy: NtfyChannelConfig { enabled: true, topic: "topic".to_string() },
            discord: DiscordChannelConfig {
                enabled: true,
                sales: REAL_SHAPED_URL.to_string(),
                finance: String::new(),
                inventory: String::new(),
                mail: String::new(),
                auto: Default::default(),
                mail_options: Default::default(),
                sale_options: Default::default(),
            },
        };
        let after = apply_input(
            existing.clone(),
            NotificationConfigInput {
                desktop_enabled: None,
                ntfy_enabled: None,
                ntfy_topic: None,
                discord_enabled: None,
                discord_sales: None,
                discord_finance: None,
                discord_inventory: None,
                discord_mail: None,
                discord_auto: None,
                discord_mail_options: None,
                discord_sale_options: None,
            },
        );
        assert_eq!(after.desktop_enabled, existing.desktop_enabled);
        assert_eq!(after.ntfy.enabled, existing.ntfy.enabled);
        assert_eq!(after.ntfy.topic, existing.ntfy.topic);
        assert_eq!(after.discord.enabled, existing.discord.enabled);
        assert_eq!(after.discord.sales, existing.discord.sales);
    }

    /// `None` means "leave the stored webhook alone", so the Settings screen
    /// can save other fields without ever having to hold the secret.
    #[test]
    fn a_none_webhook_leaves_the_stored_one_untouched() {
        let before = cfg_with(REAL_SHAPED_URL, "", "");
        let after = apply_input(before, input_with(None, None, None));
        assert!(after.discord.enabled, "the flag must have been applied");
        assert_eq!(after.discord.sales, REAL_SHAPED_URL, "the webhook must survive");
    }

    #[test]
    fn a_supplied_webhook_replaces_the_stored_one_and_a_blank_clears_it() {
        let before = cfg_with(REAL_SHAPED_URL, "", "");
        let after = apply_input(before, input_with(Some(""), Some(REAL_SHAPED_URL), None));
        assert!(after.discord.sales.is_empty(), "an explicit blank must clear it");
        assert_eq!(after.discord.finance, REAL_SHAPED_URL);
    }
}