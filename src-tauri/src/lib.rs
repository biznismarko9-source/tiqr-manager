// 2.118.0: TIQR in a phone's browser compiles these same modules to
// WebAssembly (phone.rs). There `tauri` and `reqwest` are stand-ins with the
// few parts the shared code uses (shim/), so the code reads the same on both.
// What only a computer does (a window, a file dialog, a sign-in through this
// computer's browser) is left out there, and so are the helpers only it
// calls - which that build does not warn about one by one.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]
#[cfg(target_arch = "wasm32")]
extern crate tiqr_tauri_shim as tauri;
#[cfg(target_arch = "wasm32")]
extern crate tiqr_reqwest_shim as reqwest;

// 2.118.0: the commands, listed once for both builds (`tiqr_commands!`).
#[macro_use]
mod command_list;

mod ai_categorize;
mod codes;
mod commands;
mod db;
mod diag;
mod error;
// 2.118.0: the small files beside a database - on a phone, in its browser.
mod files;
mod finance;
mod fx;
mod google_oauth;
mod google_sheets;
mod mail_store;
mod models;
mod money;
// 2.113.0: credentials beside the database, never in it.
mod secrets;
// 2.117.0: this TIQR's screens in a browser (Settings -> Web access).
#[cfg(not(target_arch = "wasm32"))]
mod web_access;
// 2.118.0: the phone build's way in - the page calls commands here.
#[cfg(target_arch = "wasm32")]
mod phone;

/// 2.118.0: a moment before trying again (a busy server, a rate limit). A
/// phone's browser runs TIQR on the one thread every screen is waiting on,
/// so there it does not wait: what was to be tried again is tried at once.
pub(crate) fn pause(time: std::time::Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(time);
    #[cfg(target_arch = "wasm32")]
    let _ = time;
}

#[cfg(not(target_arch = "wasm32"))]
use db::AppState;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use tauri::Manager;
// 2.12.1: needed ONLY by the debug-only `app.deep_link().register_all()`
// call in setup() below, so an ungated import warns on every release build.
// Gated with the identical cfg rather than removed - deleting it would break
// `cargo tauri dev`, which is the one build that call exists for.
#[cfg(all(desktop, debug_assertions))]
use tauri_plugin_deep_link::DeepLinkExt;

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 2.0.52: MUST be the first plugin registered (the plugin's own
        // documented requirement) - see the Cargo.toml comment on this same
        // dependency for why single-instance matters for this app
        // specifically (concurrent SQLite writers, not just a UI nuisance).
        // This callback runs in the ALREADY-RUNNING instance whenever a
        // second launch is attempted; that second process exits right after
        // calling it, before it ever reaches the .setup() below - so
        // db::open_connection only ever runs in the one surviving process.
        // argv/cwd from the second attempt are ignored (`_argv`/`_cwd`) -
        // this app has no CLI arguments or file-association behavior that
        // would need them, it only needs to bring the real window forward.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 2.0.5: opens the system browser for "Sign in with Google" - see
        // google_oauth.rs's module doc comment.
        .plugin(tauri_plugin_opener::init())
        // 2.5.2: registers this app's `tiqrmanager://` URL scheme for the new
        // "Forgot password?" flow - see lib/firebase.ts's
        // PASSWORD_RESET_ACTION_CODE_SETTINGS (frontend) for why a deep link
        // exists at all, and Cargo.toml's own comment on this same
        // dependency for why tauri-plugin-single-instance above needed its
        // `deep-link` feature turned on for this to reach an already-running
        // instance on Windows. The actual URL is parsed entirely on the
        // frontend (App.tsx, via @tauri-apps/plugin-deep-link's
        // getCurrent/onOpenUrl) - nothing Rust-side needs to read it.
        .plugin(tauri_plugin_deep_link::init())
        // 2.0.76: desktop notifications for the new outbound-notification
        // feature (commands/notifications.rs) - no ordering constraint
        // against the other plugins here (unlike single-instance above).
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 2.5.2: a `cargo tauri dev` build has no installer to have
            // registered `tiqrmanager://` with the OS - tauri-plugin-deep-
            // link's own documented Windows/Linux limitation is that deep
            // links only trigger for installed applications otherwise.
            // register_all() registers this build's configured schemes
            // (tauri.conf.json's `plugins.deep-link.desktop.schemes`)
            // against THIS running executable instead, purely so the
            // "Forgot password?" flow can be clicked through end to end on
            // a dev build too. Release builds get the real registration for
            // free from the NSIS installer and don't need this - best-
            // effort (`let _ =`) since a dev build failing to self-register
            // should never stop the app from starting.
            #[cfg(all(desktop, debug_assertions))]
            {
                let _ = app.deep_link().register_all();
            }
            let handle = app.handle().clone();
            let db_path = db::resolve_db_path(&handle).map_err(|e| e.to_string())?;
            // 2.116.0: the log and the crash notes, beside the data - set up
            // before anything else can fail, so even that failure is kept.
            if let Some(dir) = db_path.parent() {
                diag::init(dir.join("logs"));
            }
            diag::log(diag::Severity::Info, "start", &format!("TIQR {} starting", env!("CARGO_PKG_VERSION")));
            let conn = db::open_live_connection(&db_path).map_err(|e| {
                diag::log(diag::Severity::Fatal, "start", &format!("the database could not be opened: {e}"));
                e.to_string()
            })?;
            db::run_migrations(&conn).map_err(|e| {
                diag::log(diag::Severity::Fatal, "start", &format!("the database could not be brought up to date: {e}"));
                e.to_string()
            })?;
            // 2.113.0: tokens and keys out of the database file (secrets.rs).
            if let Err(e) = secrets::adopt_from_database(&conn) {
                diag::log(diag::Severity::Error, "start", &format!("could not move sign-ins out of the database: {e}"));
            }
            // 2.110.0: settings set up before this release get a stamp, so
            // the other computer can receive them (shared_settings.rs).
            let _ = commands::shared_settings::bootstrap(&conn);
            app.manage(AppState {
                db: Mutex::new(conn),
                // 2.0.72: this is the ONE global/legacy file, opened eagerly
                // here exactly as before - it's what serves the one command
                // that already fires before anyone is signed in (theme
                // preference on the Welcome screen). The real per-account
                // file (this legacy one, or a brand-new per-uid one) is
                // switched in immediately after sign-in + approval by
                // commands::database::switch_active_database - see that
                // module's own doc comment for the full design.
                db_path: Mutex::new(db_path),
                oauth_cancel_flag: Mutex::new(None),
                firebase_oauth_cancel_flag: Mutex::new(None),
                mail_oauth_cancel_flag: Mutex::new(None),
                price_scanner_sessions: Mutex::new(HashMap::new()),
            });
            // 2.117.0: back on if it was left on (web_access.rs).
            web_access::start_if_enabled(app.handle());
            Ok(())
        })
        .invoke_handler(tiqr_commands!(tauri::generate_handler))
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 2.1.3 (originally the hidden auto-check's own hardening,
            // adapted 2.1.9 for the Visible Scanner's session map).
            // `ExitRequested` fires once, right as the app is about to exit
            // (confirmed against Tauri's own published docs, not memory -
            // docs.rs/tauri's `RunEvent` and `App::run`). BEST-EFFORT,
            // NON-BLOCKING only: never calls `api.prevent_exit()`, so it can
            // never delay or interrupt an ordinary close - the OS reclaims
            // every window/handle this process owns on exit regardless of
            // whether this fires. Its only real effect is flipping every
            // still-open scanner session's cancel flag a little sooner than
            // process teardown alone would, in case a scan eval happens to
            // be in flight at that exact moment - it does NOT close the
            // scanner windows themselves, that's unnecessary (see this
            // block's own reasoning above: the OS reclaims them anyway) and
            // Visible Scanner windows are ordinary, user-visible windows the
            // OS already knows how to tear down normally, unlike the old
            // hidden reader this replaced. Deliberately scoped to ONLY this
            // feature's own sessions - not `oauth_cancel_flag`/
            // `firebase_oauth_cancel_flag`, a separate, unrelated feature.
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(sessions) = state.price_scanner_sessions.lock() {
                        for session in sessions.values() {
                            session.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    // 2.14.0, same best-effort rules as everything else in
                    // this block. Automatic sync tracks unsent work in an
                    // atomic that dies with the process; writing it through
                    // here covers an edit made between two automatic checks
                    // and followed straight by a close. `try_lock`, never
                    // `lock`: an exit must not wait on a sync that happens
                    // to be uploading. A missed flush costs one extra
                    // "both machines changed" prompt, never data.
                    if let Ok(conn) = state.db.try_lock() {
                        commands::cloud_sync::flush_local_dirty(&conn);
                    }
                }
            }
        });
}
