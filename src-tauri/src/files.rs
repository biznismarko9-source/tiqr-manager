//! 2.118.0 - the small files TIQR keeps beside a database (the sign-ins in
//! secrets.rs, the sync id in live_sync.rs) and the folders the databases
//! sit in.
//!
//! On a computer they are files and folders, as they always were. A phone's
//! browser has no file system to write to: there each file is one row of a
//! small database of its own in the browser's storage for this site
//! (phone.rs), written whole or not at all; a folder needs no making; and
//! whether a database file is there is for that storage to say.
use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub fn read_to_string(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write(path: &Path, contents: &str) -> io::Result<()> {
    std::fs::write(path, contents)
}

/// `contents` in place of whatever `path` held, never half of it: a computer
/// writes a temporary file beside it, readable only by this user on Unix,
/// and renames it over.
#[cfg(not(target_arch = "wasm32"))]
pub fn replace(path: &Path, contents: &str) -> io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_dir_all(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(target_arch = "wasm32")]
pub use phone::{create_dir_all, exists, read_to_string, replace, write};

#[cfg(target_arch = "wasm32")]
mod phone {
    use rusqlite::{params, Connection, OptionalExtension};
    use std::io;
    use std::path::Path;
    use std::sync::Mutex;

    /// The phone's files, in the browser's storage beside the databases.
    const STORE: &str = "/tiqr/.files.sqlite3";

    static CONN: Mutex<Option<Connection>> = Mutex::new(None);

    fn with<T>(f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> io::Result<T> {
        let mut slot = CONN.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            let conn = Connection::open(STORE).map_err(io::Error::other)?;
            conn.execute_batch("CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, contents TEXT NOT NULL);")
                .map_err(io::Error::other)?;
            *slot = Some(conn);
        }
        f(slot.as_ref().expect("opened above")).map_err(io::Error::other)
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    pub fn read_to_string(path: &Path) -> io::Result<String> {
        with(|c| {
            c.query_row("SELECT contents FROM files WHERE path = ?1", params![key(path)], |r| r.get(0)).optional()
        })?
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    /// One statement, so whole or not at all - what `replace` needs too.
    pub fn write(path: &Path, contents: &str) -> io::Result<()> {
        with(|c| {
            c.execute(
                "INSERT INTO files (path, contents) VALUES (?1, ?2)
                 ON CONFLICT(path) DO UPDATE SET contents = excluded.contents",
                params![key(path), contents],
            )
            .map(|_| ())
        })
    }

    pub fn replace(path: &Path, contents: &str) -> io::Result<()> {
        write(path, contents)
    }

    /// There are no folders to make.
    pub fn create_dir_all(_path: &Path) -> io::Result<()> {
        Ok(())
    }

    /// A file here, or a database file in the browser's storage.
    pub fn exists(path: &Path) -> bool {
        let stored = with(|c| {
            c.query_row("SELECT EXISTS(SELECT 1 FROM files WHERE path = ?1)", params![key(path)], |r| r.get(0))
        });
        stored.unwrap_or(false) || crate::phone::database_exists(path)
    }
}
