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

/// 2.30.0 - event-derived codes. Marko: "ked to je oasis tak das OASIS-001.
/// ked je miley tak MILEY-001 ... lahsie bude s tym pracovat."
///
/// Turns an event's name into a stable, code-safe prefix. MUST be pure and
/// deterministic: both of marko's machines mint codes independently from
/// synced counters, so if this ever returned a different prefix for the same
/// name on two machines, they would produce two different codes for the same
/// record and the merge would see a collision that is not one.
///
/// The rules, in order:
///   * take the part before the app's own " · " separator, so
///     "Oasis · Wembley Stadium" keys on the act, not the venue - two Oasis
///     nights then share a run of numbers, which is how marko talks about
///     them.
///   * keep ASCII letters and digits only, upper-cased. Accents are folded
///     for the handful that appear in real venue/act names (Letná -> LETNA)
///     rather than dropped, so the prefix stays readable.
///   * first word; if that is shorter than three characters, glue the next
///     one on ("U2 Live" -> U2LIVE) so a prefix is never a single letter.
///   * cap at eight characters.
///
/// Returns `None` when nothing usable survives - a name of only punctuation,
/// or an event that has none. The caller falls back to the old fixed prefix
/// rather than inventing something.
pub fn prefix_for_event(name: &str) -> Option<String> {
    let head = name.split(" · ").next().unwrap_or(name);
    let folded: String = head
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'A',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => 'O',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ý' | 'Ý' => 'Y',
            'č' | 'Č' => 'C',
            'ď' | 'Ď' => 'D',
            'ľ' | 'Ľ' | 'ĺ' | 'Ĺ' => 'L',
            'ň' | 'Ň' => 'N',
            'ř' | 'Ř' => 'R',
            'š' | 'Š' => 'S',
            'ť' | 'Ť' => 'T',
            'ž' | 'Ž' => 'Z',
            'ě' | 'Ě' => 'E',
            'ů' | 'Ů' => 'U',
            other => other,
        })
        .collect();
    let words: Vec<String> = folded
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_uppercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return None;
    }
    let mut out = words[0].clone();
    if out.chars().count() < 3 {
        if let Some(second) = words.get(1) {
            out.push_str(second);
        }
    }
    let capped: String = out.chars().take(8).collect();
    if capped.is_empty() {
        None
    } else {
        Some(capped)
    }
}

/// The counter row that numbers one kind of record within one event prefix,
/// e.g. `order:OASIS`. Orders and tickets for the same event each count from
/// 1, which is what marko's own example asks for.
fn scoped_counter(counter: &str, prefix: &str) -> String {
    format!("{counter}:{prefix}")
}

/// Like `next_code`, but numbered within `event_prefix`. Creates the counter
/// row on first use - unlike the five fixed counters, these cannot be seeded
/// by a migration because the set of events is not known in advance.
pub fn next_event_code(
    conn: &Connection,
    counter: &str,
    event_prefix: &str,
) -> rusqlite::Result<String> {
    let name = scoped_counter(counter, event_prefix);
    conn.execute(
        "INSERT OR IGNORE INTO counters (name, value) VALUES (?1, 0)",
        [&name],
    )?;
    let value: i64 = conn.query_row(
        "UPDATE counters SET value = value + 1 WHERE name = ?1 RETURNING value",
        [&name],
        |r| r.get(0),
    )?;
    Ok(format!("{event_prefix}-{value:03}"))
}

/// Batch form, for an order that creates many tickets at once.
pub fn next_event_code_batch(
    conn: &Connection,
    counter: &str,
    event_prefix: &str,
    n: i64,
) -> rusqlite::Result<Vec<String>> {
    if n <= 0 {
        return Ok(vec![]);
    }
    let name = scoped_counter(counter, event_prefix);
    conn.execute(
        "INSERT OR IGNORE INTO counters (name, value) VALUES (?1, 0)",
        [&name],
    )?;
    let new_value: i64 = conn.query_row(
        "UPDATE counters SET value = value + ?2 WHERE name = ?1 RETURNING value",
        rusqlite::params![&name, n],
        |r| r.get(0),
    )?;
    let start = new_value - n + 1;
    Ok((start..=new_value)
        .map(|v| format!("{event_prefix}-{v:03}"))
        .collect())
}

/// The prefix for one event id, or `None` when the event has no usable name.
/// Split out from `next_code_for_event` because an order mints its own code
/// and then a BATCH of ticket codes from the same event - looking the name up
/// twice for one insert would be two queries for one answer.
pub fn event_prefix_for(conn: &Connection, event_id: i64) -> Option<String> {
    let name: Option<String> = conn
        .query_row("SELECT name FROM events WHERE id = ?1", [event_id], |r| r.get(0))
        .ok();
    name.as_deref().and_then(prefix_for_event)
}

/// The prefix for the event a TICKET belongs to. Sales key on this: a sale
/// has no event of its own, it inherits the one its ticket was bought for.
pub fn event_prefix_for_ticket(conn: &Connection, ticket_id: i64) -> Option<String> {
    let name: Option<String> = conn
        .query_row(
            "SELECT e.name FROM tickets t JOIN events e ON e.id = t.event_id WHERE t.id = ?1",
            [ticket_id],
            |r| r.get(0),
        )
        .ok();
    name.as_deref().and_then(prefix_for_event)
}

/// Looks the event's name up and mints an event-scoped code, falling back to
/// the old fixed-prefix sequence when the event has no usable name. The
/// fallback matters: a code is NOT NULL UNIQUE in every table that has one,
/// so "no prefix" can never mean "no code".
pub fn next_code_for_event(
    conn: &Connection,
    counter: &str,
    fallback_prefix: &str,
    event_id: i64,
) -> rusqlite::Result<String> {
    match event_prefix_for(conn, event_id) {
        Some(p) => next_event_code(conn, counter, &p),
        None => next_code(conn, counter, fallback_prefix),
    }
}

/// One-time rewrite of every existing code to the event-derived form (2.30.0).
///
/// Runs once, guarded by an `app_settings` flag, right after migrations. It is
/// in Rust rather than in 029.sql for one reason: `prefix_for_event` folds
/// accents and applies word rules that SQL cannot express, and two different
/// implementations of that rule would be two different answers for the same
/// event - which, across marko's two machines, means two different codes for
/// the same record.
///
/// What it guarantees:
///   * the old code is preserved in `legacy_code`, so an already-synced Google
///     Sheet keeps matching its rows (see `029_event_codes.sql`).
///   * numbering is by `id` ascending within each (table, prefix), so both
///     machines rewriting their own copy of the same data land on the same
///     codes.
///   * the counters are advanced past the highest number used, so the next
///     record minted continues the run instead of colliding with it.
///   * a row whose event yields no usable prefix is LEFT ALONE - it keeps its
///     ORD-000091 code rather than being given an invented one.
///
/// It deliberately does NOT touch `batch_id`. A batch_id is the first line's
/// code as it was at creation; rewriting it would rename a group identity that
/// `GROUP_KEY_EXPR` already uses, and the group's displayed code now comes from
/// its MIN(id) row rather than from sorting codes, so the two still agree.
pub fn backfill_event_codes(conn: &Connection) -> rusqlite::Result<usize> {
    // (table, counter, how to reach the event's name)
    let plan: &[(&str, &str, &str)] = &[
        ("orders", "order", "SELECT o.id, e.name FROM orders o JOIN events e ON e.id = o.event_id ORDER BY o.id"),
        ("tickets", "ticket", "SELECT t.id, e.name FROM tickets t JOIN events e ON e.id = t.event_id ORDER BY t.id"),
        ("sales", "sale", "SELECT s.id, e.name FROM sales s JOIN tickets t ON t.id = s.ticket_id \
                           JOIN events e ON e.id = t.event_id ORDER BY s.id"),
        ("pulls", "pull", "SELECT id, event_name FROM pulls ORDER BY id"),
        ("pulls_received", "pull_received", "SELECT id, event_name FROM pulls_received ORDER BY id"),
    ];
    let mut rewritten = 0usize;
    for (table, counter, sql) in plan {
        let rows: Vec<(i64, Option<String>)> = {
            let mut stmt = conn.prepare(sql)?;
            let mapped = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };
        // Per-prefix running number, in id order - the deterministic part.
        let mut seen: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for (id, name) in rows {
            let prefix = match name.as_deref().and_then(prefix_for_event) {
                Some(p) => p,
                None => continue, // no usable name: keep the old code, invent nothing
            };
            let n = seen.entry(prefix.clone()).or_insert(0);
            *n += 1;
            let new_code = format!("{prefix}-{n:03}");
            conn.execute(
                &format!(
                    "UPDATE {table} SET legacy_code = COALESCE(legacy_code, code), code = ?1 WHERE id = ?2"
                ),
                rusqlite::params![new_code, id],
            )?;
            rewritten += 1;
        }
        // Advance each counter past what the rewrite consumed, so the next
        // minted code continues rather than colliding.
        for (prefix, used) in seen {
            let name = scoped_counter(counter, &prefix);
            conn.execute(
                "INSERT OR IGNORE INTO counters (name, value) VALUES (?1, 0)",
                [&name],
            )?;
            conn.execute(
                "UPDATE counters SET value = MAX(value, ?2) WHERE name = ?1",
                rusqlite::params![&name, used],
            )?;
        }
    }
    Ok(rewritten)
}
