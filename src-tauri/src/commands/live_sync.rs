//! 2.112.0 - live sync between computers.
//!
//! marko: "ako by sme vedeli urobit nejaky sync ktory v realnom case po
//! zatvoreni apky a otvoreni v inom zariadeni by tam boli insta informacie",
//! "ze vsetko by ako keby fungovalo spolu".
//!
//! ## What changes, and what was there
//!
//! Until 2.112.0 the two computers met through ONE FILE on Google Drive
//! (cloud_sync.rs), only when the app opened or closed, and the merge of that
//! file (cloud_merge.rs) only ever added rows and carried deletions. An EDIT
//! to a record both computers had - a price, a status, a sale - never
//! travelled. Live sync sends every change as it happens, record by record,
//! through the account's own space in Firestore, and the other computer
//! applies it the moment it arrives. The Drive file stays, as a backup.
//!
//! ## The pieces
//!
//! * **Capture.** A trigger on every table in `MERGE_TABLES` writes the
//!   record into `sync_outbox` whenever it is inserted, changed or deleted,
//!   stamped with a hybrid logical clock (`sync_control.hlc`, milliseconds,
//!   never behind the wall clock and never behind anything already seen from
//!   the other computer). Triggers and not Rust, for 027's reason: a change
//!   made by a path nobody thought of is still caught.
//! * **Send** (`take` / `ack`). The frontend (src/lib/liveSync.ts) takes a
//!   page of the outbox as documents - one per record, a reference to another
//!   record as that record's `uid`, never a local id - writes them to
//!   Firestore, whose rules refuse to overwrite a newer version, and
//!   acknowledges.
//! * **Receive** (`apply`). Documents from the other computer, in one
//!   transaction: newer edit wins (`h`, then device id for a tie); a deletion
//!   wins over any edit (migration 028's rule, unchanged); a record pointing
//!   at one not here yet waits in `sync_pending` until it is; a lookup both
//!   computers created on their own ("Ticketmaster") is matched by name and
//!   remembered in `sync_uid_alias`; two records with one code get the code
//!   decided the same way on both computers (lower uid keeps it) and the
//!   other is renumbered and sent again.
//! * **Settings** (`settings_*`). The groups shared_settings.rs shares
//!   through the Drive merge - mail rules, senders, webhooks - travel live
//!   too, whole, newest stamp wins. The Gmail credential does not (NOT_LIVE).
//!
//! ## What does not travel
//!
//! Mail (each computer reads Gmail itself), counters (each computer's own,
//! pulled up to the highest code that arrived), the Gmail credential, and
//! everything that is not in MERGE_TABLES.
use crate::commands::cloud_merge::{reconcile_counter, MergeTable, MERGE_TABLES};
use crate::commands::mail::{MAIL_ACCOUNT_EMAIL_KEY, MAIL_GRANTED_SCOPE_KEY, MAIL_REFRESH_TOKEN_KEY};
use crate::commands::sheets_sync::{delete_setting, get_setting, set_setting};
use crate::commands::shared_settings::{GROUPS, MAIL};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::types::Value as Sql;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

/// Milliseconds since 1970, in SQL - the same clock the triggers use.
const NOW_MS_SQL: &str = "CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)";

/// A column that names another record without a foreign key. `sheet_alerts.
/// row_id` is one on purpose (migration 033: an alert outlives its row), so
/// the Drive merge copies the number as it is - and it means a different row
/// on the other computer. Live sync sends it as the row's uid, and an alert
/// whose row is not there is still an alert: it is never held back for it.
const SOFT_REFS: &[(&str, &str, &str)] = &[("sheet_alerts", "row_id", "note_rows")];

/// Settings that never go to Firestore, whatever group they are in. The
/// Gmail credential stays on the computer that connected it (and in its own
/// Drive file): a Firebase password should not be a way into the mailbox.
const NOT_LIVE: &[&str] = &[MAIL_REFRESH_TOKEN_KEY, MAIL_GRANTED_SCOPE_KEY];

/// How much one `take` hands the frontend, roughly, in bytes of JSON. Pasted
/// pictures in notes are megabytes each; a page of them is one IPC message.
const TAKE_BYTES: usize = 6 * 1024 * 1024;

/// Joins a table and a uid into a Firestore document id: `orders~3fa2...`.
pub(crate) fn doc_id(table: &str, uid: &str) -> String {
    format!("{table}~{uid}")
}

fn table(name: &str) -> Option<&'static MergeTable> {
    MERGE_TABLES.iter().find(|t| t.name == name)
}

fn rank(name: &str) -> usize {
    MERGE_TABLES.iter().position(|t| t.name == name).unwrap_or(MERGE_TABLES.len())
}

/// Where a column points: `(parent table, soft)`.
fn reference(t: &MergeTable, column: &str) -> Option<(&'static str, bool)> {
    if let Some((_, Some(parent))) = t.fks.iter().find(|(c, _)| *c == column) {
        return Some((parent, false));
    }
    SOFT_REFS.iter().find(|(tn, c, _)| *tn == t.name && *c == column).map(|(_, _, p)| (*p, true))
}

// ---------------------------------------------------------------------------
// The clock.
// ---------------------------------------------------------------------------

/// The next stamp for a change made HERE.
fn next_hlc(conn: &Connection) -> AppResult<i64> {
    conn.execute(&format!("UPDATE sync_control SET hlc = MAX({NOW_MS_SQL}, hlc + 1) WHERE id = 1"), [])?;
    Ok(conn.query_row("SELECT hlc FROM sync_control WHERE id = 1", [], |r| r.get(0))?)
}

/// A change from the other computer was seen: anything made here from now on
/// is stamped after it, whatever this computer's own clock says.
fn observe_hlc(conn: &Connection, seen: i64) -> AppResult<()> {
    conn.execute("UPDATE sync_control SET hlc = MAX(hlc, ?1) WHERE id = 1", params![seen])?;
    Ok(())
}

/// Quiets the capture triggers while something that is not an edit made
/// here writes rows - this file's `apply`, and the Drive merge.
pub(crate) fn set_applying(conn: &Connection, on: bool) -> AppResult<()> {
    conn.execute("UPDATE sync_control SET applying = ?1 WHERE id = 1", params![i64::from(on)])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Capture.
// ---------------------------------------------------------------------------

/// One insert, update and delete trigger per table in `MERGE_TABLES`. Run at
/// the end of every `db::run_migrations`, and rebuilt every time rather than
/// `IF NOT EXISTS`: the update trigger names every column, so a column a
/// later migration adds is compared from the next start on.
///
/// * They capture from the first time live sync is turned on (`seeded_at`),
///   and go on capturing while it is off: the outbox holds one entry per
///   record however often it changes, and turning it back on then sends
///   exactly what changed meanwhile, stamped when it changed.
/// * An update is queued only if a column really changed - saving a form
///   untouched is not a change worth a write to Firestore.
/// * Changes being applied from the other computer (`applying`) are not
///   queued: sending them back would be an echo. A DELETE is queued even
///   then - the rows a deletion takes with it here (ON DELETE CASCADE) may be
///   ones the other computer has never had, or has under a name it matched
///   this one by, and a deletion is final either way.
pub(crate) fn install_triggers(conn: &Connection) -> rusqlite::Result<()> {
    let ready: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sync_control')",
        [],
        |r| r.get(0),
    )?;
    if !ready {
        return Ok(());
    }
    let on = "(SELECT seeded_at IS NOT NULL AND applying = 0 FROM sync_control WHERE id = 1)";
    let on_even_applying = "(SELECT seeded_at IS NOT NULL FROM sync_control WHERE id = 1)";
    let stamp = format!("UPDATE sync_control SET hlc = MAX({NOW_MS_SQL}, hlc + 1) WHERE id = 1;");
    let queue = |t: &str, row: &str, op: &str| {
        format!(
            "INSERT OR REPLACE INTO sync_outbox (table_name, row_id, uid, op, hlc, queued_at)
             VALUES ('{t}', {row}.id, {row}.uid, '{op}', (SELECT hlc FROM sync_control WHERE id = 1),
                     strftime('%Y-%m-%dT%H:%M:%fZ','now'));"
        )
    };
    for t in MERGE_TABLES {
        let n = t.name;
        let cols: Vec<String> = {
            let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1)")?;
            let rows = stmt.query_map([n], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        conn.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS {n}_live_sync_ins;
             DROP TRIGGER IF EXISTS {n}_live_sync_upd;
             DROP TRIGGER IF EXISTS {n}_live_sync_del;"
        ))?;
        if cols.is_empty() {
            continue;
        }
        let changed = cols.iter().map(|c| format!("NEW.\"{c}\" IS NOT OLD.\"{c}\"")).collect::<Vec<_>>().join(" OR ");
        conn.execute_batch(&format!(
            "CREATE TRIGGER {n}_live_sync_ins AFTER INSERT ON {n} WHEN {on}
             BEGIN {stamp} {ins} END;
             CREATE TRIGGER {n}_live_sync_upd AFTER UPDATE ON {n} WHEN {on} AND ({changed})
             BEGIN {stamp} {upd} END;
             CREATE TRIGGER {n}_live_sync_del AFTER DELETE ON {n} WHEN {on_even_applying}
             BEGIN {stamp} {del} END;",
            ins = queue(n, "NEW", "upsert"),
            upd = queue(n, "NEW", "upsert"),
            del = queue(n, "OLD", "delete"),
        ))?;
    }
    Ok(())
}

/// Every record and every tombstone into the outbox, the first time live
/// sync is turned on. Stamped with when each record was last changed
/// (`updated_at`, else `created_at`), not with "now": two computers that
/// already hold the same record then carry the same version of it, and the
/// one that really was edited later wins.
pub(crate) fn seed(conn: &Connection) -> AppResult<usize> {
    let mut n = 0;
    for t in MERGE_TABLES {
        let cols = columns(conn, t.name)?;
        if !cols.iter().any(|c| c == "uid") {
            continue;
        }
        let when = ["updated_at", "created_at"].into_iter().find(|c| cols.iter().any(|x| x == c));
        let hlc = match when {
            Some(c) => format!("COALESCE(CAST((julianday({c}) - 2440587.5) * 86400000 AS INTEGER), 0)"),
            None => "0".to_string(),
        };
        n += conn.execute(
            &format!(
                "INSERT OR IGNORE INTO sync_outbox (table_name, row_id, uid, op, hlc, queued_at)
                 SELECT '{}', id, uid, 'upsert', {hlc}, strftime('%Y-%m-%dT%H:%M:%fZ','now')
                   FROM {} WHERE uid IS NOT NULL",
                t.name, t.name
            ),
            [],
        )?;
    }
    // Deletions made before live sync was on: a negative pseudo-id, because
    // the row they were is gone.
    n += conn.execute(
        "INSERT OR IGNORE INTO sync_outbox (table_name, row_id, uid, op, hlc, queued_at)
         SELECT table_name, -rowid, uid, 'delete',
                COALESCE(CAST((julianday(deleted_at) - 2440587.5) * 86400000 AS INTEGER), 0),
                strftime('%Y-%m-%dT%H:%M:%fZ','now')
           FROM deleted_rows",
        [],
    )?;
    conn.execute(
        &format!(
            "UPDATE sync_control
                SET hlc = MAX(hlc, {NOW_MS_SQL}, IFNULL((SELECT MAX(hlc) FROM sync_outbox), 0)),
                    seeded_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
              WHERE id = 1"
        ),
        [],
    )?;
    Ok(n)
}

/// A record this computer changed while applying - a code it had to give
/// way on - queued by hand, because the triggers are quiet while applying.
fn queue_here(conn: &Connection, table_name: &str, row_id: i64) -> AppResult<()> {
    let h = next_hlc(conn)?;
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO sync_outbox (table_name, row_id, uid, op, hlc, queued_at)
             SELECT ?1, id, uid, 'upsert', ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM {table_name} WHERE id = ?3"
        ),
        params![table_name, h, row_id],
    )?;
    Ok(())
}

/// A deletion this computer works out for itself while applying (a record
/// that arrived pointing at one deleted here, whose foreign key says it goes
/// with it): remembered, and sent, so Firestore stops calling it alive.
fn queue_deletion(conn: &Connection, table_name: &str, uid: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO deleted_rows (table_name, uid) VALUES (?1, ?2)",
        params![table_name, uid],
    )?;
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM deleted_rows WHERE table_name = ?1 AND uid = ?2",
        params![table_name, uid],
        |r| r.get(0),
    )?;
    let h = next_hlc(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO sync_outbox (table_name, row_id, uid, op, hlc, queued_at)
         VALUES (?1, ?2, ?3, 'delete', ?4, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        params![table_name, -rowid, uid, h],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Values between SQLite and the documents.
// ---------------------------------------------------------------------------

fn columns(conn: &Connection, table: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare_cached("SELECT name FROM pragma_table_info(?1)")?;
    let rows = stmt.query_map([table], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn to_json(v: Sql) -> Json {
    match v {
        Sql::Null => Json::Null,
        Sql::Integer(i) => Json::from(i),
        Sql::Real(f) => serde_json::Number::from_f64(f).map(Json::Number).unwrap_or(Json::Null),
        Sql::Text(s) => Json::String(s),
        Sql::Blob(b) => {
            use base64::Engine;
            let mut m = Map::new();
            m.insert("$b64".into(), Json::String(base64::engine::general_purpose::STANDARD.encode(b)));
            Json::Object(m)
        }
    }
}

fn to_sql(v: &Json) -> Sql {
    match v {
        Json::Null => Sql::Null,
        Json::Bool(b) => Sql::Integer(i64::from(*b)),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Sql::Integer(i),
            None => n.as_f64().map(Sql::Real).unwrap_or(Sql::Null),
        },
        Json::String(s) => Sql::Text(s.clone()),
        Json::Object(m) => match m.get("$b64").and_then(|b| b.as_str()) {
            Some(b) => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.decode(b).map(Sql::Blob).unwrap_or(Sql::Null)
            }
            None => Sql::Text(v.to_string()),
        },
        Json::Array(_) => Sql::Text(v.to_string()),
    }
}

/// Equal as SQLite would compare them: `5` and `5.0` are the same number.
fn same_value(a: &Sql, b: &Sql) -> bool {
    match (a, b) {
        (Sql::Integer(x), Sql::Real(y)) | (Sql::Real(y), Sql::Integer(x)) => (*x as f64) == *y,
        _ => a == b,
    }
}

fn uid_of(conn: &Connection, table: &str, id: i64) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(&format!("SELECT uid FROM {table} WHERE id = ?1"), params![id], |r| r.get(0))
        .optional()?
        .flatten())
}

fn reference_json(uid: String) -> Json {
    let mut m = Map::new();
    m.insert("$ref".into(), Json::String(uid));
    Json::Object(m)
}

/// One record as a document body: every column but `id` and `uid`, a
/// reference to another record as `{"$ref": its uid}`.
fn row_doc(conn: &Connection, t: &MergeTable, row_id: i64) -> AppResult<Option<(String, Map<String, Json>)>> {
    let cols = columns(conn, t.name)?;
    let list = cols.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
    let sql = format!("SELECT {list} FROM {} WHERE id = ?1", t.name);
    let values: Option<Vec<Sql>> = conn
        .query_row(&sql, params![row_id], |r| (0..cols.len()).map(|i| r.get::<_, Sql>(i)).collect())
        .optional()?;
    let Some(values) = values else { return Ok(None) };
    let mut uid = None;
    let mut d = Map::new();
    for (c, v) in cols.iter().zip(values) {
        match c.as_str() {
            "id" => continue,
            "uid" => {
                if let Sql::Text(u) = v {
                    uid = Some(u);
                }
                continue;
            }
            _ => {}
        }
        if let Some((parent, _)) = reference(t, c) {
            let r = match v {
                Sql::Integer(pid) => uid_of(conn, parent, pid)?.map(reference_json).unwrap_or(Json::Null),
                _ => Json::Null,
            };
            d.insert(c.clone(), r);
            continue;
        }
        d.insert(c.clone(), to_json(v));
    }
    Ok(uid.map(|u| (u, d)))
}

// ---------------------------------------------------------------------------
// Send.
// ---------------------------------------------------------------------------

/// One record as it travels. Short field names: Firestore stores them in
/// every document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncDoc {
    /// `doc_id(t, u)`.
    pub id: String,
    pub t: String,
    pub u: String,
    /// The version: the hybrid clock of the change.
    pub h: i64,
    #[serde(default)]
    pub del: bool,
    #[serde(default)]
    pub d: Map<String, Json>,
    /// The computer that made the change.
    #[serde(default)]
    pub dev: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutItem {
    pub table: String,
    pub row_id: i64,
    pub hlc: i64,
    pub doc: SyncDoc,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ack {
    pub table: String,
    pub row_id: i64,
    pub hlc: i64,
    pub uid: String,
    /// False when Firestore already held a version at least as new.
    pub written: bool,
}

/// The next changes to send: new and changed records parents first, deletions
/// children first - the order the other computer can apply them in.
pub(crate) fn take(conn: &Connection, limit: usize, device: &str) -> AppResult<Vec<OutItem>> {
    let ranks = MERGE_TABLES
        .iter()
        .enumerate()
        .map(|(i, t)| format!("WHEN '{}' THEN {i}", t.name))
        .collect::<Vec<_>>()
        .join(" ");
    let top = MERGE_TABLES.len();
    let entries: Vec<(String, i64, Option<String>, String, i64)> = {
        let mut stmt = conn.prepare(&format!(
            "SELECT table_name, row_id, uid, op, hlc FROM sync_outbox
              ORDER BY CASE WHEN op = 'delete' THEN 1 ELSE 0 END,
                       CASE WHEN op = 'delete' THEN {top} - (CASE table_name {ranks} ELSE {top} END)
                            ELSE (CASE table_name {ranks} ELSE {top} END) END,
                       hlc, row_id
              LIMIT ?1"
        ))?;
        let rows = stmt.query_map(params![limit as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    let mut out = Vec::new();
    let mut bytes = 0usize;
    for (name, row_id, uid, op, hlc) in entries {
        let drop_entry = |conn: &Connection| {
            conn.execute(
                "DELETE FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2 AND hlc = ?3",
                params![name, row_id, hlc],
            )
        };
        let Some(t) = table(&name) else {
            drop_entry(conn)?;
            continue;
        };
        let doc = if op == "delete" {
            let Some(u) = uid else {
                drop_entry(conn)?;
                continue;
            };
            SyncDoc { id: doc_id(t.name, &u), t: t.name.into(), u, h: hlc, del: true, d: Map::new(), dev: device.into() }
        } else {
            match row_doc(conn, t, row_id)? {
                Some((u, d)) => SyncDoc { id: doc_id(t.name, &u), t: t.name.into(), u, h: hlc, del: false, d, dev: device.into() },
                // Gone since it was queued; its deletion has its own entry.
                None => {
                    drop_entry(conn)?;
                    continue;
                }
            }
        };
        let size = serde_json::to_string(&doc.d).map(|s| s.len()).unwrap_or(0) + 200;
        if !out.is_empty() && bytes + size > TAKE_BYTES {
            break;
        }
        bytes += size;
        out.push(OutItem { table: name, row_id, hlc, doc });
    }
    Ok(out)
}

/// Firestore has what was sent. An entry is removed only if the record was
/// not changed again since it was taken - otherwise the newer change goes
/// next time.
pub(crate) fn ack(conn: &Connection, acks: &[Ack], device: &str) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for a in acks {
        tx.execute(
            "DELETE FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2 AND hlc = ?3",
            params![a.table, a.row_id, a.hlc],
        )?;
        if a.written {
            record_meta(&tx, &a.table, &a.uid, a.hlc, device)?;
        }
    }
    tx.execute(
        "UPDATE sync_control SET last_push_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
        [],
    )?;
    tx.commit()?;
    Ok(())
}

fn record_meta(conn: &Connection, table_name: &str, uid: &str, hlc: i64, device: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_row_meta (table_name, uid, hlc, device) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(table_name, uid) DO UPDATE
            SET device = CASE WHEN excluded.hlc >= sync_row_meta.hlc THEN excluded.device ELSE sync_row_meta.device END,
                hlc = MAX(sync_row_meta.hlc, excluded.hlc)",
        params![table_name, uid, hlc, device],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Receive.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    /// Records written here.
    pub applied: i64,
    /// Arrived with nothing new in them.
    pub unchanged: i64,
    /// Older than what this computer has, or of a record deleted here; this
    /// computer's own stands.
    pub older: i64,
    pub deleted: i64,
    /// Waiting for a record they point at, in all - not just this call's.
    pub pending: i64,
    /// Could not be applied at all; said in `reasons`.
    pub conflicts: i64,
    /// Tables whose records changed here, for the screens to refresh.
    pub changed_tables: Vec<String>,
    pub reasons: Vec<String>,
}

enum Applied {
    Changed,
    Unchanged,
    Older,
    Deleted,
    Pending(String),
    Conflict(String),
}

fn tombstoned(conn: &Connection, table_name: &str, uid: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM deleted_rows WHERE table_name = ?1 AND uid = ?2)",
        params![table_name, uid],
        |r| r.get(0),
    )?)
}

/// This computer's id for the record the other computer calls `uid`.
fn resolve(conn: &Connection, table_name: &str, uid: &str) -> AppResult<Option<i64>> {
    let direct: Option<i64> = conn
        .query_row(&format!("SELECT id FROM {table_name} WHERE uid = ?1"), params![uid], |r| r.get(0))
        .optional()?;
    if direct.is_some() {
        return Ok(direct);
    }
    let aliased: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT a.local_id FROM sync_uid_alias a JOIN {table_name} t ON t.id = a.local_id
                  WHERE a.table_name = ?1 AND a.remote_uid = ?2"
            ),
            params![table_name, uid],
            |r| r.get(0),
        )
        .optional()?;
    Ok(aliased)
}

/// The newest version this computer knows of a record: what it last sent or
/// applied, under either uid, and any change of its own still waiting to go
/// (`None` for the device: this computer's own).
fn local_version(conn: &Connection, table_name: &str, uid: &str, local_id: Option<i64>) -> AppResult<Option<(i64, Option<String>)>> {
    let mut best: Option<(i64, Option<String>)> = None;
    let mut consider = |v: Option<(i64, Option<String>)>| {
        if let Some(v) = v {
            if best.as_ref().is_none_or(|b| v.0 > b.0) {
                best = Some(v);
            }
        }
    };
    let meta = |u: &str| -> AppResult<Option<(i64, Option<String>)>> {
        Ok(conn
            .query_row(
                "SELECT hlc, IFNULL(device, '') FROM sync_row_meta WHERE table_name = ?1 AND uid = ?2",
                params![table_name, u],
                |r| Ok((r.get(0)?, Some(r.get(1)?))),
            )
            .optional()?)
    };
    consider(meta(uid)?);
    if let Some(id) = local_id {
        if let Some(own_uid) = uid_of(conn, table_name, id)? {
            if own_uid != uid {
                consider(meta(&own_uid)?);
            }
        }
        consider(
            conn.query_row(
                "SELECT hlc FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2",
                params![table_name, id],
                |r| Ok((r.get(0)?, None)),
            )
            .optional()?,
        );
    }
    Ok(best)
}

fn is_constraint(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

/// What the schema does to `column` when the record it points at goes:
/// `SET NULL`, `CASCADE`, or anything else (`RESTRICT`, `NO ACTION`).
fn on_delete(conn: &Connection, table_name: &str, column: &str) -> AppResult<String> {
    Ok(conn
        .query_row(
            "SELECT on_delete FROM pragma_foreign_key_list(?1) WHERE \"from\" = ?2",
            params![table_name, column],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| "NO ACTION".into()))
}

/// A free code for a record that has to give way: in its own event's run if
/// it has one (`OASIS-004`), else the table's fixed sequence. The counter is
/// pulled up to what the table holds first - a code that arrived from the
/// other computer may be ahead of it.
fn fresh_code(conn: &Connection, t: &MergeTable, counter: &str, prefix: &str, old: &str) -> AppResult<String> {
    reconcile_counter(conn, t.name, counter, prefix)?;
    let event_prefix = old
        .split_once('-')
        .filter(|(p, n)| {
            !p.is_empty()
                && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && !n.is_empty()
                && n.chars().all(|c| c.is_ascii_digit())
                && !(*p == prefix && n.len() == 6)
        })
        .map(|(p, _)| p.to_string());
    for _ in 0..1000 {
        let code = match &event_prefix {
            Some(p) => crate::codes::next_event_code(conn, counter, p)?,
            None => crate::codes::next_code(conn, counter, prefix)?,
        };
        let taken: bool = conn.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {} WHERE code = ?1)", t.name),
            params![code],
            |r| r.get(0),
        )?;
        if !taken {
            return Ok(code);
        }
    }
    Err(AppError::Other(format!("{}: no free code found", t.name)))
}

fn apply_delete(conn: &Connection, t: &MergeTable, doc: &SyncDoc) -> AppResult<Applied> {
    let local = resolve(conn, t.name, &doc.u)?;
    record_meta(conn, t.name, &doc.u, doc.h, &doc.dev)?;
    // Already gone here - by this very deletion arriving twice, or by the
    // cascade of its parent's: nothing of this computer's to send about it.
    conn.execute(
        "DELETE FROM sync_outbox WHERE table_name = ?1 AND uid = ?2 AND op = 'delete'",
        params![t.name, doc.u],
    )?;
    let Some(id) = local else {
        // Not here (yet): remembered, so a late copy cannot bring it back.
        conn.execute(
            "INSERT OR IGNORE INTO deleted_rows (table_name, uid) VALUES (?1, ?2)",
            params![t.name, doc.u],
        )?;
        return Ok(Applied::Unchanged);
    };
    let own_uid = uid_of(conn, t.name, id)?;
    match conn.execute(&format!("DELETE FROM {} WHERE id = ?1", t.name), params![id]) {
        Ok(_) => {}
        // Something here still points at it (a ticket at its event): the
        // deletion of that something is usually in the next page.
        Err(e) if is_constraint(&e) => return Ok(Applied::Pending(format!("{}: still in use here", t.name))),
        Err(e) => return Err(e.into()),
    }
    // The delete trigger queued this row's own deletion. Under the same uid
    // that is an echo; under the uid this computer gave the lookup it matched
    // by name, it is the one deletion the other computer needs to hear.
    if own_uid.as_deref() == Some(doc.u.as_str()) {
        conn.execute("DELETE FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2", params![t.name, id])?;
    }
    Ok(Applied::Deleted)
}

fn apply_doc(conn: &Connection, doc: &SyncDoc, me: &str) -> AppResult<Applied> {
    let Some(t) = table(&doc.t) else {
        return Ok(Applied::Conflict(format!("{}: not a table this version syncs", doc.t)));
    };
    observe_hlc(conn, doc.h)?;
    if doc.del {
        return apply_delete(conn, t, doc);
    }

    // A deletion wins over any edit - migration 028's rule, unchanged.
    if tombstoned(conn, t.name, &doc.u)? {
        return Ok(Applied::Older);
    }
    let mut local_id = resolve(conn, t.name, &doc.u)?;
    let local_cols = columns(conn, t.name)?;

    // A lookup both computers created on their own is the same lookup - the
    // schema's UNIQUE(name) says so. Matched, and remembered.
    if local_id.is_none() {
        if let Some(key) = t.natural_key {
            if let Some(Json::String(name)) = doc.d.get(key) {
                let by_name: Option<i64> = conn
                    .query_row(&format!("SELECT id FROM {} WHERE {key} = ?1", t.name), params![name], |r| r.get(0))
                    .optional()?;
                if let Some(id) = by_name {
                    conn.execute(
                        "INSERT OR REPLACE INTO sync_uid_alias (table_name, remote_uid, local_id) VALUES (?1, ?2, ?3)",
                        params![t.name, doc.u, id],
                    )?;
                    local_id = Some(id);
                }
            }
        }
    }

    // Newer edit wins; for the same version, the higher device id.
    let older = match local_version(conn, t.name, &doc.u, local_id)? {
        Some((h, dev)) => {
            let here = dev.unwrap_or_else(|| me.to_string());
            doc.h < h || (doc.h == h && doc.dev <= here)
        }
        None => false,
    };

    // References: the other computer's uid -> this computer's id.
    let mut values: Vec<(String, Sql)> = Vec::new();
    for (c, v) in &doc.d {
        if c == "id" || c == "uid" || !local_cols.iter().any(|lc| lc == c) {
            continue;
        }
        let sv = match (reference(t, c), v) {
            (Some((parent, soft)), Json::Object(m)) if m.contains_key("$ref") => {
                let puid = m.get("$ref").and_then(|x| x.as_str()).unwrap_or_default();
                match resolve(conn, parent, puid)? {
                    Some(pid) => Sql::Integer(pid),
                    None if soft => Sql::Null,
                    // Older anyway: nothing to wait for.
                    None if older => return Ok(Applied::Older),
                    None if tombstoned(conn, parent, puid)? => match on_delete(conn, t.name, c)?.as_str() {
                        "SET NULL" => Sql::Null,
                        "CASCADE" => {
                            // It went with its parent, here as it would have there.
                            queue_deletion(conn, t.name, &doc.u)?;
                            return Ok(Applied::Older);
                        }
                        _ => {
                            return Ok(Applied::Conflict(format!(
                                "{}: points at a {parent} that was deleted here",
                                t.name
                            )))
                        }
                    },
                    None => return Ok(Applied::Pending(format!("{}: waiting for its {parent}", t.name))),
                }
            }
            (Some(_), _) => Sql::Null,
            (None, v) => to_sql(v),
        };
        values.push((c.clone(), sv));
    }

    // The same content is the same version, whoever stamped it later: two
    // computers that both held a record before live sync existed must not
    // send it to each other once each.
    if let Some(id) = local_id {
        let names: Vec<String> = values.iter().map(|(c, _)| format!("\"{c}\"")).collect();
        let same = if names.is_empty() {
            true
        } else {
            let current: Vec<Sql> = conn.query_row(
                &format!("SELECT {} FROM {} WHERE id = ?1", names.join(", "), t.name),
                params![id],
                |r| (0..names.len()).map(|i| r.get::<_, Sql>(i)).collect(),
            )?;
            current.iter().zip(values.iter()).all(|(a, (_, b))| same_value(a, b))
        };
        if same {
            record_meta(conn, t.name, &doc.u, doc.h, &doc.dev)?;
            conn.execute("DELETE FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2", params![t.name, id])?;
            return Ok(Applied::Unchanged);
        }
    }
    if older {
        return Ok(Applied::Older);
    }

    // Migration 027 named rows that already existed `legacy-<id>`. Two
    // computers that had drifted apart before it hold two DIFFERENT records
    // under one such name - the Drive merge counts them as identity clashes
    // (cloud_merge::count_identity_clashes). Here too a different code says
    // they are not one record, and neither is overwritten.
    if let (Some(id), true) = (local_id, doc.u.starts_with("legacy-")) {
        if let Some(Json::String(theirs)) = doc.d.get("code") {
            let ours: Option<String> = conn
                .query_row(&format!("SELECT code FROM {} WHERE id = ?1", t.name), params![id], |r| r.get(0))
                .optional()?
                .flatten();
            if let Some(ours) = ours.filter(|o| o != theirs) {
                return Ok(Applied::Conflict(format!(
                    "{}: {ours} here and {theirs} on the other computer are both {} - not overwritten",
                    t.name, doc.u
                )));
            }
        }
    }

    // One code, two records: decided the same way on both computers - the
    // lower uid keeps it, the other is renumbered and the new code sent.
    let mut send_again = false;
    if let Some(idx) = values.iter().position(|(c, _)| c == "code") {
        if let Sql::Text(code) = values[idx].1.clone() {
            let clash: Option<(i64, String)> = conn
                .query_row(
                    &format!("SELECT id, IFNULL(uid, '') FROM {} WHERE code = ?1 AND id IS NOT ?2", t.name),
                    params![code, local_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some((other_id, other_uid)) = clash {
                let Some((counter, prefix)) = t.code else {
                    return Ok(Applied::Conflict(format!("{}: code {code} is already used here", t.name)));
                };
                let fresh = fresh_code(conn, t, counter, prefix, &code)?;
                if doc.u > other_uid {
                    values[idx].1 = Sql::Text(fresh);
                    send_again = true;
                } else {
                    conn.execute(&format!("UPDATE {} SET code = ?1 WHERE id = ?2", t.name), params![fresh, other_id])?;
                    queue_here(conn, t.name, other_id)?;
                }
            }
        }
    }

    let id = match local_id {
        Some(id) => {
            if !values.is_empty() {
                let sets = values
                    .iter()
                    .enumerate()
                    .map(|(i, (c, _))| format!("\"{c}\" = ?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut args: Vec<Sql> = values.iter().map(|(_, v)| v.clone()).collect();
                args.push(Sql::Integer(id));
                let sql = format!("UPDATE {} SET {sets} WHERE id = ?{}", t.name, args.len());
                if let Err(e) = conn.execute(&sql, rusqlite::params_from_iter(args.iter())) {
                    return if is_constraint(&e) { Ok(Applied::Conflict(format!("{}: {e}", t.name))) } else { Err(e.into()) };
                }
            }
            // The change of this computer's that this one beat is not sent.
            conn.execute("DELETE FROM sync_outbox WHERE table_name = ?1 AND row_id = ?2", params![t.name, id])?;
            id
        }
        None => {
            let mut names: Vec<String> = vec!["uid".into()];
            let mut args: Vec<Sql> = vec![Sql::Text(doc.u.clone())];
            for (c, v) in &values {
                names.push(format!("\"{c}\""));
                args.push(v.clone());
            }
            let marks = (1..=args.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
            let sql = format!("INSERT INTO {} ({}) VALUES ({marks})", t.name, names.join(", "));
            if let Err(e) = conn.execute(&sql, rusqlite::params_from_iter(args.iter())) {
                return if is_constraint(&e) { Ok(Applied::Conflict(format!("{}: {e}", t.name))) } else { Err(e.into()) };
            }
            conn.last_insert_rowid()
        }
    };
    record_meta(conn, t.name, &doc.u, doc.h, &doc.dev)?;
    if send_again {
        queue_here(conn, t.name, id)?;
    }
    Ok(Applied::Changed)
}

fn note(out: &mut ApplyOutcome, reason: String) {
    if out.reasons.len() < 8 && !out.reasons.contains(&reason) {
        out.reasons.push(reason);
    }
}

/// One document, inside its own savepoint: a document that cannot be
/// applied leaves nothing of itself behind, and the rest go on. False while
/// it waits for a record it points at.
fn apply_one(conn: &Connection, doc: &SyncDoc, me: &str, out: &mut ApplyOutcome, changed: &mut BTreeSet<String>) -> AppResult<bool> {
    conn.execute_batch("SAVEPOINT live_sync_doc")?;
    let result = apply_doc(conn, doc, me);
    let undo = |conn: &Connection| conn.execute_batch("ROLLBACK TO live_sync_doc; RELEASE live_sync_doc");
    match result {
        Ok(Applied::Pending(reason)) => {
            undo(conn)?;
            // The newest version waits; an older one never replaces it.
            conn.execute(
                "INSERT INTO sync_pending (doc_id, body, received_at, reason)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?3)
                 ON CONFLICT(doc_id) DO UPDATE SET body = excluded.body, reason = excluded.reason
                  WHERE json_extract(excluded.body, '$.h') >= json_extract(sync_pending.body, '$.h')",
                params![doc.id, serde_json::to_string(doc).unwrap_or_default(), reason],
            )?;
            Ok(false)
        }
        Ok(Applied::Conflict(reason)) => {
            undo(conn)?;
            conn.execute("DELETE FROM sync_pending WHERE doc_id = ?1", params![doc.id])?;
            out.conflicts += 1;
            note(out, reason);
            Ok(true)
        }
        Err(e) => {
            undo(conn)?;
            conn.execute("DELETE FROM sync_pending WHERE doc_id = ?1", params![doc.id])?;
            out.conflicts += 1;
            note(out, format!("{}: {e}", doc.t));
            Ok(true)
        }
        Ok(kind) => {
            conn.execute_batch("RELEASE live_sync_doc")?;
            conn.execute("DELETE FROM sync_pending WHERE doc_id = ?1", params![doc.id])?;
            match kind {
                Applied::Changed => {
                    out.applied += 1;
                    changed.insert(doc.t.clone());
                }
                Applied::Deleted => {
                    out.deleted += 1;
                    changed.insert(doc.t.clone());
                }
                Applied::Unchanged => out.unchanged += 1,
                _ => out.older += 1,
            }
            Ok(true)
        }
    }
}

/// Deletions first, children before parents; then everything else, parents
/// before children; within a table, oldest change first.
fn apply_order(d: &SyncDoc) -> (u8, usize, i64) {
    if d.del {
        (0, MERGE_TABLES.len() - rank(&d.t), d.h)
    } else {
        (1, rank(&d.t), d.h)
    }
}

/// Changes from the other computer, in one transaction - then whatever was
/// waiting for a record that has now arrived.
pub(crate) fn apply(conn: &Connection, docs: &[SyncDoc], me: &str) -> AppResult<ApplyOutcome> {
    let tx = conn.unchecked_transaction()?;
    set_applying(&tx, true)?;
    let mut out = ApplyOutcome::default();
    let mut changed = BTreeSet::new();
    let mut order: Vec<&SyncDoc> = docs.iter().collect();
    order.sort_by_key(|d| apply_order(d));
    for doc in order {
        apply_one(&tx, doc, me, &mut out, &mut changed)?;
    }
    // Only a record that arrived (or went) can free one that waits.
    if out.applied + out.deleted > 0 {
        retry_pending(&tx, me, &mut out, &mut changed)?;
    }
    out.pending = tx.query_row("SELECT COUNT(*) FROM sync_pending", [], |r| r.get(0))?;
    // A code that arrived must never be minted again here.
    for t in MERGE_TABLES {
        if let Some((counter, prefix)) = t.code {
            if changed.contains(t.name) {
                reconcile_counter(&tx, t.name, counter, prefix)?;
            }
        }
    }
    set_applying(&tx, false)?;
    tx.execute(
        "UPDATE sync_control SET last_pull_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
        [],
    )?;
    tx.commit()?;
    out.changed_tables = changed.into_iter().collect();
    Ok(out)
}

/// Everything waiting, again, until a pass settles nothing more: each pass
/// may free the next (an order, then its tickets, then their sales).
fn retry_pending(conn: &Connection, me: &str, out: &mut ApplyOutcome, changed: &mut BTreeSet<String>) -> AppResult<()> {
    loop {
        let waiting: Vec<(String, String)> = {
            let mut stmt = conn.prepare("SELECT doc_id, body FROM sync_pending ORDER BY received_at, doc_id")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        let mut docs: Vec<SyncDoc> = Vec::with_capacity(waiting.len());
        for (id, body) in &waiting {
            match serde_json::from_str::<SyncDoc>(body) {
                Ok(d) => docs.push(d),
                // Not something this version can read: it would wait forever.
                Err(_) => {
                    conn.execute("DELETE FROM sync_pending WHERE doc_id = ?1", params![id])?;
                }
            }
        }
        docs.sort_by_key(apply_order);
        let mut settled = false;
        for doc in &docs {
            if apply_one(conn, doc, me, out, changed)? {
                settled = true;
            }
        }
        if !settled {
            return Ok(());
        }
    }
}

// ---------------------------------------------------------------------------
// Settings - the groups shared_settings.rs shares, newest stamp wins.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsDoc {
    pub group: String,
    pub stamp: String,
    #[serde(default)]
    pub keys: Map<String, Json>,
    /// Each table of the group, as its rows - every column, `id` included:
    /// a rule's id is the rule (`mail_rules.id` is its TEXT key).
    #[serde(default)]
    pub tables: Map<String, Json>,
    #[serde(default)]
    pub dev: String,
}

fn pushed_key(group: &str) -> String {
    format!("live_sync_pushed:{group}")
}

/// The groups changed here since they were last sent.
pub(crate) fn settings_take(conn: &Connection, device: &str) -> AppResult<Vec<SettingsDoc>> {
    let mut out = Vec::new();
    for g in GROUPS {
        let Some(stamp) = get_setting(conn, g.stamp_key)? else { continue };
        let pushed = get_setting(conn, &pushed_key(g.name))?;
        if pushed.as_deref().is_some_and(|p| p >= stamp.as_str()) {
            continue;
        }
        let mut keys = Map::new();
        for k in g.keys.iter().filter(|k| !NOT_LIVE.contains(k)) {
            keys.insert(k.to_string(), get_setting(conn, k)?.map(Json::String).unwrap_or(Json::Null));
        }
        let mut tables = Map::new();
        for tbl in g.tables {
            let cols = columns(conn, tbl)?;
            if cols.is_empty() {
                continue;
            }
            let list = cols.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
            let mut stmt = conn.prepare(&format!("SELECT {list} FROM {tbl}"))?;
            let rows = stmt.query_map([], |r| {
                let mut m = Map::new();
                for (i, c) in cols.iter().enumerate() {
                    m.insert(c.clone(), to_json(r.get::<_, Sql>(i)?));
                }
                Ok(Json::Object(m))
            })?;
            tables.insert(tbl.to_string(), Json::Array(rows.collect::<Result<_, _>>()?));
        }
        out.push(SettingsDoc { group: g.name.into(), stamp, keys, tables, dev: device.into() });
    }
    Ok(out)
}

/// Firestore has (or already had newer) these stamps.
pub(crate) fn settings_ack(conn: &Connection, sent: &[(String, String)]) -> AppResult<()> {
    for (group, stamp) in sent {
        let key = pushed_key(group);
        if get_setting(conn, &key)?.as_deref().is_none_or(|p| p < stamp.as_str()) {
            set_setting(conn, &key, stamp)?;
        }
    }
    Ok(())
}

/// The other computer's groups that are newer than this one's - whole,
/// deletions included, as the Drive merge does (shared_settings.rs). Two
/// exceptions, both about the mailbox: the credential never arrives
/// (NOT_LIVE), and a computer that has no mailbox never takes this one's
/// away - only another mailbox does, which is the one case this computer's
/// credential is for the wrong account.
pub(crate) fn settings_apply(conn: &Connection, docs: &[SettingsDoc]) -> AppResult<Vec<String>> {
    let tx = conn.unchecked_transaction()?;
    let mut taken = Vec::new();
    let mut other_mailbox = false;
    for doc in docs {
        let Some(g) = GROUPS.iter().find(|g| g.name == doc.group) else { continue };
        if get_setting(&tx, g.stamp_key)?.as_deref().is_some_and(|ours| ours >= doc.stamp.as_str()) {
            continue;
        }
        for k in g.keys {
            if NOT_LIVE.contains(k) {
                continue;
            }
            let incoming = match doc.keys.get(*k) {
                Some(Json::String(v)) => Some(v.as_str()),
                _ => None,
            };
            if *k == MAIL_ACCOUNT_EMAIL_KEY {
                let Some(theirs) = incoming.filter(|v| !v.is_empty()) else { continue };
                let ours = get_setting(&tx, k)?;
                if ours.as_deref().is_some_and(|o| !o.is_empty() && !o.eq_ignore_ascii_case(theirs)) {
                    crate::secrets::delete(&tx, MAIL_REFRESH_TOKEN_KEY)?;
                    crate::secrets::delete(&tx, MAIL_GRANTED_SCOPE_KEY)?;
                    other_mailbox = true;
                }
                set_setting(&tx, k, theirs)?;
                continue;
            }
            match incoming {
                Some(v) => set_setting(&tx, k, v)?,
                None => delete_setting(&tx, k)?,
            }
        }
        for tbl in g.tables {
            let Some(Json::Array(rows)) = doc.tables.get(*tbl) else { continue };
            let cols = columns(&tx, tbl)?;
            tx.execute(&format!("DELETE FROM {tbl}"), [])?;
            for row in rows {
                let Json::Object(m) = row else { continue };
                let present: Vec<&String> = cols.iter().filter(|c| m.contains_key(c.as_str())).collect();
                if present.is_empty() {
                    continue;
                }
                let marks = (1..=present.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
                let names = present.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
                let args: Vec<Sql> = present.iter().map(|c| to_sql(&m[c.as_str()])).collect();
                tx.execute(&format!("INSERT INTO {tbl} ({names}) VALUES ({marks})"), rusqlite::params_from_iter(args.iter()))?;
            }
        }
        set_setting(&tx, g.stamp_key, &doc.stamp)?;
        set_setting(&tx, &pushed_key(g.name), &doc.stamp)?;
        taken.push(g.name.to_string());
    }
    // Where this computer's reading of the old mailbox had got to means
    // nothing for the new one - what connecting it here would do too.
    if other_mailbox && taken.iter().any(|g| g == MAIL.name) && crate::mail_store::is_attached(&tx) {
        crate::commands::mail_sync::reset_engine(&tx)?;
    }
    tx.commit()?;
    Ok(taken)
}

// ---------------------------------------------------------------------------
// State, and the commands the frontend drives it with.
// ---------------------------------------------------------------------------

static DEVICE: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

/// This computer's id: a file next to the database, so it never travels with
/// the database to the other computer (a Drive "Sync down" copies the whole
/// file, and two computers with one id would take each other's changes for
/// their own).
pub(crate) fn device_id(db_path: &Path) -> AppResult<String> {
    let mut cached = DEVICE.lock().unwrap();
    if let Some((p, id)) = cached.as_ref() {
        if p == db_path {
            return Ok(id.clone());
        }
    }
    let stem = db_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "tiqr-manager".into());
    let file: PathBuf = db_path.with_file_name(format!("{stem}.device"));
    let existing = std::fs::read_to_string(&file).ok().map(|s| s.trim().to_string());
    let id = match existing {
        Some(id) if id.len() >= 16 && id.chars().all(|c| c.is_ascii_hexdigit()) => id,
        _ => {
            let id: String = (0..16).map(|_| format!("{:02x}", rand::random::<u8>())).collect();
            std::fs::write(&file, &id).map_err(|e| AppError::Io(format!("Couldn't save this computer's sync id: {e}")))?;
            id
        }
    };
    *cached = Some((db_path.to_path_buf(), id.clone()));
    Ok(id)
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveSyncStatus {
    pub enabled: bool,
    /// Whether on/off was ever decided for this database.
    pub decided: bool,
    pub device_id: String,
    pub cursor_s: Option<i64>,
    pub cursor_ns: Option<i64>,
    /// Changes made here, not sent yet.
    pub outbox: i64,
    /// Changes from the other computer waiting for a record they point at.
    pub waiting: i64,
    pub seeded_at: Option<String>,
    pub last_push_at: Option<String>,
    pub last_pull_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
    /// Off because this database was just replaced from a copy - see
    /// `pause_after_restore`.
    pub paused_after_restore: bool,
}

/// Said in Settings, and how `status_of` recognises the pause.
pub(crate) const PAUSED_AFTER_RESTORE: &str = "Paused on this computer because its data was replaced from a saved copy. \
Turning it on again brings in what changed on the other computer since that copy.";

/// A whole database put back from a copy - a backup, a restore point, an
/// older Drive version, a Sync down. Left on, live sync would at once bring
/// back over it everything the other computer changed since, deletions
/// included: the restore marko just asked for would quietly undo itself.
/// So it pauses here, says why, and marko turns it back on when he means to.
/// The capture triggers keep running, so nothing he does meanwhile is lost.
pub(crate) fn pause_after_restore(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "UPDATE sync_control
            SET enabled = 0, last_error = ?1, last_error_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
          WHERE id = 1 AND enabled = 1",
        params![PAUSED_AFTER_RESTORE],
    )?;
    Ok(())
}

pub(crate) fn status_of(conn: &Connection, device: String) -> AppResult<LiveSyncStatus> {
    let mut s: LiveSyncStatus = conn.query_row(
        "SELECT enabled, decided_at IS NOT NULL, cursor_s, cursor_ns, seeded_at, last_push_at, last_pull_at,
                last_error, last_error_at
           FROM sync_control WHERE id = 1",
        [],
        |r| {
            Ok(LiveSyncStatus {
                enabled: r.get::<_, i64>(0)? != 0,
                decided: r.get::<_, i64>(1)? != 0,
                cursor_s: r.get(2)?,
                cursor_ns: r.get(3)?,
                seeded_at: r.get(4)?,
                last_push_at: r.get(5)?,
                last_pull_at: r.get(6)?,
                last_error: r.get(7)?,
                last_error_at: r.get(8)?,
                ..Default::default()
            })
        },
    )?;
    s.device_id = device;
    s.paused_after_restore = !s.enabled && s.last_error.as_deref() == Some(PAUSED_AFTER_RESTORE);
    s.outbox = conn.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0))?;
    s.waiting = conn.query_row("SELECT COUNT(*) FROM sync_pending", [], |r| r.get(0))?;
    Ok(s)
}

/// On or off. The first time it is turned on, everything is queued for the
/// first send (`seed`); after that the triggers never stop capturing, so
/// turning it on again sends what changed while it was off.
pub(crate) fn set_enabled(conn: &Connection, enabled: bool) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE sync_control SET enabled = ?1, decided_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
        params![i64::from(enabled)],
    )?;
    let seeded: bool = tx.query_row("SELECT seeded_at IS NOT NULL FROM sync_control WHERE id = 1", [], |r| r.get(0))?;
    if enabled && !seeded {
        seed(&tx)?;
    }
    if enabled {
        clear_error(&tx)?;
    }
    tx.commit()?;
    Ok(())
}

fn device_of(state: &AppState) -> AppResult<String> {
    let path = state.db_path.lock().unwrap().clone();
    device_id(&path)
}

#[tauri::command]
pub fn live_sync_status(state: State<AppState>) -> AppResult<LiveSyncStatus> {
    let device = device_of(&state)?;
    let conn = state.db.lock().unwrap();
    status_of(&conn, device)
}

#[tauri::command]
pub fn live_sync_set_enabled(state: State<AppState>, enabled: bool) -> AppResult<LiveSyncStatus> {
    let device = device_of(&state)?;
    let conn = state.db.lock().unwrap();
    set_enabled(&conn, enabled)?;
    status_of(&conn, device)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeResult {
    pub items: Vec<OutItem>,
    pub settings: Vec<SettingsDoc>,
}

#[tauri::command]
pub fn live_sync_take(state: State<AppState>, limit: usize) -> AppResult<TakeResult> {
    let device = device_of(&state)?;
    let conn = state.db.lock().unwrap();
    Ok(TakeResult {
        items: take(&conn, limit.clamp(1, 400), &device)?,
        settings: settings_take(&conn, &device)?,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsAck {
    pub group: String,
    pub stamp: String,
}

#[tauri::command]
pub fn live_sync_ack(state: State<AppState>, acks: Vec<Ack>, settings: Vec<SettingsAck>) -> AppResult<()> {
    let device = device_of(&state)?;
    let conn = state.db.lock().unwrap();
    ack(&conn, &acks, &device)?;
    let sent: Vec<(String, String)> = settings.into_iter().map(|s| (s.group, s.stamp)).collect();
    settings_ack(&conn, &sent)?;
    clear_error(&conn)
}

#[tauri::command]
pub fn live_sync_apply(state: State<AppState>, docs: Vec<SyncDoc>) -> AppResult<ApplyOutcome> {
    let device = device_of(&state)?;
    let conn = state.db.lock().unwrap();
    let out = apply(&conn, &docs, &device)?;
    clear_error(&conn)?;
    Ok(out)
}

#[tauri::command]
pub fn live_sync_apply_settings(state: State<AppState>, docs: Vec<SettingsDoc>) -> AppResult<Vec<String>> {
    let conn = state.db.lock().unwrap();
    settings_apply(&conn, &docs)
}

/// Where reading the other computer's changes got to: Firestore's commit
/// time of the last change applied. Only ever moves forward.
#[tauri::command]
pub fn live_sync_set_cursor(state: State<AppState>, seconds: i64, nanos: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    set_cursor(&conn, seconds, nanos)
}

pub(crate) fn set_cursor(conn: &Connection, seconds: i64, nanos: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE sync_control SET cursor_s = ?1, cursor_ns = ?2
          WHERE id = 1 AND (cursor_s IS NULL OR cursor_s < ?1 OR (cursor_s = ?1 AND IFNULL(cursor_ns, 0) <= ?2))",
        params![seconds, nanos],
    )?;
    Ok(())
}

/// What went wrong talking to Firestore, kept for Settings; `None` clears it.
#[tauri::command]
pub fn live_sync_report(state: State<AppState>, error: Option<String>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    match error {
        Some(e) => {
            conn.execute(
                "UPDATE sync_control SET last_error = ?1, last_error_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
                params![crate::commands::mail::safe_detail(&e)],
            )?;
            Ok(())
        }
        None => clear_error(&conn),
    }
}

fn clear_error(conn: &Connection) -> AppResult<()> {
    conn.execute("UPDATE sync_control SET last_error = NULL, last_error_at = NULL WHERE id = 1 AND last_error IS NOT NULL", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Two computers and a Firestore, all in memory. `Cloud` keeps the rule
    //! firestore.rules enforces - a deletion replaces any record and is
    //! replaced by nothing but a newer deletion; otherwise only a newer
    //! version replaces one - and a commit counter standing in for the
    //! server timestamp the listener reads in order. `Computer::push` and `pull` do what
    //! src/lib/liveSync.ts does, in pages, own documents included.
    use super::*;
    use crate::db::test_conn;
    use std::collections::BTreeMap;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[derive(Default)]
    struct Cloud {
        docs: BTreeMap<String, (SyncDoc, u64)>,
        settings: BTreeMap<String, SettingsDoc>,
        clock: u64,
        writes: usize,
    }

    impl Cloud {
        fn write(&mut self, doc: SyncDoc) -> bool {
            if let Some((cur, _)) = self.docs.get(&doc.id) {
                let newer = doc.h > cur.h || (doc.h == cur.h && doc.dev > cur.dev);
                let allowed = match (cur.del, doc.del) {
                    (false, true) => true,
                    (true, false) => false,
                    _ => newer,
                };
                if !allowed {
                    return false;
                }
            }
            self.clock += 1;
            self.writes += 1;
            self.docs.insert(doc.id.clone(), (doc, self.clock));
            true
        }
    }

    struct Computer {
        conn: Connection,
        dev: &'static str,
        cursor: u64,
    }

    impl Computer {
        /// `before` runs before live sync is turned on - what the computer
        /// already held.
        fn new(dev: &'static str, before: &str) -> Self {
            let conn = test_conn();
            // Seeded lookups carry the time the migration ran; two real
            // computers were set up on different days anyway.
            conn.execute_batch(
                "UPDATE event_categories SET created_at = '2026-01-01T00:00:00.000Z';
                 UPDATE finance_categories SET created_at = '2026-01-01T00:00:00.000Z';",
            )
            .unwrap();
            conn.execute_batch(before).unwrap();
            set_enabled(&conn, true).unwrap();
            Computer { conn, dev, cursor: 0 }
        }

        fn exec(&self, sql: &str) {
            self.conn.execute_batch(sql).unwrap();
        }

        fn one<T: rusqlite::types::FromSql>(&self, sql: &str) -> T {
            self.conn.query_row(sql, [], |r| r.get(0)).unwrap()
        }

        fn outbox(&self) -> i64 {
            self.one("SELECT COUNT(*) FROM sync_outbox")
        }

        /// The next change made here is stamped `ms` later than anything so far.
        fn later(&self, ms: i64) {
            self.exec(&format!("UPDATE sync_control SET hlc = hlc + {ms} WHERE id = 1"));
        }

        fn push(&self, cloud: &mut Cloud) {
            loop {
                let items = take(&self.conn, 9, self.dev).unwrap();
                if items.is_empty() {
                    break;
                }
                let acks: Vec<Ack> = items
                    .into_iter()
                    .map(|it| {
                        let written = cloud.write(it.doc.clone());
                        Ack { table: it.table, row_id: it.row_id, hlc: it.hlc, uid: it.doc.u, written }
                    })
                    .collect();
                ack(&self.conn, &acks, self.dev).unwrap();
            }
            for s in settings_take(&self.conn, self.dev).unwrap() {
                if cloud.settings.get(&s.group).is_none_or(|c| s.stamp > c.stamp) {
                    cloud.settings.insert(s.group.clone(), s.clone());
                }
                settings_ack(&self.conn, &[(s.group, s.stamp)]).unwrap();
            }
        }

        fn pull(&mut self, cloud: &Cloud) -> ApplyOutcome {
            let mut docs: Vec<(SyncDoc, u64)> = cloud.docs.values().filter(|(_, at)| *at > self.cursor).cloned().collect();
            docs.sort_by_key(|(_, at)| *at);
            let mut total = ApplyOutcome::default();
            for page in docs.chunks(7) {
                let batch: Vec<SyncDoc> = page.iter().map(|(d, _)| d.clone()).collect();
                let out = apply(&self.conn, &batch, self.dev).unwrap();
                total.applied += out.applied;
                total.unchanged += out.unchanged;
                total.older += out.older;
                total.deleted += out.deleted;
                total.conflicts += out.conflicts;
                total.pending = out.pending;
                total.reasons.extend(out.reasons);
                self.cursor = page.last().unwrap().1;
            }
            let settings: Vec<SettingsDoc> = cloud.settings.values().cloned().collect();
            settings_apply(&self.conn, &settings).unwrap();
            total
        }
    }

    /// Both computers online until neither has anything left to say.
    fn settle(a: &mut Computer, b: &mut Computer, cloud: &mut Cloud) {
        for _ in 0..8 {
            let before = cloud.writes;
            a.pull(cloud);
            a.push(cloud);
            b.pull(cloud);
            b.push(cloud);
            if cloud.writes == before && a.outbox() == 0 && b.outbox() == 0 {
                return;
            }
        }
        panic!("the two computers never stopped sending each other changes");
    }

    /// Every synced row, by uid, in a form both computers can be compared in:
    /// references as the uid of what they point at, never a local id.
    fn picture(c: &Computer) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for t in MERGE_TABLES {
            let ids: Vec<i64> = {
                let mut stmt = c.conn.prepare(&format!("SELECT id FROM {}", t.name)).unwrap();
                let rows = stmt.query_map([], |r| r.get(0)).unwrap();
                rows.collect::<Result<_, _>>().unwrap()
            };
            for id in ids {
                let (u, d) = row_doc(&c.conn, t, id).unwrap().unwrap();
                out.insert(doc_id(t.name, &u), serde_json::to_string(&d).unwrap());
            }
        }
        out
    }

    const EVENT_AND_ORDER: &str = "
        INSERT INTO events (name) VALUES ('Oasis · Wembley');
        INSERT INTO orders (code, event_id, purchase_date, quantity, total_cost_cents, currency)
             VALUES ('ORD-000001', (SELECT id FROM events WHERE name = 'Oasis · Wembley'), '2026-05-01', 2, 30000, 'EUR');
        INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, currency)
             SELECT 'TKT-000001', event_id, id, 15000, 'EUR' FROM orders WHERE code = 'ORD-000001';
        INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, currency)
             SELECT 'TKT-000002', event_id, id, 15000, 'EUR' FROM orders WHERE code = 'ORD-000001';";

    #[test]
    fn a_new_order_arrives_with_its_tickets_pointing_at_the_other_computers_own_ids() {
        let mut cloud = Cloud::default();
        let a = Computer::new(A, "");
        // B already has an event of its own, so the same records get
        // different ids there - a copied id would point at the wrong thing.
        let mut b = Computer::new(B, "INSERT INTO events (name) VALUES ('Something else');");
        a.exec(EVENT_AND_ORDER);
        a.push(&mut cloud);
        let out = b.pull(&cloud);

        assert_eq!(out.applied, 4, "{out:?}");
        assert_eq!(out.pending, 0);
        let (order_event, event_id): (i64, i64) = b
            .conn
            .query_row(
                "SELECT o.event_id, e.id FROM orders o JOIN events e ON e.name = 'Oasis · Wembley'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(order_event, event_id);
        assert_ne!(event_id, a.one::<i64>("SELECT id FROM events WHERE name = 'Oasis · Wembley'"));
        let tickets_on_the_order: i64 =
            b.one("SELECT COUNT(*) FROM tickets t JOIN orders o ON o.id = t.order_id AND o.code = 'ORD-000001'");
        assert_eq!(tickets_on_the_order, 2);
        let a_uid: String = a.one("SELECT uid FROM orders");
        let b_uid: String = b.one("SELECT uid FROM orders");
        assert_eq!(a_uid, b_uid, "one record, one identity on both computers");
    }

    /// 2.116.0: a picture chosen by hand on one computer shows on the other,
    /// under the same uid - derived from the subject, so two computers that
    /// both chose one meet in one record - and the newer of two choices made
    /// apart wins on both. What a picture service answered never travels.
    #[test]
    fn a_picture_chosen_by_hand_travels_and_the_newer_choice_wins() {
        use crate::commands::event_artwork::{artwork_uid, decide, DecideInput};
        use base64::Engine;
        const PNG: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D,
            0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let png = |tail: u8| {
            let mut bytes = PNG.to_vec();
            bytes.push(tail);
            format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
        };
        let choice = |state: &str, image: Option<String>| DecideInput {
            key: "name:coldplay".into(),
            label: "Coldplay".into(),
            query_key: "artist:coldplay".into(),
            state: state.into(),
            image,
            fit: None,
            source: Some("upload".into()),
            source_id: None,
            source_title: None,
            source_page: None,
            image_url: None,
            confidence: None,
        };
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec("INSERT INTO events (name) VALUES ('Coldplay')");
        settle(&mut a, &mut b, &mut cloud);

        decide(&a.conn, &choice("set", Some(png(1)))).unwrap();
        a.exec(
            "INSERT INTO artwork_lookups (query_key, outcome, checked_at) VALUES ('any:coldplay', 'none', '2026-10-24T12:00:00.000Z')",
        );
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(b.one::<String>("SELECT uid FROM event_artworks"), artwork_uid("name:coldplay"));
        assert_eq!(b.one::<String>("SELECT image FROM event_artworks"), png(1));
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM artwork_lookups"), 0, "a service's answer stays where it was asked");

        // Apart: A says "no picture", B later picks another one.
        decide(&a.conn, &choice("none", None)).unwrap();
        b.later(60_000);
        decide(&b.conn, &choice("set", Some(png(2)))).unwrap();
        a.push(&mut cloud);
        b.push(&mut cloud);
        settle(&mut a, &mut b, &mut cloud);
        for c in [&a, &b] {
            assert_eq!(c.one::<String>("SELECT state FROM event_artworks"), "set");
            assert_eq!(c.one::<String>("SELECT image FROM event_artworks"), png(2));
            assert_eq!(c.one::<i64>("SELECT COUNT(*) FROM event_artworks"), 1);
        }
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn nothing_that_arrived_is_sent_back() {
        let mut cloud = Cloud::default();
        let a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        a.push(&mut cloud);
        let writes = cloud.writes;
        b.pull(&cloud);
        assert_eq!(b.outbox(), 0, "B must not queue what it only received");
        b.push(&mut cloud);
        assert_eq!(cloud.writes, writes);
    }

    #[test]
    fn an_edit_travels_and_the_newer_of_two_edits_wins_on_both_computers() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        settle(&mut a, &mut b, &mut cloud);

        // An edit on one, seen on the other.
        a.exec("UPDATE orders SET notes = 'paid by card'");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(b.one::<String>("SELECT notes FROM orders"), "paid by card");

        // Both edit the same order while apart; B's edit is the later one.
        a.exec("UPDATE orders SET notes = 'A wrote this first', quantity = 3");
        b.later(60_000);
        b.exec("UPDATE orders SET notes = 'B wrote this later'");
        a.push(&mut cloud);
        b.push(&mut cloud);
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(a.one::<String>("SELECT notes FROM orders"), "B wrote this later");
        assert_eq!(b.one::<String>("SELECT notes FROM orders"), "B wrote this later");
        // Newest edit wins for the RECORD, as a whole: B's version had 2.
        assert_eq!(a.one::<i64>("SELECT quantity FROM orders"), 2);
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn a_deletion_takes_the_order_and_its_tickets_off_the_other_computer() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM tickets"), 2);

        // tickets.order_id is ON DELETE CASCADE.
        a.exec("DELETE FROM orders");
        a.push(&mut cloud);
        let out = b.pull(&cloud);
        assert_eq!(out.deleted, 3, "{out:?}");
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM orders"), 0);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM tickets"), 0);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM deleted_rows WHERE table_name = 'orders'"), 1);
        assert_eq!(b.outbox(), 0, "B received the deletion; it has nothing to tell A");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn a_deletion_beats_a_later_edit_whichever_reaches_firestore_first() {
        for edit_goes_up_first in [false, true] {
            let mut cloud = Cloud::default();
            let mut a = Computer::new(A, "");
            let mut b = Computer::new(B, "");
            a.exec(EVENT_AND_ORDER);
            settle(&mut a, &mut b, &mut cloud);

            a.exec("DELETE FROM tickets WHERE code = 'TKT-000002'");
            b.later(60_000);
            b.exec("UPDATE tickets SET notes = 'edited after the delete' WHERE code = 'TKT-000002'");
            if edit_goes_up_first {
                b.push(&mut cloud);
                a.push(&mut cloud);
            } else {
                a.push(&mut cloud);
                b.push(&mut cloud);
            }
            settle(&mut a, &mut b, &mut cloud);
            assert_eq!(a.one::<i64>("SELECT COUNT(*) FROM tickets"), 1, "edit first: {edit_goes_up_first}");
            assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM tickets"), 1, "edit first: {edit_goes_up_first}");
            assert!(cloud.docs.values().any(|(d, _)| d.del && d.t == "tickets"));
            assert_eq!(picture(&a), picture(&b));
        }
    }

    #[test]
    fn one_platform_typed_on_both_computers_stays_one_platform() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        let order = |code: &str, event: &str| {
            format!(
                "INSERT INTO events (name) VALUES ('{event}');
                 INSERT INTO orders (code, event_id, platform_id, purchase_date, quantity, total_cost_cents, currency)
                      VALUES ('{code}', (SELECT id FROM events WHERE name = '{event}'),
                              (SELECT id FROM platforms WHERE name = 'Ticketmaster'), '2026-05-01', 1, 100, 'EUR');"
            )
        };
        a.exec(&format!("INSERT INTO platforms (name, kind) VALUES ('Ticketmaster', 'both');{}", order("ORD-000001", "Gig A")));
        b.exec(&format!("INSERT INTO platforms (name, kind) VALUES ('Ticketmaster', 'both');{}", order("ORD-000002", "Gig B")));
        settle(&mut a, &mut b, &mut cloud);

        for c in [&a, &b] {
            assert_eq!(c.one::<i64>("SELECT COUNT(*) FROM platforms"), 1);
            let on_it: i64 = c.one("SELECT COUNT(*) FROM orders WHERE platform_id = (SELECT id FROM platforms)");
            assert_eq!(on_it, 2, "both orders point at the one platform");
        }

        // Deleting it on one computer deletes it on the other, although
        // each knows it by its own uid.
        b.exec("DELETE FROM platforms");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(a.one::<i64>("SELECT COUNT(*) FROM platforms"), 0);
        // orders.platform_id is ON DELETE SET NULL.
        assert_eq!(a.one::<i64>("SELECT COUNT(*) FROM orders WHERE platform_id IS NULL"), 2);
    }

    #[test]
    fn two_orders_with_one_code_end_up_with_two_codes_and_the_same_ones_on_both() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        // Both counted up from the same place while apart.
        let mine = |event: &str| {
            format!(
                "INSERT INTO events (name) VALUES ('{event}');
                 UPDATE counters SET value = 1 WHERE name = 'order';
                 INSERT INTO orders (code, event_id, purchase_date, quantity, total_cost_cents, currency)
                      VALUES ('ORD-000001', (SELECT id FROM events WHERE name = '{event}'), '2026-05-01', 1, 100, 'EUR');"
            )
        };
        a.exec(&mine("Bought on A"));
        b.exec(&mine("Bought on B"));
        a.push(&mut cloud);
        b.push(&mut cloud);
        settle(&mut a, &mut b, &mut cloud);

        for c in [&a, &b] {
            assert_eq!(c.one::<i64>("SELECT COUNT(*) FROM orders"), 2);
            assert_eq!(c.one::<i64>("SELECT COUNT(DISTINCT code) FROM orders"), 2);
        }
        assert_eq!(picture(&a), picture(&b), "the same code for the same order on both");
        // The lower uid kept it.
        let keeper: String = a.one("SELECT uid FROM orders WHERE code = 'ORD-000001'");
        let lowest: String = a.one("SELECT MIN(uid) FROM orders");
        assert_eq!(keeper, lowest);
        // And neither computer will mint either code again.
        for c in [&a, &b] {
            let next = crate::codes::next_code(&c.conn, "order", "ORD").unwrap();
            assert!(c.one::<i64>(&format!("SELECT COUNT(*) FROM orders WHERE code = '{next}'")) == 0);
        }
    }

    #[test]
    fn event_codes_are_renumbered_within_their_own_event() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "INSERT INTO events (name, uid) VALUES ('Oasis · Wembley', 'event-oasis');");
        a.exec("INSERT INTO events (name, uid) VALUES ('Oasis · Wembley', 'event-oasis');");
        settle(&mut a, &mut b, &mut cloud);
        let order = "INSERT INTO orders (code, event_id, purchase_date, quantity, total_cost_cents, currency)
                          VALUES ('OASIS-001', (SELECT id FROM events), '2026-05-01', 1, 100, 'EUR');";
        a.exec(order);
        b.exec(order);
        settle(&mut a, &mut b, &mut cloud);
        let codes: String = a.one("SELECT group_concat(code, ',') FROM (SELECT code FROM orders ORDER BY code)");
        assert_eq!(codes, "OASIS-001,OASIS-002");
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn a_ticket_that_arrives_before_its_order_waits_for_it() {
        let a = Computer::new(A, "");
        let b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        let mut docs: Vec<SyncDoc> = take(&a.conn, 100, A).unwrap().into_iter().map(|i| i.doc).collect();
        let event = docs.remove(docs.iter().position(|d| d.t == "events").unwrap());
        let order = docs.remove(docs.iter().position(|d| d.t == "orders").unwrap());

        let out = apply(&b.conn, &docs, B).unwrap();
        assert_eq!(out.applied, 0);
        assert_eq!(out.pending, 2, "both tickets wait for their order");
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM tickets"), 0, "nothing half-written");

        let out = apply(&b.conn, &[order], B).unwrap();
        assert_eq!(out.pending, 3, "the order waits for its event");

        let out = apply(&b.conn, &[event], B).unwrap();
        assert_eq!(out.pending, 0, "{out:?}");
        assert_eq!(out.applied, 4);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM tickets"), 2);
    }

    #[test]
    fn computers_that_already_shared_everything_send_each_record_once() {
        let shared = "
            INSERT INTO events (uid, name, created_at, updated_at)
                 VALUES ('ev-1', 'Shared', '2026-03-01T10:00:00.000Z', '2026-03-01T10:00:00.000Z');
            INSERT INTO orders (uid, code, event_id, purchase_date, quantity, total_cost_cents, currency, created_at, updated_at)
                 VALUES ('or-1', 'ORD-000001', (SELECT id FROM events WHERE uid = 'ev-1'), '2026-03-01', 1, 100, 'EUR',
                         '2026-03-01T10:00:00.000Z', '2026-03-01T10:00:00.000Z');";
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, shared);
        // B also edited the event before live sync existed, an hour later.
        let mut b = Computer::new(
            B,
            &format!("{shared} UPDATE events SET venue = 'Wembley', updated_at = '2026-03-01T11:00:00.000Z';"),
        );
        let records = a.outbox() as usize;
        a.push(&mut cloud);
        assert_eq!(cloud.writes, records);
        b.pull(&cloud);
        b.push(&mut cloud);
        assert_eq!(cloud.writes, records + 1, "only B's newer event goes up again");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(a.one::<String>("SELECT venue FROM events"), "Wembley");
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn a_change_made_while_live_sync_was_off_goes_when_it_is_back_on() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        settle(&mut a, &mut b, &mut cloud);
        set_enabled(&a.conn, false).unwrap();
        a.exec("UPDATE orders SET notes = 'while off'");
        set_enabled(&a.conn, true).unwrap();
        assert_eq!(a.outbox(), 1, "exactly the one change, not a second seed");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(b.one::<String>("SELECT notes FROM orders"), "while off");
    }

    #[test]
    fn nothing_is_captured_until_live_sync_is_first_turned_on_and_an_untouched_save_is_not_a_change() {
        let conn = test_conn();
        conn.execute_batch(EVENT_AND_ORDER).unwrap();
        let queued = |c: &Connection| c.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(queued(&conn), 0);
        set_enabled(&conn, true).unwrap();
        let seeded = queued(&conn);
        assert!(seeded >= 4, "the seed queues every record");
        conn.execute("DELETE FROM sync_outbox", []).unwrap();
        conn.execute("UPDATE orders SET notes = notes, quantity = quantity", []).unwrap();
        assert_eq!(queued(&conn), 0, "saving a form untouched is not a change");
        conn.execute("UPDATE orders SET quantity = 5", []).unwrap();
        assert_eq!(queued(&conn), 1);
    }

    #[test]
    fn every_synced_table_has_its_three_capture_triggers() {
        let conn = test_conn();
        for t in MERGE_TABLES {
            for kind in ["ins", "upd", "del"] {
                let name = format!("{}_live_sync_{kind}", t.name);
                let exists: bool = conn
                    .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = ?1)", [&name], |r| r.get(0))
                    .unwrap();
                assert!(exists, "{name} is missing");
            }
        }
        // And they survive the migrations running again on the next start.
        crate::db::run_migrations(&conn).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name LIKE '%_live_sync_%'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n as usize, MERGE_TABLES.len() * 3);
    }

    #[test]
    fn a_bad_document_is_reported_and_the_rest_of_its_page_still_lands() {
        let b = Computer::new(B, "");
        let good = SyncDoc {
            id: doc_id("events", "ev-good"),
            t: "events".into(),
            u: "ev-good".into(),
            h: 1,
            del: false,
            d: serde_json::from_str(r#"{"name": "Arrives"}"#).unwrap(),
            dev: A.into(),
        };
        let unknown_table = SyncDoc { id: "nope~x".into(), t: "nope".into(), u: "x".into(), ..good.clone() };
        // events.name is NOT NULL.
        let broken = SyncDoc {
            id: doc_id("events", "ev-broken"),
            u: "ev-broken".into(),
            d: serde_json::from_str(r#"{"name": null}"#).unwrap(),
            ..good.clone()
        };
        let out = apply(&b.conn, &[unknown_table, broken, good], B).unwrap();
        assert_eq!(out.applied, 1);
        assert_eq!(out.conflicts, 2);
        assert_eq!(out.reasons.len(), 2, "{:?}", out.reasons);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM events WHERE uid IS NOT NULL AND uid LIKE 'ev-%'"), 1);
        assert_eq!(b.one::<i64>("SELECT applying FROM sync_control"), 0);
    }

    #[test]
    fn a_record_pointing_at_one_deleted_here_follows_its_foreign_key() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        a.exec(EVENT_AND_ORDER);
        settle(&mut a, &mut b, &mut cloud);
        // B deletes the order (and with it the tickets) while A, not yet
        // knowing, adds a listing to a ticket (CASCADE) and a finance entry
        // on the order (SET NULL).
        b.exec("DELETE FROM orders");
        a.exec(
            "INSERT INTO ticket_listings (ticket_id, marketplace_id, price_cents)
                  SELECT id, 2, 20000 FROM tickets WHERE code = 'TKT-000001';
             INSERT INTO finance_entries (entry_type, entry_date, amount_cents, order_id)
                  SELECT 'expense', '2026-05-02', 500, id FROM orders;",
        );
        b.push(&mut cloud);
        a.push(&mut cloud);
        let out = b.pull(&cloud);
        assert_eq!(out.pending, 0, "{out:?}");
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM ticket_listings"), 0, "a listing goes with its ticket");
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM finance_entries WHERE order_id IS NULL"), 1, "an expense stays, unlinked");
        settle(&mut a, &mut b, &mut cloud);
        assert_eq!(picture(&a), picture(&b));
    }

    #[test]
    fn a_reminder_follows_its_sheet_row_by_identity_and_outlives_it() {
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "INSERT INTO note_sheets (name) VALUES ('B only'); INSERT INTO note_rows (sheet_id) SELECT id FROM note_sheets;");
        a.exec(
            "INSERT INTO note_sheets (name) VALUES ('Shared');
             INSERT INTO note_rows (sheet_id) SELECT id FROM note_sheets WHERE name = 'Shared';
             INSERT INTO sheet_alerts (sheet_id, row_id, title, remind_at)
                  SELECT s.id, r.id, 'Call the buyer', '2026-06-01T09:00:00Z'
                    FROM note_sheets s JOIN note_rows r ON r.sheet_id = s.id WHERE s.name = 'Shared';",
        );
        settle(&mut a, &mut b, &mut cloud);
        let row_on_b: i64 = b.one("SELECT r.id FROM note_rows r JOIN note_sheets s ON s.id = r.sheet_id AND s.name = 'Shared'");
        assert_eq!(b.one::<i64>("SELECT row_id FROM sheet_alerts"), row_on_b);
        assert_ne!(row_on_b, a.one::<i64>("SELECT row_id FROM sheet_alerts"), "the number itself was not copied");
    }

    #[test]
    fn two_records_that_only_share_a_legacy_name_are_neither_overwritten() {
        let mut cloud = Cloud::default();
        let legacy = |code: &str| {
            format!(
                "INSERT INTO events (uid, name) VALUES ('legacy-1', 'Gig');
                 INSERT INTO orders (uid, code, event_id, purchase_date, quantity, total_cost_cents, currency, updated_at)
                      VALUES ('legacy-7', '{code}', (SELECT id FROM events), '2026-03-01', 1, 100, 'EUR', '2026-03-0{}T10:00:00Z');",
                if code.ends_with('7') { 1 } else { 2 }
            )
        };
        let mut a = Computer::new(A, &legacy("ORD-000007"));
        let mut b = Computer::new(B, &legacy("ORD-000009"));
        a.push(&mut cloud);
        b.pull(&cloud);
        b.push(&mut cloud);
        let out = a.pull(&cloud);
        assert_eq!(out.conflicts, 1, "{out:?}");
        assert!(out.reasons[0].contains("legacy-7"), "{:?}", out.reasons);
        assert_eq!(a.one::<String>("SELECT code FROM orders"), "ORD-000007");
        assert_eq!(b.one::<String>("SELECT code FROM orders"), "ORD-000009");
    }

    #[test]
    fn mail_rules_and_webhooks_travel_and_the_gmail_credential_stays_home() {
        use crate::commands::shared_settings::{touch, NOTIFICATIONS};
        let mut cloud = Cloud::default();
        let mut a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        crate::secrets::set(&a.conn, MAIL_REFRESH_TOKEN_KEY, "1//secret-of-a").unwrap();
        set_setting(&a.conn, MAIL_ACCOUNT_EMAIL_KEY, "biznismarko9@gmail.com").unwrap();
        a.exec(
            "INSERT INTO mail_rules (id, position, enabled, field, match_kind, needle, action, created_at)
                  VALUES ('r1', 1, 1, 'subject', 'contains', 'payout', 'review', 'x');",
        );
        touch(&a.conn, &MAIL).unwrap();
        set_setting(&a.conn, "notification_config", r#"{"discord":{"enabled":true}}"#).unwrap();
        touch(&a.conn, &NOTIFICATIONS).unwrap();
        a.push(&mut cloud);
        assert!(!serde_json::to_string(&cloud.settings).unwrap().contains("secret-of-a"), "the token never leaves A");

        b.pull(&cloud);
        assert_eq!(b.one::<i64>("SELECT COUNT(*) FROM mail_rules WHERE id = 'r1'"), 1);
        assert!(get_setting(&b.conn, "notification_config").unwrap().unwrap().contains("discord"));
        assert_eq!(get_setting(&b.conn, MAIL_ACCOUNT_EMAIL_KEY).unwrap().as_deref(), Some("biznismarko9@gmail.com"));
        assert_eq!(crate::secrets::get(&b.conn, MAIL_REFRESH_TOKEN_KEY).unwrap(), None);

        // B connects the same mailbox itself, then edits a rule: A keeps its
        // own connection, and nothing is echoed.
        crate::secrets::set(&b.conn, MAIL_REFRESH_TOKEN_KEY, "1//secret-of-b").unwrap();
        b.exec("UPDATE mail_rules SET needle = 'payouts'");
        touch(&b.conn, &MAIL).unwrap();
        b.push(&mut cloud);
        a.pull(&cloud);
        assert_eq!(a.one::<String>("SELECT needle FROM mail_rules"), "payouts");
        assert_eq!(crate::secrets::get(&a.conn, MAIL_REFRESH_TOKEN_KEY).unwrap().as_deref(), Some("1//secret-of-a"));

        // A computer with no mailbox never disconnects one that has it.
        let c_side = Computer::new("cccccccccccccccccccccccccccccccc", "");
        touch(&c_side.conn, &MAIL).unwrap();
        c_side.push(&mut cloud);
        a.pull(&cloud);
        assert_eq!(crate::secrets::get(&a.conn, MAIL_REFRESH_TOKEN_KEY).unwrap().as_deref(), Some("1//secret-of-a"));

        // Another mailbox connected on B: A's credential is for the wrong
        // account now, and goes.
        set_setting(&b.conn, MAIL_ACCOUNT_EMAIL_KEY, "other@gmail.com").unwrap();
        touch(&b.conn, &MAIL).unwrap();
        b.push(&mut cloud);
        a.pull(&cloud);
        assert_eq!(crate::secrets::get(&a.conn, MAIL_REFRESH_TOKEN_KEY).unwrap(), None);
        assert_eq!(get_setting(&a.conn, MAIL_ACCOUNT_EMAIL_KEY).unwrap().as_deref(), Some("other@gmail.com"));
    }

    #[test]
    fn what_the_drive_merge_brings_in_is_not_sent_on_as_a_new_change() {
        let a = Computer::new(A, "");
        let path = std::env::temp_dir().join(format!(
            "tiqr-live-merge-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        {
            let remote = Connection::open(&path).unwrap();
            crate::db::run_migrations(&remote).unwrap();
            remote.execute_batch("INSERT INTO events (name) VALUES ('From the Drive file');").unwrap();
        }
        a.conn.execute("DELETE FROM sync_outbox", []).unwrap();
        a.conn.execute("ATTACH DATABASE ?1 AS remote", [path.to_string_lossy().to_string()]).unwrap();
        crate::commands::cloud_merge::merge_attached(&a.conn).unwrap();
        a.conn.execute("DETACH DATABASE remote", []).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(a.one::<i64>("SELECT COUNT(*) FROM events WHERE name = 'From the Drive file'"), 1);
        assert_eq!(a.outbox(), 0);
        assert_eq!(a.one::<i64>("SELECT applying FROM sync_control"), 0);
        // And an edit made here afterwards is captured as usual.
        a.exec("UPDATE events SET venue = 'here'");
        assert_eq!(a.outbox(), 1);
    }

    #[test]
    fn a_pasted_picture_travels_whole() {
        let mut cloud = Cloud::default();
        let a = Computer::new(A, "");
        let mut b = Computer::new(B, "");
        let picture_data = format!("data:image/png;base64,{}", "A".repeat(2_000_000));
        a.conn
            .execute_batch("INSERT INTO notes (title) VALUES ('With a picture');")
            .unwrap();
        a.conn
            .execute("INSERT INTO note_images (note_id, data_uri) SELECT id, ?1 FROM notes", [&picture_data])
            .unwrap();
        a.push(&mut cloud);
        b.pull(&cloud);
        assert_eq!(b.one::<String>("SELECT data_uri FROM note_images"), picture_data);
    }

    #[test]
    fn a_take_never_hands_over_more_than_its_byte_budget() {
        let a = Computer::new(A, "INSERT INTO notes (title) VALUES ('Pictures');");
        for _ in 0..5 {
            a.conn
                .execute(
                    "INSERT INTO note_images (note_id, data_uri) SELECT id, ?1 FROM notes",
                    [format!("data:image/png;base64,{}", "B".repeat(2_000_000))],
                )
                .unwrap();
        }
        let items = take(&a.conn, 100, A).unwrap();
        let images = items.iter().filter(|i| i.table == "note_images").count();
        assert!(images < 5 && images >= 1, "{images} pictures in one take");
    }

    #[test]
    fn a_restored_database_pauses_live_sync_until_it_is_turned_on_again() {
        let dir = std::env::temp_dir().join(format!(
            "tiqr-live-restore-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("copy.sqlite3");
        let mut live = Connection::open(dir.join("live.sqlite3")).unwrap();
        crate::db::run_migrations(&live).unwrap();
        set_enabled(&live, true).unwrap();
        live.execute_batch("INSERT INTO events (name) VALUES ('In the copy');").unwrap();
        live.execute("VACUUM INTO ?1", [copy.to_string_lossy().to_string()]).unwrap();

        crate::commands::backup::restore_database_impl(&mut live, &copy, &dir.join("safety")).unwrap();
        let s = status_of(&live, "dev".into()).unwrap();
        assert!(!s.enabled);
        assert!(s.paused_after_restore);
        assert_eq!(s.last_error.as_deref(), Some(PAUSED_AFTER_RESTORE));
        // Still captured meanwhile - turning it on later sends it.
        live.execute("UPDATE events SET venue = 'after the restore'", []).unwrap();
        assert!(live.query_row("SELECT COUNT(*) FROM sync_outbox WHERE table_name = 'events'", [], |r| r.get::<_, i64>(0)).unwrap() >= 1);
        set_enabled(&live, true).unwrap();
        let s = status_of(&live, "dev".into()).unwrap();
        assert!(s.enabled && !s.paused_after_restore && s.last_error.is_none());
        drop(live);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cursor_only_moves_forward_and_the_device_id_is_kept() {
        let conn = test_conn();
        set_cursor(&conn, 100, 5).unwrap();
        set_cursor(&conn, 99, 999).unwrap();
        set_cursor(&conn, 100, 4).unwrap();
        let (s, ns): (i64, i64) = conn.query_row("SELECT cursor_s, cursor_ns FROM sync_control", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((s, ns), (100, 5));
        set_cursor(&conn, 100, 6).unwrap();
        assert_eq!(conn.query_row("SELECT cursor_ns FROM sync_control", [], |r| r.get::<_, i64>(0)).unwrap(), 6);

        let dir = std::env::temp_dir().join(format!("tiqr-device-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tiqr-manager.sqlite3");
        let _ = std::fs::remove_file(dir.join("tiqr-manager.device"));
        *DEVICE.lock().unwrap() = None;
        let first = device_id(&db).unwrap();
        assert_eq!(first.len(), 32);
        *DEVICE.lock().unwrap() = None;
        assert_eq!(device_id(&db).unwrap(), first, "read back from the file, not made up again");
        assert!(dir.join("tiqr-manager.device").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
