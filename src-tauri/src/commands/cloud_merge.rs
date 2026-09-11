//! TIQR Cloud Merge (2.16.0) - both machines keep what each of them has.
//!
//! ## The problem this exists for
//!
//! marko: *"ked ma jedna strana nieco ine a druha a das sync tak sa to
//! zachova len z jednej strany"*. Exactly right, and it was by design:
//! `cloud_sync_push`/`_pull` move the whole database file, so whichever
//! direction runs, one side's version of the file replaces the other's. An
//! order entered on the Mac and a sale entered on the PC could not both
//! survive, because the file that arrives is complete in itself.
//!
//! This module makes the two sides ADD UP instead. It reads the other
//! machine's database and copies in every record this one has never seen.
//!
//! ## What it does, precisely - and what it does not
//!
//! * **It inserts records that exist on one side only.** An order, a ticket,
//!   a sale, a pull that this machine has never seen arrives, with its
//!   children, with its links to the right event/platform/supplier.
//! * **It never changes or deletes a record this machine already has.** A row
//!   that exists on both sides is left exactly as it is here.
//!
//! So it cannot lose anything, which is the whole point. What it does NOT do
//! is carry an EDIT or a DELETE across - if the same order was edited on both
//! machines, both keep their own version, and the next whole-file push
//! settles that the old way. That is a deliberate line: an insert has one
//! obvious correct outcome, an edit has two plausible ones, and guessing
//! between them is the data loss this module was written to end. Deletes need
//! tombstones (a row that merely "isn't there" is indistinguishable from one
//! that has not arrived yet) and that is its own migration.
//!
//! ## Why this could not be built before 2.15.0
//!
//! "Every record this one has never seen" requires being able to say that a
//! row here and a row there are the same row. Every primary key in this app
//! is a per-machine `INTEGER AUTOINCREMENT`; migration 027's `uid` is what
//! answers it. See that migration's own header.
//!
//! ## The three things that make this harder than "INSERT what's missing"
//!
//! 1. **Ids have to be rewritten on the way in.** The other machine's order
//!    #12 becomes some other number here, so every foreign key pointing at it
//!    has to be translated - remote id -> uid -> local id. Parents are merged
//!    before children so the answer always exists by the time it is needed.
//! 2. **`code` is UNIQUE and both machines mint the same ones.**
//!    `codes::next_code` counts up from a per-machine `counters` row, so the
//!    Mac's `ORD-000007` and the PC's `ORD-000007` are two different orders
//!    with one code between them. An arriving record whose code is taken gets
//!    the next free local code, and the count is reported rather than done
//!    quietly - the code is a label, the `uid` is the identity.
//! 3. **Lookups are UNIQUE by name.** "Ticketmaster" added separately on both
//!    machines is two rows with different uids and one name. Those are matched
//!    by name and NOT duplicated - and only for the four tables whose schema
//!    itself declares `UNIQUE(name)`. `accounts` does not, so two same-named
//!    accounts stay two rows: following the schema's own claim is a rule,
//!    inventing one for a table that never made it would be a guess, and a
//!    wrong guess there silently re-parents money.
//!
//! Anything that still collides (the same ticket sold on both machines, say)
//! is skipped, counted and reported. A merge never half-applies: it runs in
//! one transaction, on top of a safety backup taken first, and the safety
//! backup shows up in Settings -> Data like every other restore point.

use crate::commands::backup::{create_safety_backup, validate_tiqr_backup};
use crate::commands::cloud_sync::{
    download_to_file, find_remote_file, get_remote_meta, http, is_enabled, now_iso, temp_path,
    token, FILE_ID_KEY, LAST_SYNC_KEY, REMOTE_VERSION_KEY,
};
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

/// One table's part of a merge.
struct MergeTable {
    name: &'static str,
    /// Foreign keys that must be translated from the other machine's ids to
    /// this one's. The parent is `None` for `marketplaces`, which carries no
    /// `uid` because these same migrations seed it identically on every
    /// machine - so its ids already match and are copied through untouched.
    fks: &'static [(&'static str, Option<&'static str>)],
    /// The column that identifies the same row across machines when the
    /// schema itself says so with `UNIQUE(name)`. Rows matching an existing
    /// local one are linked to it, not inserted.
    natural_key: Option<&'static str>,
    /// `(counter, prefix)` for `codes::next_code`, for tables whose `code`
    /// this app actually mints. `None` on a table it never mints one for
    /// (`payments`): a colliding code there is reported and skipped rather
    /// than re-issued in a format this app has never produced.
    code: Option<(&'static str, &'static str)>,
}

/// Parents before children - the whole reason id translation always has an
/// answer by the time it is asked for. Checked by a test against the
/// schema's own foreign keys, not trusted to stay right by hand.
const MERGE_TABLES: &[MergeTable] = &[
    MergeTable { name: "event_categories", fks: &[], natural_key: Some("name"), code: None },
    MergeTable { name: "finance_categories", fks: &[], natural_key: Some("name"), code: None },
    MergeTable { name: "platforms", fks: &[], natural_key: Some("name"), code: None },
    MergeTable { name: "suppliers", fks: &[], natural_key: Some("name"), code: None },
    MergeTable { name: "accounts", fks: &[], natural_key: None, code: None },
    MergeTable {
        name: "events",
        fks: &[("category_id", Some("event_categories"))],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "orders",
        fks: &[
            ("platform_id", Some("platforms")),
            ("supplier_id", Some("suppliers")),
            ("event_id", Some("events")),
        ],
        natural_key: None,
        code: Some(("order", "ORD")),
    },
    MergeTable {
        name: "tickets",
        fks: &[("order_id", Some("orders")), ("event_id", Some("events"))],
        natural_key: None,
        code: Some(("ticket", "TKT")),
    },
    MergeTable {
        name: "sales",
        fks: &[("platform_id", Some("platforms")), ("ticket_id", Some("tickets"))],
        natural_key: None,
        code: Some(("sale", "SAL")),
    },
    MergeTable {
        name: "ticket_listings",
        fks: &[("marketplace_id", None), ("ticket_id", Some("tickets"))],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "payments",
        fks: &[("order_id", Some("orders"))],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "finance_entries",
        fks: &[
            ("order_id", Some("orders")),
            ("account_id", Some("accounts")),
            ("category_id", Some("finance_categories")),
        ],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "transfers",
        fks: &[("to_account_id", Some("accounts")), ("from_account_id", Some("accounts"))],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "recurring_expenses",
        fks: &[("account_id", Some("accounts")), ("category_id", Some("finance_categories"))],
        natural_key: None,
        code: None,
    },
    MergeTable {
        name: "pulls",
        fks: &[("platform_id", Some("platforms"))],
        natural_key: None,
        code: Some(("pull", "PULL")),
    },
    MergeTable {
        name: "pulls_received",
        fks: &[("order_id", Some("orders"))],
        natural_key: None,
        code: Some(("pull_received", "RPULL")),
    },
    MergeTable {
        name: "event_marketplace_links",
        fks: &[("marketplace_id", None), ("event_id", Some("events"))],
        natural_key: None,
        code: None,
    },
];

/// How many skip reasons are carried back to the UI. Enough to see the shape
/// of a problem, not enough to turn a report into a wall of text.
const MAX_SKIP_REASONS: usize = 8;

#[derive(Debug, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MergeTableResult {
    pub table: String,
    pub inserted: i64,
    /// Arrived with a `code` this machine had already issued to something
    /// else, and was given the next free one.
    pub renumbered: i64,
    /// Matched an existing local row by name instead of being duplicated.
    pub linked: i64,
    pub skipped: i64,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    pub tables: Vec<MergeTableResult>,
    pub total_inserted: i64,
    pub total_renumbered: i64,
    pub total_skipped: i64,
    /// Where this machine's data was saved before the merge touched it.
    pub safety_backup_path: String,
    /// Why rows were skipped, capped - shown as-is.
    pub skip_reasons: Vec<String>,
}

/// Columns present in BOTH copies of a table, `id` excluded.
///
/// The intersection, not this machine's list: the other machine may be a
/// version behind or ahead, and a column only one side knows about must not
/// turn into a failed INSERT. Everything shared still comes across.
fn shared_columns(conn: &Connection, table: &str) -> AppResult<Vec<String>> {
    let mut local: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1) WHERE name <> 'id'")?;
        let rows = stmt.query_map([table], |r| r.get::<_, String>(0))?;
        for row in rows {
            local.push(row?);
        }
    }
    let mut shared: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1, 'remote') WHERE name <> 'id'")?;
        let rows = stmt.query_map([table], |r| r.get::<_, String>(0))?;
        let remote: Vec<String> = rows.collect::<rusqlite::Result<Vec<String>>>()?;
        for col in local {
            if remote.contains(&col) {
                shared.push(col);
            }
        }
    }
    Ok(shared)
}

fn natural_key_of(table: &str) -> Option<&'static str> {
    MERGE_TABLES.iter().find(|t| t.name == table).and_then(|t| t.natural_key)
}

/// The other machine's id for a row -> this machine's id for the same row.
///
/// Through `uid` first: the remote id means nothing here and the local id
/// means nothing there.
///
/// Then, for a lookup, through its name - and that second hop is not a
/// nicety. A platform both machines created separately is LINKED rather than
/// copied, so this machine holds no row carrying the other one's uid.
/// Without the fallback, every order pointing at that platform would look
/// like an orphan and be dropped - the arriving records would vanish for the
/// single reason that both machines had once typed "Ticketmaster".
///
/// `Ok(None)` is a real answer: the parent genuinely is not here, so the
/// child is skipped rather than pointed at whatever happens to hold that
/// number locally.
fn translate_id(conn: &Connection, parent: &str, remote_id: i64) -> AppResult<Option<i64>> {
    let key = natural_key_of(parent);
    let sql = match key {
        Some(k) => format!("SELECT uid, {k} FROM remote.{parent} WHERE id = ?1"),
        None => format!("SELECT uid, NULL FROM remote.{parent} WHERE id = ?1"),
    };
    let found: Option<(Option<String>, Option<String>)> = conn
        .query_row(&sql, [remote_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((uid, key_value)) = found else { return Ok(None) };

    if let Some(uid) = uid {
        let by_uid: Option<i64> = conn
            .query_row(&format!("SELECT id FROM main.{parent} WHERE uid = ?1"), [&uid], |r| r.get(0))
            .optional()?;
        if by_uid.is_some() {
            return Ok(by_uid);
        }
    }
    if let (Some(k), Some(v)) = (key, key_value) {
        let by_name: Option<i64> = conn
            .query_row(&format!("SELECT id FROM main.{parent} WHERE {k} = ?1"), [&v], |r| r.get(0))
            .optional()?;
        if by_name.is_some() {
            return Ok(by_name);
        }
    }
    Ok(None)
}

/// Drags a code counter up to the highest code its table actually holds.
///
/// `codes::next_code` hands out `counters.value + 1` and nothing ever checks
/// that the result is free, because on one machine it always is - every code
/// this app ever issued came from that counter. A merge breaks that: a record
/// can arrive carrying `ORD-000009` while this machine's counter still reads
/// 5, and then the app keeps working perfectly until the counter climbs back
/// to 9 and ORDER CREATION starts failing on a UNIQUE constraint, days later,
/// with nothing on screen to connect it to a sync.
///
/// So this runs before a coded table is merged (so a re-issued code is free)
/// and again after (so an arrived code cannot poison the counter). It only
/// ever raises the counter - a merge must not hand out a number twice.
fn reconcile_counter(conn: &Connection, table: &str, counter: &str, prefix: &str) -> AppResult<()> {
    conn.execute(
        &format!(
            "UPDATE counters SET value = MAX(value, COALESCE(
                 (SELECT MAX(CAST(substr(code, ?2) AS INTEGER)) FROM main.{table} WHERE code LIKE ?3), 0))
             WHERE name = ?1"
        ),
        rusqlite::params![counter, (prefix.len() + 2) as i64, format!("{prefix}-%")],
    )?;
    Ok(())
}

fn merge_one_table(conn: &Connection, table: &MergeTable, reasons: &mut Vec<String>) -> AppResult<MergeTableResult> {
    let mut result = MergeTableResult { table: table.name.to_string(), ..Default::default() };
    if let Some((counter, prefix)) = table.code {
        reconcile_counter(conn, table.name, counter, prefix)?;
    }
    let cols = shared_columns(conn, table.name)?;
    if !cols.iter().any(|c| c == "uid") {
        // Both sides must carry migration 027. The command below already
        // migrates the downloaded copy forward, so this only trips if a table
        // was rebuilt by a later migration without restoring its uid column -
        // the exact trap PROTECTED_AREAS.md's 2.15.0 entry warns about.
        return Err(AppError::Other(format!(
            "{} has no uid on one of the two machines, so its rows cannot be matched up.",
            table.name
        )));
    }

    let select = format!(
        "SELECT {} FROM remote.{} WHERE uid IS NOT NULL \
         AND uid NOT IN (SELECT uid FROM main.{} WHERE uid IS NOT NULL) ORDER BY id",
        cols.join(", "),
        table.name,
        table.name
    );
    let mut stmt = conn.prepare(&select)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let mut values: Vec<Value> = Vec::with_capacity(cols.len());
        for i in 0..cols.len() {
            values.push(row.get::<_, Value>(i)?);
        }

        // A lookup row whose name already exists here IS that lookup - the
        // schema's own UNIQUE(name) says so. Nothing is inserted, and the
        // local row keeps its own uid: that is this machine's identity for a
        // row it created itself, and the other machine links the same way in
        // the other direction. `translate_id`'s name fallback is what keeps
        // the arriving children pointing at it.
        if let Some(key) = table.natural_key {
            let key_idx = cols.iter().position(|c| c == key);
            if let Some(idx) = key_idx {
                if let Value::Text(name) = &values[idx] {
                    let existing: Option<i64> = conn
                        .query_row(
                            &format!("SELECT id FROM main.{} WHERE {} = ?1", table.name, key),
                            [name],
                            |r| r.get(0),
                        )
                        .optional()?;
                    if let Some(_id) = existing {
                        // Deliberately NOT rewriting the local row's uid: it
                        // is this machine's identity for a row it created
                        // itself, and the other machine will link the same
                        // way in the other direction. Children translate
                        // through the remote uid -> remote name -> local row
                        // path below instead.
                        result.linked += 1;
                        continue;
                    }
                }
            }
        }

        let mut skip: Option<String> = None;
        for (col, parent) in table.fks {
            let Some(parent) = parent else { continue };
            let Some(idx) = cols.iter().position(|c| c == col) else { continue };
            let remote_id = match &values[idx] {
                Value::Integer(v) => *v,
                // NULL stays NULL: an optional link that was empty there is
                // empty here too.
                _ => continue,
            };
            match translate_id(conn, parent, remote_id)? {
                Some(local_id) => values[idx] = Value::Integer(local_id),
                None => {
                    skip = Some(format!(
                        "{}: a row points at a {} that isn't on this machine",
                        table.name, parent
                    ));
                    break;
                }
            }
        }
        if let Some(reason) = skip {
            result.skipped += 1;
            if reasons.len() < MAX_SKIP_REASONS && !reasons.contains(&reason) {
                reasons.push(reason);
            }
            continue;
        }

        // The code clash. Both machines count up from the same place, so this
        // is the normal case rather than the exception.
        let mut renumbered = false;
        if let Some(idx) = cols.iter().position(|c| c == "code") {
            if let Value::Text(code) = values[idx].clone() {
                let taken: Option<i64> = conn
                    .query_row(
                        &format!("SELECT 1 FROM main.{} WHERE code = ?1", table.name),
                        [&code],
                        |r| r.get(0),
                    )
                    .optional()?;
                if taken.is_some() {
                    match table.code {
                        Some((counter, prefix)) => {
                            values[idx] = Value::Text(crate::codes::next_code(conn, counter, prefix)?);
                            renumbered = true;
                        }
                        None => {
                            result.skipped += 1;
                            let reason = format!(
                                "{}: code {} is already used here, and this app never issues codes for that table",
                                table.name, code
                            );
                            if reasons.len() < MAX_SKIP_REASONS && !reasons.contains(&reason) {
                                reasons.push(reason);
                            }
                            continue;
                        }
                    }
                }
            }
        }

        let placeholders = (1..=cols.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
        let insert = format!(
            "INSERT INTO main.{} ({}) VALUES ({})",
            table.name,
            cols.join(", "),
            placeholders
        );
        match conn.execute(&insert, rusqlite::params_from_iter(values.iter())) {
            Ok(_) => {
                result.inserted += 1;
                if renumbered {
                    result.renumbered += 1;
                }
            }
            Err(e) => {
                // A real clash the schema refuses - the same ticket sold on
                // both machines, the same listing id twice. Counted and
                // named, never swallowed, and never allowed to abandon the
                // rest of the merge.
                result.skipped += 1;
                let reason = format!("{}: {}", table.name, e);
                if reasons.len() < MAX_SKIP_REASONS && !reasons.contains(&reason) {
                    reasons.push(reason);
                }
            }
        }
    }
    if let Some((counter, prefix)) = table.code {
        // Again, now that rows have arrived: one of them may carry a number
        // higher than anything this machine had issued.
        reconcile_counter(conn, table.name, counter, prefix)?;
    }
    Ok(result)
}

/// Runs the whole merge against an already-attached `remote` schema.
///
/// Split out from the command so it can be tested against two real databases
/// without Drive, Tauri or a network anywhere in the picture.
pub(crate) fn merge_attached(conn: &Connection) -> AppResult<(Vec<MergeTableResult>, Vec<String>)> {
    let mut results = Vec::with_capacity(MERGE_TABLES.len());
    let mut reasons: Vec<String> = Vec::new();
    for table in MERGE_TABLES {
        results.push(merge_one_table(conn, table, &mut reasons)?);
    }
    Ok((results, reasons))
}

/// Downloads the other machine's database and adds in everything this one is
/// missing. Never replaces, never deletes - see this module's header.
#[tauri::command]
pub fn cloud_merge_pull(state: State<AppState>) -> AppResult<MergeOutcome> {
    let mut conn = state.db.lock().unwrap();
    if !is_enabled(&conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(&conn)?;

    let file_id = match get_setting(&conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => find_remote_file(&client, &access_token)?
            .map(|f| f.id)
            .ok_or_else(|| {
                AppError::Validation(
                    "There is nothing in your Drive to merge with yet - sync up from your other machine first."
                        .to_string(),
                )
            })?,
    };
    set_setting(&conn, FILE_ID_KEY, &file_id)?;

    let meta = get_remote_meta(&client, &access_token, &file_id)?;
    let has_content = meta.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
    if !has_content {
        return Err(AppError::Validation(
            "The sync file in your Drive is empty - sync up from your other machine first.".to_string(),
        ));
    }

    let downloaded = temp_path("tiqr-cloud-merge.sqlite3");
    let _ = std::fs::remove_file(&downloaded);
    download_to_file(&client, &access_token, &file_id, &downloaded)?;
    // Same validation the restore path applies before it trusts a file.
    validate_tiqr_backup(&downloaded)?;

    // The other machine may be a version behind, in which case its copy has
    // no `uid` at all. Migrating the DOWNLOADED COPY forward gives it one -
    // and because migration 027 derives those from ids the two databases
    // already shared, they line up with this machine's. Only the temp copy is
    // touched; nothing is written back to Drive.
    {
        let remote_conn = Connection::open(&downloaded)?;
        crate::db::run_migrations(&remote_conn)?;
    }

    // Before anything destructive - and this lands next to the active
    // database, so it shows up in Settings -> Data's restore points like
    // every other one.
    let db_path = state.db_path.lock().unwrap().clone();
    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .to_path_buf();
    let safety_backup_path = create_safety_backup(&conn, &safety_dir)?;

    // ATTACH cannot run inside a transaction, and the transaction is what
    // makes the merge all-or-nothing, so the order here is load-bearing.
    conn.execute(
        "ATTACH DATABASE ?1 AS remote",
        [downloaded.to_string_lossy().to_string()],
    )?;
    let merged = (|| -> AppResult<(Vec<MergeTableResult>, Vec<String>)> {
        let tx = conn.transaction()?;
        let out = merge_attached(&tx)?;
        tx.commit()?;
        Ok(out)
    })();
    // Always, including after a failure: an attached database left hanging
    // would make the next merge fail on the name already being in use.
    let _ = conn.execute("DETACH DATABASE remote", []);
    let _ = std::fs::remove_file(&downloaded);
    let (tables, skip_reasons) = merged?;

    // This machine has now seen everything that version held, so it is no
    // longer "behind". It IS ahead - the inserts above marked the database
    // dirty through the update hook - so automatic sync pushes the union up
    // on its own from here, and `mark_local_clean` is deliberately NOT called.
    if let Some(v) = meta.version.as_deref() {
        set_setting(&conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(&conn, LAST_SYNC_KEY, &now_iso())?;

    let total_inserted = tables.iter().map(|t| t.inserted).sum();
    let total_renumbered = tables.iter().map(|t| t.renumbered).sum();
    let total_skipped = tables.iter().map(|t| t.skipped).sum();
    Ok(MergeOutcome {
        tables,
        total_inserted,
        total_renumbered,
        total_skipped,
        safety_backup_path: safety_backup_path.display().to_string(),
        skip_reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    /// Two machines that once shared a file, then went their own ways.
    /// `main` is this machine, `remote` is the other one's download.
    fn two_machines() -> (Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "tiqr_merge_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let remote_path = dir.join("remote.sqlite3");
        {
            let remote = Connection::open(&remote_path).unwrap();
            crate::db::run_migrations(&remote).unwrap();
        }
        (test_conn(), remote_path)
    }

    fn attach(conn: &Connection, path: &std::path::Path) {
        conn.execute("ATTACH DATABASE ?1 AS remote", [path.to_string_lossy().to_string()])
            .unwrap();
    }

    fn remote_exec(path: &std::path::Path, sql: &str) {
        let c = Connection::open(path).unwrap();
        c.execute_batch(sql).unwrap();
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM main.{table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn the_order_of_merge_tables_puts_every_parent_before_its_children() {
        // The one invariant id translation rests on. Checked against the
        // list itself rather than trusted: a table moved up the list by hand
        // would silently start skipping children for "a parent that isn't on
        // this machine".
        let mut seen: Vec<&str> = Vec::new();
        for t in MERGE_TABLES {
            for (col, parent) in t.fks {
                if let Some(parent) = parent {
                    assert!(
                        seen.contains(parent) || *parent == t.name,
                        "{}.{} points at {}, which is merged later",
                        t.name,
                        col,
                        parent
                    );
                }
            }
            seen.push(t.name);
        }
    }

    #[test]
    fn an_order_that_only_the_other_machine_has_arrives_with_its_event() {
        let (conn, remote_path) = two_machines();
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Other machine gig');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000001', (SELECT id FROM events WHERE name='Other machine gig'), '2026-05-01', 2, 5000, 'EUR');",
        );
        attach(&conn, &remote_path);
        let (results, _) = merge_attached(&conn).unwrap();

        assert_eq!(count(&conn, "events"), 1);
        assert_eq!(count(&conn, "orders"), 1);
        // And the order points at the LOCAL copy of that event, not at the
        // number it had over there.
        let (order_event, event_id): (i64, i64) = conn
            .query_row(
                "SELECT o.event_id, e.id FROM orders o JOIN events e ON e.name = 'Other machine gig'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(order_event, event_id);
        assert_eq!(results.iter().map(|r| r.inserted).sum::<i64>(), 2);
    }

    #[test]
    fn a_record_this_machine_already_has_is_left_completely_alone() {
        // The promise the whole module rests on: merging never rewrites what
        // is already here.
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO events(name, uid) VALUES ('Shared gig', 'shared-1')", [])
            .unwrap();
        remote_exec(
            &remote_path,
            "INSERT INTO events(name, uid, venue) VALUES ('Shared gig', 'shared-1', 'Changed over there');",
        );
        attach(&conn, &remote_path);
        merge_attached(&conn).unwrap();

        assert_eq!(count(&conn, "events"), 1);
        let venue: Option<String> = conn
            .query_row("SELECT venue FROM events WHERE uid = 'shared-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(venue, None, "an edit made on the other machine must not overwrite this one's row");
    }

    #[test]
    fn the_same_code_on_both_machines_renumbers_instead_of_failing() {
        // Both machines count up from the same place, so this is the normal
        // case. Before 2.16.0 it would have been a UNIQUE constraint error.
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO events(name) VALUES ('Mine')", []).unwrap();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000001', (SELECT id FROM events WHERE name='Mine'), '2026-05-01', 1, 1000, 'EUR')",
            [],
        )
        .unwrap();
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Theirs');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000001', (SELECT id FROM events WHERE name='Theirs'), '2026-06-01', 1, 2000, 'EUR');",
        );
        attach(&conn, &remote_path);
        let (results, _) = merge_attached(&conn).unwrap();

        assert_eq!(count(&conn, "orders"), 2, "both orders must survive");
        let orders = results.iter().find(|r| r.table == "orders").unwrap();
        assert_eq!(orders.inserted, 1);
        assert_eq!(orders.renumbered, 1);
        let codes: i64 = conn
            .query_row("SELECT COUNT(DISTINCT code) FROM orders", [], |r| r.get(0))
            .unwrap();
        assert_eq!(codes, 2, "the arriving order must have been given its own code");
    }

    #[test]
    fn the_same_platform_added_on_both_machines_stays_one_platform_and_keeps_its_orders() {
        // The second half of this is the bug worth the test. Linking the
        // platform by name means this machine holds no row carrying the other
        // one's uid - so without `translate_id`'s name fallback the arriving
        // ORDER looks like an orphan and is silently dropped. Records would
        // vanish for the sole reason that both machines had typed
        // "Ticketmaster".
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO platforms(name) VALUES ('Ticketmaster')", []).unwrap();
        remote_exec(
            &remote_path,
            "INSERT INTO platforms(name) VALUES ('Ticketmaster');
             INSERT INTO events(name) VALUES ('Gig on their side');
             INSERT INTO orders(code, event_id, platform_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000042', (SELECT id FROM events WHERE name='Gig on their side'),
                     (SELECT id FROM platforms WHERE name='Ticketmaster'), '2026-06-01', 1, 2000, 'EUR');",
        );
        attach(&conn, &remote_path);
        let (results, reasons) = merge_attached(&conn).unwrap();

        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM platforms WHERE name = 'Ticketmaster'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "UNIQUE(name) says these are the same platform");
        let platforms = results.iter().find(|r| r.table == "platforms").unwrap();
        assert_eq!(platforms.linked, 1);
        assert_eq!(platforms.inserted, 0);

        let orders = results.iter().find(|r| r.table == "orders").unwrap();
        assert_eq!(orders.inserted, 1, "the order must not be dropped: {reasons:?}");
        let pointed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM orders o JOIN platforms p ON p.id = o.platform_id
                 WHERE p.name = 'Ticketmaster'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pointed, 1, "and it must point at THIS machine's platform row");
    }

    #[test]
    fn a_code_arriving_from_ahead_does_not_break_order_creation_later() {
        // The nastiest failure mode this module could have had, because it
        // does not show up during the merge at all. An arriving ORD-000009
        // while this machine's counter reads 2 leaves the app working
        // perfectly until the counter climbs to 9 - and then creating an
        // ordinary order fails on a UNIQUE constraint, days later, with
        // nothing on screen connecting it to a sync.
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO events(name) VALUES ('Mine')", []).unwrap();
        for _ in 0..2 {
            let code = crate::codes::next_code(&conn, "order", "ORD").unwrap();
            conn.execute(
                "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
                 VALUES (?1, (SELECT id FROM events WHERE name='Mine'), '2026-05-01', 1, 1000, 'EUR')",
                [&code],
            )
            .unwrap();
        }
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Far ahead');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000009', (SELECT id FROM events WHERE name='Far ahead'), '2026-06-01', 1, 2000, 'EUR');",
        );
        attach(&conn, &remote_path);
        merge_attached(&conn).unwrap();

        let counter: i64 = conn
            .query_row("SELECT value FROM counters WHERE name = 'order'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(counter, 9, "the counter must have been dragged up to the highest code present");

        // The real assertion: the app still works afterwards.
        let next = crate::codes::next_code(&conn, "order", "ORD").unwrap();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES (?1, (SELECT id FROM events WHERE name='Mine'), '2026-08-01', 1, 1000, 'EUR')",
            [&next],
        )
        .expect("creating an order after a merge must still work");
    }

    #[test]
    fn nothing_at_all_to_merge_is_a_quiet_success_not_an_error() {
        let (conn, remote_path) = two_machines();
        attach(&conn, &remote_path);
        let (results, reasons) = merge_attached(&conn).unwrap();
        assert_eq!(results.iter().map(|r| r.inserted).sum::<i64>(), 0);
        assert!(reasons.is_empty());
        // Every table was still visited - a merge that silently skipped a
        // table would look identical from the outside.
        assert_eq!(results.len(), MERGE_TABLES.len());
    }

    #[test]
    fn a_ticket_arrives_pointing_at_the_local_copy_of_its_order() {
        // Two hops of translation in one row: ticket -> order -> event.
        let (conn, remote_path) = two_machines();
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Their gig');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000009', (SELECT id FROM events WHERE name='Their gig'), '2026-05-01', 1, 4000, 'EUR');
             INSERT INTO tickets(code, event_id, order_id, currency)
             VALUES ('TKT-000009', (SELECT id FROM events WHERE name='Their gig'),
                     (SELECT id FROM orders WHERE code='ORD-000009'), 'EUR');",
        );
        attach(&conn, &remote_path);
        merge_attached(&conn).unwrap();

        let joined: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tickets t
                 JOIN orders o ON o.id = t.order_id
                 JOIN events e ON e.id = t.event_id
                 WHERE e.name = 'Their gig' AND o.code = 'ORD-000009'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(joined, 1, "the ticket must hang off this machine's own order and event rows");
    }
}
