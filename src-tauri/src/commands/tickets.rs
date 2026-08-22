use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{BulkTicketField, BulkTicketStatusInput, BulkTicketUpdateInput, Ticket, TicketUpdateInput};
use rusqlite::{params, Connection, Row};
use std::collections::HashSet;
use tauri::State;

// Safety cap on unfiltered list views. Ordinary use (hundreds to low
// thousands of tickets) never hits this; it only kicks in for very large,
// unfiltered inventories so the UI never has to serialize/render an
// unbounded number of rows in one go. Results are already ordered, so a
// capped result is simply "the most relevant N", not an arbitrary cut.
const LIST_CAP: i64 = 5000;

// The `sa.payment_status != 'refunded'` join guard matters as of migration
// 004: a ticket can now legitimately have more than one `sales` row over its
// lifetime (a refunded sale plus a later active resale - see BUG #1 fix), so
// an unfiltered join here would fan a single ticket out into two result
// rows. Restricting the join to the ACTIVE sale (there is at most one, by
// construction - see idx_sales_ticket_active_unique) keeps this a true
// one-row-per-ticket view and makes `sale_price_cents` reflect the current
// sale, never a stale refunded one. Same pattern already used in
// orders.rs's fetch_sales_summary and events.rs's stats query.
const BASE_SQL: &str = "
    SELECT t.id, t.code, t.event_id, e.name as event_name, t.order_id, o.code as order_code,
      t.section, t.row_label, t.seat, t.ticket_type,
      t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents,
      t.listing_price_cents, t.currency, t.status, t.resale_status, t.delivery_status,
      t.notes, t.is_demo,
      t.created_at, t.updated_at, sa.sale_price_cents as sale_price_cents
    FROM tickets t
    JOIN events e ON e.id = t.event_id
    JOIN orders o ON o.id = t.order_id
    LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
";

fn map_ticket(row: &Row) -> rusqlite::Result<Ticket> {
    let purchase_cost_cents: i64 = row.get("purchase_cost_cents")?;
    let purchase_fees_cents: i64 = row.get("purchase_fees_cents")?;
    let other_costs_cents: i64 = row.get("other_costs_cents")?;
    Ok(Ticket {
        id: row.get("id")?,
        code: row.get("code")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        order_id: row.get("order_id")?,
        order_code: row.get("order_code")?,
        section: row.get("section")?,
        row_label: row.get("row_label")?,
        seat: row.get("seat")?,
        ticket_type: row.get("ticket_type")?,
        purchase_cost_cents,
        purchase_fees_cents,
        other_costs_cents,
        total_cost_cents: purchase_cost_cents + purchase_fees_cents + other_costs_cents,
        listing_price_cents: row.get("listing_price_cents")?,
        currency: row.get("currency")?,
        status: row.get("status")?,
        resale_status: row.get("resale_status")?,
        delivery_status: row.get("delivery_status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        sale_price_cents: row.get("sale_price_cents")?,
    })
}

/// Split out from the `list_tickets` command (same pattern as
/// list_orders/list_sale_groups) so it's directly unit-testable against a
/// plain `&Connection` - in particular so BUG #1's fix can be verified end
/// to end: a ticket with both a refunded and a new active sale must still
/// come back as exactly one row here, carrying the active sale's price.
#[allow(clippy::too_many_arguments)]
pub(crate) fn list_tickets_impl(
    conn: &Connection,
    search: Option<String>,
    status: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> AppResult<Vec<Ticket>> {
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(s) = status.as_deref() {
        if !s.is_empty() {
            // Accepts a single status or a comma-separated list (e.g. "available,listed").
            let statuses: Vec<String> = s
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();
            if statuses.len() == 1 {
                sql.push_str(" AND t.status = ?");
                params_vec.push(Box::new(statuses[0].clone()));
            } else if statuses.len() > 1 {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                sql.push_str(&format!(" AND t.status IN ({placeholders})"));
                for st in statuses {
                    params_vec.push(Box::new(st));
                }
            }
        }
    }
    if let Some(eid) = event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(oid) = order_id {
        sql.push_str(" AND t.order_id = ?");
        params_vec.push(Box::new(oid));
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (t.code LIKE ? OR t.section LIKE ? OR t.seat LIKE ? OR t.row_label LIKE ? OR e.name LIKE ? OR o.code LIKE ?)");
            let like = format!("%{q}%");
            for _ in 0..6 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }

    let sort_col = match sort_by.as_deref() {
        Some("event") => "e.name",
        Some("status") => "t.status",
        Some("price") => "t.listing_price_cents",
        Some("cost") => "(t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents)",
        Some("created") => "t.created_at",
        Some("code") => "t.code",
        _ => "t.id",
    };
    let dir = match sort_dir.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    sql.push_str(&format!(" ORDER BY {sort_col} {dir}, t.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_ticket)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn list_tickets(
    state: State<AppState>,
    search: Option<String>,
    status: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> AppResult<Vec<Ticket>> {
    let conn = state.db.lock().unwrap();
    list_tickets_impl(&conn, search, status, event_id, order_id, sort_by, sort_dir)
}

#[tauri::command]
pub fn get_ticket(state: State<AppState>, id: i64) -> AppResult<Ticket> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{BASE_SQL} WHERE t.id = ?1");
    conn.query_row(&sql, [id], map_ticket)
        .map_err(|_| AppError::NotFound(format!("Ticket #{id} not found")))
}

/// Core logic behind `update_ticket` - split out (same pattern as
/// `list_tickets_impl`/`create_sales_batch_impl`) so it's directly
/// unit-testable against a plain `&Connection` without a full Tauri context.
/// Byte-identical validation/SQL to the previous inline version - this is a
/// mechanical extraction, not a behaviour change.
pub(crate) fn update_ticket_impl(conn: &Connection, id: i64, input: &TicketUpdateInput) -> AppResult<()> {
    let current_status: String = conn
        .query_row("SELECT status FROM tickets WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .map_err(|_| AppError::NotFound(format!("Ticket #{id} not found")))?;

    if let Some(new_status) = &input.status {
        if !["available", "listed", "sold", "cancelled"].contains(&new_status.as_str()) {
            return Err(AppError::Validation(format!("Invalid status '{new_status}'")));
        }
        if (current_status == "sold" && new_status != "sold")
            || (new_status == "sold" && current_status != "sold")
        {
            return Err(AppError::Validation(
                "Ticket sold status can only be changed via the Sales screen (create or delete a sale)."
                    .into(),
            ));
        }
    }

    if let Some(price) = input.listing_price_cents {
        if price < 0 {
            return Err(AppError::Validation("Listing price cannot be negative".into()));
        }
    }

    let next_status = input.status.clone().unwrap_or(current_status);

    conn.execute(
        "UPDATE tickets SET section=?1, row_label=?2, seat=?3, ticket_type=?4,
         listing_price_cents=?5, status=?6, resale_status=?7, delivery_status=?8, notes=?9,
         updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?10",
        params![
            input.section,
            input.row_label,
            input.seat,
            input.ticket_type,
            input.listing_price_cents,
            next_status,
            input.resale_status,
            input.delivery_status,
            input.notes,
            id,
        ],
    )?;

    Ok(())
}

#[tauri::command]
pub fn update_ticket(
    state: State<AppState>,
    id: i64,
    input: TicketUpdateInput,
) -> AppResult<Ticket> {
    let conn = state.db.lock().unwrap();
    update_ticket_impl(&conn, id, &input)?;
    let sql = format!("{BASE_SQL} WHERE t.id = ?1");
    Ok(conn.query_row(&sql, [id], map_ticket)?)
}

/// Core logic behind `bulk_update_tickets` - changes ONE ticket field across
/// MANY tickets in a single all-or-nothing transaction (same
/// validate-everything-then-write shape as `create_sales_batch_impl`; every
/// ticket id is confirmed to exist before anything is written, and if any id
/// is missing or the value is invalid, nothing is changed - rusqlite's
/// `Transaction` rolls back automatically on Drop whenever `commit()` is
/// never reached).
///
/// `field` is a closed enum (`BulkTicketField`), never a caller-supplied
/// column name - the SQL column is chosen via `match` into one of a fixed set
/// of `&'static str` literals, the same safe pattern `list_tickets_impl`
/// already uses for its `sort_col`. This makes it impossible to compile a
/// bulk UPDATE against `tickets.status`: status is deliberately NOT one of
/// the bulk-editable fields, because a naive bulk status change could
/// silently create a `status='sold'` ticket with no active `sales` row (or
/// the reverse) - the exact class of corruption `update_ticket_impl`'s own
/// guard exists to prevent for a single ticket, and nothing else in the app
/// could detect or repair it afterwards. Section, row, seat and listing
/// price have no such coupling to sales/inventory state - the existing
/// single-ticket edit flow (`TicketEditModal`) already allows editing all
/// four regardless of a ticket's current status - so they're safe to change
/// in bulk the same way. (1.9.1: TicketType was also in this set until
/// marko asked for it to move to order-creation time instead - see
/// `BulkTicketField`'s doc comment.)
pub(crate) fn bulk_update_tickets_impl(
    conn: &mut Connection,
    input: &BulkTicketUpdateInput,
) -> AppResult<Vec<i64>> {
    if input.ticket_ids.is_empty() {
        return Err(AppError::Validation("Select at least one ticket to edit".into()));
    }
    if let BulkTicketField::ListingPriceCents = input.field {
        if let Some(price) = input.cents_value {
            if price < 0 {
                return Err(AppError::Validation("Listing price cannot be negative".into()));
            }
        }
    }

    let column: &'static str = match input.field {
        BulkTicketField::Section => "section",
        BulkTicketField::RowLabel => "row_label",
        BulkTicketField::Seat => "seat",
        BulkTicketField::ListingPriceCents => "listing_price_cents",
    };

    // Dedupe so the same id appearing twice (e.g. a stale double click) is
    // applied once, not treated as two separate writes.
    let mut ids: Vec<i64> = Vec::new();
    {
        let mut seen = HashSet::new();
        for &id in &input.ticket_ids {
            if seen.insert(id) {
                ids.push(id);
            }
        }
    }

    let tx = conn.transaction()?;

    // Validate every id exists BEFORE writing anything - all-or-nothing.
    // 1.8.3 (section 17, performance audit): one SELECT ... IN (...) query
    // instead of one query per id - same reasoning/technique as the
    // refetch in the `bulk_update_tickets` wrapper below. The specific
    // "Ticket #N does not exist" message is preserved by computing the
    // missing id in Rust (set difference) rather than looping in SQL.
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let existing_ids: HashSet<i64> = {
        let mut stmt = tx.prepare(&format!("SELECT id FROM tickets WHERE id IN ({placeholders})"))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |r| r.get::<_, i64>(0))?;
        rows.collect::<Result<HashSet<_>, _>>()?
    };
    if let Some(missing) = ids.iter().copied().find(|id| !existing_ids.contains(id)) {
        return Err(AppError::Validation(format!("Ticket #{missing} does not exist")));
    }

    // Same "one statement, not one per id" treatment for the write itself:
    // every selected ticket gets the exact same new value, so a single
    // UPDATE ... WHERE id IN (...) does the whole batch at once. The new
    // value (text_value or cents_value, depending on `field`) is bound
    // first as ?1, then every id fills the IN (...) list - built as
    // `Box<dyn ToSql>` because the leading value and the trailing ids are
    // different Rust types, the same dynamic-parameter approach already
    // used throughout dashboard.rs for its variable-shaped queries.
    let sql = format!(
        "UPDATE tickets SET {column} = ?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN ({placeholders})"
    );
    let mut update_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(ids.len() + 1);
    if let BulkTicketField::ListingPriceCents = input.field {
        update_params.push(Box::new(input.cents_value));
    } else {
        update_params.push(Box::new(input.text_value.clone()));
    }
    for &id in &ids {
        update_params.push(Box::new(id));
    }
    let update_refs: Vec<&dyn rusqlite::ToSql> = update_params.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, update_refs.as_slice())?;

    tx.commit()?;
    Ok(ids)
}

/// Bulk-edit one field (section, row, seat or listing price) across many
/// tickets at once - e.g. correcting the section label for a
/// whole block of seats in one action instead of one ticket at a time.
/// Shared by Sale Detail and Order Detail's bulk-selection UI (one command,
/// one implementation - see `BulkTicketEditBar.tsx`). Deliberately does NOT
/// support ticket status - see `bulk_update_tickets_impl`'s doc comment.
/// Returns every updated ticket, refetched in a single query (not one query
/// per id) so this stays cheap even for a large selection.
#[tauri::command]
pub fn bulk_update_tickets(state: State<AppState>, input: BulkTicketUpdateInput) -> AppResult<Vec<Ticket>> {
    let mut conn = state.db.lock().unwrap();
    let ids = bulk_update_tickets_impl(&mut conn, &input)?;
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("{BASE_SQL} WHERE t.id IN ({placeholders}) ORDER BY t.id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), map_ticket)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Input for `bulk_update_ticket_status` (1.9.3): set many tickets' `status`
/// in one all-or-nothing transaction, restricted to the same three values a
/// ticket can freely move between outside of a sale - see the doc comment on
/// `bulk_update_tickets_impl` above for the full reasoning: `status='sold'`
/// must always correspond to an active `sales` row, so a bulk endpoint that
/// could set or unset it freely could silently create exactly the orphaned
/// state `update_ticket_impl`'s single-ticket guard exists to prevent, with
/// nothing else in the app able to detect or repair it afterwards. `sold` is
/// therefore unreachable here both as a target status AND as a starting
/// point being moved away from - moving a ticket into or out of `sold` still
/// only ever happens via the Sales screen (create, refund or delete a sale).
pub(crate) fn bulk_update_ticket_status_impl(
    conn: &mut Connection,
    ticket_ids: &[i64],
    status: &str,
) -> AppResult<Vec<i64>> {
    if ticket_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one ticket to update".into(),
        ));
    }
    if !["available", "listed", "cancelled"].contains(&status) {
        return Err(AppError::Validation(
            "Ticket status can only be bulk-changed to Available, Listed or Cancelled - Sold can only be set via the Sales screen (create a sale)."
                .into(),
        ));
    }

    // Dedupe so the same id selected twice (e.g. a stale double click) is
    // applied once, not treated as two separate writes - same convention as
    // bulk_update_tickets_impl.
    let mut ids: Vec<i64> = Vec::new();
    {
        let mut seen = HashSet::new();
        for &id in ticket_ids {
            if seen.insert(id) {
                ids.push(id);
            }
        }
    }

    let tx = conn.transaction()?;

    // Validate every id exists AND is not currently sold BEFORE writing
    // anything - all-or-nothing, one query not one per id (same technique as
    // bulk_update_tickets_impl's own validation step, and the same shape as
    // bulk_update_sale_payment_status_impl's refunded guard in sales.rs).
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let existing: std::collections::HashMap<i64, String> = {
        let mut stmt = tx.prepare(&format!("SELECT id, status FROM tickets WHERE id IN ({placeholders})"))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<std::collections::HashMap<_, _>, _>>()?
    };
    if let Some(missing) = ids.iter().copied().find(|id| !existing.contains_key(id)) {
        return Err(AppError::Validation(format!("Ticket #{missing} does not exist")));
    }
    if existing.values().any(|s| s == "sold") {
        return Err(AppError::Validation(
            "One of the selected tickets is sold and can only be changed via the Sales screen (create, refund or delete a sale) - nothing was changed. Deselect it and try again.".into(),
        ));
    }

    let sql = format!(
        "UPDATE tickets SET status = ?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN ({placeholders})"
    );
    let mut update_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(ids.len() + 1);
    update_params.push(Box::new(status.to_string()));
    for &id in &ids {
        update_params.push(Box::new(id));
    }
    let update_refs: Vec<&dyn rusqlite::ToSql> = update_params.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, update_refs.as_slice())?;

    tx.commit()?;
    Ok(ids)
}

/// Sets `status` (available/listed/cancelled only - see the impl's doc
/// comment) for many tickets at once. Lives next to Order Detail's selection
/// checkboxes as a single narrow action - replaces the general
/// `BulkTicketEditBar` there (1.9.3: marko didn't want the Section/Row/
/// Seat/Listing-price editor on Order Detail any more, just a status
/// switch). Sale Detail is untouched by this - it already has its own
/// narrow action (`bulk_update_sale_payment_status`) for `sales.
/// payment_status`, a completely different column on a different table.
/// Returns the updated tickets, refetched in a single query the same way
/// `bulk_update_tickets` does.
#[tauri::command]
pub fn bulk_update_ticket_status(state: State<AppState>, input: BulkTicketStatusInput) -> AppResult<Vec<Ticket>> {
    let mut conn = state.db.lock().unwrap();
    let ids = bulk_update_ticket_status_impl(&mut conn, &input.ticket_ids, &input.status)?;
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("{BASE_SQL} WHERE t.id IN ({placeholders}) ORDER BY t.id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), map_ticket)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The 5 options the app's own "New order" form (Orders.tsx) has always
/// offered for Ticket Type, now also the seed for the Orders & Sales sheet's
/// own Ticket Type dropdown (commands::orders_sheet_sync) - see
/// `known_ticket_type_names`'s own doc comment for why these always come
/// first, and never disappear even if nothing currently uses them.
pub(crate) const TICKET_TYPE_SEED: &[&str] = &["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"];

/// Every ticket type marko can currently pick, in the app or in the sheet:
/// `TICKET_TYPE_SEED` above, followed by any other value already sitting on
/// a real ticket that isn't one of those 5 (alphabetically, deduped
/// case-insensitively so e.g. a sheet row typed "e-ticket" doesn't produce a
/// second entry next to the seeded "E-ticket").
///
/// 2.0.19 (marko's own request): there is deliberately no separate lookup
/// table for this, unlike Platforms/Suppliers - `Ticket.ticket_type` stays
/// the plain free-text column it always was (Order sync/push already read
/// and write it exactly as before, completely unchanged). Whoever types a
/// brand-new value - marko in the app's "Other..." field, or directly into
/// a sheet cell that Order sync then reads - it lands on a real ticket
/// immediately, which is all this query needs to pick it up as a known
/// option for next time. "Known" here always means "used somewhere right
/// now, or one of the 5 defaults" - there is no way for a value to become
/// unknown again short of every ticket that ever used it being deleted.
pub(crate) fn known_ticket_type_names(conn: &Connection) -> AppResult<Vec<String>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut names: Vec<String> = vec![];
    for seed in TICKET_TYPE_SEED {
        if seen.insert(seed.to_lowercase()) {
            names.push(seed.to_string());
        }
    }
    let mut stmt = conn.prepare(
        "SELECT DISTINCT ticket_type FROM tickets
         WHERE ticket_type IS NOT NULL AND TRIM(ticket_type) != '' AND is_demo = 0
         ORDER BY ticket_type COLLATE NOCASE",
    )?;
    let extra: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
    for name in extra {
        if seen.insert(name.to_lowercase()) {
            names.push(name);
        }
    }
    Ok(names)
}

/// Powers the "New order" form's Ticket Type field (Orders.tsx) - was a
/// hardcoded array of the 5 `TICKET_TYPE_SEED` values before 2.0.19, now a
/// live, growable list via `known_ticket_type_names` above. The "Other..."
/// free-text toggle next to it is unchanged: typing a new value there still
/// just becomes that order's (and its tickets') `ticket_type` directly, and
/// is picked up as a known option from then on with no extra step.
#[tauri::command]
pub fn list_ticket_types(state: State<AppState>) -> AppResult<Vec<String>> {
    let conn = state.db.lock().unwrap();
    known_ticket_type_names(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::orders::insert_order_with_tickets;
    use crate::commands::sales::{create_sale_impl, refund_sale_impl};
    use crate::db::test_conn;
    use crate::models::{OrderInput, SaleInput};

    fn seed_one_ticket(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
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
        conn.query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| r.get(0))
            .unwrap()
    }

    /// Same shape as `seed_one_ticket`, but lets a `known_ticket_type_names`
    /// test control the one thing that function actually looks at.
    fn seed_ticket_with_type(conn: &Connection, ticket_type: Option<&str>, is_demo: bool) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: ticket_type.map(|s| s.to_string()),
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(conn, &input, is_demo).unwrap();
        conn.query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn known_ticket_type_names_starts_with_the_five_seed_defaults_on_an_empty_database() {
        let conn = test_conn();
        assert_eq!(
            known_ticket_type_names(&conn).unwrap(),
            vec!["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"]
        );
    }

    #[test]
    fn known_ticket_type_names_includes_an_extra_value_actually_used_on_a_ticket() {
        let conn = test_conn();
        seed_ticket_with_type(&conn, Some("Season pass"), false);
        let names = known_ticket_type_names(&conn).unwrap();
        assert_eq!(names.len(), 6, "5 seed defaults + the 1 real extra value: {names:?}");
        assert_eq!(names.last(), Some(&"Season pass".to_string()));
    }

    #[test]
    fn known_ticket_type_names_never_lists_the_same_value_twice_regardless_of_case() {
        let conn = test_conn();
        // Sheet-typed lowercase "e-ticket" must not sit next to the seeded
        // "E-ticket" as if it were a genuinely different option.
        seed_ticket_with_type(&conn, Some("e-ticket"), false);
        let names = known_ticket_type_names(&conn).unwrap();
        assert_eq!(names, vec!["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"]);
    }

    #[test]
    fn known_ticket_type_names_ignores_demo_tickets() {
        let conn = test_conn();
        seed_ticket_with_type(&conn, Some("Demo-only type"), true);
        let names = known_ticket_type_names(&conn).unwrap();
        assert!(!names.contains(&"Demo-only type".to_string()), "{names:?}");
    }

    #[test]
    fn known_ticket_type_names_ignores_blank_and_null_ticket_type() {
        let conn = test_conn();
        seed_ticket_with_type(&conn, None, false);
        seed_ticket_with_type(&conn, Some("   "), false);
        let names = known_ticket_type_names(&conn).unwrap();
        assert_eq!(names, vec!["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"]);
    }

    /// BUG #1 fix, ticket-view half: once a ticket can carry both a
    /// refunded sale and a new active one (migration 004), list_tickets_impl
    /// must still show that ticket exactly once - never fanned out into two
    /// rows by the LEFT JOIN sales - and its `sale_price_cents` must reflect
    /// the current active sale, not the refunded one.
    #[test]
    fn ticket_with_a_refunded_and_a_new_active_sale_appears_exactly_once() {
        let mut conn = test_conn();
        let ticket_id = seed_one_ticket(&conn);
        let ticket_code: String = conn
            .query_row("SELECT code FROM tickets WHERE id=?1", [ticket_id], |r| r.get(0))
            .unwrap();

        let first_sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id_1 = create_sale_impl(&mut conn, &first_sale).unwrap();
        refund_sale_impl(&mut conn, sale_id_1, Some("buyer cancelled")).unwrap();

        let second_sale = SaleInput {
            sale_price_cents: 1800,
            ..first_sale
        };
        create_sale_impl(&mut conn, &second_sale).unwrap();

        let results = list_tickets_impl(&conn, Some(ticket_code), None, None, None, None, None).unwrap();
        assert_eq!(results.len(), 1, "the ticket must appear exactly once, never duplicated by the sales join");
        assert_eq!(
            results[0].sale_price_cents,
            Some(1800),
            "sale_price_cents must reflect the current active sale, not the refunded one"
        );

        // Same guarantee for get_ticket's single-row lookup.
        let sql = format!("{BASE_SQL} WHERE t.id = ?1");
        let single = conn.query_row(&sql, [ticket_id], map_ticket).unwrap();
        assert_eq!(single.sale_price_cents, Some(1800));
    }

    // --- 1.8.3: bulk_update_tickets_impl -----------------------------------

    /// 1.8.3 brief test scenario, verbatim: "4 tickets -> select 3 -> only
    /// those 3 change". None of the other bulk tests leave an unselected
    /// control ticket in place, so this is the one that actually proves
    /// selection is respected rather than accidentally updating everything.
    #[test]
    fn bulk_update_tickets_impl_only_changes_the_selected_tickets_out_of_four() {
        let mut conn = test_conn();
        let ids: Vec<i64> = (0..4).map(|_| seed_one_ticket(&conn)).collect();
        let selected = vec![ids[0], ids[1], ids[2]];
        let untouched = ids[3];

        let input = BulkTicketUpdateInput {
            ticket_ids: selected.clone(),
            field: BulkTicketField::Section,
            text_value: Some("Block A".to_string()),
            cents_value: None,
        };
        let updated_ids = bulk_update_tickets_impl(&mut conn, &input).unwrap();
        assert_eq!(updated_ids.len(), 3);

        for &id in &selected {
            let section: Option<String> = conn
                .query_row("SELECT section FROM tickets WHERE id=?1", [id], |r| r.get(0))
                .unwrap();
            assert_eq!(section.as_deref(), Some("Block A"));
        }
        let untouched_section: Option<String> = conn
            .query_row("SELECT section FROM tickets WHERE id=?1", [untouched], |r| r.get(0))
            .unwrap();
        assert_eq!(untouched_section, None, "the 4th ticket was never selected, so it must stay untouched");
    }

    /// PRIORITA #1 (1.8.3 brief, section 2/3): bulk-editing a safe field must
    /// work regardless of a ticket's status, and must NEVER touch `status`
    /// itself or the sale behind a sold ticket - the same contract the
    /// existing single-ticket `TicketEditModal` already relies on.
    #[test]
    fn bulk_update_tickets_impl_changes_selected_fields_and_ignores_status() {
        let mut conn = test_conn();
        let available_id = seed_one_ticket(&conn);
        let sold_id = seed_one_ticket(&conn);
        let sale = SaleInput {
            ticket_id: sold_id,
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            sale_price_cents: 5000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        create_sale_impl(&mut conn, &sale).unwrap();

        let input = BulkTicketUpdateInput {
            ticket_ids: vec![available_id, sold_id],
            field: BulkTicketField::Section,
            text_value: Some("Block A".to_string()),
            cents_value: None,
        };
        let updated_ids = bulk_update_tickets_impl(&mut conn, &input).unwrap();
        assert_eq!(updated_ids.len(), 2);

        for id in [available_id, sold_id] {
            let (section, status): (Option<String>, String) = conn
                .query_row("SELECT section, status FROM tickets WHERE id=?1", [id], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .unwrap();
            assert_eq!(section.as_deref(), Some("Block A"));
            if id == sold_id {
                assert_eq!(status, "sold", "bulk edit must never change ticket status");
            } else {
                assert_eq!(status, "available");
            }
        }

        // The sold ticket's sale itself must be completely untouched.
        let sale_price: i64 = conn
            .query_row("SELECT sale_price_cents FROM sales WHERE ticket_id=?1", [sold_id], |r| r.get(0))
            .unwrap();
        assert_eq!(sale_price, 5000);
    }

    /// BULK UPDATE SAFETY (1.8.3 brief, section 3): one bad id in the batch
    /// must roll back EVERY change, not just skip the bad one.
    #[test]
    fn bulk_update_tickets_impl_is_all_or_nothing() {
        let mut conn = test_conn();
        let id1 = seed_one_ticket(&conn);
        let id2 = seed_one_ticket(&conn);

        let input = BulkTicketUpdateInput {
            ticket_ids: vec![id1, id2, 999_999],
            field: BulkTicketField::Section,
            text_value: Some("Should not stick".to_string()),
            cents_value: None,
        };
        assert!(bulk_update_tickets_impl(&mut conn, &input).is_err());

        for id in [id1, id2] {
            let section: Option<String> = conn
                .query_row("SELECT section FROM tickets WHERE id=?1", [id], |r| r.get(0))
                .unwrap();
            assert_eq!(section, None, "a failed bulk edit must change nothing at all");
        }
    }

    #[test]
    fn bulk_update_tickets_impl_rejects_negative_listing_price() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        let input = BulkTicketUpdateInput {
            ticket_ids: vec![id],
            field: BulkTicketField::ListingPriceCents,
            text_value: None,
            cents_value: Some(-100),
        };
        assert!(bulk_update_tickets_impl(&mut conn, &input).is_err());
        let price: Option<i64> = conn
            .query_row("SELECT listing_price_cents FROM tickets WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(price, None);
    }

    #[test]
    fn bulk_update_tickets_impl_rejects_empty_selection() {
        let mut conn = test_conn();
        let input = BulkTicketUpdateInput {
            ticket_ids: vec![],
            field: BulkTicketField::Section,
            text_value: Some("x".to_string()),
            cents_value: None,
        };
        assert!(bulk_update_tickets_impl(&mut conn, &input).is_err());
    }

    #[test]
    fn bulk_update_tickets_impl_dedupes_ids() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        let input = BulkTicketUpdateInput {
            ticket_ids: vec![id, id, id],
            field: BulkTicketField::Section,
            text_value: Some("Once".to_string()),
            cents_value: None,
        };
        let updated_ids = bulk_update_tickets_impl(&mut conn, &input).unwrap();
        assert_eq!(updated_ids, vec![id]);
    }

    /// Cross-check with BUG #1's own regression test above: bulk-editing an
    /// unrelated field (row label) must never disturb a ticket's
    /// refund/resale history or the sales-join dedup it relies on.
    #[test]
    fn bulk_update_tickets_impl_does_not_disturb_refund_history() {
        let mut conn = test_conn();
        let ticket_id = seed_one_ticket(&conn);
        let first_sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id_1 = create_sale_impl(&mut conn, &first_sale).unwrap();
        refund_sale_impl(&mut conn, sale_id_1, Some("buyer cancelled")).unwrap();
        let second_sale = SaleInput {
            sale_price_cents: 1800,
            ..first_sale
        };
        create_sale_impl(&mut conn, &second_sale).unwrap();

        let input = BulkTicketUpdateInput {
            ticket_ids: vec![ticket_id],
            field: BulkTicketField::RowLabel,
            text_value: Some("12".to_string()),
            cents_value: None,
        };
        bulk_update_tickets_impl(&mut conn, &input).unwrap();

        let results = list_tickets_impl(&conn, None, None, None, None, None, None).unwrap();
        let ticket = results.iter().find(|t| t.id == ticket_id).unwrap();
        assert_eq!(ticket.row_label.as_deref(), Some("12"));
        assert_eq!(
            ticket.sale_price_cents,
            Some(1800),
            "bulk-editing an unrelated field must not disturb which sale is active"
        );
        assert_eq!(ticket.status, "sold");
    }

    // --- 1.9.3: bulk_update_ticket_status_impl ------------------------------

    fn ticket_status(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT status FROM tickets WHERE id=?1", [id], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn bulk_update_ticket_status_only_changes_the_selected_tickets_out_of_four() {
        let mut conn = test_conn();
        let ids: Vec<i64> = (0..4).map(|_| seed_one_ticket(&conn)).collect();
        let selected = vec![ids[0], ids[1], ids[2]];
        let untouched = ids[3];

        let updated_ids = bulk_update_ticket_status_impl(&mut conn, &selected, "listed").unwrap();
        assert_eq!(updated_ids.len(), 3);

        for &id in &selected {
            assert_eq!(ticket_status(&conn, id), "listed");
        }
        assert_eq!(
            ticket_status(&conn, untouched),
            "available",
            "the 4th ticket was never selected, so it must stay untouched"
        );
    }

    #[test]
    fn bulk_update_ticket_status_rejects_a_sold_ticket_and_changes_nothing() {
        let mut conn = test_conn();
        let available_id = seed_one_ticket(&conn);
        let sold_id = seed_one_ticket(&conn);
        let sale = SaleInput {
            ticket_id: sold_id,
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            sale_price_cents: 5000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        create_sale_impl(&mut conn, &sale).unwrap();

        let result = bulk_update_ticket_status_impl(&mut conn, &[available_id, sold_id], "cancelled");
        assert!(
            result.is_err(),
            "a batch containing a sold ticket must be rejected entirely"
        );
        assert_eq!(ticket_status(&conn, available_id), "available");
        assert_eq!(ticket_status(&conn, sold_id), "sold");
    }

    #[test]
    fn bulk_update_ticket_status_is_all_or_nothing_with_a_missing_id() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        let result = bulk_update_ticket_status_impl(&mut conn, &[id, 999_999], "listed");
        assert!(result.is_err());
        assert_eq!(
            ticket_status(&conn, id),
            "available",
            "a failed bulk update must change nothing at all"
        );
    }

    #[test]
    fn bulk_update_ticket_status_rejects_sold_as_a_target_status() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        let result = bulk_update_ticket_status_impl(&mut conn, &[id], "sold");
        assert!(
            result.is_err(),
            "a ticket can only become sold via the Sales screen, never through this bulk action"
        );
        assert_eq!(ticket_status(&conn, id), "available");
    }

    #[test]
    fn bulk_update_ticket_status_rejects_empty_selection() {
        let mut conn = test_conn();
        assert!(bulk_update_ticket_status_impl(&mut conn, &[], "listed").is_err());
    }

    #[test]
    fn bulk_update_ticket_status_dedupes_ids() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        let updated_ids = bulk_update_ticket_status_impl(&mut conn, &[id, id, id], "listed").unwrap();
        assert_eq!(updated_ids, vec![id]);
        assert_eq!(ticket_status(&conn, id), "listed");
    }

    #[test]
    fn bulk_update_ticket_status_moves_between_available_listed_and_cancelled_freely() {
        let mut conn = test_conn();
        let id = seed_one_ticket(&conn);
        for target in ["listed", "cancelled", "available", "listed"] {
            bulk_update_ticket_status_impl(&mut conn, &[id], target).unwrap();
            assert_eq!(ticket_status(&conn, id), target);
        }
    }

    /// Edge case: a ticket whose only sale was refunded is back to
    /// `available` (see `refund_sale_impl`) - the "sold" guard above looks
    /// at `tickets.status`, not "has this ticket ever had a sale", so it
    /// must be reachable through this bulk action again like any other
    /// available ticket.
    #[test]
    fn bulk_update_ticket_status_allows_a_ticket_whose_sale_was_refunded() {
        let mut conn = test_conn();
        let ticket_id = seed_one_ticket(&conn);
        let sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id = create_sale_impl(&mut conn, &sale).unwrap();
        refund_sale_impl(&mut conn, sale_id, Some("buyer cancelled")).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "available");

        bulk_update_ticket_status_impl(&mut conn, &[ticket_id], "listed").unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "listed");
    }
}
