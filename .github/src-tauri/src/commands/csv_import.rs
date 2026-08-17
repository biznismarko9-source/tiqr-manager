use crate::commands::orders::insert_order_with_tickets;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::OrderInput;
use crate::money::parse_decimal_to_cents;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvPreviewRow {
    pub row_number: i64,
    pub values: BTreeMap<String, String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvPreview {
    pub headers: Vec<String>,
    pub rows: Vec<CsvPreviewRow>,
    pub valid_count: i64,
    pub error_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub imported_orders: i64,
    pub imported_tickets: i64,
    pub errors: Vec<String>,
}

fn field<'a>(
    map: &std::collections::HashMap<String, usize>,
    record: &'a csv::StringRecord,
    names: &[&str],
) -> Option<&'a str> {
    for name in names {
        if let Some(&idx) = map.get(*name) {
            if let Some(v) = record.get(idx) {
                let v = v.trim();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

struct ParsedRow {
    row_number: i64,
    values: BTreeMap<String, String>,
    errors: Vec<String>,
    order_input: Option<OrderInput>,
    supplier_name: Option<String>,
    platform_name: Option<String>,
}

fn normalize(h: &str) -> String {
    h.trim().to_lowercase().replace(' ', "_")
}

fn parse_rows(conn: &Connection, path: &str) -> AppResult<(Vec<String>, Vec<ParsedRow>)> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| AppError::Validation(format!("Could not read CSV file: {e}")))?;

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::Validation(format!("Could not read CSV headers: {e}")))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut header_map = std::collections::HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        header_map.insert(normalize(h), i);
    }
    if !header_map.contains_key("event") && !header_map.contains_key("event_name") {
        return Err(AppError::Validation(
            "CSV is missing a required 'event' column".into(),
        ));
    }

    let mut rows = vec![];
    for (i, result) in reader.records().enumerate() {
        let row_number = (i + 2) as i64; // header is row 1
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                rows.push(ParsedRow {
                    row_number,
                    values: BTreeMap::new(),
                    errors: vec![format!("Could not parse row: {e}")],
                    order_input: None,
                    supplier_name: None,
                    platform_name: None,
                });
                continue;
            }
        };

        let mut values = BTreeMap::new();
        for h in &headers {
            if let Some(&idx) = header_map.get(&normalize(h)) {
                values.insert(h.clone(), record.get(idx).unwrap_or("").to_string());
            }
        }

        let mut errors = vec![];

        let event_name = field(&header_map, &record, &["event", "event_name"]);
        let purchase_date_raw = field(&header_map, &record, &["purchase_date", "date"]);
        let quantity_raw = field(&header_map, &record, &["quantity", "qty"]);
        let unit_price_raw = field(&header_map, &record, &["unit_price", "price", "purchase_price"]);
        let fees_raw = field(&header_map, &record, &["fees", "purchase_fees"]);
        let other_raw = field(&header_map, &record, &["other_costs", "other_cost"]);
        let currency = field(&header_map, &record, &["currency"]).unwrap_or("EUR");
        let payment_status = field(&header_map, &record, &["payment_status"]).unwrap_or("unpaid");
        let notes = field(&header_map, &record, &["notes"]);
        let ticket_type = field(&header_map, &record, &["ticket_type"]);
        let section = field(&header_map, &record, &["section"]);
        let row_label = field(&header_map, &record, &["row", "row_label"]);
        let seats_raw = field(&header_map, &record, &["seats", "seat"]);
        let supplier_name = field(&header_map, &record, &["supplier"]);
        let platform_name = field(&header_map, &record, &["platform"]);

        let event_id: Option<i64> = match event_name {
            None => {
                errors.push("Missing 'event' value".to_string());
                None
            }
            Some(name) => {
                let id: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM events WHERE LOWER(name) = LOWER(?1)",
                        [name],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(AppError::from)?;
                if id.is_none() {
                    errors.push(format!("Event '{name}' not found - create it in TIQR Manager first"));
                }
                id
            }
        };

        let purchase_date = match purchase_date_raw {
            Some(d) => d.to_string(),
            None => {
                errors.push("Missing 'purchase_date' value".to_string());
                String::new()
            }
        };

        let quantity: i64 = match quantity_raw.map(|q| q.parse::<i64>()) {
            Some(Ok(q)) if q > 0 => q,
            Some(Ok(_)) => {
                errors.push("'quantity' must be greater than 0".to_string());
                0
            }
            _ => {
                errors.push("'quantity' is missing or not a whole number".to_string());
                0
            }
        };

        let unit_price_cents = match unit_price_raw.map(parse_decimal_to_cents) {
            Some(Ok(v)) if v >= 0 => v,
            Some(Ok(_)) => {
                errors.push("'unit_price' cannot be negative".to_string());
                0
            }
            Some(Err(e)) => {
                errors.push(format!("'unit_price': {e}"));
                0
            }
            None => {
                errors.push("Missing 'unit_price' value".to_string());
                0
            }
        };

        let fees_cents = match fees_raw.map(parse_decimal_to_cents) {
            Some(Ok(v)) if v >= 0 => v,
            Some(Ok(_)) => {
                errors.push("'fees' cannot be negative".to_string());
                0
            }
            Some(Err(e)) => {
                errors.push(format!("'fees': {e}"));
                0
            }
            None => 0,
        };

        let other_costs_cents = match other_raw.map(parse_decimal_to_cents) {
            Some(Ok(v)) if v >= 0 => v,
            Some(Ok(_)) => {
                errors.push("'other_costs' cannot be negative".to_string());
                0
            }
            Some(Err(e)) => {
                errors.push(format!("'other_costs': {e}"));
                0
            }
            None => 0,
        };

        if !["unpaid", "partial", "paid"].contains(&payment_status) {
            errors.push(format!(
                "'payment_status' must be one of unpaid/partial/paid, got '{payment_status}'"
            ));
        }

        // "seats" is a comma-separated list, one label per ticket, e.g.
        // "11,12,13,14" for quantity=4. Leaving it blank is valid - tickets
        // are then created without a seat number, same as manual entry.
        let seats: Option<Vec<String>> = seats_raw.map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        });
        if let Some(seat_list) = &seats {
            if !seat_list.is_empty() && quantity > 0 && seat_list.len() as i64 != quantity {
                errors.push(format!(
                    "'seats' has {} value(s) but quantity is {} - provide one seat per ticket or leave 'seats' empty",
                    seat_list.len(),
                    quantity
                ));
            }
        }

        let order_input = if errors.is_empty() {
            Some(OrderInput {
                event_id: event_id.unwrap(),
                supplier_id: None,
                platform_id: None,
                purchase_date,
                quantity,
                unit_price_cents,
                fees_cents,
                other_costs_cents,
                currency: currency.to_string(),
                payment_status: Some(payment_status.to_string()),
                notes: notes.map(|s| s.to_string()),
                ticket_type: ticket_type.map(|s| s.to_string()),
                section: section.map(|s| s.to_string()),
                row_label: row_label.map(|s| s.to_string()),
                seats: seats.filter(|s| !s.is_empty()),
            })
        } else {
            None
        };

        rows.push(ParsedRow {
            row_number,
            values,
            errors,
            order_input,
            supplier_name: supplier_name.map(|s| s.to_string()),
            platform_name: platform_name.map(|s| s.to_string()),
        });
    }

    Ok((headers, rows))
}

#[tauri::command]
pub fn preview_orders_csv(state: State<AppState>, path: String) -> AppResult<CsvPreview> {
    let conn = state.db.lock().unwrap();
    let (headers, rows) = parse_rows(&conn, &path)?;
    let valid_count = rows.iter().filter(|r| r.errors.is_empty()).count() as i64;
    let error_count = rows.len() as i64 - valid_count;
    Ok(CsvPreview {
        headers,
        rows: rows
            .into_iter()
            .map(|r| CsvPreviewRow {
                row_number: r.row_number,
                values: r.values,
                errors: r.errors,
            })
            .collect(),
        valid_count,
        error_count,
    })
}

fn resolve_or_create_supplier(conn: &Connection, name: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM suppliers WHERE LOWER(name) = LOWER(?1)",
            [name],
            |r| r.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute("INSERT INTO suppliers(name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

fn resolve_or_create_platform(conn: &Connection, name: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM platforms WHERE LOWER(name) = LOWER(?1)",
            [name],
            |r| r.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO platforms(name, kind) VALUES (?1, 'purchase')",
        [name],
    )?;
    Ok(conn.last_insert_rowid())
}

/// All-or-nothing bulk import: every row is validated BEFORE any database
/// write happens. If anything is invalid, nothing is written at all. Takes
/// a plain connection so it's directly testable without a Tauri app.
fn import_orders_csv_impl(conn: &mut Connection, path: &str) -> AppResult<CsvImportResult> {
    let (_, rows) = parse_rows(conn, path)?;

    let mut all_errors: Vec<String> = vec![];
    for row in &rows {
        for e in &row.errors {
            all_errors.push(format!("Row {}: {}", row.row_number, e));
        }
    }
    if !all_errors.is_empty() {
        return Ok(CsvImportResult {
            imported_orders: 0,
            imported_tickets: 0,
            errors: all_errors,
        });
    }
    if rows.is_empty() {
        return Ok(CsvImportResult {
            imported_orders: 0,
            imported_tickets: 0,
            errors: vec!["CSV file has no data rows".to_string()],
        });
    }

    let tx = conn.transaction()?;
    let mut imported_orders = 0i64;
    let mut imported_tickets = 0i64;
    for row in &rows {
        let mut input = row.order_input.clone().expect("validated above");
        if let Some(name) = &row.supplier_name {
            input.supplier_id = Some(resolve_or_create_supplier(&tx, name)?);
        }
        if let Some(name) = &row.platform_name {
            input.platform_id = Some(resolve_or_create_platform(&tx, name)?);
        }
        insert_order_with_tickets(&tx, &input, false)?;
        imported_orders += 1;
        imported_tickets += input.quantity;
    }
    tx.commit()?;

    Ok(CsvImportResult {
        imported_orders,
        imported_tickets,
        errors: vec![],
    })
}

#[tauri::command]
pub fn import_orders_csv(state: State<AppState>, path: String) -> AppResult<CsvImportResult> {
    let mut conn = state.db.lock().unwrap();
    import_orders_csv_impl(&mut conn, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Writes `contents` to a fresh file under the OS temp dir and returns
    /// its path. No extra crate needed - std::fs is enough for this. The
    /// counter keeps concurrent test threads from colliding on one filename.
    struct TempCsv(std::path::PathBuf);
    impl TempCsv {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempCsv {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    fn write_csv(contents: &str) -> TempCsv {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tiqr-manager-test-{}-{n}.csv",
            std::process::id()
        ));
        std::fs::write(&path, contents).unwrap();
        TempCsv(path)
    }

    fn seed_event(conn: &Connection, name: &str) {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name])
            .unwrap();
    }

    #[test]
    fn imports_seats_as_one_ticket_per_seat() {
        let mut conn = test_conn();
        seed_event(&conn, "Concert A");
        let csv = write_csv(
            "event,purchase_date,quantity,unit_price,section,row,seats\n\
             Concert A,2026-01-01,4,20.00,A,12,\"11,12,13,14\"\n",
        );
        let result = import_orders_csv_impl(&mut conn, csv.path().to_str().unwrap()).unwrap();
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        assert_eq!(result.imported_orders, 1);
        assert_eq!(result.imported_tickets, 4);

        let mut stmt = conn
            .prepare("SELECT seat, row_label FROM tickets ORDER BY id")
            .unwrap();
        let rows: Vec<(Option<String>, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (Some("11".to_string()), Some("12".to_string())),
                (Some("12".to_string()), Some("12".to_string())),
                (Some("13".to_string()), Some("12".to_string())),
                (Some("14".to_string()), Some("12".to_string())),
            ]
        );
    }

    #[test]
    fn import_is_valid_without_seats_column() {
        let mut conn = test_conn();
        seed_event(&conn, "Concert A");
        let csv = write_csv(
            "event,purchase_date,quantity,unit_price\nConcert A,2026-01-01,2,15.00\n",
        );
        let result = import_orders_csv_impl(&mut conn, csv.path().to_str().unwrap()).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.imported_tickets, 2);
    }

    #[test]
    fn seat_count_mismatch_is_reported_and_nothing_imports() {
        let mut conn = test_conn();
        seed_event(&conn, "Concert A");
        let csv = write_csv(
            "event,purchase_date,quantity,unit_price,seats\n\
             Concert A,2026-01-01,4,20.00,\"11,12,13\"\n",
        );
        let result = import_orders_csv_impl(&mut conn, csv.path().to_str().unwrap()).unwrap();
        assert_eq!(result.imported_orders, 0);
        assert_eq!(result.imported_tickets, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("seats"));
    }

    #[test]
    fn import_is_all_or_nothing_across_rows() {
        let mut conn = test_conn();
        seed_event(&conn, "Concert A");
        // Second row references an event that doesn't exist - the whole
        // file must be rejected, including the otherwise-valid first row.
        let csv = write_csv(
            "event,purchase_date,quantity,unit_price\n\
             Concert A,2026-01-01,2,15.00\n\
             Unknown Event,2026-01-02,1,10.00\n",
        );
        let result = import_orders_csv_impl(&mut conn, csv.path().to_str().unwrap()).unwrap();
        assert_eq!(result.imported_orders, 0);
        assert_eq!(result.imported_tickets, 0);
        assert!(!result.errors.is_empty());

        let order_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
            .unwrap();
        assert_eq!(order_count, 0, "nothing should be written when any row is invalid");
    }
}
