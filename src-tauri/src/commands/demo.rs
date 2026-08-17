use crate::codes;
use crate::commands::orders::insert_order_with_tickets;
use crate::db::AppState;
use crate::error::AppResult;
use crate::models::OrderInput;
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

fn ensure_platform(conn: &Connection, name: &str, kind: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row("SELECT id FROM platforms WHERE name = ?1", [name], |r| r.get(0))
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO platforms(name, kind, is_demo) VALUES (?1, ?2, 0)",
        params![name, kind],
    )?;
    Ok(conn.last_insert_rowid())
}

fn ensure_demo_supplier(conn: &Connection, name: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row("SELECT id FROM suppliers WHERE name = ?1", [name], |r| r.get(0))
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO suppliers(name, is_demo) VALUES (?1, 1)",
        [name],
    )?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
fn insert_demo_event(
    conn: &Connection,
    name: &str,
    artist: &str,
    venue: &str,
    city: &str,
    country: &str,
    date: &str,
    category: &str,
    status: &str,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO events (name, artist_team, venue, city, country, event_date, category, status, notes, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1)",
        params![
            name,
            artist,
            venue,
            city,
            country,
            date,
            category,
            status,
            "Sample demo event - safe to remove via Settings \u{2192} Clear Demo Data.",
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn tickets_for_order(conn: &Connection, order_id: i64) -> AppResult<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id")?;
    let ids = stmt
        .query_map([order_id], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn sell_demo_ticket(
    conn: &Connection,
    ticket_id: i64,
    platform_id: i64,
    sale_date: &str,
    price_cents: i64,
    fees_cents: i64,
) -> AppResult<()> {
    let code = codes::next_code(conn, "sale", "SAL")?;
    conn.execute(
        "INSERT INTO sales(code, ticket_id, platform_id, sale_date, sale_price_cents, selling_fees_cents, currency, payment_status, buyer_reference, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,'EUR','paid','Demo buyer',1)",
        params![code, ticket_id, platform_id, sale_date, price_cents, fees_cents],
    )?;
    conn.execute(
        "UPDATE tickets SET status='sold', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [ticket_id],
    )?;
    Ok(())
}

pub fn seed_demo_data(conn: &Connection) -> AppResult<()> {
    let ticketmaster = ensure_platform(conn, "Ticketmaster", "both")?;
    let stubhub = ensure_platform(conn, "StubHub", "both")?;
    let viagogo = ensure_platform(conn, "Viagogo", "sale")?;
    ensure_platform(conn, "Tixel", "sale")?;
    ensure_platform(conn, "SeatGeek", "sale")?;

    let supplier_a = ensure_demo_supplier(conn, "Demo Supplier A")?;
    let supplier_b = ensure_demo_supplier(conn, "Demo Supplier B")?;

    // Event 1: upcoming, partially sold.
    let ev1 = insert_demo_event(
        conn, "Champions League Final", "UEFA Champions League", "Wembley Stadium",
        "London", "England", "2027-05-29", "Sports", "upcoming",
    )?;
    let order1 = insert_order_with_tickets(
        conn,
        &OrderInput {
            event_id: ev1,
            supplier_id: Some(supplier_a),
            platform_id: Some(ticketmaster),
            purchase_date: "2026-07-01".into(),
            quantity: 10,
            unit_price_cents: 25_000,
            fees_cents: 5_000,
            other_costs_cents: 0,
            currency: "EUR".into(),
            payment_status: Some("paid".into()),
            notes: None,
            ticket_type: Some("Category 1".into()),
            section: Some("Lower Tier".into()),
        },
        true,
    )?;
    let t1 = tickets_for_order(conn, order1)?;
    for (i, tid) in t1.iter().take(4).enumerate() {
        sell_demo_ticket(conn, *tid, ticketmaster, "2026-08-05", 42_000 + (i as i64) * 1_000, 2_500)?;
    }

    // Event 2: upcoming, mix of listed + sold + available.
    let ev2 = insert_demo_event(
        conn, "Taylor Swift - The Eras Tour", "Taylor Swift", "Stade de France",
        "Paris", "France", "2026-09-12", "Concert", "upcoming",
    )?;
    let order2 = insert_order_with_tickets(
        conn,
        &OrderInput {
            event_id: ev2,
            supplier_id: Some(supplier_b),
            platform_id: Some(stubhub),
            purchase_date: "2026-05-10".into(),
            quantity: 8,
            unit_price_cents: 18_000,
            fees_cents: 3_600,
            other_costs_cents: 0,
            currency: "EUR".into(),
            payment_status: Some("paid".into()),
            notes: None,
            ticket_type: Some("Standard".into()),
            section: Some("Block 12".into()),
        },
        true,
    )?;
    let t2 = tickets_for_order(conn, order2)?;
    for tid in t2.iter().take(5) {
        conn.execute(
            "UPDATE tickets SET status='listed', listing_price_cents=32000 WHERE id=?1",
            [*tid],
        )?;
    }
    for (i, tid) in t2.iter().skip(5).take(2).enumerate() {
        sell_demo_ticket(conn, *tid, viagogo, "2026-08-10", 33_000 + (i as i64) * 500, 1_800)?;
    }

    // Event 3: upcoming, fresh inventory, nothing sold yet.
    let ev3 = insert_demo_event(
        conn, "Formula 1 Monaco Grand Prix", "Formula 1", "Circuit de Monaco",
        "Monte Carlo", "Monaco", "2027-05-24", "Motorsport", "upcoming",
    )?;
    insert_order_with_tickets(
        conn,
        &OrderInput {
            event_id: ev3,
            supplier_id: Some(supplier_a),
            platform_id: Some(ticketmaster),
            purchase_date: "2026-08-01".into(),
            quantity: 6,
            unit_price_cents: 65_000,
            fees_cents: 9_000,
            other_costs_cents: 3_000,
            currency: "EUR".into(),
            payment_status: Some("partial".into()),
            notes: Some("Grandstand K".into()),
            ticket_type: Some("Grandstand".into()),
            section: Some("K".into()),
        },
        true,
    )?;

    // Event 4: completed, fully sold - a clean profit story end to end.
    let ev4 = insert_demo_event(
        conn, "Ed Sheeran - Mathematics Tour", "Ed Sheeran", "Tipsport Arena",
        "Bratislava", "Slovakia", "2026-06-15", "Concert", "completed",
    )?;
    let order4 = insert_order_with_tickets(
        conn,
        &OrderInput {
            event_id: ev4,
            supplier_id: Some(supplier_b),
            platform_id: Some(stubhub),
            purchase_date: "2026-03-20".into(),
            quantity: 5,
            unit_price_cents: 8_000,
            fees_cents: 2_000,
            other_costs_cents: 0,
            currency: "EUR".into(),
            payment_status: Some("paid".into()),
            notes: None,
            ticket_type: Some("Fan Pit".into()),
            section: None,
        },
        true,
    )?;
    let t4 = tickets_for_order(conn, order4)?;
    for (i, tid) in t4.iter().enumerate() {
        sell_demo_ticket(conn, *tid, stubhub, "2026-06-10", 15_000 + (i as i64) * 500, 900)?;
    }

    // Event 5: upcoming, small order, includes one cancelled ticket example.
    let ev5 = insert_demo_event(
        conn, "UFC 310", "UFC", "T-Mobile Arena", "Las Vegas", "USA",
        "2026-11-08", "Sports", "upcoming",
    )?;
    let order5 = insert_order_with_tickets(
        conn,
        &OrderInput {
            event_id: ev5,
            supplier_id: Some(supplier_a),
            platform_id: Some(ticketmaster),
            purchase_date: "2026-08-12".into(),
            quantity: 4,
            unit_price_cents: 40_000,
            fees_cents: 4_000,
            other_costs_cents: 0,
            currency: "USD".into(),
            payment_status: Some("paid".into()),
            notes: None,
            ticket_type: Some("Cageside".into()),
            section: None,
        },
        true,
    )?;
    let t5 = tickets_for_order(conn, order5)?;
    if let Some(tid) = t5.first() {
        conn.execute(
            "UPDATE tickets SET status='cancelled', notes='Duplicate seat - cancelled by venue' WHERE id=?1",
            [*tid],
        )?;
    }

    Ok(())
}

pub fn seed_if_empty(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
    if count == 0 {
        seed_demo_data(conn)?;
    }
    Ok(())
}

/// Removes all demo transactional data (events/orders/tickets/sales) but
/// leaves lookup tables (platforms) and any real user data untouched.
#[tauri::command]
pub fn clear_demo_data(state: State<AppState>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM sales WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM tickets WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM orders WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM events WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM suppliers WHERE is_demo = 1", [])?;
    Ok(())
}

#[tauri::command]
pub fn reset_demo_data(state: State<AppState>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM sales WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM tickets WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM orders WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM events WHERE is_demo = 1", [])?;
    conn.execute("DELETE FROM suppliers WHERE is_demo = 1", [])?;
    seed_demo_data(&conn)?;
    Ok(())
}
