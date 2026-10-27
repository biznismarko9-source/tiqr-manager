//! 2.118.0 - TIQR in a phone's browser: the way in.
//!
//! All of TIQR's Rust runs in a worker of the page (src/phone/worker.ts), the
//! same modules the computer's app runs. The page calls a command by its name
//! with JSON arguments, as it calls Tauri on a computer, and `invoke` finds
//! the same function the computer runs, from the same list
//! (command_list.rs). An event a command sends goes back to the page through
//! the function `start` was given, and a request to the internet (exchange
//! rates, ntfy) through the other one (`http`, the worker's own).
//!
//! The database is SQLite in the browser's own storage for this site (OPFS,
//! through `sqlite_wasm_vfs::sahpool`): kept on the phone like a file on a
//! computer, until the site's data is cleared. Live sync fills it from the
//! account's other devices.
use crate::db::{self, AppState};
use crate::{commands, diag};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tauri::ipc::{Pending, Reply};
use tauri::{AppHandle, Manager};
use wasm_bindgen::prelude::*;

/// What a command the phone build leaves out answers.
pub(crate) const COMPUTER_ONLY: &str = "This works in TIQR on the computer.";

/// Where the phone's databases live in the browser's storage, and the name
/// of that storage's SQLite file system.
const STORAGE_DIRECTORY: &str = ".tiqr";

thread_local! {
    /// The page's function for events: `(event, payload as JSON)`.
    static TO_PAGE: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    /// The worker's function for a request to the internet (worker.ts,
    /// `http`): answers at once - the worker waits for it.
    static HTTP: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
}

/// `reqwest` on the phone (shim/reqwest) sends through here.
fn send(request: reqwest::OutgoingRequest) -> Result<reqwest::IncomingResponse, String> {
    use js_sys::{Array, Reflect, Uint8Array};
    let f = HTTP.with(|f| f.borrow().clone()).ok_or_else(|| reqwest::NO_TRANSPORT.to_string())?;
    let headers = serde_json::to_string(&request.headers).map_err(|e| e.to_string())?;
    let body = match &request.body {
        Some(b) => Uint8Array::from(b.as_slice()).into(),
        None => JsValue::UNDEFINED,
    };
    let args = Array::of5(
        &JsValue::from_str(&request.method),
        &JsValue::from_str(&request.url),
        &JsValue::from_str(&headers),
        &body,
        &JsValue::from_f64(request.timeout_ms.unwrap_or(0) as f64),
    );
    let answer = f.apply(&JsValue::NULL, &args).map_err(|e| e.as_string().unwrap_or_else(|| "could not connect".to_string()))?;
    let field = |name: &str| Reflect::get(&answer, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED);
    let status = field("status").as_f64().unwrap_or(0.0) as u16;
    // `getAllResponseHeaders()`: one "name: value" per line.
    let headers = field("headers")
        .as_string()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once(':').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
        .collect();
    let body = field("body").dyn_into::<Uint8Array>().map(|b| b.to_vec()).unwrap_or_default();
    let url = field("url").as_string().unwrap_or(request.url);
    Ok(reqwest::IncomingResponse { status, headers, body, url })
}

fn to_page(event: &str, payload: &str) {
    TO_PAGE.with(|f| {
        if let Some(f) = f.borrow().as_ref() {
            let _ = f.call2(&JsValue::NULL, &JsValue::from_str(event), &JsValue::from_str(payload));
        }
    });
}

/// Runs `cmd`, or `None` when the phone build does not have it.
fn call(app: &AppHandle, cmd: &str, args: &Value) -> Option<Pending> {
    (tiqr_commands!(tauri::generate_handler))(app, cmd, args)
}

fn fail(what: &str, e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&format!("{what}: {e}"))
}

/// Opens the browser's storage and the database, as the computer's start
/// does (lib.rs, `run`). `emit` hears every event: `(event, payload JSON)`;
/// `http` sends a request: `(method, url, headers JSON, body, timeout ms)`.
#[wasm_bindgen]
pub async fn start(emit: js_sys::Function, http: js_sys::Function) -> Result<(), JsValue> {
    TO_PAGE.with(|f| *f.borrow_mut() = Some(emit));
    HTTP.with(|f| *f.borrow_mut() = Some(http));
    tauri::set_emit_hook(|event, payload| to_page(event, &payload));
    reqwest::set_transport(send);
    tauri::set_package_info("TIQR Manager", env!("CARGO_PKG_VERSION"));
    // A stop the code did not plan for leaves this worker unusable; the page
    // hears it and offers to reload instead of waiting on answers that will
    // not come.
    std::panic::set_hook(Box::new(|info| {
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let text = serde_json::json!({ "where": place, "what": diag::redact(&what) }).to_string();
        to_page("tiqr-phone-stopped", &text);
    }));

    let options = sqlite_wasm_vfs::sahpool::OpfsSAHPoolCfgBuilder::new()
        .directory(STORAGE_DIRECTORY)
        .initial_capacity(16)
        .build();
    sqlite_wasm_vfs::sahpool::install::<sqlite_wasm_rs::WasmOsCallback>(&options, true)
        .await
        .map_err(|e| fail("This browser cannot keep TIQR's data", e))?;

    let app = AppHandle::new();
    let db_path = db::resolve_db_path(&app).map_err(|e| fail("TIQR could not find its database", e))?;
    let conn = db::open_live_connection(&db_path).map_err(|e| fail("TIQR could not open its database", e))?;
    db::run_migrations(&conn).map_err(|e| fail("TIQR could not bring its database up to date", e))?;
    if let Err(e) = crate::secrets::adopt_from_database(&conn) {
        diag::log(diag::Severity::Error, "start", &format!("could not move sign-ins out of the database: {e}"));
    }
    let _ = commands::shared_settings::bootstrap(&conn);
    app.manage(AppState {
        db: Mutex::new(conn),
        db_path: Mutex::new(db_path),
        oauth_cancel_flag: Mutex::new(None),
        firebase_oauth_cancel_flag: Mutex::new(None),
        mail_oauth_cancel_flag: Mutex::new(None),
        price_scanner_sessions: Mutex::new(HashMap::new()),
    });
    Ok(())
}

/// One command, as Tauri's `invoke` runs it: resolves with the answer (JSON
/// text, or the bytes of a file), rejects with the command's error (JSON
/// text) - or with `COMPUTER_ONLY` for one the phone build leaves out.
#[wasm_bindgen]
pub async fn invoke(cmd: String, args: String) -> Result<JsValue, JsValue> {
    let args: Value = serde_json::from_str(&args).map_err(|e| fail("The command's arguments could not be read", e))?;
    let app = AppHandle::new();
    let Some(pending) = call(&app, &cmd, &args) else {
        return Err(JsValue::from_str(&Value::String(COMPUTER_ONLY.to_string()).to_string()));
    };
    match pending.await {
        Ok(Reply::Json(value)) => Ok(JsValue::from_str(&value.to_string())),
        Ok(Reply::Bytes(bytes)) => Ok(js_sys::Uint8Array::from(bytes.as_slice()).into()),
        Err(error) => Err(JsValue::from_str(&error.to_string())),
    }
}

/// Whether a database file is in the browser's storage - what `Path::exists`
/// says on a computer (files.rs).
pub(crate) fn database_exists(path: &Path) -> bool {
    use sqlite_wasm_rs as ffi;
    let Ok(name) = std::ffi::CString::new(path.to_string_lossy().as_bytes()) else { return false };
    // SAFETY: the default file system is the storage `start` installed, and
    // lives as long as the page; SQLite is not used from anywhere else
    // while this runs (the worker has one thread). The name is copied into
    // a buffer of the size the file system asks for before it is looked up.
    unsafe {
        let vfs = ffi::sqlite3_vfs_find(std::ptr::null());
        if vfs.is_null() {
            return false;
        }
        let (Some(full_pathname), Some(access)) = ((*vfs).xFullPathname, (*vfs).xAccess) else { return false };
        let size = ((*vfs).mxPathname.max(1) + 1) as usize;
        let mut full = vec![0 as std::ffi::c_char; size];
        if full_pathname(vfs, name.as_ptr(), size as std::ffi::c_int, full.as_mut_ptr()) != ffi::SQLITE_OK {
            return false;
        }
        let mut found: std::ffi::c_int = 0;
        access(vfs, full.as_ptr(), ffi::SQLITE_ACCESS_EXISTS, &mut found) == ffi::SQLITE_OK && found != 0
    }
}
