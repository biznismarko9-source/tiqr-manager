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

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial_schema",
        include_str!("../migrations/001_initial_schema.sql"),
    ),
    (
        "002_refunds",
        include_str!("../migrations/002_refunds.sql"),
    ),
];

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

/// In-memory, fully-migrated connection for unit tests. Foreign keys are
/// enabled (in-memory DBs default them off, same as any SQLite connection)
/// so delete-safety / cascade behaviour is exercised exactly like production.
#[cfg(test)]
pub fn test_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign keys");
    run_migrations(&conn).expect("run migrations");
    conn
}

/// Performance sanity check (section 10 of the stability pass): seeds a
/// realistic-shape, disk-backed (WAL mode, same pragmas as production)
/// database at 10k/50k/100k tickets and times the exact query shapes the
/// Dashboard, Tickets, Sales and Orders screens run, plus checks the
/// filtered queries hit an index instead of a full table scan.
///
/// Ignored by default (`cargo test` stays fast) - run explicitly with:
///   cargo test --release -- --ignored --nocapture perf_smoke
#[cfg(test)]
mod perf_smoke {
    use super::*;
    use std::time::Instant;

    struct TempDb {
        path: std::path::PathBuf,
    }
    impl TempDb {
        fn new(tag: &str) -> Self {
            let pid = std::process::id();
            let path = std::env::temp_dir().join(format!("tiqr_perf_{tag}_{pid}.sqlite3"));
            let _ = std::fs::remove_file(&path);
            TempDb { path }
        }
    }
    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-shm"));
        }
    }

    /// Seeds `n_tickets` tickets spread across ~`n_tickets/50` orders and
    /// ~`n_tickets/200` events, with ~70% of tickets sold (so the sales
    /// table is large too), matching the general shape of a real reseller
    /// database rather than one giant order.
    fn seed(conn: &mut Connection, n_tickets: i64) {
        let tx = conn.transaction().unwrap();
        let n_events = (n_tickets / 200).max(1);
        let n_orders = (n_tickets / 50).max(1);
        for i in 0..n_events {
            tx.execute(
                "INSERT INTO events (name, event_date, status) VALUES (?1, ?2, 'upcoming')",
                rusqlite::params![
                    format!("Perf Event {i}"),
                    format!("2026-{:02}-01", (i % 12) + 1)
                ],
            )
            .unwrap();
        }
        for i in 0..n_orders {
            let event_id = (i % n_events) + 1;
            tx.execute(
                "INSERT INTO orders (code, event_id, purchase_date, quantity, unit_price_cents, fees_cents, total_cost_cents, currency)
                 VALUES (?1, ?2, ?3, 1, 5000, 100, 5100, 'EUR')",
                rusqlite::params![format!("PERF-ORD-{i}"), event_id, format!("2026-{:02}-15", (i % 12) + 1)],
            )
            .unwrap();
        }
        for i in 0..n_tickets {
            let order_id = (i % n_orders) + 1;
            let event_id = (i % n_events) + 1;
            let sold = i % 10 < 7; // ~70% sold
            let status = if sold { "sold" } else { "available" };
            tx.execute(
                "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, purchase_fees_cents, currency, status)
                 VALUES (?1, ?2, ?3, 5000, 100, 'EUR', ?4)",
                rusqlite::params![format!("PERF-TKT-{i}"), event_id, order_id, status],
            )
            .unwrap();
            if sold {
                let ticket_id = i + 1;
                tx.execute(
                    "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, selling_fees_cents, currency, payment_status)
                     VALUES (?1, ?2, ?3, 7000, 200, 'EUR', 'paid')",
                    rusqlite::params![format!("PERF-SAL-{i}"), ticket_id, format!("2026-{:02}-20", (i % 12) + 1)],
                )
                .unwrap();
            }
        }
        tx.commit().unwrap();
    }

    fn explain_uses_index(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> String {
        let plan_sql = format!("EXPLAIN QUERY PLAN {sql}");
        let mut stmt = conn.prepare(&plan_sql).unwrap();
        let rows = stmt
            .query_map(params, |r| r.get::<_, String>(3))
            .unwrap();
        rows.map(|r| r.unwrap()).collect::<Vec<_>>().join(" | ")
    }

    fn run_at_scale(n_tickets: i64) {
        let db = TempDb::new(&n_tickets.to_string());
        let mut conn = open_connection(&db.path).expect("open temp db");
        run_migrations(&conn).expect("migrate");

        let t_seed = Instant::now();
        seed(&mut conn, n_tickets);
        eprintln!("[perf {n_tickets}] seed: {:?}", t_seed.elapsed());

        // ---- Dashboard: currency-mix distinct scan -----------------------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*) FROM (SELECT DISTINCT currency FROM tickets)",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] distinct currency: {:?}", t.elapsed());

        // ---- Dashboard: inventory snapshot (filtered by currency) --------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(purchase_cost_cents+purchase_fees_cents+other_costs_cents),0)
             FROM tickets WHERE currency = ?1",
            ["EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] inventory snapshot: {:?}", t.elapsed());
        let plan = explain_uses_index(
            &conn,
            "SELECT COUNT(*) FROM tickets WHERE currency = ?1",
            &[&"EUR"],
        );
        eprintln!("[perf {n_tickets}] plan(tickets by currency): {plan}");

        // ---- Dashboard: period-filtered purchase/sales aggregates --------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(t.id), COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
             FROM tickets t JOIN orders o ON o.id = t.order_id
             WHERE o.purchase_date BETWEEN ?1 AND ?2 AND t.currency = ?3",
            rusqlite::params!["0001-01-01", "9999-12-31", "EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] period purchase aggregate: {:?}", t.elapsed());

        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(s.sale_price_cents),0)
             FROM sales s JOIN tickets t ON t.id = s.ticket_id
             WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'",
            rusqlite::params!["0001-01-01", "9999-12-31", "EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] period sales aggregate: {:?}", t.elapsed());

        // ---- Tickets screen: filtered by event (should use idx_tickets_event) --
        let plan = explain_uses_index(
            &conn,
            "SELECT t.id FROM tickets t WHERE t.event_id = ?1",
            &[&1i64],
        );
        eprintln!("[perf {n_tickets}] plan(tickets by event_id): {plan}");
        assert!(
            plan.contains("USING INDEX") || plan.contains("idx_tickets_event"),
            "tickets-by-event should hit an index, got: {plan}"
        );

        // ---- Tickets screen: unfiltered full list (worst case, no filters) --
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.code, t.event_id, e.name, t.order_id, o.code,
                    t.section, t.row_label, t.seat, t.ticket_type,
                    t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents,
                    t.listing_price_cents, t.currency, t.status, t.notes, t.is_demo,
                    t.created_at, t.updated_at, sa.sale_price_cents
                 FROM tickets t
                 JOIN events e ON e.id = t.event_id
                 JOIN orders o ON o.id = t.order_id
                 LEFT JOIN sales sa ON sa.ticket_id = t.id
                 ORDER BY t.id DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] full unfiltered ticket list ({} rows): {:?}",
            rows.len(),
            t.elapsed()
        );

        // ---- Sales screen: unfiltered full list ---------------------------
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.code, s.ticket_id, t.code, t.event_id, e.name,
                    s.platform_id, p.name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
                    s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
                    s.refunded_at, s.refund_reason,
                    (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents)
                 FROM sales s
                 JOIN tickets t ON t.id = s.ticket_id
                 JOIN events e ON e.id = t.event_id
                 LEFT JOIN platforms p ON p.id = s.platform_id
                 ORDER BY s.sale_date DESC, s.id DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] full unfiltered sales list ({} rows): {:?}",
            rows.len(),
            t.elapsed()
        );

        eprintln!("[perf {n_tickets}] db file size: {} bytes", std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0));
    }

    #[test]
    #[ignore]
    fn perf_10k() {
        run_at_scale(10_000);
    }

    #[test]
    #[ignore]
    fn perf_50k() {
        run_at_scale(50_000);
    }

    #[test]
    #[ignore]
    fn perf_100k() {
        run_at_scale(100_000);
    }
}
