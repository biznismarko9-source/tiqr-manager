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

#[tauri::command]
pub fn export_events_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, artist_team, venue, city, country, event_date, category, status, notes, is_demo, created_at
         FROM events ORDER BY id",
    )?;
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "id", "name", "artist_team", "venue", "city", "country", "event_date", "category",
        "status", "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query([])?;
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
pub fn export_orders_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT o.code, e.name, sup.name, p.name, o.purchase_date, o.quantity,
                o.unit_price_cents, o.fees_cents, o.other_costs_cents, o.total_cost_cents,
                o.currency, o.payment_status, o.notes, o.is_demo, o.created_at
         FROM orders o
         JOIN events e ON e.id = o.event_id
         LEFT JOIN suppliers sup ON sup.id = o.supplier_id
         LEFT JOIN platforms p ON p.id = o.platform_id
         ORDER BY o.id",
    )?;
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "order_code", "event", "supplier", "platform", "purchase_date", "quantity",
        "unit_price", "fees", "other_costs", "total_cost", "currency", "payment_status",
        "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query([])?;
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

fn export_tickets_inner(
    state: &State<AppState>,
    path: &str,
    status_filter: Option<Vec<String>>,
    event_id: Option<i64>,
) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
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
    let statuses = status.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    });
    export_tickets_inner(&state, &path, statuses, event_id)
}

/// "Inventory" = current stock only (available + listed), excluding sold/cancelled.
#[tauri::command]
pub fn export_inventory_csv(
    state: State<AppState>,
    path: String,
    event_id: Option<i64>,
) -> AppResult<i64> {
    export_tickets_inner(
        &state,
        &path,
        Some(vec!["available".to_string(), "listed".to_string()]),
        event_id,
    )
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
                s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at
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
        let payment_status: String = row.get(9)?;
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
        let profit = if payment_status == "refunded" { 0 } else { sale_price - cost - fees };
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            opt(row.get(3)?),
            row.get::<_, String>(4)?,
            format_cents(sale_price),
            format_cents(fees),
            format_cents(cost),
            format_cents(profit),
            row.get::<_, String>(8)?,
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
}
