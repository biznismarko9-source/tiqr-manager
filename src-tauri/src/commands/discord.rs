//! 2.113.0 - Discord messages for what TIQR records.
//!
//! marko: "opravit webhooks, lebo trz sa nerobia" (fix the webhooks, nothing
//! arrives now), "pri kazdom evente, sale, finance atd vies kliknut na
//! vytvorit webhook aby sa ti poslal" (a button on every event, sale, finance
//! entry...), "aj na maily keby pride nieco dolezite ako predaj, nakup,
//! dolezita info aby nejak v skratke to zhrnula" (a short summary of an
//! important email) and "urobit ten webhook nejak pekny, nech reprezentujeme
//! tiqr ako top nastroj" (make it look like a top tool).
//!
//! ## Why nothing arrived
//!
//! Before this only a record Mail Intelligence created posted anything, and a
//! sale from mail always failed, so the Sales channel never heard a word. An
//! event, order, sale or finance entry saved in TIQR itself posted nothing at
//! all. A post that failed was dropped without a trace, and it ran on the
//! command's own thread while the database was locked.
//!
//! ## Now
//!
//! - **One place builds every message** (`post_for`), from the database, so
//!   a record saved by hand, one made from an email and the "Send to Discord"
//!   button look the same.
//! - **What posts by itself** is set per kind (`DiscordAutoConfig`, all on):
//!   a new event or order goes to Inventory, a sale to Sales, a finance entry
//!   to Finance, an important email to the channel its subject belongs to.
//!   Only a command marko pressed posts: records arriving from the other
//!   computer, a CSV import or a restore never do.
//! - **One sender thread** (`enqueue`) posts in order, waits out Discord's
//!   rate limit and tries a temporary failure (no connection, 429, a Discord
//!   server error) again, three attempts at most. A timeout is NOT tried
//!   again: Discord may already have the message (notifications.rs, note 3).
//! - **Every outcome is kept** (the last 20, in memory) and Settings shows
//!   them, so a webhook that stopped working says so.
//! - **What a message never carries**: a webhook URL, a one-time code, an
//!   email's body, a sender's full address. Text from records and emails is
//!   escaped, so a subject cannot become a link, a heading or a mention.
use crate::commands::notifications::{
    interpret_discord_status, load_notification_config, validate_webhook_url, DiscordDestination,
};
use crate::commands::mail_facts::MailFact;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{DiscordAutoConfig, DiscordChannelConfig, DiscordDelivery, DiscordSendResult};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::State;

/// The logo file in the repository itself (`docs/tiqr-logo.png` on `main`),
/// straight from GitHub. 2.113.1: it used to point at GitHub Pages, which is
/// not switched on for this repository - every `github.io` address answers
/// 404 - so each message showed a broken picture where the logo belongs
/// (marko: "ked pridu webhooks tak tam nieje tiqr logo"). This address
/// answers with the PNG itself, checked when it was changed. It needs only
/// the file on `main` and the repository being public, which the updater
/// already depends on.
pub(crate) const LOGO_URL: &str = "https://raw.githubusercontent.com/biznismarko9-source/tiqr-manager/main/docs/tiqr-logo.png";
pub(crate) const BRAND: &str = "TIQR Manager";

// One colour per kind, so a channel reads at a glance.
const GREEN: u32 = 0x22C55E;
const BLUE: u32 = 0x3B82F6;
const PURPLE: u32 = 0x8B5CF6;
const AMBER: u32 = 0xF59E0B;
const RED: u32 = 0xEF4444;
const MAGENTA: u32 = 0xC026D3;

/// What a message is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Subject {
    Event(i64),
    Order(i64),
    /// A sale together with every line sold with it (one `batch_id`).
    Sale(i64),
    Finance(i64),
    Mail(i64),
}

impl Subject {
    pub(crate) fn parse(kind: &str, id: i64) -> AppResult<Self> {
        match kind.trim() {
            "event" => Ok(Subject::Event(id)),
            "order" => Ok(Subject::Order(id)),
            "sale" => Ok(Subject::Sale(id)),
            "finance" => Ok(Subject::Finance(id)),
            "mail" => Ok(Subject::Mail(id)),
            other => Err(AppError::Validation(format!("\"{other}\" cannot be sent to Discord"))),
        }
    }

    fn auto_on(self, auto: &DiscordAutoConfig) -> bool {
        match self {
            Subject::Event(_) => auto.events,
            Subject::Order(_) => auto.orders,
            Subject::Sale(_) => auto.sales,
            Subject::Finance(_) => auto.finance,
            Subject::Mail(_) => auto.mail,
        }
    }
}

/// Why it is being sent - it decides the first line of the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occasion {
    /// Saved in TIQR.
    Created,
    /// Made from an email: by Automatic (`auto`), or by marko's click.
    FromMail { auto: bool },
    /// The "Send to Discord" button.
    Manual,
    /// An important email arrived.
    MailArrived,
}

/// One message, ready to post.
#[derive(Debug, Clone)]
pub(crate) struct Post {
    /// Where it belongs, best first. The first one with a webhook set wins.
    pub routes: Vec<DiscordDestination>,
    /// What Settings shows for it ("New sale · Oasis").
    pub title: String,
    pub payload: Value,
}

// ---------------------------------------------------------------------------
// Text.
// ---------------------------------------------------------------------------

/// Text from a record or an email, for a place Discord reads as Markdown (a
/// description, a field). Every Markdown character is escaped, so `**` stays
/// two stars, `[x](y)` is not a link, `<@1>` is not a mention and `# x` is
/// not a heading. Control characters go; line breaks stay.
pub(crate) fn md(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in defang(raw).chars() {
        match ch {
            '\\' | '*' | '_' | '~' | '`' | '|' | '>' | '<' | '#' | '[' | ']' | '-' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push('\n'),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out.trim().to_string()
}

/// Text for a place Discord shows as it is (a title, the top line, the
/// footer): one line, no control characters, links disarmed.
pub(crate) fn line(raw: &str) -> String {
    let flat: String = defang(raw).chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `https://x` -> `https://` with a zero-width space after the colon, which
/// Discord does not turn into a link. Text from an email must never become
/// something to tap in a chat.
fn defang(raw: &str) -> String {
    raw.replace("://", ":\u{200B}//")
}

/// At most `max` characters, with an ellipsis when something was cut.
pub(crate) fn clip(raw: &str, max: usize) -> String {
    if raw.chars().count() <= max {
        return raw.to_string();
    }
    let mut out: String = raw.chars().take(max.saturating_sub(1)).collect();
    // Never leave a lone escape at the end: it would escape the ellipsis.
    if out.ends_with('\\') {
        out.pop();
    }
    out.push('…');
    out
}

/// "€1,234.56", "$12.00", "£3.50"; any other currency after the number
/// ("1,500.00 CZK").
pub(crate) fn money(cents: i64, currency: &str) -> String {
    let code = currency.trim().to_ascii_uppercase();
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let digits = (abs / 100).to_string();
    let mut whole = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            whole.push(',');
        }
        whole.push(ch);
    }
    let body = format!("{whole}.{:02}", abs % 100);
    match code.as_str() {
        "EUR" => format!("{sign}€{body}"),
        "USD" => format!("{sign}${body}"),
        "GBP" => format!("{sign}£{body}"),
        "" => format!("{sign}{body}"),
        other => format!("{sign}{body} {other}"),
    }
}

/// Amounts in several currencies, never added together: "€120.00 + $40.00".
fn money_list(parts: &[(String, i64)]) -> String {
    parts
        .iter()
        .filter(|(_, c)| *c != 0)
        .map(|(cur, c)| money(*c, cur))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn date_of(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim().get(..10)?, "%Y-%m-%d").ok()
}

/// "Sat 26 Jul 2026", with the time when the record has one ("· 19:30").
pub(crate) fn day(raw: &str) -> Option<String> {
    let d = date_of(raw)?;
    let mut out = d.format("%a %-d %b %Y").to_string();
    let rest = raw.trim().get(10..).unwrap_or("");
    let time = rest.trim_start_matches(['T', ' ']);
    if time.len() >= 5 && time.as_bytes()[2] == b':' && time[..5].chars().filter(|c| c.is_ascii_digit()).count() == 4 {
        out.push_str(" · ");
        out.push_str(&time[..5]);
    }
    Some(out)
}

/// Discord's own "in 3 weeks", shown in each reader's language and kept up
/// to date by Discord. Noon UTC, so a date without a time never lands on the
/// day before.
fn relative(raw: &str) -> Option<String> {
    let ts = date_of(raw)?.and_hms_opt(12, 0, 0)?.and_utc().timestamp();
    Some(format!("<t:{ts}:R>"))
}

/// 2.114.0: an event date on one line - the day, and how far away it is
/// ("Sun 26 Jul 2026 · in 9 days", kept current by Discord).
fn when_line(raw: Option<&str>) -> String {
    match raw.and_then(|r| day(r).map(|d| (d, relative(r)))) {
        Some((d, Some(rel))) => format!("{} · {rel}", md(&d)),
        Some((d, None)) => md(&d),
        None => "Date to be announced".to_string(),
    }
}

/// "Wembley Stadium, London, UK" - on one line.
fn place_line(venue: &str, city: &str, country: &str) -> String {
    let parts: Vec<&str> = [venue.trim(), city.trim(), country.trim()].into_iter().filter(|s| !s.is_empty()).collect();
    md(&parts.join(", "))
}

/// 2.114.0: marko - "tie webhooks viac zjednodusit lebo tam je vela info a je
/// to potom neprehladne, fakt len to najdolezitejsie". A message is a title
/// and a few short lines; empty ones are left out.
fn lines(parts: Vec<String>) -> String {
    parts.into_iter().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n")
}

/// The seats on one line: the first two groups, then how many more.
fn seats_line(tickets: &[(String, String, String)]) -> String {
    let all = seat_lines(tickets);
    let groups: Vec<&str> = all.lines().filter(|l| !l.trim().is_empty()).collect();
    match groups.len() {
        0 => String::new(),
        1 | 2 => format!("💺 {}", groups.join(" · ")),
        n => format!("💺 {} · +{} more", groups[..2].join(" · "), n - 2),
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Seats, one line per section and row: "Sec 112 · Row F · 14, 15".
fn seat_lines(tickets: &[(String, String, String)]) -> String {
    let mut groups: Vec<((String, String), Vec<String>)> = Vec::new();
    for (section, row, seat) in tickets {
        let key = (section.trim().to_string(), row.trim().to_string());
        let seat = seat.trim().to_string();
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, seats)) => seats.push(seat),
            None => groups.push((key, vec![seat])),
        }
    }
    if groups.iter().all(|((s, r), seats)| s.is_empty() && r.is_empty() && seats.iter().all(|x| x.is_empty())) {
        return String::new();
    }
    let mut out: Vec<String> = Vec::new();
    for ((section, row), seats) in groups.iter().take(6) {
        let mut parts: Vec<String> = Vec::new();
        if !section.is_empty() {
            parts.push(format!("Sec {section}"));
        }
        if !row.is_empty() {
            parts.push(format!("Row {row}"));
        }
        let named: Vec<&str> = seats.iter().map(String::as_str).filter(|s| !s.is_empty()).collect();
        if !named.is_empty() {
            parts.push(named.join(", "));
        } else {
            parts.push(plural(seats.len() as i64, "ticket", "tickets"));
        }
        out.push(md(&parts.join(" · ")));
    }
    if groups.len() > 6 {
        out.push(format!("+ {} more", groups.len() - 6));
    }
    out.join("\n")
}

fn now_stamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

// ---------------------------------------------------------------------------
// The message.
// ---------------------------------------------------------------------------

struct Embed {
    routes: Vec<DiscordDestination>,
    color: u32,
    /// The small line at the top ("💰 New sale").
    headline: String,
    title: String,
    url: Option<String>,
    description: String,
    /// (name, value, inline). An empty value leaves the field out.
    fields: Vec<(String, String, bool)>,
    /// The record's own code, next to the brand in the footer.
    code: Option<String>,
}

impl Embed {
    fn into_post(self) -> Post {
        let headline = clip(&line(&self.headline), 200);
        let title = clip(&line(&self.title), 250);
        let footer = match self.code.as_deref().map(line).filter(|c| !c.is_empty()) {
            Some(code) => format!("{BRAND} · {}", clip(&code, 80)),
            None => BRAND.to_string(),
        };
        let description = clip(&self.description, 1800);
        // Discord refuses a message whose text adds up to more than 6000
        // characters, so fields stop well before that.
        let mut budget: usize = 5600usize
            .saturating_sub(headline.chars().count() + title.chars().count() + footer.chars().count())
            .saturating_sub(description.chars().count());
        let mut fields: Vec<Value> = Vec::new();
        for (name, value, inline) in self.fields {
            if value.trim().is_empty() || fields.len() == 24 {
                continue;
            }
            let (name, value) = (clip(&name, 200), clip(&value, 1000));
            let cost = name.chars().count() + value.chars().count();
            if cost > budget {
                break;
            }
            budget -= cost;
            fields.push(json!({ "name": name, "value": value, "inline": inline }));
        }
        let mut embed = json!({
            "author": { "name": headline, "icon_url": LOGO_URL },
            "title": title,
            "description": description,
            "color": self.color,
            "fields": fields,
            "footer": { "text": footer, "icon_url": LOGO_URL },
            "timestamp": now_stamp(),
        });
        if let Some(url) = self.url {
            embed["url"] = json!(url);
        }
        let summary = clip(&format!("{} · {}", strip_emoji(&headline), title), 140);
        Post {
            routes: self.routes,
            title: summary,
            payload: json!({
                "username": BRAND,
                "avatar_url": LOGO_URL,
                // Nothing in a message may ping anyone, whatever it says.
                "allowed_mentions": { "parse": [] },
                "embeds": [embed],
            }),
        }
    }
}

/// "💰 New sale" -> "New sale", for the list in Settings.
fn strip_emoji(s: &str) -> String {
    s.trim_start_matches(|c: char| !c.is_alphanumeric()).to_string()
}

fn headline(icon: &str, created: &str, from_mail: &str, manual: &str, occasion: Occasion) -> String {
    let words = match occasion {
        Occasion::Created => created.to_string(),
        Occasion::FromMail { auto: true } => format!("{from_mail} · created automatically"),
        Occasion::FromMail { auto: false } => from_mail.to_string(),
        Occasion::Manual | Occasion::MailArrived => manual.to_string(),
    };
    format!("{icon} {words}")
}

/// Builds the message for one record, or `None` when the record is gone.
pub(crate) fn post_for(conn: &Connection, subject: Subject, occasion: Occasion) -> AppResult<Option<Post>> {
    let embed = match subject {
        Subject::Event(id) => event_embed(conn, id, occasion)?,
        Subject::Order(id) => order_embed(conn, id, occasion)?,
        Subject::Sale(id) => sale_embed(conn, id, occasion)?,
        Subject::Finance(id) => finance_embed(conn, id, occasion)?,
        Subject::Mail(id) => mail_embed(conn, id, occasion)?,
    };
    Ok(embed.map(Embed::into_post))
}

fn event_embed(conn: &Connection, id: i64, occasion: Occasion) -> AppResult<Option<Embed>> {
    type Row = (String, String, String, String, String, Option<String>, String, String);
    let row: Option<Row> = conn
        .query_row(
            "SELECT name, IFNULL(artist_team,''), IFNULL(venue,''), IFNULL(city,''), IFNULL(country,''),
                    event_date, IFNULL(category,''), status
               FROM events WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?;
    let Some((name, artist, venue, city, country, date, category, status)) = row else {
        return Ok(None);
    };
    let (total, in_stock, sold): (i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*), IFNULL(SUM(status IN ('available','listed')),0), IFNULL(SUM(status = 'sold'),0)
           FROM tickets WHERE event_id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let spent = sums(conn, "SELECT currency, SUM(total_cost_cents) FROM orders WHERE event_id = ?1 GROUP BY currency", id)?;
    let revenue = sums(
        conn,
        "SELECT s.currency, SUM(s.sale_price_cents) FROM sales s JOIN tickets t ON t.id = s.ticket_id
          WHERE t.event_id = ?1 AND s.payment_status <> 'refunded' GROUP BY s.currency",
        id,
    )?;

    let artist_line = (!artist.trim().is_empty() && !name.to_lowercase().contains(&artist.trim().to_lowercase()))
        .then(|| format!("**{}**", md(&artist)))
        .unwrap_or_default();
    let where_ = place_line(&venue, &city, &country);
    let tickets = if total == 0 {
        String::new()
    } else {
        format!("🎟️ {} · {in_stock} in stock · {sold} sold", plural(total, "ticket", "tickets"))
    };
    let mut money_parts = Vec::new();
    if !money_list(&spent).is_empty() {
        money_parts.push(format!("Bought {}", money_list(&spent)));
    }
    if !money_list(&revenue).is_empty() {
        money_parts.push(format!("Sold {}", money_list(&revenue)));
    }
    let description = lines(vec![
        if status == "cancelled" { "**Cancelled**".to_string() } else { String::new() },
        artist_line,
        format!("📅 {}{}", when_line(date.as_deref()), if where_.is_empty() { String::new() } else { format!(" · {where_}") }),
        tickets,
        money_parts.join(" · "),
    ]);
    let _ = &category;
    Ok(Some(Embed {
        routes: vec![DiscordDestination::Inventory],
        color: if status == "cancelled" { RED } else { PURPLE },
        headline: headline("📅", "New event", "Event from email", "Event", occasion),
        title: name,
        url: None,
        description,
        fields: Vec::new(),
        code: None,
    }))
}

fn sums(conn: &Connection, sql: &str, id: i64) -> AppResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn tickets_of(conn: &Connection, sql: &str, id: i64) -> AppResult<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn payment_label(status: &str) -> &'static str {
    match status {
        "paid" => "Paid",
        "partial" => "Partly paid",
        "refunded" => "Refunded",
        "pending" => "Pending",
        _ => "Not paid",
    }
}

fn order_embed(conn: &Connection, id: i64, occasion: Occasion) -> AppResult<Option<Embed>> {
    type Row = (
        String, i64, i64, i64, i64, i64, String, String, String, Option<String>,
        String, Option<String>, String, String, Option<String>, Option<String>,
    );
    let row: Option<Row> = conn
        .query_row(
            "SELECT o.code, o.quantity, o.unit_price_cents, o.fees_cents, o.other_costs_cents, o.total_cost_cents,
                    o.currency, o.payment_status, o.purchase_date, o.external_reference,
                    e.name, e.event_date, IFNULL(e.venue,''), IFNULL(e.city,''), p.name, s.name
               FROM orders o
               JOIN events e ON e.id = o.event_id
               LEFT JOIN platforms p ON p.id = o.platform_id
               LEFT JOIN suppliers s ON s.id = o.supplier_id
              WHERE o.id = ?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?,
                    r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?, r.get(13)?, r.get(14)?, r.get(15)?,
                ))
            },
        )
        .optional()?;
    let Some((code, qty, unit, fees, other, total, cur, paid, bought_on, reference, event, date, venue, city, platform, supplier)) =
        row
    else {
        return Ok(None);
    };
    let seats = seats_line(&tickets_of(
        conn,
        "SELECT IFNULL(section,''), IFNULL(row_label,''), IFNULL(seat,'') FROM tickets WHERE order_id = ?1 ORDER BY id",
        id,
    )?);
    let source = platform.clone().or(supplier.clone()).filter(|s| !s.trim().is_empty());
    let mut first = format!("**{}** bought", plural(qty, "ticket", "tickets"));
    if let Some(s) = &source {
        first.push_str(&format!(" on **{}**", md(s)));
    }
    first.push_str(&format!(" for **{}**", money(total, &cur)));
    if paid != "paid" {
        first.push_str(&format!(" · {}", payment_label(&paid).to_lowercase()));
    }
    let where_ = place_line(&venue, &city, "");
    let description = lines(vec![
        first,
        format!("📅 {}{}", when_line(date.as_deref()), if where_.is_empty() { String::new() } else { format!(" · {where_}") }),
        seats,
    ]);
    let _ = (unit, fees, other, bought_on, reference);
    Ok(Some(Embed {
        routes: vec![DiscordDestination::Inventory],
        color: BLUE,
        headline: headline("🛒", "New purchase", "Purchase from email", "Purchase", occasion),
        title: event,
        url: None,
        description,
        fields: Vec::new(),
        code: Some(code),
    }))
}

fn sale_embed(conn: &Connection, id: i64, occasion: Occasion) -> AppResult<Option<Embed>> {
    struct Line {
        code: String,
        price: i64,
        fees: i64,
        cur: String,
        status: String,
        cost: i64,
        ticket_cur: String,
        seat: (String, String, String),
        event: String,
        date: Option<String>,
        platform: Option<String>,
        sale_date: String,
    }
    let batch: Option<Option<String>> =
        conn.query_row("SELECT batch_id FROM sales WHERE id = ?1", [id], |r| r.get(0)).optional()?;
    let Some(batch) = batch else {
        return Ok(None);
    };
    let mut stmt = conn.prepare(
        "SELECT s.code, s.sale_price_cents, s.selling_fees_cents, s.currency, s.payment_status,
                t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents, t.currency,
                IFNULL(t.section,''), IFNULL(t.row_label,''), IFNULL(t.seat,''),
                e.name, e.event_date, p.name, s.sale_date
           FROM sales s
           JOIN tickets t ON t.id = s.ticket_id
           JOIN events e ON e.id = t.event_id
           LEFT JOIN platforms p ON p.id = s.platform_id
          WHERE (?2 IS NOT NULL AND s.batch_id = ?2) OR s.id = ?1
          ORDER BY s.id",
    )?;
    let lines: Vec<Line> = stmt
        .query_map(params![id, batch], |r| {
            Ok(Line {
                code: r.get(0)?,
                price: r.get(1)?,
                fees: r.get(2)?,
                cur: r.get(3)?,
                status: r.get(4)?,
                cost: r.get(5)?,
                ticket_cur: r.get(6)?,
                seat: (r.get(7)?, r.get(8)?, r.get(9)?),
                event: r.get(10)?,
                date: r.get(11)?,
                platform: r.get(12)?,
                sale_date: r.get(13)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let Some(first) = lines.first() else {
        return Ok(None);
    };
    let n = lines.len() as i64;
    let code = batch.clone().unwrap_or_else(|| first.code.clone());

    // Per currency, never blended.
    let mut revenue: Vec<(String, i64)> = Vec::new();
    let mut fees: Vec<(String, i64)> = Vec::new();
    for l in &lines {
        add_to(&mut revenue, &l.cur, l.price);
        add_to(&mut fees, &l.cur, l.fees);
    }
    let one_currency = lines.iter().all(|l| l.cur == first.cur && l.ticket_cur == first.cur);
    let profit = one_currency.then(|| {
        let net: i64 = lines.iter().map(|l| l.price - l.fees).sum();
        let cost: i64 = lines.iter().map(|l| l.cost).sum();
        (net - cost, cost)
    });

    let mut events: Vec<&str> = Vec::new();
    for l in &lines {
        if !events.contains(&l.event.as_str()) {
            events.push(&l.event);
        }
    }
    let title = match events.len() {
        1 => first.event.clone(),
        k => format!("{} + {} more", first.event, k - 1),
    };
    let platform = first.platform.clone().filter(|p| !p.trim().is_empty());
    let mut top = format!("**{}** sold", plural(n, "ticket", "tickets"));
    if let Some(p) = &platform {
        top.push_str(&format!(" on **{}**", md(p)));
    }
    top.push_str(&format!(" for **{}**", money_list(&revenue)));
    let refunded = lines.iter().all(|l| l.status == "refunded");
    let same_status = lines.iter().all(|l| l.status == first.status);
    if refunded {
        top.push_str(" · refunded");
    } else if !same_status {
        top.push_str(" · partly paid");
    } else if first.status != "paid" {
        top.push_str(&format!(" · {}", payment_label(&first.status).to_lowercase()));
    }
    let profit_line = match profit {
        Some((p, cost)) if cost > 0 => {
            let roi = (p as f64) * 100.0 / (cost as f64);
            format!("Profit **{}** · ROI {roi:.0}%", money(p, &first.cur))
        }
        Some((p, _)) => format!("Profit **{}**", money(p, &first.cur)),
        None => String::new(),
    };
    let seats: Vec<(String, String, String)> = lines.iter().map(|l| l.seat.clone()).collect();
    let description = self::lines(vec![top, profit_line, format!("📅 {}", when_line(first.date.as_deref())), seats_line(&seats)]);
    let _ = (&fees, &first.sale_date);
    Ok(Some(Embed {
        routes: vec![DiscordDestination::Sales],
        color: if refunded { AMBER } else { GREEN },
        headline: headline("💰", "New sale", "Sale from email", "Sale", occasion),
        title,
        url: None,
        description,
        fields: Vec::new(),
        code: Some(code),
    }))
}

fn add_to(list: &mut Vec<(String, i64)>, cur: &str, cents: i64) {
    match list.iter_mut().find(|(c, _)| c == cur) {
        Some((_, total)) => *total += cents,
        None => list.push((cur.to_string(), cents)),
    }
}

fn finance_embed(conn: &Connection, id: i64, occasion: Occasion) -> AppResult<Option<Embed>> {
    type Row = (String, String, i64, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>);
    let row: Option<Row> = conn
        .query_row(
            "SELECT e.entry_type, e.entry_date, e.amount_cents, e.currency, e.scope, e.place, e.note,
                    c.name, a.name, o.code
               FROM finance_entries e
               LEFT JOIN finance_categories c ON c.id = e.category_id
               LEFT JOIN accounts a ON a.id = e.account_id
               LEFT JOIN orders o ON o.id = e.order_id
              WHERE e.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
        )
        .optional()?;
    let Some((kind, date, amount, cur, scope, place_name, note, category, account, order)) = row else {
        return Ok(None);
    };
    let income = kind == "income";
    let (icon, created, from_mail, manual) = if income {
        ("💶", "New income", "Income from email", "Income")
    } else {
        ("💸", "New expense", "Expense from email", "Expense")
    };
    let nonempty = |v: &Option<String>| v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let title = nonempty(&category)
        .or_else(|| nonempty(&place_name))
        .unwrap_or_else(|| manual.to_string());
    let signed = if income { money(amount, &cur) } else { money(-amount, &cur) };
    let mut description = format!("**{}** {}", signed, if income { "in" } else { "out" });
    if let Some(p) = nonempty(&place_name).filter(|p| Some(p) != nonempty(&category).as_ref()) {
        description.push_str(&format!(" · {}", md(&p)));
    }
    let meta: Vec<String> = [
        day(&date).map(|d| md(&d)),
        nonempty(&account).map(|a| md(&a)),
        Some(if scope == "business" { "Business" } else { "Personal" }.to_string()),
        nonempty(&order).map(|o| md(&o)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let description = lines(vec![
        description,
        format!("📅 {}", meta.join(" · ")),
        nonempty(&note).map(|n| md(&clip(&n, 140))).unwrap_or_default(),
    ]);
    Ok(Some(Embed {
        routes: vec![DiscordDestination::Finance],
        color: if income { GREEN } else { RED },
        headline: headline(icon, created, from_mail, manual, occasion),
        title,
        url: None,
        description,
        fields: Vec::new(),
        code: None,
    }))
}

/// What a mail category is called on a message, its colour and where it
/// goes, best channel first.
fn mail_kind(category: &str) -> (&'static str, &'static str, u32, Vec<DiscordDestination>) {
    use DiscordDestination::{Finance, Inventory, Mail, Sales};
    match category {
        "sale_confirmation" => ("💰", "Sale confirmation", GREEN, vec![Sales, Mail]),
        "purchase_confirmation" => ("🛒", "Purchase confirmation", BLUE, vec![Inventory, Mail]),
        "order_status" => ("📦", "Order update", BLUE, vec![Inventory, Mail]),
        "payout" => ("💶", "Payout", GREEN, vec![Finance, Mail]),
        "refund" => ("↩️", "Refund", AMBER, vec![Finance, Mail]),
        "cancellation" => ("⛔", "Cancellation", RED, vec![Mail, Inventory]),
        "event_update" => ("📅", "Event change", AMBER, vec![Mail, Inventory]),
        "account_notice" => ("🔐", "Account notice", MAGENTA, vec![Mail]),
        _ => ("✉️", "Email", MAGENTA, vec![Mail]),
    }
}

/// The categories worth a message when they arrive. An account notice (a
/// sign-in, a code) never is.
fn mail_is_important(category: &str) -> bool {
    matches!(
        category,
        "sale_confirmation" | "purchase_confirmation" | "order_status" | "payout" | "refund" | "cancellation" | "event_update"
    )
}

fn mail_embed(conn: &Connection, id: i64, occasion: Occasion) -> AppResult<Option<Embed>> {
    type Row = (
        String, String, String, String, String, String, String, Option<String>,
        Option<i64>, Option<String>, Option<i64>, Option<String>, Option<String>,
    );
    let row: Option<Row> = conn
        .query_row(
            "SELECT provider_message_id, from_address, IFNULL(from_name,''), subject, IFNULL(snippet,''),
                    received_at, category, detected_platform, extracted_amount_cents, extracted_currency,
                    extracted_quantity, extracted_reference, otp_code
               FROM mail_messages WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?,
                    r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?,
                ))
            },
        )
        .optional()?;
    let Some((gmail_id, from, from_name, subject, snippet, received, category, platform, amount, currency, qty, reference, otp)) =
        row
    else {
        return Ok(None);
    };
    let (icon, label, color, routes) = mail_kind(&category);
    // A one-time code never leaves TIQR, wherever it appears.
    let hide = |text: &str| match otp.as_deref().filter(|c| c.len() >= 4) {
        Some(code) => text.replace(code, "••••••"),
        None => text.to_string(),
    };
    let domain = crate::commands::mail::domain_of(&from).unwrap_or_default();
    let sender = platform
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| (!from_name.trim().is_empty()).then(|| from_name.trim().to_string()))
        .unwrap_or_else(|| domain.clone());
    let price = match (amount, currency.as_deref()) {
        (Some(c), Some(cur)) if c > 0 => Some(money(c, cur)),
        _ => None,
    };
    let tickets = qty.filter(|q| *q > 0).map(|q| plural(q, "ticket", "tickets"));
    // 2.113.1: the lines that matter, from the message's own text - the
    // opened copy, else the body read on arrival, else Gmail's preview.
    let text: Option<String> = conn
        .query_row(
            "SELECT COALESCE((SELECT NULLIF(text, '') FROM mail_bodies WHERE message_id = ?1), extract_text)
               FROM mail_messages WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    let options = load_notification_config(conn)?.discord.mail_options;
    let facts = crate::commands::mail_view::facts_for(conn, id, text.as_deref().unwrap_or(&snippet));
    // A person's details are spoilers in the text and dots in the title.
    let secrets: Vec<String> = facts
        .iter()
        .filter(|f| f.private)
        .map(|f| hide(&f.value))
        .filter(|v| v.chars().count() >= 3)
        .collect();
    // 2.114.0: a few short lines and nothing else (`lines`). Who and how
    // much; where the tickets go, by when, for which event; the opening
    // words only when nothing better is known.
    let digits = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    let said = |value: &str, field: Option<&String>| field.is_some_and(|v| !digits(v).is_empty() && digits(v) == digits(value));
    let mut top = vec![format!("**{}**", md(&sender))];
    top.extend(tickets.clone());
    top.extend(price.as_ref().map(|p| format!("**{p}**")));
    let mut out: Vec<String> = vec![top.join(" · ")];
    if options.key_details {
        let of = |kinds: &[&str]| facts.iter().filter(|f| kinds.contains(&f.kind)).collect::<Vec<_>>();
        let value = |f: &MailFact| md(&hide(&f.value));
        let labelled = |f: &MailFact| format!("{} {}", md(&f.label), value(f));
        if options.personal {
            let people: Vec<String> = facts.iter().filter(|f| f.private).map(|f| format!("||{}||", value(f))).collect();
            if !people.is_empty() {
                out.push(format!("📤 {}", people.join(" · ")));
            }
        }
        if let Some(f) = of(&["deadline"]).first() {
            out.push(format!("⏰ {}", labelled(f)));
        }
        let event: Vec<String> = of(&["event", "when", "venue"]).into_iter().map(value).collect();
        if !event.is_empty() {
            out.push(format!("🎫 {}", event.join(" · ")));
        }
        let cash: Vec<String> = of(&["money"]).into_iter().filter(|f| !said(&f.value, price.as_ref())).map(labelled).collect();
        if !cash.is_empty() {
            out.push(format!("💶 {}", cash.join(" · ")));
        }
        let seats: Vec<String> = of(&["seats"]).into_iter().map(labelled).collect();
        if !seats.is_empty() {
            out.push(format!("💺 {}", seats.join(" · ")));
        }
    }
    if out.len() == 1 && price.is_none() && tickets.is_none() && options.preview {
        let preview = md_spoilered(&hide(&snippet), &secrets, 220, options.personal);
        if !preview.is_empty() {
            out.push(format!("> {preview}"));
        }
    }
    // Whole lines only, well inside Discord's limit: a spoiler is never cut.
    let mut description = String::new();
    for l in out {
        if description.chars().count() + l.chars().count() + 1 > 1500 {
            break;
        }
        if !description.is_empty() {
            description.push('\n');
        }
        description.push_str(&l);
    }
    // Gmail's ids are hex; anything else does not become a link.
    let url = (!gmail_id.is_empty() && gmail_id.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("https://mail.google.com/mail/u/0/#all/{gmail_id}"));
    let manual_label = if matches!(occasion, Occasion::Manual) { "Email" } else { label };
    let mut title = hide(&subject);
    for secret in &secrets {
        title = replace_ignoring_case(&title, secret, "•••");
    }
    let _ = (&received, &domain);
    Ok(Some(Embed {
        routes,
        color,
        headline: format!("{icon} {manual_label}"),
        title,
        url,
        description,
        fields: Vec::new(),
        // The order or sale number goes by the brand, out of the way.
        code: reference.map(|r| hide(&r)),
    }))
}

/// `needle` replaced wherever it appears, whatever its case - compared
/// letter by letter, so a letter whose lower case is longer cannot shift
/// where a match is.
fn replace_ignoring_case(text: &str, needle: &str, with: &str) -> String {
    let needle: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let (mut j, mut k) = (i, 0);
        while k < needle.len() && j < chars.len() {
            let lower: Vec<char> = chars[j].to_lowercase().collect();
            if needle.len() - k < lower.len() || needle[k..k + lower.len()] != lower[..] {
                break;
            }
            k += lower.len();
            j += 1;
        }
        if k == needle.len() {
            out.push_str(with);
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Email text for a Markdown place (`md`), at most `max` characters, with
/// each of `secrets` as a spoiler. A secret the cut would split is left
/// out whole, never shown half.
fn md_spoilered(raw: &str, secrets: &[String], max: usize, spoil: bool) -> String {
    const OPEN: char = '\u{E000}';
    const CLOSE: char = '\u{E001}';
    // Marks no email can contain stand in for the secrets until the escaping
    // is done.
    let unmarked = |s: &str| s.chars().filter(|c| *c != OPEN && *c != CLOSE).collect::<String>();
    let secrets: Vec<String> = secrets.iter().map(|s| unmarked(s)).collect();
    let mut order: Vec<(usize, &String)> = secrets.iter().enumerate().filter(|(_, s)| !s.trim().is_empty()).collect();
    order.sort_by_key(|(_, s)| std::cmp::Reverse(s.chars().count()));
    let mut text = unmarked(raw);
    for (i, secret) in &order {
        text = replace_ignoring_case(&text, secret, &format!("{OPEN}{i}{CLOSE}"));
    }
    let mut cut = clip(&text, max);
    if let Some(open) = cut.rfind(OPEN) {
        if !cut[open..].contains(CLOSE) {
            cut.truncate(open);
            cut = format!("{}…", cut.trim_end());
        }
    }
    let escaped = md(&cut);
    let mut out = escaped.clone();
    for (i, secret) in secrets.iter().enumerate() {
        out = out.replace(&format!("{OPEN}{i}{CLOSE}"), &format!("||{}||", md(secret)));
    }
    // Discord's own cut at 1800 must never fall inside a spoiler: a preview
    // that would grow that long hides its secrets as dots instead - and so
    // does one whose personal details are switched off.
    if !spoil || out.chars().count() > 1400 {
        out = escaped;
        for i in 0..secrets.len() {
            out = out.replace(&format!("{OPEN}{i}{CLOSE}"), "•••");
        }
    }
    out
}

// ---------------------------------------------------------------------------
// When a message goes out by itself.
// ---------------------------------------------------------------------------

/// The first destination in `routes` that has a webhook set.
fn route(cfg: &DiscordChannelConfig, routes: &[DiscordDestination]) -> Option<(DiscordDestination, String)> {
    routes.iter().find_map(|d| {
        let url = d.url_in(cfg).trim();
        (!url.is_empty()).then(|| (*d, url.to_string()))
    })
}

/// What an automatic post would do: `None` when it should not happen at all
/// (Discord off, this kind switched off, no channel for it, record gone).
pub(crate) fn plan(conn: &Connection, subject: Subject, occasion: Occasion) -> AppResult<Option<(DiscordDestination, String, Post)>> {
    let cfg = load_notification_config(conn)?.discord;
    if !cfg.enabled || !subject.auto_on(&cfg.auto) {
        return Ok(None);
    }
    // 2.114.0: which kinds of mail are passed on (Settings, Mail -> Discord).
    if let Subject::Mail(id) = subject {
        let category: Option<String> =
            conn.query_row("SELECT category FROM mail_messages WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        if !category.is_some_and(|c| mail_kind_on(&cfg.mail_options, &c)) {
            return Ok(None);
        }
    }
    let Some(post) = post_for(conn, subject, occasion)? else {
        return Ok(None);
    };
    let Some((dest, url)) = route(&cfg, &post.routes) else {
        return Ok(None);
    };
    Ok(Some((dest, url, post)))
}

/// Whether this kind of mail is passed on by itself.
pub(crate) fn mail_kind_on(o: &crate::models::DiscordMailConfig, category: &str) -> bool {
    match category {
        "sale_confirmation" => o.sales,
        "purchase_confirmation" => o.purchases,
        "payout" | "refund" => o.money,
        "cancellation" | "event_update" => o.changes,
        "order_status" => o.order_updates,
        _ => false,
    }
}

/// Posts a message about something that just happened. Never fails and never
/// waits: the message is built here, under the caller's lock, and sent by the
/// sender thread after the caller has returned.
pub(crate) fn announce(conn: &Connection, subject: Subject, occasion: Occasion) {
    match plan(conn, subject, occasion) {
        Ok(Some((dest, url, post))) => enqueue(Job { dest, url, post, manual: false, reply: None }),
        Ok(None) => {}
        // A record that cannot be read is no reason to fail what was saved.
        Err(_) => {}
    }
}

/// How long an email may have been in the mailbox and still be announced as
/// it arrives. Older mail found by a walk through the history is not news.
const MAIL_NEWS_WINDOW_HOURS: i64 = 48;

/// Whether a just-stored email deserves a message of its own: it is new, it
/// is about a sale, a purchase, money or a change to an event, the classifier
/// is sure enough, it is not in Spam or Trash, and no record was made from
/// it (that record's own message already says it).
pub(crate) fn mail_is_news(conn: &Connection, id: i64, now: DateTime<Utc>) -> bool {
    let row: Option<(String, String, String, Option<String>, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT received_at, category, confidence, labels, trashed_at, created_id
               FROM mail_messages WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()
        .ok()
        .flatten();
    let Some((received, category, confidence, labels, trashed, created)) = row else {
        return false;
    };
    let Ok(at) = DateTime::parse_from_rfc3339(&received) else {
        return false;
    };
    let age = now.signed_duration_since(at.with_timezone(&Utc));
    if age > chrono::Duration::hours(MAIL_NEWS_WINDOW_HOURS) || trashed.is_some() || created.is_some() {
        return false;
    }
    let labels = labels.unwrap_or_default().to_ascii_lowercase();
    if labels.split(',').any(|l| matches!(l.trim(), "spam" | "trash")) {
        return false;
    }
    // "Your tickets are ready" wording is everywhere; only a sure match of it
    // is worth a ping.
    let sure_enough = match category.as_str() {
        "order_status" => confidence == "high",
        _ => confidence != "low",
    };
    mail_is_important(&category) && sure_enough
}

// ---------------------------------------------------------------------------
// The sender.
// ---------------------------------------------------------------------------

struct Job {
    dest: DiscordDestination,
    url: String,
    post: Post,
    manual: bool,
    reply: Option<mpsc::Sender<(&'static str, String)>>,
}

const MAX_ATTEMPTS: u32 = 3;
const KEPT_DELIVERIES: usize = 20;

fn deliveries() -> &'static Mutex<VecDeque<DiscordDelivery>> {
    static LOG: OnceLock<Mutex<VecDeque<DiscordDelivery>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn record(job: &Job, status: &str, message: &str) {
    let mut log = deliveries().lock().unwrap_or_else(|e| e.into_inner());
    log.push_front(DiscordDelivery {
        at: now_stamp(),
        destination: job.dest.key().to_string(),
        title: job.post.title.clone(),
        status: status.to_string(),
        message: message.to_string(),
        manual: job.manual,
    });
    log.truncate(KEPT_DELIVERIES);
}

fn enqueue(job: Job) {
    static QUEUE: OnceLock<Option<Mutex<mpsc::Sender<Job>>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("tiqr-discord".to_string())
            .spawn(move || run_sender(rx))
            .ok()
            .map(|_| Mutex::new(tx))
    });
    let refused = match queue {
        Some(tx) => tx.lock().unwrap_or_else(|e| e.into_inner()).send(job).err().map(|e| e.0),
        None => Some(job),
    };
    if let Some(job) = refused {
        let message = "TIQR could not start its Discord sender. Restart the app and try again.";
        record(&job, "failed", message);
        if let Some(reply) = &job.reply {
            let _ = reply.send(("failed", message.to_string()));
        }
    }
}

fn run_sender(rx: mpsc::Receiver<Job>) {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(15)).build();
    for job in rx {
        let (status, message, pause) = match &client {
            Ok(c) => deliver(c, &job.url, &job.post.payload),
            Err(e) => ("failed", format!("could not build an HTTP client: {e}"), Duration::ZERO),
        };
        record(&job, status, &message);
        if let Some(reply) = &job.reply {
            let _ = reply.send((status, message));
        }
        // Discord allows a webhook about five messages every two seconds;
        // when it says the allowance is used up, the next one waits.
        std::thread::sleep(pause.max(Duration::from_millis(250)));
    }
}

/// One message: posted, and posted again after a temporary failure. Returns
/// the outcome, what to tell marko and how long the next message must wait.
fn deliver(client: &reqwest::blocking::Client, url: &str, payload: &Value) -> (&'static str, String, Duration) {
    // Checked again here: the webhook is read from a file that syncs.
    if let Err(e) = validate_webhook_url(url) {
        return ("failed", e.to_string(), Duration::ZERO);
    }
    let mut attempt = 0;
    loop {
        attempt += 1;
        match client.post(url).json(payload).send() {
            Ok(r) => {
                let code = r.status().as_u16();
                let header = |name: &str| {
                    r.headers().get(name).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<f64>().ok())
                };
                let retry_after = header("retry-after").map(|s| Duration::from_secs_f64(s.clamp(0.0, 30.0)));
                let pause = match header("x-ratelimit-remaining") {
                    Some(left) if left <= 0.0 => header("x-ratelimit-reset-after")
                        .map(|s| Duration::from_secs_f64(s.clamp(0.0, 10.0)))
                        .unwrap_or(Duration::from_secs(2)),
                    _ => Duration::ZERO,
                };
                if attempt < MAX_ATTEMPTS && (code == 429 || (500..=599).contains(&code)) {
                    std::thread::sleep(retry_after.unwrap_or_else(|| backoff(attempt)));
                    continue;
                }
                let (status, _, message) = interpret_discord_status(code);
                return (status, message, pause);
            }
            // NOT tried again: Discord may have the message already.
            Err(e) if e.is_timeout() => {
                return (
                    "unknown",
                    "Discord did not answer in time. The message may or may not have arrived.".to_string(),
                    Duration::ZERO,
                )
            }
            Err(e) if e.is_connect() => {
                if attempt < MAX_ATTEMPTS {
                    std::thread::sleep(backoff(attempt));
                    continue;
                }
                return (
                    "failed",
                    "Could not reach Discord - check the internet connection.".to_string(),
                    Duration::ZERO,
                );
            }
            Err(_) => {
                return (
                    "unknown",
                    "The request did not complete, so delivery could not be confirmed.".to_string(),
                    Duration::ZERO,
                )
            }
        }
    }
}

/// 2 s, then 6 s.
fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(2 * 3u64.pow(attempt.saturating_sub(1)))
}

/// The same record sent twice in a row by an impatient double click is one
/// message: a second press within this window is refused.
const MANUAL_COOLDOWN: Duration = Duration::from_secs(5);

fn manual_cooldown(subject: Subject, now: Instant) -> bool {
    static LAST: OnceLock<Mutex<HashMap<Subject, Instant>>> = OnceLock::new();
    let mut last = LAST.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(prev) = last.get(&subject) {
        if now.saturating_duration_since(*prev) < MANUAL_COOLDOWN {
            return true;
        }
    }
    last.insert(subject, now);
    false
}

// ---------------------------------------------------------------------------
// Commands.
// ---------------------------------------------------------------------------

fn send_result(success: bool, status: &str, dest: &str, message: impl Into<String>) -> DiscordSendResult {
    DiscordSendResult { success, status: status.to_string(), destination: dest.to_string(), message: message.into() }
}

/// The "Send to Discord" button on an event, an order, a sale, a finance
/// entry or an email. Waits for Discord's answer, so the button can say
/// whether the message arrived.
#[tauri::command]
pub async fn send_to_discord(state: State<'_, AppState>, kind: String, id: i64) -> AppResult<DiscordSendResult> {
    let subject = Subject::parse(&kind, id)?;
    let (dest, url, post) = {
        let conn = state.db.lock().unwrap();
        let cfg = load_notification_config(&conn)?.discord;
        if !cfg.enabled {
            return Ok(send_result(false, "failed", "", "Discord is off. Turn it on in Settings → Notifications."));
        }
        let Some(post) = post_for(&conn, subject, Occasion::Manual)? else {
            return Err(AppError::NotFound("That record no longer exists.".to_string()));
        };
        let Some((dest, url)) = route(&cfg, &post.routes) else {
            let wanted = post.routes.first().map(|d| d.label()).unwrap_or("a");
            return Ok(send_result(
                false,
                "failed",
                "",
                format!("No {wanted} webhook is set. Add one in Settings → Notifications → Discord."),
            ));
        };
        (dest, url, post)
    };
    if manual_cooldown(subject, Instant::now()) {
        return Ok(send_result(false, "failed", dest.key(), "Just sent - wait a few seconds before sending it again."));
    }
    let (tx, rx) = mpsc::channel();
    enqueue(Job { dest, url, post, manual: true, reply: Some(tx) });
    let answer = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(90))).await;
    Ok(match answer {
        Ok(Ok((status, message))) => {
            let delivered = status == "delivered";
            let text = if delivered { format!("Sent to {}.", dest.label()) } else { message };
            send_result(delivered, status, dest.key(), text)
        }
        _ => send_result(
            false,
            "unknown",
            dest.key(),
            "Still waiting for Discord. Check the channel before sending again.",
        ),
    })
}

/// The newest record a channel would carry - what Settings previews.
fn newest_for(conn: &Connection, dest: DiscordDestination) -> AppResult<Option<(Subject, Occasion)>> {
    let newest = |sql: &str| -> AppResult<Option<i64>> { Ok(conn.query_row(sql, [], |r| r.get(0)).optional()?) };
    Ok(match dest {
        DiscordDestination::Sales => newest("SELECT id FROM sales ORDER BY id DESC LIMIT 1")?.map(|id| (Subject::Sale(id), Occasion::Created)),
        DiscordDestination::Finance => {
            newest("SELECT id FROM finance_entries ORDER BY id DESC LIMIT 1")?.map(|id| (Subject::Finance(id), Occasion::Created))
        }
        DiscordDestination::Inventory => match newest("SELECT id FROM orders ORDER BY id DESC LIMIT 1")? {
            Some(id) => Some((Subject::Order(id), Occasion::Created)),
            None => newest("SELECT id FROM events ORDER BY id DESC LIMIT 1")?.map(|id| (Subject::Event(id), Occasion::Created)),
        },
        // The mail Discord would hear about: an important one, the newest.
        DiscordDestination::Mail => newest(
            "SELECT id FROM mail_messages
              WHERE category IN ('cancellation','event_update','sale_confirmation','purchase_confirmation','payout','refund','order_status')
                AND trashed_at IS NULL
              ORDER BY received_at DESC, id DESC LIMIT 1",
        )
        .unwrap_or(None)
        .map(|id| (Subject::Mail(id), Occasion::MailArrived)),
    })
}

/// What a channel's newest message looks like - exactly the embed TIQR would
/// post, from the newest record it would carry, without posting anything.
#[tauri::command]
pub fn preview_discord(state: State<AppState>, destination: String) -> AppResult<Option<Value>> {
    let dest = DiscordDestination::parse(&destination)?;
    let conn = state.db.lock().unwrap();
    let Some((subject, occasion)) = newest_for(&conn, dest)? else {
        return Ok(None);
    };
    Ok(post_for(&conn, subject, occasion)?.map(|p| p.payload["embeds"][0].clone()))
}

/// The last messages TIQR posted or tried to, newest first.
#[tauri::command]
pub fn get_discord_deliveries() -> Vec<DiscordDelivery> {
    deliveries().lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::models::NotificationConfig;

    fn embed(post: &Post) -> &Value {
        &post.payload["embeds"][0]
    }

    fn all_text(post: &Post) -> String {
        post.payload.to_string()
    }

    fn seed_event(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO events (name, artist_team, venue, city, country, event_date, category, status)
             VALUES ('Oasis Live', 'Oasis', 'Wembley Stadium', 'London', 'UK', '2026-07-26', 'Concert', 'upcoming')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_order(conn: &Connection, event: i64, code: &str, qty: i64, unit: i64, cur: &str) -> i64 {
        conn.execute("INSERT OR IGNORE INTO platforms (name, kind) VALUES ('Ticketmaster', 'both')", []).unwrap();
        let platform: i64 = conn.query_row("SELECT id FROM platforms WHERE name = 'Ticketmaster'", [], |r| r.get(0)).unwrap();
        conn.execute(
            "INSERT INTO orders (code, event_id, platform_id, purchase_date, quantity, unit_price_cents, fees_cents,
                                 total_cost_cents, currency, payment_status, external_reference)
             VALUES (?1, ?2, ?3, '2026-03-01', ?4, ?5, 500, ?4 * ?5 + 500, ?6, 'paid', '45-1234/LON')",
            params![code, event, platform, qty, unit, cur],
        )
        .unwrap();
        let order = conn.last_insert_rowid();
        for i in 0..qty {
            conn.execute(
                "INSERT INTO tickets (code, event_id, order_id, section, row_label, seat, purchase_cost_cents, currency)
                 VALUES (?1, ?2, ?3, '112', 'F', ?4, ?5, ?6)",
                params![format!("{code}-T{i}"), event, order, (14 + i).to_string(), unit, cur],
            )
            .unwrap();
        }
        order
    }

    fn configure(conn: &Connection, f: impl FnOnce(&mut NotificationConfig)) {
        let mut cfg = NotificationConfig::default();
        cfg.discord.enabled = true;
        f(&mut cfg);
        crate::commands::sheets_sync::set_setting(conn, "notification_config", &serde_json::to_string(&cfg).unwrap()).unwrap();
    }

    const HOOK: &str = "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz";

    #[test]
    fn markdown_links_mentions_and_headings_in_record_text_stay_plain_text() {
        let hostile = "# Big **win** [click](https://evil.example) <@123> `x` > quote ~~gone~~ | a_b";
        let out = md(hostile);
        assert!(!out.contains("[click]("), "a masked link survives: {out}");
        assert!(out.contains("\\[click\\]"));
        assert!(out.contains("\\<@123\\>"));
        assert!(out.starts_with("\\#"));
        assert!(out.contains("\\*\\*win\\*\\*"));
        assert!(!out.contains("https://"), "a bare link stays tappable: {out}");
        assert_eq!(line("Line one\nline\ttwo  https://x.y"), "Line one line two https:\u{200B}//x.y");
    }

    #[test]
    fn clipping_counts_characters_and_never_ends_on_an_escape() {
        assert_eq!(clip("Wembley", 10), "Wembley");
        assert_eq!(clip("ŽŽŽŽŽŽ", 4), "ŽŽŽ…");
        assert_eq!(clip("ab\\*cd", 4), "ab…");
    }

    #[test]
    fn money_reads_like_money() {
        assert_eq!(money(123456, "EUR"), "€1,234.56");
        assert_eq!(money(-4500, "eur"), "-€45.00");
        assert_eq!(money(1200, "USD"), "$12.00");
        assert_eq!(money(5, "GBP"), "£0.05");
        assert_eq!(money(150000, "CZK"), "1,500.00 CZK");
        assert_eq!(money(100000000, "EUR"), "€1,000,000.00");
        assert_eq!(money_list(&[("EUR".into(), 12000), ("USD".into(), 4000), ("GBP".into(), 0)]), "€120.00 + $40.00");
    }

    #[test]
    fn dates_read_like_dates() {
        assert_eq!(day("2026-07-26").as_deref(), Some("Sun 26 Jul 2026"));
        assert_eq!(day("2026-07-26T19:30:00").as_deref(), Some("Sun 26 Jul 2026 · 19:30"));
        assert_eq!(day("2026-07-26 19:30").as_deref(), Some("Sun 26 Jul 2026 · 19:30"));
        assert_eq!(day("TBD"), None);
        assert_eq!(relative("2026-07-26").as_deref(), Some("<t:1785067200:R>"));
        assert_eq!(when_line(None), "Date to be announced");
    }

    #[test]
    fn seats_group_by_section_and_row() {
        let t = |s: &str, r: &str, x: &str| (s.to_string(), r.to_string(), x.to_string());
        assert_eq!(seat_lines(&[t("112", "F", "14"), t("112", "F", "15"), t("", "", "")]), "Sec 112 · Row F · 14, 15\n1 ticket");
        assert_eq!(seat_lines(&[t("", "", ""), t("", "", "")]), "");
        let many: Vec<_> = (0..9).map(|i| t(&i.to_string(), "A", "1")).collect();
        assert!(seat_lines(&many).ends_with("+ 3 more"));
    }

    #[test]
    fn an_order_message_carries_the_event_the_money_and_the_seats() {
        let conn = test_conn();
        let event = seed_event(&conn);
        let order = seed_order(&conn, event, "OASIS-001", 2, 11000, "EUR");
        let post = post_for(&conn, Subject::Order(order), Occasion::Created).unwrap().unwrap();
        assert_eq!(post.routes, vec![DiscordDestination::Inventory]);
        let e = embed(&post);
        assert_eq!(e["title"], "Oasis Live");
        assert_eq!(e["author"]["name"], "🛒 New purchase");
        assert_eq!(e["footer"]["text"], "TIQR Manager · OASIS-001");
        assert_eq!(e["color"], BLUE);
        // 2.114.0: "fakt len to najdolezitejsie" - three short lines, no grid.
        assert_eq!(
            e["description"],
            "**2 tickets** bought on **Ticketmaster** for **€225.00**\n\
             📅 Sun 26 Jul 2026 · <t:1785067200:R> · Wembley Stadium, London\n\
             💺 Sec 112 · Row F · 14, 15"
        );
        assert_eq!(e["fields"], json!([]));
        // The branding, the timestamp Discord accepts and no pings.
        assert_eq!(post.payload["username"], "TIQR Manager");
        assert_eq!(post.payload["avatar_url"], LOGO_URL);
        assert_eq!(post.payload["allowed_mentions"]["parse"], json!([]));
        let ts = e["timestamp"].as_str().unwrap();
        assert!(ts.ends_with('Z') && ts.len() == 24, "{ts}");
        assert_eq!(post.title, "New purchase · Oasis Live");
    }

    #[test]
    fn a_sale_message_covers_the_whole_batch_and_shows_profit_only_in_one_currency() {
        let mut conn = test_conn();
        let event = seed_event(&conn);
        let order = seed_order(&conn, event, "OASIS-001", 2, 11000, "EUR");
        let tickets: Vec<i64> = {
            let mut s = conn.prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id").unwrap();
            let rows = s.query_map([order], |r| r.get(0)).unwrap();
            rows.collect::<Result<_, _>>().unwrap()
        };
        let input = crate::models::SaleBatchInput {
            platform_id: None,
            sale_date: "2026-04-02".to_string(),
            payment_status: Some("pending".to_string()),
            buyer_reference: Some("Jane Buyer jane@example.com".to_string()),
            notes: None,
            currency: Some("EUR".to_string()),
            lines: tickets
                .iter()
                .map(|t| crate::models::SaleBatchLineInput { ticket_id: *t, sale_price_cents: 30000, selling_fees_cents: 3000 })
                .collect(),
        };
        let ids = crate::commands::sales::create_sales_batch_impl(&mut conn, &input).unwrap();
        // Any line of the batch gives the same message.
        let post = post_for(&conn, Subject::Sale(ids[1]), Occasion::Created).unwrap().unwrap();
        assert_eq!(post.routes, vec![DiscordDestination::Sales]);
        let e = embed(&post);
        // 540 net - 220 cost = 320, on 220 spent.
        assert_eq!(
            e["description"],
            "**2 tickets** sold for **€600.00** · pending\n\
             Profit **€320.00** · ROI 145%\n\
             📅 Sun 26 Jul 2026 · <t:1785067200:R>\n\
             💺 Sec 112 · Row F · 14, 15"
        );
        assert_eq!(e["fields"], json!([]));
        assert_eq!(e["color"], GREEN);
        // Who bought is not Discord's business.
        assert!(!all_text(&post).contains("jane"), "{}", all_text(&post));

        // In another currency the profit is not made up.
        conn.execute("UPDATE sales SET currency = 'USD' WHERE id = ?1", [ids[0]]).unwrap();
        let post = post_for(&conn, Subject::Sale(ids[0]), Occasion::Manual).unwrap().unwrap();
        let d = embed(&post)["description"].as_str().unwrap().to_string();
        assert!(d.starts_with("**2 tickets** sold for **$300.00 + €300.00**"), "{d}");
        assert!(!d.contains("Profit"), "no profit made up across currencies: {d}");
        assert_eq!(embed(&post)["author"]["name"], "💰 Sale");
    }

    #[test]
    fn a_finance_message_says_in_or_out_and_where() {
        let conn = test_conn();
        conn.execute("INSERT INTO finance_categories (name, kind) VALUES ('Fuel', 'expense')", []).unwrap();
        let cat = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO finance_entries (entry_type, entry_date, amount_cents, currency, scope, category_id, place, note)
             VALUES ('expense', '2026-05-04', 4550, 'EUR', 'business', ?1, 'Shell', 'Trip to **Vienna**')",
            [cat],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        let post = post_for(&conn, Subject::Finance(id), Occasion::Created).unwrap().unwrap();
        let e = embed(&post);
        assert_eq!(post.routes, vec![DiscordDestination::Finance]);
        assert_eq!(e["author"]["name"], "💸 New expense");
        assert_eq!(e["title"], "Fuel");
        assert_eq!(e["description"], "**-€45.50** out · Shell\n📅 Mon 4 May 2026 · Business\nTrip to \\*\\*Vienna\\*\\*");
        assert_eq!(e["color"], RED);
        assert_eq!(e["fields"], json!([]));
    }

    #[test]
    fn an_event_message_counts_its_tickets() {
        let conn = test_conn();
        let event = seed_event(&conn);
        seed_order(&conn, event, "OASIS-001", 3, 10000, "EUR");
        conn.execute("UPDATE tickets SET status = 'sold' WHERE seat = '14'", []).unwrap();
        let post = post_for(&conn, Subject::Event(event), Occasion::Manual).unwrap().unwrap();
        assert_eq!(embed(&post)["author"]["name"], "📅 Event");
        // "Oasis" is already in "Oasis Live", so no artist line.
        assert_eq!(
            embed(&post)["description"],
            "📅 Sun 26 Jul 2026 · <t:1785067200:R> · Wembley Stadium, London, UK\n\
             🎟️ 3 tickets · 2 in stock · 1 sold\n\
             Bought €305.00"
        );
    }

    fn seed_mail(conn: &Connection, category: &str, confidence: &str, subject: &str, received: &str) -> i64 {
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        let gmail_id = format!("18c2f0a1b2c3{n:04x}");
        conn.execute(
            "INSERT INTO mail_messages (provider, provider_message_id, account_email, from_address, from_name, subject,
                                        snippet, received_at, category, confidence, detected_platform, first_seen_at,
                                        updated_at, extracted_amount_cents, extracted_currency, extracted_quantity, otp_code)
             VALUES ('gmail', ?5, 'me@example.com', 'noreply@stubhub.com', 'StubHub', ?1,
                     'Great news! You sold 2 tickets. Your code is 482913.', ?2, ?3, ?4, 'StubHub', ?2, ?2,
                     60000, 'EUR', 2, '482913')",
            params![subject, received, category, confidence, gmail_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn a_mail_summary_is_short_safe_and_goes_where_the_mail_belongs() {
        let conn = test_conn();
        let now = Utc::now().to_rfc3339();
        let id = seed_mail(&conn, "sale_confirmation", "high", "You sold 2 tickets [claim](https://evil.example) 482913", &now);
        let post = post_for(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap();
        assert_eq!(post.routes, vec![DiscordDestination::Sales, DiscordDestination::Mail]);
        let e = embed(&post);
        assert_eq!(e["author"]["name"], "💰 Sale confirmation");
        assert_eq!(e["url"], "https://mail.google.com/mail/u/0/#all/18c2f0a1b2c30000");
        let text = all_text(&post);
        assert!(!text.contains("482913"), "a one-time code left TIQR: {text}");
        assert!(!text.contains("noreply@"), "the sender's full address left TIQR: {text}");
        assert!(!text.contains("https://evil"), "{text}");
        // 2.114.0: one line - who and how much - when that is what is known;
        // the opening words only when nothing better is.
        assert_eq!(e["description"], "**StubHub** · 2 tickets · **€600.00**");
        assert_eq!(e["fields"], json!([]));
        conn.execute("UPDATE mail_messages SET extracted_amount_cents = NULL, extracted_quantity = NULL WHERE id = ?1", [id]).unwrap();
        let post = post_for(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap();
        let d = embed(&post)["description"].as_str().unwrap().to_string();
        assert!(d.starts_with("**StubHub**\n> Great news! You sold 2 tickets."), "{d}");
        assert!(!d.contains("482913"), "{d}");
    }

    /// 2.114.0: "co posielat dalej, ako posielat" - which mail and what of it.
    #[test]
    fn mail_settings_decide_which_mail_goes_and_what_it_carries() {
        let conn = test_conn();
        let mut cfg = load_notification_config(&conn).unwrap();
        cfg.discord.enabled = true;
        cfg.discord.mail = "https://discord.com/api/webhooks/1/abc".into();
        cfg.discord.sales = "https://discord.com/api/webhooks/2/def".into();
        crate::commands::notifications::save_notification_config(&conn, &cfg).unwrap();
        let now = Utc::now().to_rfc3339();
        let id = seed_mail(&conn, "sale_confirmation", "high", "Please send your tickets", &now);
        conn.execute(
            "UPDATE mail_messages SET extract_text = 'Please send your tickets to the buyer.\nBuyer email: anna.k@example.com\nSend by: 14 Nov 2026' WHERE id = ?1",
            [id],
        )
        .unwrap();
        assert!(plan(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().is_some());
        let text = |c: &Connection| all_text(&post_for(c, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap());
        assert!(text(&conn).contains("||anna.k@example.com||"));

        cfg.discord.mail_options.personal = false;
        crate::commands::notifications::save_notification_config(&conn, &cfg).unwrap();
        let t = text(&conn);
        assert!(!t.contains("anna.k"), "personal details off: never sent: {t}");
        assert!(t.contains("Send by 14 Nov 2026"), "{t}");

        cfg.discord.mail_options.key_details = false;
        crate::commands::notifications::save_notification_config(&conn, &cfg).unwrap();
        assert!(!text(&conn).contains("Send by"));

        cfg.discord.mail_options.sales = false;
        crate::commands::notifications::save_notification_config(&conn, &cfg).unwrap();
        assert!(plan(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().is_none(), "sale mail switched off");
    }

    /// 2.113.1: "poslat tam, meno a mail a to daj ako hidden ... a aj cena".
    #[test]
    fn a_mail_summary_lists_its_key_details_and_hides_a_persons_details() {
        let conn = test_conn();
        let now = Utc::now().to_rfc3339();
        let id = seed_mail(&conn, "sale_confirmation", "high", "Please send your tickets to Anna Kovacova", &now);
        conn.execute(
            "UPDATE mail_messages SET snippet = 'Please send the tickets to ANNA.K@example.com before Friday.',
                    extract_text = ?2 WHERE id = ?1",
            params![
                id,
                "Please send your tickets\nBuyer's name: Anna Kovacova\nBuyer email: anna.k@example.com\nSend by: 14 Nov 2026\n\
                 Event: Oasis Live 25\nQuantity: 2\nSale Price: €650.00\nTotal Proceeds: €600.00\nYour code: 482913"
            ],
        )
        .unwrap();
        let post = post_for(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap();
        let e = embed(&post);
        // 2.114.0: a few lines - who and how much, where the tickets go, by
        // when, for which event, and the other price.
        assert_eq!(
            e["description"],
            "**StubHub** · 2 tickets · **€600.00**\n\
             📤 ||Anna Kovacova|| · ||anna.k@example.com||\n\
             ⏰ Send by 14 Nov 2026\n\
             🎫 Oasis Live 25\n\
             💶 Sale Price €650.00"
        );
        assert!(!all_text(&post).contains("482913"), "a one-time code never leaves TIQR");
        assert_eq!(e["title"], "Please send your tickets to •••");
        // Outside a spoiler the details appear nowhere.
        let text = all_text(&post);
        let bare = text.replace("||Anna Kovacova||", "").replace("||anna.k@example.com||", "");
        assert!(!bare.to_lowercase().contains("anna"), "{text}");
    }

    #[test]
    fn a_spoiler_is_never_cut_in_half() {
        let secrets = vec!["anna.k@example.com".to_string()];
        let long = format!("{} send to anna.k@example.com", "word ".repeat(10));
        let cut = md_spoilered(&long, &secrets, 60, true);
        assert!(!cut.contains("anna"), "{cut}");
        assert!(cut.ends_with('…'), "{cut}");
        assert_eq!(md_spoilered("To ANNA.K@example.com!", &secrets, 280, true), "To ||anna.k@example.com||!");
        assert_eq!(md_spoilered("a *b* \u{E000}0\u{E001}", &[], 280, true), "a \\*b\\* 0");
        assert_eq!(replace_ignoring_case("Hi ANNA and anna", "Anna", "•"), "Hi • and •");
        assert_eq!(replace_ignoring_case("İstanbul: Ján NOVÁK", "ján novák", "•"), "İstanbul: •");
        // So many spoilers that the text would outgrow Discord's cut: dots.
        let many = vec!["anna.kovacova@example.com".to_string()];
        let crowded = md_spoilered(&"anna.kovacova@example.com ".repeat(70), &many, 280, true);
        assert!(!crowded.contains("anna") && crowded.contains("•••"), "{crowded}");
        assert!(crowded.chars().count() <= 1400);
    }

    #[test]
    fn only_new_sure_important_mail_with_nothing_made_from_it_is_news() {
        let conn = test_conn();
        let now = Utc::now();
        let fresh = now.to_rfc3339();
        let old = (now - chrono::Duration::days(5)).to_rfc3339();
        let sale = seed_mail(&conn, "sale_confirmation", "medium", "Sold", &fresh);
        assert!(mail_is_news(&conn, sale, now));
        let backlog = seed_mail(&conn, "sale_confirmation", "high", "Sold long ago", &old);
        assert!(!mail_is_news(&conn, backlog, now), "history is not news");
        let unsure = seed_mail(&conn, "refund", "low", "Refund?", &fresh);
        assert!(!mail_is_news(&conn, unsure, now));
        let code = seed_mail(&conn, "account_notice", "high", "Your code", &fresh);
        assert!(!mail_is_news(&conn, code, now), "a sign-in code is never announced");
        let status = seed_mail(&conn, "order_status", "medium", "Transfer", &fresh);
        assert!(!mail_is_news(&conn, status, now), "order updates need a sure match");
        conn.execute("UPDATE mail_messages SET created_kind = 'sale', created_id = 1 WHERE id = ?1", [sale]).unwrap();
        assert!(!mail_is_news(&conn, sale, now), "the record's own message says it");
        let spam = seed_mail(&conn, "payout", "high", "Payout", &fresh);
        conn.execute("UPDATE mail_messages SET labels = 'spam' WHERE id = ?1", [spam]).unwrap();
        assert!(!mail_is_news(&conn, spam, now));
    }

    #[test]
    fn nothing_is_planned_while_discord_or_the_kind_is_off_or_no_channel_fits() {
        let conn = test_conn();
        let event = seed_event(&conn);
        let order = seed_order(&conn, event, "OASIS-001", 1, 1000, "EUR");
        // Nothing set up.
        assert!(plan(&conn, Subject::Order(order), Occasion::Created).unwrap().is_none());
        // On, but only a Sales webhook: an order has nowhere to go.
        configure(&conn, |c| c.discord.sales = HOOK.to_string());
        assert!(plan(&conn, Subject::Order(order), Occasion::Created).unwrap().is_none());
        // An Inventory webhook: it goes there.
        configure(&conn, |c| c.discord.inventory = HOOK.to_string());
        let (dest, url, _) = plan(&conn, Subject::Order(order), Occasion::Created).unwrap().unwrap();
        assert_eq!((dest, url.as_str()), (DiscordDestination::Inventory, HOOK));
        // Orders switched off.
        configure(&conn, |c| {
            c.discord.inventory = HOOK.to_string();
            c.discord.auto.orders = false;
        });
        assert!(plan(&conn, Subject::Order(order), Occasion::Created).unwrap().is_none());
        // Discord off.
        configure(&conn, |c| {
            c.discord.inventory = HOOK.to_string();
            c.discord.enabled = false;
        });
        assert!(plan(&conn, Subject::Order(order), Occasion::Created).unwrap().is_none());
        // A deleted record.
        configure(&conn, |c| c.discord.inventory = HOOK.to_string());
        assert!(plan(&conn, Subject::Order(9999), Occasion::Created).unwrap().is_none());
    }

    #[test]
    fn mail_falls_back_to_the_mail_channel_when_its_own_is_not_set() {
        let conn = test_conn();
        let id = seed_mail(&conn, "payout", "high", "Payout sent", &Utc::now().to_rfc3339());
        configure(&conn, |c| c.discord.mail = HOOK.to_string());
        let (dest, _, _) = plan(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap();
        assert_eq!(dest, DiscordDestination::Mail);
        configure(&conn, |c| {
            c.discord.mail = HOOK.to_string();
            c.discord.finance = HOOK.to_string();
        });
        let (dest, _, _) = plan(&conn, Subject::Mail(id), Occasion::MailArrived).unwrap().unwrap();
        assert_eq!(dest, DiscordDestination::Finance);
    }

    #[test]
    fn a_config_from_before_this_version_posts_every_kind() {
        let old = r#"{"desktopEnabled":false,"ntfy":{"enabled":false,"topic":""},"discord":{"enabled":true,"sales":"","finance":"","inventory":""}}"#;
        let cfg: NotificationConfig = serde_json::from_str(old).unwrap();
        let a = &cfg.discord.auto;
        assert!(a.events && a.orders && a.sales && a.finance && a.mail);
        assert_eq!(cfg.discord.mail, "");
    }

    #[test]
    fn a_huge_record_still_fits_in_one_discord_message() {
        let conn = test_conn();
        let long = "x".repeat(10_000);
        conn.execute(
            "INSERT INTO events (name, artist_team, venue, city, country, category) VALUES (?1, ?1, ?1, ?1, ?1, ?1)",
            [&long],
        )
        .unwrap();
        let post = post_for(&conn, Subject::Event(conn.last_insert_rowid()), Occasion::Created).unwrap().unwrap();
        let e = embed(&post);
        let mut total = 0;
        for key in ["title", "description"] {
            total += e[key].as_str().unwrap().chars().count();
        }
        total += e["author"]["name"].as_str().unwrap().chars().count();
        total += e["footer"]["text"].as_str().unwrap().chars().count();
        for f in e["fields"].as_array().unwrap() {
            let (n, v) = (f["name"].as_str().unwrap(), f["value"].as_str().unwrap());
            assert!(v.chars().count() <= 1024 && n.chars().count() <= 256);
            total += n.chars().count() + v.chars().count();
        }
        assert!(total <= 6000, "{total}");
        assert!(e["title"].as_str().unwrap().chars().count() <= 256);
    }

    #[test]
    fn a_double_click_sends_once() {
        let now = Instant::now();
        let s = Subject::Event(424242);
        assert!(!manual_cooldown(s, now));
        assert!(manual_cooldown(s, now + Duration::from_secs(1)));
        assert!(!manual_cooldown(s, now + Duration::from_secs(6)));
        assert!(!manual_cooldown(Subject::Order(424242), now));
    }

    #[test]
    fn a_channel_previews_its_newest_record() {
        let conn = test_conn();
        assert!(newest_for(&conn, DiscordDestination::Sales).unwrap().is_none());
        let event = seed_event(&conn);
        assert_eq!(
            newest_for(&conn, DiscordDestination::Inventory).unwrap(),
            Some((Subject::Event(event), Occasion::Created)),
            "an event when there is no purchase yet"
        );
        let order = seed_order(&conn, event, "OASIS-001", 1, 1000, "EUR");
        assert_eq!(newest_for(&conn, DiscordDestination::Inventory).unwrap(), Some((Subject::Order(order), Occasion::Created)));
        let mail = seed_mail(&conn, "cancellation", "high", "Event cancelled", &Utc::now().to_rfc3339());
        assert_eq!(newest_for(&conn, DiscordDestination::Mail).unwrap(), Some((Subject::Mail(mail), Occasion::MailArrived)));
    }

    #[test]
    fn only_the_five_kinds_can_be_sent() {
        assert_eq!(Subject::parse("sale", 3).unwrap(), Subject::Sale(3));
        assert!(Subject::parse("settings", 1).is_err());
    }
}
