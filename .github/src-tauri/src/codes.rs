use rusqlite::Connection;

/// Reserves the next sequential number for `counter` and formats it as a
/// user-friendly code, e.g. next_code(conn, "order", "ORD") -> "ORD-000001".
pub fn next_code(conn: &Connection, counter: &str, prefix: &str) -> rusqlite::Result<String> {
    let value: i64 = conn.query_row(
        "UPDATE counters SET value = value + 1 WHERE name = ?1 RETURNING value",
        [counter],
        |r| r.get(0),
    )?;
    Ok(format!("{prefix}-{value:06}"))
}

/// Reserves `n` sequential numbers at once (single UPDATE) and returns them
/// pre-formatted. Used when an order generates many tickets at once so we
/// never do one DB round-trip per ticket.
pub fn next_code_batch(
    conn: &Connection,
    counter: &str,
    prefix: &str,
    n: i64,
) -> rusqlite::Result<Vec<String>> {
    if n <= 0 {
        return Ok(vec![]);
    }
    let new_value: i64 = conn.query_row(
        "UPDATE counters SET value = value + ?2 WHERE name = ?1 RETURNING value",
        rusqlite::params![counter, n],
        |r| r.get(0),
    )?;
    let start = new_value - n + 1;
    Ok((start..=new_value)
        .map(|v| format!("{prefix}-{v:06}"))
        .collect())
}
