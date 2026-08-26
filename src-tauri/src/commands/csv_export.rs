use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::money::format_cents;
use rusqlite::Connection;
use tauri::State;

fn opt(v: Option<String>) -> String {
    v.unwrap_or_default()
}
fn yesno(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// Split out from the `export_events_csv` command (same "impl function +
/// thin tauri::command wrapper" pattern as `export_sales_csv`/
/// `write_sales_csv`) so it's directly unit-testable and shared with the new
/// 1.9.1 "Export selected" wrapper below - `ids: None` means "every event"
/// (unchanged pre-1.9.1 behaviour), `Some(ids)` restricts to exactly those.
fn export_events_csv_impl(conn: &Connection, path: &str, ids: Option<&[i64]>) -> AppResult<i64> {
    let mut sql = "SELECT id, name, artist_team, venue, city, country, event_date, category, status, notes, is_demo, created_at
         FROM events"
        .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(ids) = ids {
        if ids.is_empty() {
            return Err(AppError::Validation("Select at least one event to export".into()));
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" WHERE id IN ({placeholders})"));
        for i in ids {
            params_vec.push(Box::new(*i));
        }
    }
    sql.push_str(" ORDER BY id");

    let mut stmt = conn.prepare(&sql)?;
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "id", "name", "artist_team", "venue", "city", "country", "event_date", "category",
        "status", "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query(param_refs.as_slice())?;
    while let Some(row) = rows.next()? {
        wtr.write_record([
            row.get::<_, i64>(0)?.to_string(),
            row.get::<_, String>(1)?,
            opt(row.get(2)?),
            opt(row.get(3)?),
            opt(row.get(4)?),
            opt(row.get(5)?),
            opt(row.get(6)?),
            opt(row.get(7)?),
            row.get::<_, String>(8)?,
            opt(row.get(9)?),
            yesno(row.get(10)?).to_string(),
            row.get::<_, String>(11)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

#[tauri::command]
pub fn export_events_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_events_csv_impl(&conn, &path, None)
}

/// 1.9.1: "pick specific events" export for the new Settings -> Data picker -
/// same idea as `export_sales_csv_selected` (1.8.0), just for events.
#[tauri::command]
pub fn export_events_csv_selected(state: State<AppState>, path: String, ids: Vec<i64>) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_events_csv_impl(&conn, &path, Some(&ids))
}

/// Split out from the `export_orders_csv` command - same pattern/reasoning
/// as `export_events_csv_impl` above. `ids: None` means "every order"
/// (unchanged pre-1.9.1 behaviour), `Some(ids)` restricts to exactly those.
fn export_orders_csv_impl(conn: &Connection, path: &str, ids: Option<&[i64]>) -> AppResult<i64> {
    let mut sql = "SELECT o.code, e.name, sup.name, p.name, o.purchase_date, o.quantity,
                o.unit_price_cents, o.fees_cents, o.other_costs_cents, o.total_cost_cents,
                o.currency, o.payment_status, o.notes, o.is_demo, o.created_at
         FROM orders o
         JOIN events e ON e.id = o.event_id
         LEFT JOIN suppliers sup ON sup.id = o.supplier_id
         LEFT JOIN platforms p ON p.id = o.platform_id"
        .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(ids) = ids {
        if ids.is_empty() {
            return Err(AppError::Validation("Select at least one order to export".into()));
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" WHERE o.id IN ({placeholders})"));
        for i in ids {
            params_vec.push(Box::new(*i));
        }
    }
    sql.push_str(" ORDER BY o.id");

    let mut stmt = conn.prepare(&sql)?;
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "order_code", "event", "supplier", "platform", "purchase_date", "quantity",
        "unit_price", "fees", "other_costs", "total_cost", "currency", "payment_status",
        "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query(param_refs.as_slice())?;
    while let Some(row) = rows.next()? {
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            opt(row.get(2)?),
            opt(row.get(3)?),
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?.to_string(),
            format_cents(row.get(6)?),
            format_cents(row.get(7)?),
            format_cents(row.get(8)?),
            format_cents(row.get(9)?),
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            opt(row.get(12)?),
            yesno(row.get(13)?).to_string(),
            row.get::<_, String>(14)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

#[tauri::command]
pub fn export_orders_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_orders_csv_impl(&conn, &path, None)
}

/// 1.9.1: "pick specific orders" export for the new Settings -> Data picker -
/// same idea as `export_sales_csv_selected` (1.8.0), just for orders.
#[tauri::command]
pub fn export_orders_csv_selected(state: State<AppState>, path: String, ids: Vec<i64>) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_orders_csv_impl(&conn, &path, Some(&ids))
}

/// 1.9.1: took a plain `&Connection` instead of `&State<AppState>` from here
/// on (the callers below now lock the db themselves, one extra line each) so
/// this is directly unit-testable - same "impl function unit-testable
/// against a plain Connection" convention as every other command module -
/// and so it can gain an `ids` filter shared by the three commands below.
fn export_tickets_inner(
    conn: &Connection,
    path: &str,
    status_filter: Option<Vec<String>>,
    event_id: Option<i64>,
    ids: Option<&[i64]>,
) -> AppResult<i64> {
    let mut sql = "SELECT t.code, e.name, o.code, t.section, t.row_label, t.seat, t.ticket_type,
                t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents, t.listing_price_cents,
                t.currency, t.status, t.notes, t.is_demo, t.created_at
         FROM tickets t
         JOIN events e ON e.id = t.event_id
         JOIN orders o ON o.id = t.order_id
         WHERE 1=1"
        .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(statuses) = &status_filter {
        if !statuses.is_empty() {
            let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            sql.push_str(&format!(" AND t.status IN ({placeholders})"));
            for s in statuses {
                params_vec.push(Box::new(s.clone()));
            }
        }
    }
    if let Some(eid) = event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(ids) = ids {
        if ids.is_empty() {
            return Err(AppError::Validation("Select at least one ticket to export".into()));
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND t.id IN ({placeholders})"));
        for i in ids {
            params_vec.push(Box::new(*i));
        }
    }
    sql.push_str(" ORDER BY t.id");

    let mut stmt = conn.prepare(&sql)?;
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "ticket_code", "event", "order_code", "section", "row", "seat", "ticket_type",
        "purchase_cost", "purchase_fees", "other_costs", "listing_price", "currency",
        "status", "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query(param_refs.as_slice())?;
    while let Some(row) = rows.next()? {
        let listing_price: Option<i64> = row.get(10)?;
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            opt(row.get(3)?),
            opt(row.get(4)?),
            opt(row.get(5)?),
            opt(row.get(6)?),
            format_cents(row.get(7)?),
            format_cents(row.get(8)?),
            format_cents(row.get(9)?),
            listing_price.map(format_cents).unwrap_or_default(),
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            opt(row.get(13)?),
            yesno(row.get(14)?).to_string(),
            row.get::<_, String>(15)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

#[tauri::command]
pub fn export_tickets_csv(
    state: State<AppState>,
    path: String,
    status: Option<String>,
    event_id: Option<i64>,
) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let statuses = status.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    });
    export_tickets_inner(&conn, &path, statuses, event_id, None)
}

/// "Inventory" = current stock only (available + listed), excluding sold/cancelled.
#[tauri::command]
pub fn export_inventory_csv(
    state: State<AppState>,
    path: String,
    event_id: Option<i64>,
) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_tickets_inner(
        &conn,
        &path,
        Some(vec!["available".to_string(), "listed".to_string()]),
        event_id,
        None,
    )
}

/// 1.9.1: "pick specific tickets" export for the new Settings -> Data picker
/// - same idea as `export_sales_csv_selected` (1.8.0), just for tickets.
/// Also powers the Inventory picker: the frontend only ever offers
/// available/listed tickets to choose from there (mirroring
/// `export_inventory_csv`'s own status restriction above), so this command
/// doesn't need to re-enforce that restriction itself - it exports exactly
/// the ids it's given, the same "trust the already-filtered selection"
/// approach `export_sales_csv_selected` already uses for its group ids.
#[tauri::command]
pub fn export_tickets_csv_selected(state: State<AppState>, path: String, ids: Vec<i64>) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_tickets_inner(&conn, &path, None, None, Some(&ids))
}

#[tauri::command]
pub fn export_sales_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_sales_csv_impl(&conn, &path)
}

/// Shared row-writer behind both `export_sales_csv_impl` (every sale) and
/// `export_sales_csv_selected_impl` (1.8.0, only the selected Sales screen
/// rows). Keeping ONE implementation of the column list and the 1.6.0 audit
/// H6 realized-only profit rule means "Export selected" can never quietly
/// drift from what "Export all" already does - `where_extra` is the only
/// difference between the two callers (e.g. `"WHERE s.id IN (?,?,?)"`, or ""
/// for no filtering at all).
fn write_sales_csv(
    conn: &Connection,
    path: &str,
    where_extra: &str,
    extra_params: &[&dyn rusqlite::ToSql],
) -> AppResult<i64> {
    let sql = format!(
        "SELECT s.code, t.code, e.name, p.name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
                (t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents) as cost_cents,
                s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at,
                t.currency as ticket_currency
         FROM sales s
         JOIN tickets t ON t.id = s.ticket_id
         JOIN events e ON e.id = t.event_id
         LEFT JOIN platforms p ON p.id = s.platform_id
         {where_extra}
         ORDER BY s.id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "sale_code", "ticket_code", "event", "platform", "sale_date", "sale_price",
        "selling_fees", "cost", "profit", "currency", "payment_status", "buyer_reference",
        "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query(extra_params)?;
    while let Some(row) = rows.next()? {
        let sale_price: i64 = row.get(5)?;
        let fees: i64 = row.get(6)?;
        let cost: i64 = row.get(7)?;
        let sale_currency: String = row.get(8)?;
        let payment_status: String = row.get(9)?;
        let ticket_currency: String = row.get(14)?;
        // 2.0.57: New Sale can now record a sale in a currency that differs
        // from its own ticket's purchase currency (see
        // SaleBatchInput::currency) - `cost` above is always in the
        // ticket's currency, so subtracting it from `sale_price` (in
        // `sale_currency`) is only valid when the two agree. Left blank
        // rather than a silently wrong number when they don't - same
        // "never blend currencies" rule map_sale/GROUP_BASE_SELECT already
        // enforce for the in-app views (sales.rs).
        let currency_mismatch = sale_currency != ticket_currency;
        // 1.6.0 audit H6: every other profit total in this app is
        // "realized-only" - a refunded sale contributes 0, never a negative
        // "we lost the fees" or positive "we still made this" number (see
        // finance.rs's doc comment + events.rs/dashboard.rs excluding
        // refunded sales from their joins entirely). This export used to
        // compute sale_price-cost-fees unconditionally, so a refunded row's
        // profit overstated the real, realized total if anyone summed this
        // column. sale_price/fees/cost stay as the row's own historical
        // values either way - payment_status already shows "refunded" so
        // it's clear why profit is 0 despite a nonzero sale_price.
        let profit = if payment_status == "refunded" {
            Some(0)
        } else if currency_mismatch {
            None
        } else {
            Some(sale_price - cost - fees)
        };
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            opt(row.get(3)?),
            row.get::<_, String>(4)?,
            format_cents(sale_price),
            format_cents(fees),
            format_cents(cost),
            profit.map(format_cents).unwrap_or_default(),
            sale_currency,
            payment_status.clone(),
            opt(row.get(10)?),
            opt(row.get(11)?),
            yesno(row.get(12)?).to_string(),
            row.get::<_, String>(13)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

/// Split out from the `export_sales_csv` command (same "impl function + thin
/// tauri::command wrapper" pattern already used by get_dashboard/
/// list_sale_groups) so the 1.6.0 audit H6 fix (refunded rows must not
/// contribute a nonzero profit) is directly unit-testable against a plain
/// `&Connection`, without needing a Tauri `State<AppState>`. Unchanged
/// behavior/signature since 1.6.0 - now just a thin call into the shared
/// `write_sales_csv` (1.8.0), with no extra filter applied.
fn export_sales_csv_impl(conn: &Connection, path: &str) -> AppResult<i64> {
    write_sales_csv(conn, path, "", &[])
}

/// 1.8.0: "Export selected" on the Sales screen. `group_ids` are SaleGroup
/// representative ids (what the frontend has checked) - resolved via
/// `resolve_group_sale_ids` (sales.rs) to every underlying sale line across
/// all of them, so a selected 4-ticket batch exports all 4 lines, not just
/// its representative row. Uses the exact same column layout and H6
/// realized-only profit rule as "Export all" (`write_sales_csv`), just
/// restricted to the resolved ids.
fn export_sales_csv_selected_impl(conn: &Connection, path: &str, group_ids: &[i64]) -> AppResult<i64> {
    if group_ids.is_empty() {
        return Err(AppError::Validation("Select at least one sale to export".into()));
    }
    let ids = crate::commands::sales::resolve_group_sale_ids(conn, group_ids)?;
    if ids.is_empty() {
        return Err(AppError::Validation("None of the selected sales could be found".into()));
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let where_extra = format!("WHERE s.id IN ({placeholders})");
    let param_boxes: Vec<Box<dyn rusqlite::ToSql>> =
        ids.iter().map(|i| Box::new(*i) as Box<dyn rusqlite::ToSql>).collect();
    let param_refs: Vec<&dyn rusqlite::ToSql> = param_boxes.iter().map(|p| p.as_ref()).collect();
    write_sales_csv(conn, path, &where_extra, &param_refs)
}

#[tauri::command]
pub fn export_sales_csv_selected(state: State<AppState>, path: String, ids: Vec<i64>) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    export_sales_csv_selected_impl(&conn, &path, &ids)
}

/// 1.8.3 (section 10): downloadable header template for the CSV import (see
/// csv_import.rs's `parse_rows`) - lets a user build a compatible file from
/// scratch instead of guessing column names. There is only ONE CSV import in
/// this app (orders + their tickets together, see csv_import.rs) - no
/// separate tickets-only or sales-only import exists, so only this one
/// template is offered, rather than inventing formats the app doesn't
/// actually support. Columns and order match the Settings screen's own
/// "Columns: ..." description exactly (the primary/first-recognized name for
/// each field - parse_rows also accepts a few synonyms, e.g. "row_label" for
/// "row", but the template only ever shows the one preferred name so there's
/// no ambiguity about what to type). Includes one filled-in example row so a
/// blank column isn't mistaken for "required" when it's actually optional
/// (e.g. supplier, platform, seats, notes). Doesn't touch the database at
/// all, so it needs no connection/state.
#[tauri::command]
pub fn export_orders_csv_template(path: String) -> AppResult<()> {
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "event", "purchase_date", "supplier", "platform", "quantity", "unit_price", "fees",
        "other_costs", "currency", "payment_status", "ticket_type", "section", "row", "seats", "notes",
    ])?;
    wtr.write_record([
        "Example Event", "2026-01-01", "", "", "2", "45.00", "2.50", "0", "EUR", "unpaid", "",
        "A", "12", "11,12", "",
    ])?;
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::orders::insert_order_with_tickets;
    use crate::commands::sales::{create_sale_impl, create_sales_batch_impl, refund_sale_impl};
    use crate::db::test_conn;
    use crate::models::{OrderInput, SaleBatchInput, SaleBatchLineInput, SaleInput};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Same idea as csv_import.rs's own TempCsv test helper - a unique path
    /// under the OS temp dir, cleaned up on drop. This one is for a path the
    /// export writes TO (csv_import's is for a path already containing
    /// fixture content to read FROM), so it doesn't pre-write anything.
    struct TempCsvPath(std::path::PathBuf);
    impl Drop for TempCsvPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    fn temp_csv_path() -> TempCsvPath {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        TempCsvPath(std::env::temp_dir().join(format!(
            "tiqr-manager-test-export-{}-{n}.csv",
            std::process::id()
        )))
    }

    /// Inserts one event + one order of `qty` tickets (cost 1000 cents each,
    /// EUR) and returns the ticket ids - mirrors the equivalent seed helpers
    /// already used by sales.rs's own test module.
    fn seed_tickets(conn: &mut Connection, qty: i64) -> Vec<i64> {
        conn.execute("INSERT INTO events (name) VALUES ('CSV Export Test Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: qty,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(conn, &input, false).unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM tickets WHERE order_id=?1 ORDER BY id")
            .unwrap();
        stmt.query_map([order_id], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<i64>, _>>()
            .unwrap()
    }

    fn sale_input(ticket_id: i64, price_cents: i64) -> SaleInput {
        SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-01-15".to_string(),
            sale_price_cents: price_cents,
            selling_fees_cents: 100,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        }
    }

    fn read_csv_rows(path: &std::path::Path) -> Vec<Vec<String>> {
        let mut rdr = csv::Reader::from_path(path).unwrap();
        rdr.records()
            .map(|r| r.unwrap().iter().map(|f| f.to_string()).collect())
            .collect()
    }

    #[test]
    fn active_sale_profit_matches_sale_price_minus_cost_minus_fees() {
        // Baseline: an ordinary active sale must keep exporting a real
        // profit exactly as before - only a REFUNDED row is supposed to
        // change (1.6.0 audit H6).
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1); // cost 1000 cents
        create_sale_impl(&mut conn, &sale_input(tickets[0], 2000)).unwrap(); // fees 100

        let out = temp_csv_path();
        let count = export_sales_csv_impl(&conn, out.0.to_str().unwrap()).unwrap();
        assert_eq!(count, 1);

        let rows = read_csv_rows(&out.0);
        assert_eq!(rows.len(), 1);
        // columns: sale_code,ticket_code,event,platform,sale_date,sale_price,
        // selling_fees,cost,profit,currency,payment_status,...
        assert_eq!(rows[0][5], "20.00", "sale_price");
        assert_eq!(rows[0][7], "10.00", "cost");
        assert_eq!(rows[0][8], "9.00", "profit: 20.00 - 10.00 - 1.00 fees");
        assert_eq!(rows[0][10], "paid");
    }

    #[test]
    fn a_sale_whose_currency_mismatches_its_own_tickets_currency_exports_a_blank_profit() {
        // 2.0.57: New Sale can now record a sale in a currency that differs
        // from its own ticket's purchase currency (SaleBatchInput::currency).
        // `cost` here is always in the ticket's currency - subtracting it
        // from a sale_price in a DIFFERENT currency would silently export a
        // meaningless number, so this row's profit must come out blank
        // instead (never a real-looking but wrong figure someone might sum
        // in a spreadsheet) - same "never blend currencies" rule
        // map_sale/GROUP_BASE_SELECT already enforce for the in-app views.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let usd_order = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "USD".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(&mut conn, &usd_order, false).unwrap();
        let usd_ticket: i64 = conn
            .query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| r.get(0))
            .unwrap();

        create_sales_batch_impl(
            &mut conn,
            &SaleBatchInput {
                lines: vec![SaleBatchLineInput { ticket_id: usd_ticket, sale_price_cents: 5000, selling_fees_cents: 0 }],
                platform_id: None,
                sale_date: "2026-02-01".to_string(),
                payment_status: Some("paid".to_string()),
                buyer_reference: None,
                notes: None,
                currency: Some("EUR".to_string()),
            },
        )
        .unwrap();

        let out = temp_csv_path();
        export_sales_csv_impl(&conn, out.0.to_str().unwrap()).unwrap();
        let rows = read_csv_rows(&out.0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][5], "50.00", "sale_price still exports normally, in its own (EUR) currency");
        assert_eq!(rows[0][7], "10.00", "cost still exports normally, in its own (USD) currency");
        assert_eq!(rows[0][8], "", "profit left blank rather than silently subtracting USD cost from EUR revenue");
        assert_eq!(rows[0][9], "EUR", "the currency column reflects the sale's OWN currency");
    }

    #[test]
    fn refunded_sale_exports_zero_profit_instead_of_a_misleading_realized_number() {
        // The exact H6 bug: before this fix, a refunded row still exported
        // sale_price-cost-fees as if it were realized profit, overstating
        // the total if anyone summed the column - unlike every other profit
        // total in this app, which is realized-only (excludes refunds).
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1); // cost 1000 cents
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 2000)).unwrap();
        refund_sale_impl(&mut conn, sale_id, Some("test refund")).unwrap();

        let out = temp_csv_path();
        let count = export_sales_csv_impl(&conn, out.0.to_str().unwrap()).unwrap();
        assert_eq!(count, 1, "refunded sale still appears in the export (full history)");

        let rows = read_csv_rows(&out.0);
        assert_eq!(rows[0][10], "refunded", "payment_status column shows why profit is 0");
        // sale_price/cost/fees stay as this row's own real historical
        // values - only the profit column changes for a refunded row.
        assert_eq!(rows[0][5], "20.00", "sale_price is still the row's real historical value");
        assert_eq!(rows[0][8], "0.00", "profit must be 0, not 9.00 - refunded sales are never realized");
    }

    #[test]
    fn mixed_active_and_refunded_export_sums_to_the_realized_only_total() {
        // Matches the app's own realized-only convention end to end: sum the
        // exported profit column across a mix of active + refunded sales and
        // it must equal just the active line's profit, same as Dashboard/
        // Sales/SaleGroup would report for the same data.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2); // cost 1000 cents each
        create_sale_impl(&mut conn, &sale_input(tickets[0], 2000)).unwrap(); // stays active
        let refunded_id = create_sale_impl(&mut conn, &sale_input(tickets[1], 5000)).unwrap();
        refund_sale_impl(&mut conn, refunded_id, Some("test refund")).unwrap();

        let out = temp_csv_path();
        let count = export_sales_csv_impl(&conn, out.0.to_str().unwrap()).unwrap();
        assert_eq!(count, 2);

        let rows = read_csv_rows(&out.0);
        let total_profit_cents: i64 = rows
            .iter()
            .map(|r| (r[8].parse::<f64>().unwrap() * 100.0).round() as i64)
            .sum();
        // Only the active line: 2000 - 1000 - 100 = 900 cents. The refunded
        // line (5000 sale price) contributes 0, not 5000-1000-100=3900.
        assert_eq!(total_profit_cents, 900);
    }

    // ---- 1.8.0: Export selected ------------------------------------------

    fn batch_input(tickets: &[i64]) -> SaleBatchInput {
        SaleBatchInput {
            lines: tickets
                .iter()
                .map(|&tid| SaleBatchLineInput {
                    ticket_id: tid,
                    sale_price_cents: 2000,
                    selling_fees_cents: 0,
                })
                .collect(),
            platform_id: None,
            sale_date: "2026-01-20".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        }
    }

    #[test]
    fn export_selected_exports_only_the_chosen_groups_full_lines_not_the_whole_table() {
        let mut conn = test_conn();
        let a = seed_tickets(&mut conn, 1);
        let a_id = create_sale_impl(&mut conn, &sale_input(a[0], 1000)).unwrap();
        let a_code: String = conn.query_row("SELECT code FROM tickets WHERE id=?1", [a[0]], |r| r.get(0)).unwrap();

        let b = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &sale_input(b[0], 1000)).unwrap(); // NOT selected - must not appear

        let c = seed_tickets(&mut conn, 2);
        let c_ids = create_sales_batch_impl(&mut conn, &batch_input(&c)).unwrap();
        assert_eq!(c_ids.len(), 2);
        let c_codes: Vec<String> = c
            .iter()
            .map(|&tid| conn.query_row("SELECT code FROM tickets WHERE id=?1", [tid], |r| r.get(0)).unwrap())
            .collect();

        let out = temp_csv_path();
        let count = export_sales_csv_selected_impl(&conn, out.0.to_str().unwrap(), &[a_id, c_ids[0]]).unwrap();
        assert_eq!(count, 3, "the single sale (1 line) plus the whole 2-ticket batch (2 lines) = 3");

        let rows = read_csv_rows(&out.0);
        let exported_ticket_codes: Vec<&String> = rows.iter().map(|r| &r[1]).collect();
        assert!(exported_ticket_codes.contains(&&a_code), "the selected single sale's ticket must be exported");
        for code in &c_codes {
            assert!(exported_ticket_codes.contains(&code), "every line of the selected batch must be exported: {code}");
        }
        assert_eq!(rows.len(), 3, "no extra rows beyond the 3 selected lines");
    }

    #[test]
    fn export_selected_rejects_an_empty_selection() {
        let conn = test_conn();
        let out = temp_csv_path();
        let err = export_sales_csv_selected_impl(&conn, out.0.to_str().unwrap(), &[]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(!out.0.exists(), "nothing should be written to disk for a rejected empty selection");
    }

    #[test]
    fn export_selected_applies_the_same_h6_realized_only_profit_rule() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2); // cost 1000 cents each
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets)).unwrap();
        refund_sale_impl(&mut conn, ids[0], Some("test refund")).unwrap();

        let out = temp_csv_path();
        let count = export_sales_csv_selected_impl(&conn, out.0.to_str().unwrap(), &[ids[0]]).unwrap();
        assert_eq!(count, 2, "selecting either line of the batch resolves to the whole 2-line group");

        let rows = read_csv_rows(&out.0);
        let refunded_row = rows.iter().find(|r| r[10] == "refunded").expect("the refunded line must still be exported");
        assert_eq!(refunded_row[8], "0.00", "refunded line must export 0 profit, same rule as Export all");
        let active_row = rows.iter().find(|r| r[10] == "paid").expect("the active line must still be exported");
        // batch_input() above: sale_price 20.00, selling_fees 0.00, cost 10.00 (seed_tickets' 1000 cents) -> profit 10.00.
        assert_eq!(active_row[8], "10.00", "active line's real profit must still export correctly, unaffected by the other line's refund");
    }

    // ---- 1.9.1: "Export selected" for events/orders/tickets (Settings -> Data picker) ----

    /// Inserts one order (mirrors `seed_tickets` above) and returns just its
    /// order id, for tests that only care about the order-level export.
    fn seed_order_id(conn: &mut Connection) -> i64 {
        let tickets = seed_tickets(conn, 1);
        conn.query_row("SELECT order_id FROM tickets WHERE id=?1", [tickets[0]], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn export_events_csv_impl_with_no_ids_exports_every_event_unchanged() {
        let conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Event A')", []).unwrap();
        conn.execute("INSERT INTO events (name) VALUES ('Event B')", []).unwrap();

        let out = temp_csv_path();
        let count = export_events_csv_impl(&conn, out.0.to_str().unwrap(), None).unwrap();
        assert_eq!(count, 2, "None must still mean 'export everything' - unchanged pre-1.9.1 behaviour");
    }

    #[test]
    fn export_events_csv_selected_exports_only_the_chosen_ids() {
        let conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Event A')", []).unwrap();
        let a_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO events (name) VALUES ('Event B')", []).unwrap(); // NOT selected

        let out = temp_csv_path();
        let count = export_events_csv_impl(&conn, out.0.to_str().unwrap(), Some(&[a_id])).unwrap();
        assert_eq!(count, 1);
        let rows = read_csv_rows(&out.0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], "Event A", "only the selected event's row must be exported");
    }

    #[test]
    fn export_events_csv_selected_rejects_an_empty_selection() {
        let conn = test_conn();
        let out = temp_csv_path();
        let err = export_events_csv_impl(&conn, out.0.to_str().unwrap(), Some(&[])).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(!out.0.exists(), "nothing should be written to disk for a rejected empty selection");
    }

    #[test]
    fn export_orders_csv_impl_with_no_ids_exports_every_order_unchanged() {
        let mut conn = test_conn();
        seed_order_id(&mut conn);
        seed_order_id(&mut conn);

        let out = temp_csv_path();
        let count = export_orders_csv_impl(&conn, out.0.to_str().unwrap(), None).unwrap();
        assert_eq!(count, 2, "None must still mean 'export everything' - unchanged pre-1.9.1 behaviour");
    }

    #[test]
    fn export_orders_csv_selected_exports_only_the_chosen_ids() {
        let mut conn = test_conn();
        let order_a = seed_order_id(&mut conn);
        seed_order_id(&mut conn); // NOT selected

        let out = temp_csv_path();
        let count = export_orders_csv_impl(&conn, out.0.to_str().unwrap(), Some(&[order_a])).unwrap();
        assert_eq!(count, 1, "only the selected order must be exported");
    }

    #[test]
    fn export_orders_csv_selected_rejects_an_empty_selection() {
        let conn = test_conn();
        let out = temp_csv_path();
        let err = export_orders_csv_impl(&conn, out.0.to_str().unwrap(), Some(&[])).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(!out.0.exists(), "nothing should be written to disk for a rejected empty selection");
    }

    #[test]
    fn export_tickets_csv_selected_exports_the_chosen_ids_regardless_of_status() {
        // The plain export_tickets_csv command still filters by status, but
        // "Export selected" must not - the frontend picker already decided
        // which tickets are offered (e.g. all statuses for Tickets, only
        // available/listed for Inventory); once specific ids are picked, this
        // must export exactly them.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [tickets[0]]).unwrap();

        let out = temp_csv_path();
        let count =
            export_tickets_inner(&conn, out.0.to_str().unwrap(), None, None, Some(&[tickets[0], tickets[1]]))
                .unwrap();
        assert_eq!(count, 2, "both selected tickets export regardless of their differing status");
    }

    #[test]
    fn export_tickets_csv_selected_rejects_an_empty_selection() {
        let conn = test_conn();
        let out = temp_csv_path();
        let err = export_tickets_inner(&conn, out.0.to_str().unwrap(), None, None, Some(&[])).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(!out.0.exists(), "nothing should be written to disk for a rejected empty selection");
    }

    #[test]
    fn export_tickets_inner_status_and_ids_filters_still_compose() {
        // Guards the refactor from 1.8.x's &State-based signature to a plain
        // &Connection (needed for unit-testability, see export_tickets_inner's
        // doc comment): export_tickets_csv/export_inventory_csv's existing
        // status_filter/event_id behaviour must be completely unaffected by
        // the new `ids` parameter when callers pass None for it.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        conn.execute("UPDATE tickets SET status='listed' WHERE id=?1", [tickets[0]]).unwrap();
        // tickets[1] stays 'available'.

        let out = temp_csv_path();
        let count = export_tickets_inner(
            &conn,
            out.0.to_str().unwrap(),
            Some(vec!["available".to_string(), "listed".to_string()]),
            None,
            None,
        )
        .unwrap();
        assert_eq!(count, 2, "both tickets match the available+listed status filter, unchanged by the ids param");
    }
}
