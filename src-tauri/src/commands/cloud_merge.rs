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
    record_outcome, retrying, token, SyncGuard, CONFLICT_KEY, FILE_ID_KEY, LAST_ERROR_KEY, LAST_SYNC_KEY,
    REMOTE_VERSION_KEY,
};
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
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
    // 2.51.0: Notes. Standalone (no fks out of note_sheets), and note_rows
    // names its parent so the other machine's sheet ids are translated to
    // this one's - parent before child, same as every pair below.
    MergeTable { name: "note_sheets", fks: &[], natural_key: None, code: None },
    MergeTable { name: "note_rows", fks: &[("sheet_id", Some("note_sheets"))], natural_key: None, code: None },
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
    /// Records that WEAR the same identity as one of this machine's but are
    /// plainly not the same record. See `count_identity_clashes`.
    pub identity_clashes: i64,
    /// 2.20.0: rows removed here because the other machine deleted them.
    pub deleted: i64,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    pub tables: Vec<MergeTableResult>,
    pub total_inserted: i64,
    pub total_renumbered: i64,
    pub total_skipped: i64,
    /// Sum of `identity_clashes`. Non-zero means the one manual whole-file
    /// sync that migration 027 asks for was never done - see
    /// `count_identity_clashes`.
    pub total_identity_clashes: i64,
    /// 2.20.0: rows this merge removed because the other machine had deleted
    /// them - see `apply_tombstones`.
    pub total_deleted: i64,
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
///
/// 2.30.1: it now covers the EVENT-SCOPED counters too. 2.30.0 started minting
/// `CELINE-001` from a `order:CELINE` row, and those rows did not exist when
/// the fixed-prefix statement below was written - so nothing raised them, and
/// a merge could leave `order:CELINE` reading 3 while `CELINE-004` had already
/// arrived. That is exactly the days-later UNIQUE failure this function exists
/// to prevent, just one counter row over.
fn reconcile_counter(conn: &Connection, table: &str, counter: &str, prefix: &str) -> AppResult<()> {
    conn.execute(
        &format!(
            "UPDATE counters SET value = MAX(value, COALESCE(
                 (SELECT MAX(CAST(substr(code, ?2) AS INTEGER)) FROM main.{table} WHERE code LIKE ?3), 0))
             WHERE name = ?1"
        ),
        rusqlite::params![counter, (prefix.len() + 2) as i64, format!("{prefix}-%")],
    )?;
    // One statement for every event prefix the table actually holds, whether
    // or not this machine has ever minted for that event. The name it builds
    // is `codes::scoped_counter`'s - `order:CELINE` - and it must stay that
    // way or the reconciliation silently raises a row nothing mints from.
    //
    // Splitting at the FIRST '-' is safe: `prefix_for_event` keeps ASCII
    // letters and digits only, so an event prefix never contains one.
    //
    // The exclusion is `ORD-______` (six single-character wildcards), not
    // `ORD-%`: the old fixed-width codes are the ones the statement above
    // already handles, while an event legitimately named so that its prefix
    // IS "ORD" would produce `ORD-001` and still needs its own counter.
    conn.execute(
        &format!(
            "INSERT INTO counters (name, value)
                 SELECT ?1 || ':' || substr(code, 1, instr(code, '-') - 1),
                        MAX(CAST(substr(code, instr(code, '-') + 1) AS INTEGER))
                   FROM main.{table}
                  WHERE code LIKE '%-%' AND code NOT LIKE ?2
                  GROUP BY substr(code, 1, instr(code, '-') - 1)
             ON CONFLICT(name) DO UPDATE SET value = MAX(counters.value, excluded.value)"
        ),
        rusqlite::params![counter, format!("{prefix}-______")],
    )?;
    Ok(())
}

/// Counts records that wear the same identity as one of this machine's while
/// plainly being a different record.
///
/// Migration 027 gave rows that already existed the identity `legacy-<id>`,
/// which is what lets two databases descended from one file agree without
/// talking. It only holds for rows they actually shared. If the two machines
/// had already DRIFTED APART before the update - each holding records the
/// other had never seen - then both counted up from the same place, and the
/// Mac's seventh order and the PC's seventh order are two different orders
/// both calling themselves `legacy-7`.
///
/// The merge then looks at the arriving one, sees that identity already here,
/// and skips it as something this machine already has. Nothing breaks, nothing
/// is reported, and a real order simply never arrives. That is the one silent
/// failure this whole design exists to prevent, so it is detected and said out
/// loud instead: a shared identity whose `code` differs is not one record.
///
/// It is NOT repaired automatically. Deciding which `legacy-7` is which is a
/// human's call, and the fix is the one whole-file sync migration 027 already
/// asks for. Only tables with a `code` can be checked, which is fine - those
/// are the records marko actually creates.
fn count_identity_clashes(conn: &Connection, table: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM remote.{table} r JOIN main.{table} l ON l.uid = r.uid
             WHERE r.uid LIKE 'legacy-%' AND r.code IS NOT l.code"
        ),
        [],
        |row| row.get(0),
    )?;
    Ok(n)
}

fn merge_one_table(conn: &Connection, table: &MergeTable, reasons: &mut Vec<String>) -> AppResult<MergeTableResult> {
    let mut result = MergeTableResult { table: table.name.to_string(), ..Default::default() };
    if let Some((counter, prefix)) = table.code {
        reconcile_counter(conn, table.name, counter, prefix)?;
        result.identity_clashes = count_identity_clashes(conn, table.name)?;
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

    // 2.20.0: the second NOT IN is the whole point of tombstones. Without it
    // a record deleted here is simply a record "this machine has never seen",
    // so the next merge copies it straight back and the deletion undoes
    // itself. Absence carries no information; the tombstone does.
    let select = format!(
        "SELECT {} FROM remote.{} WHERE uid IS NOT NULL \
         AND uid NOT IN (SELECT uid FROM main.{} WHERE uid IS NOT NULL) \
         AND uid NOT IN (SELECT uid FROM main.deleted_rows WHERE table_name = '{}') \
         ORDER BY id",
        cols.join(", "),
        table.name,
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

/// Brings the other machine's tombstones over, then carries out what they say.
///
/// Three steps, in this order, and the order is load-bearing:
///
/// 1. **Copy the tombstones in.** They are keyed by `(table_name, uid)`,
///    which means the same thing on both machines, so this is a plain
///    `INSERT OR IGNORE` with no id translation anywhere. It also has to
///    happen before the insert pass, whose filter reads this table.
/// 2. **Delete what they name, children first.** `tickets.event_id` is
///    `ON DELETE RESTRICT`, so an event cannot go before its tickets do -
///    hence reverse `MERGE_TABLES` order, which is parent-before-child read
///    backwards. A delete the schema still refuses is skipped and counted,
///    never forced.
/// 3. Only then the ordinary insert pass runs.
///
/// A local delete that fires here writes a NEW local tombstone through 028's
/// triggers. That is harmless and idempotent - same `(table_name, uid)`,
/// `INSERT OR REPLACE`.
fn apply_tombstones(conn: &Connection, reasons: &mut Vec<String>) -> AppResult<Vec<(String, i64)>> {
    conn.execute(
        "INSERT OR IGNORE INTO main.deleted_rows(table_name, uid, deleted_at)
         SELECT table_name, uid, deleted_at FROM remote.deleted_rows",
        [],
    )?;

    let mut deleted: Vec<(String, i64)> = Vec::new();
    for table in MERGE_TABLES.iter().rev() {
        let uids: Vec<String> = {
            let mut stmt = conn.prepare(&format!(
                "SELECT uid FROM main.deleted_rows d WHERE d.table_name = ?1
                 AND EXISTS (SELECT 1 FROM main.{} t WHERE t.uid = d.uid)",
                table.name
            ))?;
            let rows = stmt.query_map([table.name], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()?
        };
        let mut count = 0i64;
        for uid in uids {
            match conn.execute(
                &format!("DELETE FROM main.{} WHERE uid = ?1", table.name),
                [&uid],
            ) {
                Ok(n) => count += n as i64,
                Err(e) => {
                    let reason = format!(
                        "{}: a record the other machine deleted is still referenced here ({e})",
                        table.name
                    );
                    if reasons.len() < MAX_SKIP_REASONS && !reasons.contains(&reason) {
                        reasons.push(reason);
                    }
                }
            }
        }
        if count > 0 {
            deleted.push((table.name.to_string(), count));
        }
    }
    Ok(deleted)
}

/// Runs the whole merge against an already-attached `remote` schema.
///
/// Split out from the command so it can be tested against two real databases
/// without Drive, Tauri or a network anywhere in the picture.
pub(crate) fn merge_attached(conn: &Connection) -> AppResult<(Vec<MergeTableResult>, Vec<String>)> {
    let mut reasons: Vec<String> = Vec::new();
    // Deletions first - see `apply_tombstones` for why the order matters.
    let deleted = apply_tombstones(conn, &mut reasons)?;
    let mut results = Vec::with_capacity(MERGE_TABLES.len());
    for table in MERGE_TABLES {
        let mut r = merge_one_table(conn, table, &mut reasons)?;
        r.deleted = deleted
            .iter()
            .find(|(name, _)| name == table.name)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        results.push(r);
    }
    Ok((results, reasons))
}

// --- what came from the other computer, kept (2.20.0) --------------------
//
// marko: the merge already reported what it did, but the report vanished with
// the toast. Kept here so sync stops being magic - "12 Sep: 3 orders and 4
// tickets arrived from the other computer" is the difference between trusting
// it and hoping.
//
// Stored as JSON in `app_settings` rather than a table on purpose: it is a
// display log, nothing queries it, and `app_settings` is one of the
// bookkeeping tables `db::is_bookkeeping_table` keeps out of the dirty flag -
// so writing the log cannot itself make the app think it needs to sync.

pub(crate) const MERGE_LOG_KEY: &str = "cloud_sync_merge_log";
/// Old entries fall off the end. This is a log to glance at, not an audit
/// trail, and an unbounded JSON blob in a settings row is a slow leak.
const MERGE_LOG_MAX: usize = 20;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MergeLogTable {
    pub table: String,
    pub inserted: i64,
    pub deleted: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MergeLogEntry {
    pub at: String,
    pub inserted: i64,
    pub deleted: i64,
    pub renumbered: i64,
    pub skipped: i64,
    pub identity_clashes: i64,
    /// Only the tables that actually changed - a list of seventeen zeroes is
    /// not a log entry.
    pub changed: Vec<MergeLogTable>,
}

fn append_merge_log(conn: &Connection, outcome: &MergeOutcome) {
    let mut log: Vec<MergeLogEntry> = get_setting(conn, MERGE_LOG_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();

    log.insert(
        0,
        MergeLogEntry {
            at: now_iso(),
            inserted: outcome.total_inserted,
            deleted: outcome.total_deleted,
            renumbered: outcome.total_renumbered,
            skipped: outcome.total_skipped,
            identity_clashes: outcome.total_identity_clashes,
            changed: outcome
                .tables
                .iter()
                .filter(|t| t.inserted > 0 || t.deleted > 0)
                .map(|t| MergeLogTable {
                    table: t.table.clone(),
                    inserted: t.inserted,
                    deleted: t.deleted,
                })
                .collect(),
        },
    );
    log.truncate(MERGE_LOG_MAX);
    // Best-effort: a log that cannot be written must never fail a merge that
    // already succeeded.
    if let Ok(raw) = serde_json::to_string(&log) {
        let _ = set_setting(conn, MERGE_LOG_KEY, &raw);
    }
}

/// Newest first. Read-only.
#[tauri::command(async)]
pub fn cloud_merge_history(state: State<'_, AppState>) -> AppResult<Vec<MergeLogEntry>> {
    let conn = state.db.lock().unwrap();
    let log: Vec<MergeLogEntry> = get_setting(&conn, MERGE_LOG_KEY)?
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    Ok(log)
}

/// Downloads the other machine's database and adds in everything this one is
/// missing. Never replaces, never deletes - see this module's header.
#[tauri::command(async)]
pub fn cloud_merge_pull(state: State<'_, AppState>) -> AppResult<MergeOutcome> {
    // 2.18.0: the same one-at-a-time guard every other sync entry point takes.
    // A merge running alongside a push would upload a database that is being
    // written to underneath it.
    let Some(_guard) = SyncGuard::acquire() else {
        return Err(AppError::Validation(
            "A sync is already running - wait for it to finish.".to_string(),
        ));
    };
    let mut conn = state.db.lock().unwrap();
    // Same lock order as `cloud_sync_pull`: db first, then db_path. Taking
    // them the other way round in one place and this way in another is how a
    // deadlock gets built.
    let db_path = state.db_path.lock().unwrap().clone();
    let outcome = merge_inner(&mut conn, db_path);
    // Same split as `cloud_sync_pull`: the body does the work, the command
    // records how it ended so the panel can still say so once the toast has
    // gone.
    record_outcome(&conn, &outcome);
    if let Ok(o) = &outcome {
        append_merge_log(&conn, o);
    }
    outcome
}

fn merge_inner(conn: &mut Connection, db_path: std::path::PathBuf) -> AppResult<MergeOutcome> {
    if !is_enabled(conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(conn)?;

    let file_id = match get_setting(conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => retrying(|| find_remote_file(&client, &access_token))?
            .map(|f| f.id)
            .ok_or_else(|| {
                AppError::Validation(
                    "There is nothing in your Drive to merge with yet - sync up from your other machine first."
                        .to_string(),
                )
            })?,
    };
    set_setting(conn, FILE_ID_KEY, &file_id)?;

    let meta = retrying(|| get_remote_meta(&client, &access_token, &file_id))?;
    let has_content = meta.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
    if !has_content {
        return Err(AppError::Validation(
            "The sync file in your Drive is empty - sync up from your other machine first.".to_string(),
        ));
    }

    let downloaded = temp_path("tiqr-cloud-merge.sqlite3");
    let _ = std::fs::remove_file(&downloaded);
    retrying(|| download_to_file(&client, &access_token, &file_id, &downloaded))?;
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
    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .to_path_buf();
    let safety_backup_path = create_safety_backup(conn, &safety_dir)?;

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
        set_setting(conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(conn, LAST_SYNC_KEY, &now_iso())?;

    let total_inserted = tables.iter().map(|t| t.inserted).sum();
    let total_renumbered = tables.iter().map(|t| t.renumbered).sum();
    let total_skipped: i64 = tables.iter().map(|t| t.skipped).sum();
    let total_identity_clashes: i64 = tables.iter().map(|t| t.identity_clashes).sum();
    let total_deleted: i64 = tables.iter().map(|t| t.deleted).sum();

    // 2.18.0: a merge that could not settle everything is a CONFLICT, and it
    // stays one until a merge comes back clean. Not cleared by an ordinary
    // push: the two machines still disagree about those records, and hiding
    // that behind a green tick is how it would never get fixed. Records what
    // it knows and stops - deciding which of two `legacy-7` orders is which
    // is marko's call, not a rule this app owns.
    let unresolved = total_skipped > 0 || total_identity_clashes > 0;
    set_setting(conn, CONFLICT_KEY, if unresolved { "true" } else { "false" })?;
    if !unresolved {
        let _ = set_setting(conn, LAST_ERROR_KEY, "");
    }
    Ok(MergeOutcome {
        tables,
        total_inserted,
        total_renumbered,
        total_skipped,
        total_identity_clashes,
        total_deleted,
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

    // --- tombstones (2.20.0) -----------------------------------------

    #[test]
    fn a_record_deleted_here_is_not_copied_back_from_the_other_machine() {
        // The hole marko named. Without a tombstone, a record deleted here is
        // simply one "this machine has never seen", so the merge copies it
        // back and the deletion undoes itself.
        let (conn, remote_path) = two_machines();
        let shared = "INSERT INTO events(name, uid) VALUES ('Shared','ev-1');
                      INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
                      VALUES ('ORD-000001', (SELECT id FROM events WHERE uid='ev-1'), '2026-05-01', 1, 1000, 'EUR', 'or-1');";
        conn.execute_batch(shared).unwrap();
        remote_exec(&remote_path, shared);

        // Deleted HERE - 028's trigger leaves the tombstone.
        conn.execute("DELETE FROM orders WHERE uid = 'or-1'", []).unwrap();
        let tombstoned: i64 = conn
            .query_row("SELECT COUNT(*) FROM deleted_rows WHERE uid = 'or-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tombstoned, 1, "the delete must leave a tombstone behind");

        attach(&conn, &remote_path);
        merge_attached(&conn).unwrap();
        assert_eq!(count(&conn, "orders"), 0, "the order must NOT come back");
    }

    #[test]
    fn a_record_the_other_machine_deleted_goes_away_here_too() {
        let (conn, remote_path) = two_machines();
        let shared = "INSERT INTO events(name, uid) VALUES ('Shared','ev-1');
                      INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
                      VALUES ('ORD-000001', (SELECT id FROM events WHERE uid='ev-1'), '2026-05-01', 1, 1000, 'EUR', 'or-1');";
        conn.execute_batch(shared).unwrap();
        remote_exec(&remote_path, shared);
        remote_exec(&remote_path, "DELETE FROM orders WHERE uid = 'or-1';");

        attach(&conn, &remote_path);
        let (results, _) = merge_attached(&conn).unwrap();
        assert_eq!(count(&conn, "orders"), 0);
        let orders = results.iter().find(|r| r.table == "orders").unwrap();
        assert_eq!(orders.deleted, 1);
        // And the tombstone came across, so a third machine cannot resurrect it.
        let here: i64 = conn
            .query_row("SELECT COUNT(*) FROM deleted_rows WHERE uid = 'or-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(here, 1);
    }

    #[test]
    fn a_deletion_the_schema_still_refuses_is_reported_not_forced() {
        // `tickets.event_id` is ON DELETE RESTRICT, so an event cannot go
        // while a ticket here still points at it. That is the schema's
        // decision, not something a merge may override - it is reported and
        // the rest of the merge carries on.
        let (conn, remote_path) = two_machines();
        conn.execute_batch(
            "INSERT INTO events(name, uid) VALUES ('Keep','ev-9');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
             VALUES ('ORD-000009', (SELECT id FROM events WHERE uid='ev-9'), '2026-05-01', 1, 1000, 'EUR', 'or-9');
             INSERT INTO tickets(code, event_id, order_id, currency, uid)
             VALUES ('TKT-000009', (SELECT id FROM events WHERE uid='ev-9'),
                     (SELECT id FROM orders WHERE uid='or-9'), 'EUR', 'tk-9');",
        )
        .unwrap();
        remote_exec(
            &remote_path,
            "INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
             VALUES ('events','ev-9','2026-09-12T10:00:00Z');",
        );

        attach(&conn, &remote_path);
        let (_, reasons) = merge_attached(&conn).unwrap();
        assert_eq!(count(&conn, "events"), 1, "the event is held by a ticket and must survive");
        assert_eq!(count(&conn, "tickets"), 1, "and the ticket must be untouched");
        assert!(
            reasons.iter().any(|r| r.contains("events")),
            "the refusal has to be said out loud: {reasons:?}"
        );
    }

    #[test]
    fn two_different_orders_wearing_the_same_legacy_identity_are_reported_not_swallowed() {
        // The one silent failure this design can still have. Machines that
        // drifted apart BEFORE migration 027 each counted up from the same
        // place, so the Mac's seventh order and the PC's seventh order both
        // call themselves `legacy-7`. The merge would see that identity
        // already present, skip the arriving order as "already have it", and
        // report nothing at all - a real order would simply never arrive.
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO events(name) VALUES ('Coldplay')", []).unwrap();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
             VALUES ('ORD-000007', (SELECT id FROM events WHERE name='Coldplay'), '2026-05-01', 1, 1000, 'EUR', 'legacy-7')",
            [],
        )
        .unwrap();
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Sparta');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
             VALUES ('ORD-000008', (SELECT id FROM events WHERE name='Sparta'), '2026-05-02', 1, 2000, 'EUR', 'legacy-7');",
        );
        attach(&conn, &remote_path);
        let (results, _) = merge_attached(&conn).unwrap();

        let orders = results.iter().find(|r| r.table == "orders").unwrap();
        assert_eq!(orders.identity_clashes, 1, "the collision must be counted and said out loud");
    }

    #[test]
    fn a_record_both_machines_genuinely_share_is_never_reported_as_a_clash() {
        // The other half: the `legacy-` scheme working exactly as intended
        // must stay silent, or the warning becomes noise nobody reads.
        let (conn, remote_path) = two_machines();
        for target in [None, Some(&remote_path)] {
            let sql = "INSERT INTO events(name) VALUES ('Shared');
                       INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency, uid)
                       VALUES ('ORD-000003', (SELECT id FROM events WHERE name='Shared'), '2026-05-01', 1, 1000, 'EUR', 'legacy-3');";
            match target {
                Some(path) => remote_exec(path, sql),
                None => conn.execute_batch(sql).unwrap(),
            }
        }
        attach(&conn, &remote_path);
        let (results, _) = merge_attached(&conn).unwrap();
        let orders = results.iter().find(|r| r.table == "orders").unwrap();
        assert_eq!(orders.identity_clashes, 0);
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
    fn an_arrived_event_code_drags_its_own_counter_up_too() {
        // The 2.30.0 version of the test above. Codes are minted per event now,
        // so the counter that can be left behind is `order:CELINE`, not
        // `order` - and the failure is the same one: everything works until
        // this machine's own numbering climbs back into what arrived.
        let (conn, remote_path) = two_machines();
        conn.execute("INSERT INTO events(name) VALUES ('Celine Dion')", []).unwrap();
        // Captured once: `events` has no natural key, so the merge inserts the
        // other machine's Celine Dion as a SECOND local event, and a name
        // lookup after the merge would no longer be the row we started from.
        let event_id: i64 = conn
            .query_row("SELECT id FROM events WHERE name='Celine Dion'", [], |r| r.get(0))
            .unwrap();
        for _ in 0..2 {
            let code = crate::codes::next_code_for_event(&conn, "order", "ORD", event_id).unwrap();
            conn.execute(
                "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
                 VALUES (?1, ?2, '2026-05-01', 1, 1000, 'EUR')",
                rusqlite::params![&code, event_id],
            )
            .unwrap();
        }
        // The other machine got further with the same event. CELINE-004 does
        // not clash with anything here, so it arrives untouched.
        remote_exec(
            &remote_path,
            "INSERT INTO events(name) VALUES ('Celine Dion');
             INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('CELINE-004', (SELECT id FROM events WHERE name='Celine Dion'), '2026-06-01', 1, 2000, 'EUR');",
        );
        attach(&conn, &remote_path);
        merge_attached(&conn).unwrap();

        let counter: i64 = conn
            .query_row("SELECT value FROM counters WHERE name = 'order:CELINE'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(counter, 4, "the event's own counter must have been dragged up as well");

        let next = crate::codes::next_code_for_event(&conn, "order", "ORD", event_id).unwrap();
        assert_eq!(next, "CELINE-005");
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES (?1, ?2, '2026-08-01', 1, 1000, 'EUR')",
            rusqlite::params![&next, event_id],
        )
        .expect("creating an order for that event after a merge must still work");
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
