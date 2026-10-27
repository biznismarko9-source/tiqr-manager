//! 2.116.0 - what went wrong, kept where it can be found again.
//!
//! Until this release a failure left a trace only when a screen happened to
//! show it: a background job's error was printed to a console nobody sees,
//! and a panic - the release build aborts on one (`panic = "abort"`) - made
//! the window vanish with nothing written anywhere. Now:
//!
//! * `log` writes one line per event to `<app data>/logs/tiqr.log`: time,
//!   severity, a short id the screen can show, where, and what - redacted
//!   (`redact`) before it is written, so the file is safe to send. At most
//!   1 MB per file; the previous one is kept as `tiqr.1.log`.
//! * a panic hook writes the panic's message and place to the log and to
//!   `last-crash.txt` before the process ends; the next start shows it once
//!   (`take_crash_report`) and keeps it for the diagnostic report.
//! * the screen's own failures (src/lib/diagnostics.ts) arrive through
//!   `report_client_error` and land in the same log.
//! * `export_diagnostics` builds a report that is safe to send: version,
//!   system, schema, record counts (never their content), sync state, the
//!   last log lines and crash notes - every line redacted.
use crate::db::AppState;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::State;

static DIR: OnceLock<PathBuf> = OnceLock::new();
static WRITING: Mutex<()> = Mutex::new(());
const MAX_LOG_BYTES: u64 = 1024 * 1024;
const LOG: &str = "tiqr.log";
const OLD_LOG: &str = "tiqr.1.log";
const LAST_CRASH: &str = "last-crash.txt";
const KEPT_CRASHES: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    fn name(self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Error => "ERROR",
            Severity::Fatal => "FATAL",
        }
    }
}

/// Where the log lives - set once at start, before anything can fail.
pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    if DIR.set(dir).is_ok() {
        install_panic_hook();
    }
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// "E-7K2Q": short enough to read out, enough to find the line.
fn short_id() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    let tail: String = (0..4).map(|_| CHARS[rng.random_range(0..CHARS.len())] as char).collect();
    format!("E-{tail}")
}

/// Whatever could identify a person or open an account, replaced - applied
/// to every line before it is kept or sent. On top of
/// `mail::safe_detail` (tokens and long secrets by their shape): e-mail
/// addresses, `name=value` and `name: value` secrets, inline pictures.
pub fn redact(raw: &str) -> String {
    const SECRET_NAMES: &[&str] = &[
        "access_token", "refresh_token", "id_token", "client_secret", "api_key", "apikey", "key", "token",
        "password", "passwd", "pwd", "secret", "code", "sig", "signature", "auth", "authorization", "cookie",
        "session",
    ];
    let mut words: Vec<String> = Vec::new();
    let mut hide_next = false;
    for word in raw.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let bare = lower.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
        let out = if hide_next {
            hide_next = false;
            "[redacted]".to_string()
        } else if lower.starts_with("data:") && lower.contains(";base64,") {
            "data:[picture]".to_string()
        } else if lower.starts_with("eyj") && word.matches('.').count() >= 2 && word.len() > 30 {
            "[token]".to_string()
        } else if lower.starts_with("sk-") && word.len() > 12 {
            "[key]".to_string()
        } else if is_email(word) {
            "[email]".to_string()
        } else if let Some(cut) = secret_assignment(word, SECRET_NAMES) {
            cut
        } else if (lower.ends_with(':') || lower == "bearer" || lower == "basic") && SECRET_NAMES.contains(&bare)
            || lower == "bearer"
            || lower == "basic"
        {
            // "password: hunter2", "Bearer abc" - the next word is the secret.
            hide_next = true;
            word.to_string()
        } else {
            word.to_string()
        };
        words.push(out);
    }
    let joined = words.join(" ");
    let scrubbed = crate::commands::mail::safe_detail(&joined);
    // safe_detail already bounds the length, on a character boundary.
    scrubbed
}

fn is_email(word: &str) -> bool {
    let w = word.trim_matches(|c: char| matches!(c, '<' | '>' | '(' | ')' | ',' | ';' | '"' | '\'' | '.' | ':'));
    match w.split_once('@') {
        Some((user, host)) => !user.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.'),
        None => false,
    }
}

/// `token=abc`, `"password":"x"`, `?code=4/0Ae&state=1` - the value hidden,
/// the name kept, so the line still says what failed.
fn secret_assignment(word: &str, names: &[&str]) -> Option<String> {
    let lower = word.to_ascii_lowercase();
    for sep in ['=', ':'] {
        let mut out = String::new();
        let mut changed = false;
        for (i, part) in word.split(['&', '?', ',', ';']).enumerate() {
            let pl = part.to_ascii_lowercase();
            if i > 0 {
                out.push('&');
            }
            match pl.find(sep) {
                Some(at) => {
                    let name = pl[..at].trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
                    if names.contains(&name) && at + 1 < part.len() {
                        out.push_str(&part[..at + 1]);
                        out.push_str("[redacted]");
                        changed = true;
                    } else {
                        out.push_str(part);
                    }
                }
                None => out.push_str(part),
            }
        }
        if changed && lower.contains(sep) {
            return Some(out);
        }
    }
    None
}

fn append(dir: &Path, line: &str) {
    let path = dir.join(LOG);
    if fs::metadata(&path).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = fs::rename(&path, dir.join(OLD_LOG));
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// One line in the log - redacted - and the id it was given.
pub fn log(severity: Severity, area: &str, message: &str) -> String {
    let id = short_id();
    let line = format!("{} {:<7} [{}] {}: {}\n", now(), severity.name(), id, redact(area), redact(message));
    eprint!("TIQR {line}");
    if let Some(dir) = DIR.get() {
        let _guard = WRITING.lock().unwrap_or_else(|e| e.into_inner());
        append(dir, &line);
    }
    id
}

/// What a panic says, and where. Takes the pieces rather than the hook's
/// own argument, whose type was renamed in Rust 1.81 - this crate still
/// builds on 1.77 (`rust-version`).
fn panic_text(payload: &(dyn std::any::Any + Send), location: Option<&std::panic::Location<'_>>) -> String {
    let what = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown".to_string());
    let place = location.map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_else(|| "unknown".to_string());
    let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
    format!(
        "TIQR stopped unexpectedly\nwhen: {}\nversion: {}\nthread: {}\nwhere: {}\nwhat: {}\n",
        now(),
        env!("CARGO_PKG_VERSION"),
        thread,
        place,
        redact(&what)
    )
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best effort, and never waiting: the panic may have happened while
        // this very thread was writing the log.
        if let Some(dir) = DIR.get() {
            let text = panic_text(info.payload(), info.location());
            let _ = fs::write(dir.join(LAST_CRASH), &text);
            if let Ok(_guard) = WRITING.try_lock() {
                append(dir, &format!("{} FATAL   [crash] panic: {}\n", now(), text.replace('\n', " | ")));
            }
        }
        previous(info);
    }));
}

/// Keeps the last crash note as `crash-<time>.txt`, at most five of them.
fn keep_crash(dir: &Path, text: &str) {
    // Time first, so the names sort in order; the id, so two in one
    // millisecond are still two files.
    let name = format!("crash-{}-{}.txt", chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f"), short_id());
    let _ = fs::write(dir.join(name), text);
    let mut kept: Vec<PathBuf> = fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("crash-")))
                .collect()
        })
        .unwrap_or_default();
    kept.sort();
    while kept.len() > KEPT_CRASHES {
        let _ = fs::remove_file(kept.remove(0));
    }
}

fn take_crash_in(dir: &Path) -> Option<String> {
    let path = dir.join(LAST_CRASH);
    let text = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    keep_crash(dir, &text);
    Some(text)
}

fn tail_lines(path: &Path, n: usize) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|l| redact(l)).collect()
}

fn count(conn: &rusqlite::Connection, sql: &str) -> String {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0)).map(|n| n.to_string()).unwrap_or_else(|_| "?".to_string())
}

fn text(conn: &rusqlite::Connection, sql: &str) -> String {
    conn.query_row(sql, [], |r| r.get::<_, Option<String>>(0))
        .ok()
        .flatten()
        .map(|s| redact(&s))
        .unwrap_or_else(|| "-".to_string())
}

/// The report. Counts and states only - no names, no addresses, no amounts.
pub(crate) fn report(conn: &rusqlite::Connection, dir: Option<&Path>) -> String {
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    line("TIQR Manager - diagnostic report (redacted: no names, e-mail addresses, keys or record contents)".into());
    line(format!("Generated: {}", now()));
    line(format!("Version: {} ({} {})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
    line(format!(
        "Schema: {} migrations, newest {}",
        count(conn, "SELECT COUNT(*) FROM schema_migrations"),
        text(conn, "SELECT MAX(version) FROM schema_migrations")
    ));
    line(format!(
        "Database: {} pages of {} bytes, {} free",
        count(conn, "SELECT page_count FROM pragma_page_count()"),
        count(conn, "SELECT page_size FROM pragma_page_size()"),
        count(conn, "SELECT freelist_count FROM pragma_freelist_count()")
    ));
    line(format!(
        "Records: {} events, {} orders, {} tickets, {} sales, {} pulls, {} notes",
        count(conn, "SELECT COUNT(*) FROM events"),
        count(conn, "SELECT COUNT(*) FROM orders"),
        count(conn, "SELECT COUNT(*) FROM tickets"),
        count(conn, "SELECT COUNT(*) FROM sales"),
        count(conn, "SELECT COUNT(*) FROM pulls"),
        count(conn, "SELECT COUNT(*) FROM notes"),
    ));
    line(format!(
        "Live sync: on since {}, {} waiting to send, {} waiting for their record, last error: {}",
        text(conn, "SELECT seeded_at FROM sync_control WHERE id = 1"),
        count(conn, "SELECT COUNT(*) FROM sync_outbox"),
        count(conn, "SELECT COUNT(*) FROM sync_pending"),
        text(conn, "SELECT last_error FROM sync_control WHERE id = 1"),
    ));
    line(format!(
        "Drive backup: last {}, last error: {}",
        text(conn, "SELECT value FROM app_settings WHERE key = 'cloud_sync_last_sync_at'"),
        text(conn, "SELECT value FROM app_settings WHERE key = 'cloud_sync_last_error'"),
    ));
    line(format!("Mail: {} messages kept on this computer", count(conn, "SELECT COUNT(*) FROM mail.mail_messages")));
    if let Some(dir) = dir {
        let mut crashes: Vec<PathBuf> = fs::read_dir(dir)
            .map(|r| {
                r.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("crash-")))
                    .collect()
            })
            .unwrap_or_default();
        crashes.sort();
        line(String::new());
        line(format!("Crash notes ({}):", crashes.len()));
        for c in crashes {
            for l in fs::read_to_string(&c).unwrap_or_default().lines() {
                line(format!("  {}", redact(l)));
            }
        }
        line(String::new());
        line("Last log lines:".into());
        for l in tail_lines(&dir.join(OLD_LOG), 60).into_iter().chain(tail_lines(&dir.join(LOG), 200)) {
            line(format!("  {l}"));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Commands.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReport {
    pub id: String,
    pub severity: Severity,
    #[serde(rename = "where")]
    pub where_: String,
    pub message: String,
    pub count: i64,
}

/// A failure on the screen, into the same log - redacted again here: the
/// screen already did it, and nothing is trusted to have.
#[tauri::command]
pub fn report_client_error(report: ClientReport) {
    let times = if report.count > 1 { format!(" (x{})", report.count) } else { String::new() };
    let id: String = report.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(8).collect();
    let line = format!(
        "{} {:<7} [{}] screen {}: {}{}\n",
        now(),
        report.severity.name(),
        id,
        redact(&report.where_),
        redact(&report.message),
        times
    );
    if let Some(dir) = DIR.get() {
        let _guard = WRITING.lock().unwrap_or_else(|e| e.into_inner());
        append(dir, &line);
    }
}

/// The crash note from the last run, once - then kept with the others.
#[tauri::command]
pub fn take_crash_report() -> Option<String> {
    DIR.get().and_then(|d| take_crash_in(d)).map(|t| redact(&t.replace('\n', " | ")))
}

#[tauri::command]
pub fn export_diagnostics(state: State<AppState>) -> AppResult<String> {
    let conn = state.db.lock().unwrap();
    Ok(report(&conn, DIR.get().map(|d| d.as_path())))
}

/// Saves the report where the person chose (the dialog gave the path).
#[tauri::command]
pub fn save_diagnostics(state: State<AppState>, path: String) -> AppResult<()> {
    let body = {
        let conn = state.db.lock().unwrap();
        report(&conn, DIR.get().map(|d| d.as_path()))
    };
    fs::write(&path, body).map_err(|e| crate::error::AppError::Io(format!("the report could not be saved: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_that_opens_an_account_or_names_a_person_reaches_the_log() {
        let cases = [
            ("mail to marko.novak+tickets@gmail.com failed", "marko.novak"),
            ("GET https://x.example/cb?code=4/0AeanS0aB1cD2&state=abc", "4/0AeanS0aB1cD2"),
            ("refresh_token=1//09abcdefGHIJKLmnopQRST and more", "1//09abcdef"),
            ("{\"access_token\":\"ya29.a0AfH6SMBxyz123\"}", "ya29.a0AfH6SMBxyz123"),
            ("Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjM0In0.c2lnbmF0dXJlLXZhbHVl", "eyJhbGciOiJSUzI1NiJ9"),
            ("key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123 rejected", "sk-ant-api03"),
            ("password: hunter2", "hunter2"),
            ("picture data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB too big", "iVBORw0KGgo"),
        ];
        for (raw, secret) in cases {
            let out = redact(raw);
            assert!(!out.contains(secret), "{raw} -> {out}");
        }
        assert_eq!(redact("Event #12 not found"), "Event #12 not found");
        assert!(redact(&"é".repeat(5000)).len() <= 410, "bounded, on a character boundary");
    }

    #[test]
    fn a_crash_note_is_shown_once_and_kept_with_at_most_five_others() {
        let dir = std::env::temp_dir().join(format!("tiqr_diag_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(take_crash_in(&dir).is_none());
        for i in 0..7 {
            fs::write(dir.join(LAST_CRASH), format!("TIQR stopped unexpectedly\nwhat: test {i}\n")).unwrap();
            assert!(take_crash_in(&dir).is_some_and(|t| t.contains(&format!("test {i}"))));
            assert!(take_crash_in(&dir).is_none(), "shown once");
        }
        let kept = fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("crash-")).count();
        assert_eq!(kept, KEPT_CRASHES);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_panic_is_written_down_with_its_place_and_without_its_secrets() {
        let seen = std::sync::Arc::new(Mutex::new(String::new()));
        let keep = seen.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let text = panic_text(info.payload(), info.location());
            // The hook is the whole process's: only this test's panic counts.
            if text.contains("diag-test-marker") {
                *keep.lock().unwrap_or_else(|e| e.into_inner()) = text;
            }
        }));
        let _ = std::panic::catch_unwind(|| panic!("diag-test-marker token=abcdef123456 for someone@example.com"));
        std::panic::set_hook(previous);
        let text = seen.lock().unwrap().clone();
        assert!(text.starts_with("TIQR stopped unexpectedly"), "{text}");
        assert!(text.contains("where: src/diag.rs:"), "{text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
        assert!(!text.contains("abcdef123456"), "{text}");
        assert!(!text.contains("someone@example.com"), "{text}");
    }

    #[test]
    fn the_log_turns_over_at_its_size_and_keeps_one_old_file() {
        let dir = std::env::temp_dir().join(format!("tiqr_diag_log_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let big = "x".repeat(1024);
        for _ in 0..1100 {
            append(&dir, &format!("{big}\n"));
        }
        let now = fs::metadata(dir.join(LOG)).unwrap().len();
        assert!(now <= MAX_LOG_BYTES + 2048, "{now}");
        assert!(dir.join(OLD_LOG).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_report_holds_counts_and_states_never_contents() {
        let conn = crate::db::test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('A very private event name')", []).unwrap();
        conn.execute(
            "UPDATE sync_control SET last_error = 'refused for marko@example.com token=abcdef123456' WHERE id = 1",
            [],
        )
        .unwrap();
        let r = report(&conn, None);
        assert!(r.contains("1 events"));
        assert!(!r.contains("A very private event name"));
        assert!(!r.contains("marko@example.com"));
        assert!(!r.contains("abcdef123456"));
        assert!(r.contains("Schema: 48 migrations"));
    }
}
