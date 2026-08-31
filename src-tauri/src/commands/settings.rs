use crate::db::AppState;
use crate::error::AppResult;
use rusqlite::{params, Connection};
use tauri::State;

/// Generic key/value storage backed by the `app_settings` table, used for
/// small UI preferences (e.g. theme) that don't warrant their own column or
/// command. Not for business data - everything else in this app has a
/// proper typed table and command.

#[tauri::command]
pub fn get_app_setting(state: State<AppState>, key: String) -> AppResult<Option<String>> {
    let conn = state.db.lock().unwrap();
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .ok();
    Ok(value)
}

#[tauri::command]
pub fn set_app_setting(state: State<AppState>, key: String, value: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// --- Anthropic API key (2.1.6) ---------------------------------------------
// marko's own request - "kludne mozme pridat anthropic api keby vedel
// pomoct" (we could go ahead and add the Anthropic API if it could help) -
// for the new AI-assisted price-extraction fallback; see
// commands::price_checker_auto's module doc comment ("AI-assisted
// extraction fallback") for exactly when and how this key gets used.
//
// Deliberately its OWN table (`app_secrets`, migrations/
// 017_price_checker_viagogo.sql), never the generic `app_settings` KV store
// above, even though the shape looks identical: `app_settings` is read back
// to the frontend VERBATIM by `get_app_setting` (already used for things
// like the remembered dashboard tab, useListTab.ts), and a real secret
// marko pastes in here must never be reachable that way. The three
// functions below are the ONLY code anywhere in this app that ever touches
// `app_secrets` - `get_anthropic_api_key_configured`/`set_anthropic_api_key`
// are the only two Tauri commands, and neither one ever returns the actual
// stored value to the frontend, only whether a key is currently set (same
// "presence flag, never the value" convention Settings.tsx's ntfy topic
// field already established, commands::notifications). The third,
// `read_anthropic_api_key`, is `pub(crate)`, never a `#[tauri::command]` -
// genuinely unreachable from the frontend - and is the one thing
// commands::price_checker_auto actually calls to get the real value.

pub(crate) const ANTHROPIC_API_KEY_SETTING: &str = "anthropic_api_key";

pub(crate) fn get_anthropic_api_key_configured_impl(conn: &Connection) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_secrets WHERE key = ?1 AND value != '')",
        params![ANTHROPIC_API_KEY_SETTING],
        |r| r.get(0),
    )?)
}

#[tauri::command]
pub fn get_anthropic_api_key_configured(state: State<AppState>) -> AppResult<bool> {
    let conn = state.db.lock().unwrap();
    get_anthropic_api_key_configured_impl(&conn)
}

/// A blank/whitespace-only `key` clears whatever is stored - same "blank
/// means clear" convention `save_event_marketplace_link_impl`
/// (commands::price_checker) already uses for a saved URL. Trims before
/// storing, so a stray leading/trailing space/newline from a copy-paste
/// never silently breaks the real key.
pub(crate) fn set_anthropic_api_key_impl(conn: &Connection, key: &str) -> AppResult<()> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        conn.execute("DELETE FROM app_secrets WHERE key = ?1", params![ANTHROPIC_API_KEY_SETTING])?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO app_secrets (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![ANTHROPIC_API_KEY_SETTING, trimmed],
    )?;
    Ok(())
}

#[tauri::command]
pub fn set_anthropic_api_key(state: State<AppState>, key: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    set_anthropic_api_key_impl(&conn, &key)
}

/// The one place the real key value is ever read back out - called only
/// from `commands::price_checker_auto`'s AI-extraction fallback, never
/// exposed as a command. `None` when nothing is configured (or it's
/// somehow blank), exactly like every other "not set" case in this module.
pub(crate) fn read_anthropic_api_key(conn: &Connection) -> Option<String> {
    conn.query_row("SELECT value FROM app_secrets WHERE key = ?1", params![ANTHROPIC_API_KEY_SETTING], |r| {
        r.get::<_, String>(0)
    })
    .ok()
    .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod anthropic_api_key_tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn not_configured_and_unreadable_when_nothing_is_stored() {
        let conn = test_conn();
        assert!(!get_anthropic_api_key_configured_impl(&conn).unwrap());
        assert_eq!(read_anthropic_api_key(&conn), None);
    }

    #[test]
    fn setting_a_key_makes_it_configured_and_readable() {
        let conn = test_conn();
        set_anthropic_api_key_impl(&conn, "sk-ant-fake-test-key").unwrap();
        assert!(get_anthropic_api_key_configured_impl(&conn).unwrap());
        assert_eq!(read_anthropic_api_key(&conn).as_deref(), Some("sk-ant-fake-test-key"));
    }

    #[test]
    fn setting_a_key_twice_overwrites_rather_than_erroring_or_duplicating() {
        let conn = test_conn();
        set_anthropic_api_key_impl(&conn, "sk-ant-old").unwrap();
        set_anthropic_api_key_impl(&conn, "sk-ant-new").unwrap();
        assert_eq!(read_anthropic_api_key(&conn).as_deref(), Some("sk-ant-new"));
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM app_secrets", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "must upsert, never duplicate");
    }

    #[test]
    fn a_blank_key_clears_whatever_was_stored() {
        let conn = test_conn();
        set_anthropic_api_key_impl(&conn, "sk-ant-fake-test-key").unwrap();
        set_anthropic_api_key_impl(&conn, "   ").unwrap();
        assert!(!get_anthropic_api_key_configured_impl(&conn).unwrap());
        assert_eq!(read_anthropic_api_key(&conn), None);
    }

    #[test]
    fn a_key_with_surrounding_whitespace_is_trimmed_before_storing() {
        let conn = test_conn();
        set_anthropic_api_key_impl(&conn, "  sk-ant-fake-test-key  \n").unwrap();
        assert_eq!(read_anthropic_api_key(&conn).as_deref(), Some("sk-ant-fake-test-key"));
    }
}
