use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{Event, EventInput, EventWithStats};
use rusqlite::{params, Connection, Row};
use tauri::State;

const STATS_SQL: &str = "
    SELECT
      e.id, e.name, e.artist_team, e.venue, e.city, e.country, e.event_date,
      e.category, e.status, e.notes, e.is_demo, e.created_at, e.updated_at,
      COUNT(DISTINCT t.id) AS purchased_tickets,
      COUNT(DISTINCT CASE WHEN t.status='available' THEN t.id END) AS available_tickets,
      COUNT(DISTINCT CASE WHEN t.status='listed' THEN t.id END) AS listed_tickets,
      COUNT(DISTINCT CASE WHEN t.status='sold' THEN t.id END) AS sold_tickets,
      COUNT(DISTINCT CASE WHEN t.status='cancelled' THEN t.id END) AS cancelled_tickets,
      COALESCE(SUM(t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents), 0) AS total_cost_cents,
      COALESCE(SUM(CASE WHEN t.status='sold' THEN t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents ELSE 0 END), 0) AS cogs_cents,
      COALESCE(SUM(s.sale_price_cents), 0) AS revenue_cents,
      COALESCE(SUM(s.selling_fees_cents), 0) AS selling_fees_cents,
      CASE WHEN COUNT(DISTINCT t.currency) <= 1 THEN MIN(t.currency) ELSE NULL END AS currency
    FROM events e
    LEFT JOIN tickets t ON t.event_id = e.id
    -- Refunded sales stay in the table (history) but must never count as
    -- revenue - excluding them from the join keeps every aggregate above
    -- correct without a second pass, and matches tickets whose status has
    -- already been returned to 'available' by the refund itself.
    LEFT JOIN sales s ON s.ticket_id = t.id AND s.payment_status != 'refunded'
";

fn map_event_with_stats(row: &Row) -> rusqlite::Result<EventWithStats> {
    let event = Event {
        id: row.get("id")?,
        name: row.get("name")?,
        artist_team: row.get("artist_team")?,
        venue: row.get("venue")?,
        city: row.get("city")?,
        country: row.get("country")?,
        event_date: row.get("event_date")?,
        category: row.get("category")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    };
    let stats = finance::compute_summary(
        row.get("purchased_tickets")?,
        row.get("available_tickets")?,
        row.get("listed_tickets")?,
        row.get("sold_tickets")?,
        row.get("cancelled_tickets")?,
        row.get("total_cost_cents")?,
        row.get("cogs_cents")?,
        row.get("revenue_cents")?,
        row.get("selling_fees_cents")?,
        row.get("currency")?,
    );
    Ok(EventWithStats { event, stats })
}

fn map_event_plain(row: &Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get("id")?,
        name: row.get("name")?,
        artist_team: row.get("artist_team")?,
        venue: row.get("venue")?,
        city: row.get("city")?,
        country: row.get("country")?,
        event_date: row.get("event_date")?,
        category: row.get("category")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub(crate) fn fetch_recent(conn: &Connection, limit: i64) -> AppResult<Vec<EventWithStats>> {
    let sql = format!("{STATS_SQL} GROUP BY e.id ORDER BY e.created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_event_with_stats)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_events(state: State<AppState>, search: Option<String>) -> AppResult<Vec<EventWithStats>> {
    let conn = state.db.lock().unwrap();
    let mut sql = format!("{STATS_SQL} WHERE 1=1");
    let mut like = String::new();
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (e.name LIKE ?1 OR e.artist_team LIKE ?1 OR e.venue LIKE ?1 OR e.city LIKE ?1)");
            like = format!("%{q}%");
        }
    }
    sql.push_str(" GROUP BY e.id ORDER BY (e.event_date IS NULL), e.event_date DESC, e.id DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = if like.is_empty() {
        stmt.query_map([], map_event_with_stats)?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([&like], map_event_with_stats)?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(rows)
}

#[tauri::command]
pub fn get_event(state: State<AppState>, id: i64) -> AppResult<EventWithStats> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{STATS_SQL} WHERE e.id = ?1 GROUP BY e.id");
    conn.query_row(&sql, [id], map_event_with_stats)
        .map_err(|_| AppError::NotFound(format!("Event #{id} not found")))
}

fn validate_input(input: &EventInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Event name is required".into()));
    }
    if let Some(status) = &input.status {
        if !["upcoming", "completed", "cancelled"].contains(&status.as_str()) {
            return Err(AppError::Validation(format!("Invalid event status '{status}'")));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn create_event(state: State<AppState>, input: EventInput) -> AppResult<Event> {
    validate_input(&input)?;
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO events (name, artist_team, venue, city, country, event_date, category, status, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            input.name.trim(),
            input.artist_team,
            input.venue,
            input.city,
            input.country,
            input.event_date,
            input.category,
            input.status.unwrap_or_else(|| "upcoming".to_string()),
            input.notes,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(
        "SELECT * FROM events WHERE id = ?1",
        [id],
        map_event_plain,
    )?)
}

#[tauri::command]
pub fn update_event(state: State<AppState>, id: i64, input: EventInput) -> AppResult<Event> {
    validate_input(&input)?;
    let conn = state.db.lock().unwrap();
    let changed = conn.execute(
        "UPDATE events SET name=?1, artist_team=?2, venue=?3, city=?4, country=?5, event_date=?6,
         category=?7, status=?8, notes=?9, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?10",
        params![
            input.name.trim(),
            input.artist_team,
            input.venue,
            input.city,
            input.country,
            input.event_date,
            input.category,
            input.status.unwrap_or_else(|| "upcoming".to_string()),
            input.notes,
            id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Event #{id} not found")));
    }
    Ok(conn.query_row(
        "SELECT * FROM events WHERE id = ?1",
        [id],
        map_event_plain,
    )?)
}

#[tauri::command]
pub fn delete_event(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let order_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM orders WHERE event_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if order_count > 0 {
        return Err(AppError::Validation(
            "This event has orders/tickets linked to it and cannot be deleted. Delete its orders first.".into(),
        ));
    }
    let changed = conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Event #{id} not found")));
    }
    Ok(())
}
