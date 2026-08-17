use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// Shared app state: a single mutex-guarded SQLite connection.
/// The app is single-user/local-first, so a single serialized connection
/// is simple and safe, and plenty fast for tens of thousands of rows.
pub struct AppState {
    pub db: Mutex<Connection>,
}

const MIGRATIONS: &[(&str, &str)] = &[(
    "001_initial_schema",
    include_str!("../migrations/001_initial_schema.sql"),
)];

/// Resolves the per-user, per-installation database file path.
/// On Windows this lives under `%APPDATA%\com.tiqrmanager.app\` (never inside
/// the Program Files install folder), on Linux under `~/.local/share/...`.
pub fn resolve_db_path(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("tiqr-manager.sqlite3"))
}

pub fn open_connection(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // journal_mode returns a row with the resulting mode, so query_row it explicitly.
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

/// Very small forward-only migration runner. Each migration is applied at
/// most once, tracked in `schema_migrations`. Future features (invoices,
/// payments, fx rates, ...) just add a new `002_xxx.sql` file + array entry.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
         );",
    )?;
    for (version, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |r| r.get(0),
        )?;
        if already {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [version],
        )?;
    }
    Ok(())
}
