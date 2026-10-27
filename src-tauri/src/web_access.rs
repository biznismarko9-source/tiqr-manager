//! 2.117.0 - Web access: this TIQR's screens in a browser.
//!
//! marko: "tak skor by som siel do webu" - TIQR on the Windows server is a
//! computer like the other two (live sync, mail, Discord), and a browser - an
//! iPhone's too - uses that TIQR. Nothing here does business of its own: a
//! request names a command and its arguments exactly as the window does, and
//! it goes to the window's own command handler (`Webview::on_message`), so a
//! screen in a browser and the same screen in the window get the same answer
//! from the same code.
//!
//! What a request meets, in order:
//! * **Only this computer can connect.** The server listens on 127.0.0.1; the
//!   internet reaches it through a tunnel running on the same computer
//!   (Cloudflare Tunnel, `app.tiqr.site`).
//! * **A signed-in account, and only the one this TIQR has open.** Every API
//!   request carries a Firebase ID token, checked here against Google's
//!   published keys: signed, not expired, issued for this Firebase project,
//!   and for the account whose database is open (`note_signed_in`). Nobody
//!   signed in on this computer, nobody gets in.
//! * **Nothing that only means something on this computer.** A file by its
//!   path, a window, a sign-in page, the sync loops this computer's own window
//!   runs - `refusal` says no, with a reason a person can act on.
//! * **Off until switched on**, per computer, in a file beside the data
//!   (`web-access.json`) - not in the database, so no backup, merge or sync
//!   carries it to another computer.
//!
//! One thing more is served without a token, because it comes before one:
//! Firebase's Google sign-in pages (`sign_in_routes`).
use crate::diag::{self, Severity};
use crate::error::{AppError, AppResult};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponse, InvokeResponseBody};
use tauri::webview::InvokeRequest;
use tauri::{AppHandle, Listener, Manager, Runtime};
use tokio::sync::broadcast;

// The web screens (`vite build --mode web` -> dist-web), embedded by
// build.rs: `WEB_ASSETS`, empty when this build was made without them.
include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));

/// The Firebase project TIQR signs in with (src/lib/firebase.ts).
const PROJECT_ID: &str = "tiqr-manager-b890a";
/// Google's public keys for Firebase ID tokens.
const KEYS_URL: &str = "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";
pub(crate) const DEFAULT_PORT: u16 = 8787;
const CONFIG_FILE: &str = "web-access.json";
const MAIN_WINDOW: &str = "main";
/// A pasted picture in a note, or an image for the AI import, travels as
/// base64 inside the arguments.
const MAX_BODY_BYTES: usize = 40 * 1024 * 1024;
/// A Sheets sync or a mail action can take minutes; nothing takes longer.
const CALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Events the window listens for that a browser needs as well.
const FORWARDED_EVENTS: &[&str] = &["mail-sync"];
/// Unknown key ids are looked up at most this often, so a stream of made-up
/// tokens cannot turn into a stream of requests to Google.
const KEY_REFETCH_GAP: Duration = Duration::from_secs(60);

/// Where Firebase keeps this project's sign-in pages (`sign_in_routes`).
const FIREBASE_PAGES: &str = "https://tiqr-manager-b890a.firebaseapp.com";
/// A sign-in form is small; nothing bigger comes that way.
const MAX_SIGN_IN_BODY: usize = 64 * 1024;
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(20);
const SIGN_IN_UNREACHABLE: &str =
    "Google sign-in could not be reached from TIQR on the server. Go back to TIQR and try again in a moment.";

/// What a browser may load and talk to. The screens are TIQR's own files;
/// sign-in and Firestore are Google's (apis.google.com is Firebase's loader
/// for the Google sign-in window).
const CSP: &str = "default-src 'self'; script-src 'self' https://apis.google.com; style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob: https:; font-src 'self' data:; \
connect-src 'self' https://*.googleapis.com https://*.firebaseio.com wss://*.firebaseio.com; \
frame-src 'self' blob: data: https://tiqr-manager-b890a.firebaseapp.com; object-src 'none'; \
base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

// ---------------------------------------------------------------------------
// Settings, per computer.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAccessOptions {
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for WebAccessOptions {
    fn default() -> Self {
        Self { enabled: false, port: DEFAULT_PORT }
    }
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(CONFIG_FILE))
}

/// A missing or unreadable file is "off" - never a reason not to start.
fn read_options(path: Option<&PathBuf>) -> WebAccessOptions {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_options(path: &PathBuf, options: &WebAccessOptions) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(options).map_err(|e| AppError::Other(e.to_string()))?;
    std::fs::write(path, text)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Whose screens these are.
// ---------------------------------------------------------------------------

static SIGNED_IN: Mutex<Option<String>> = Mutex::new(None);

/// The account whose database this TIQR has open (commands/database.rs).
/// Only that account's tokens are let in.
pub(crate) fn note_signed_in(uid: &str) {
    *SIGNED_IN.lock().unwrap() = Some(uid.to_string());
}

fn signed_in() -> Option<String> {
    SIGNED_IN.lock().unwrap().clone()
}

// ---------------------------------------------------------------------------
// The server.
// ---------------------------------------------------------------------------

struct Running {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// At start: on if it was left on.
pub(crate) fn start_if_enabled<R: Runtime>(app: &AppHandle<R>) {
    let options = read_options(config_path(app).as_ref());
    if options.enabled {
        apply(app, options);
    }
}

/// Starts or stops the server to match `options`. A port that cannot be used
/// is kept as the error the Settings card shows.
fn apply<R: Runtime>(app: &AppHandle<R>, options: WebAccessOptions) {
    stop();
    *LAST_ERROR.lock().unwrap() = None;
    if !options.enabled {
        return;
    }
    match start(app.clone(), options.port) {
        Ok(port) => {
            diag::log(Severity::Info, "web access", &format!("serving on 127.0.0.1:{port}"));
        }
        Err(e) => {
            diag::log(Severity::Error, "web access", &e);
            *LAST_ERROR.lock().unwrap() = Some(e);
        }
    }
}

fn stop() {
    if let Some(running) = RUNNING.lock().unwrap().take() {
        let _ = running.stop.send(());
    }
}

/// Binds 127.0.0.1 - never another address - and serves until `stop`.
/// Returns the port actually bound (0 asks the system for a free one).
fn start<R: Runtime>(app: AppHandle<R>, port: u16) -> Result<u16, String> {
    // The server just stopped may still hold the port for a moment.
    let mut tries = 0;
    let listener = loop {
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => break listener,
            Err(_) if tries < 20 => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("port {port} cannot be used: {e}")),
        }
    };
    let bound = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let hub = event_hub(&app);
    let router = router(app, hub);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let served = match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stop_rx.await;
                })
                .await
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = served {
            diag::log(Severity::Error, "web access", &format!("stopped: {e}"));
            *LAST_ERROR.lock().unwrap() = Some(format!("stopped: {e}"));
        }
    });
    *RUNNING.lock().unwrap() = Some(Running { port: bound, stop: stop_tx });
    Ok(bound)
}

struct Shared<R: Runtime> {
    app: AppHandle<R>,
    events: broadcast::Sender<(String, String)>,
}

// By hand: a derive would ask the runtime type itself to be Clone.
impl<R: Runtime> Clone for Shared<R> {
    fn clone(&self) -> Self {
        Self { app: self.app.clone(), events: self.events.clone() }
    }
}

fn router<R: Runtime>(app: AppHandle<R>, events: broadcast::Sender<(String, String)>) -> Router {
    routes(app, events, FIREBASE_PAGES.to_string())
}

/// `sign_in_pages`: Firebase's host, or a stand-in for it in the tests.
fn routes<R: Runtime>(app: AppHandle<R>, events: broadcast::Sender<(String, String)>, sign_in_pages: String) -> Router {
    let screens = Router::new()
        .route("/api/health", get(health))
        .route("/api/invoke", post(invoke::<R>))
        .route("/api/events", get(events_stream::<R>))
        .fallback(asset)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(axum::middleware::map_response(protect))
        .with_state(Shared { app, events });
    sign_in_routes(sign_in_pages).merge(screens)
}

/// Firebase's Google sign-in pages (`/__/auth/...`, `/__/firebase/...`),
/// fetched from Firebase and handed over as this address's own.
///
/// Google's sign-in page hands back to the address it was opened from. From
/// firebaseapp.com, a browser that keeps each site's storage to itself -
/// Safari, and an iPhone's home-screen app above all - comes back to TIQR
/// without the sign-in. From here it is all one site: Firebase's own advice
/// for exactly this ("proxy auth requests to firebaseapp.com"), and
/// src/lib/firebase.ts sends sign-in here.
///
/// Only these two folders, only to Firebase's host, and only what a page
/// needs goes out: no cookies (this address's cookies are Cloudflare
/// Access's), and none come back. These answers keep Firebase's own rules,
/// not `protect`'s: TIQR's page has to frame Firebase's sign-in frame, and
/// the scripts on these pages are Firebase's.
fn sign_in_routes(pages: String) -> Router {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(SIGN_IN_TIMEOUT)
        .build()
        .unwrap_or_default();
    Router::new()
        .route("/__/auth/{*rest}", get(sign_in_page).post(sign_in_page))
        .route("/__/firebase/{*rest}", get(sign_in_page))
        .layer(DefaultBodyLimit::max(MAX_SIGN_IN_BODY))
        .with_state(SignIn { pages, client })
}

#[derive(Clone)]
struct SignIn {
    pages: String,
    client: reqwest::Client,
}

async fn sign_in_page(
    State(sign_in): State<SignIn>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    if path.split('/').any(|part| part == "..") || path.contains("//") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let target = uri.path_and_query().map_or(path, |p| p.as_str());
    let mut request = sign_in.client.request(method, format!("{}{target}", sign_in.pages));
    for name in [
        header::ACCEPT,
        header::ACCEPT_LANGUAGE,
        header::CONTENT_TYPE,
        header::USER_AGENT,
        header::IF_NONE_MATCH,
        header::IF_MODIFIED_SINCE,
    ] {
        if let Some(value) = headers.get(&name) {
            request = request.header(name, value.clone());
        }
    }
    if !body.is_empty() {
        request = request.body(body);
    }
    // A page the person is looking at, so words rather than JSON.
    let unreachable = || {
        (
            StatusCode::BAD_GATEWAY,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            SIGN_IN_UNREACHABLE,
        )
            .into_response()
    };
    let Ok(answer) = request.send().await else {
        return unreachable();
    };
    let mut response = Response::builder().status(answer.status());
    for name in [
        header::CONTENT_TYPE,
        header::CACHE_CONTROL,
        header::ETAG,
        header::LAST_MODIFIED,
        header::EXPIRES,
        header::LOCATION,
        header::VARY,
        header::CONTENT_LANGUAGE,
    ] {
        if let Some(value) = answer.headers().get(&name) {
            response = response.header(name, value.clone());
        }
    }
    let Ok(bytes) = answer.bytes().await else {
        return unreachable();
    };
    response.body(axum::body::Body::from(bytes)).unwrap_or_else(|_| unreachable())
}

/// One hub per app: the window's events, copied for every browser listening.
static HUB_LOCK: Mutex<()> = Mutex::new(());
struct EventHub(broadcast::Sender<(String, String)>);

fn event_hub<R: Runtime>(app: &AppHandle<R>) -> broadcast::Sender<(String, String)> {
    let _guard = HUB_LOCK.lock().unwrap();
    if let Some(hub) = app.try_state::<EventHub>() {
        return hub.0.clone();
    }
    let (tx, _) = broadcast::channel(256);
    for name in FORWARDED_EVENTS {
        let tx = tx.clone();
        app.listen_any(*name, move |event| {
            let _ = tx.send((name.to_string(), event.payload().to_string()));
        });
    }
    app.manage(EventHub(tx.clone()));
    tx
}

/// Every answer: no sniffing, no framing, no referrer, and the CSP above.
async fn protect(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("permissions-policy", HeaderValue::from_static("camera=(), microphone=(), geolocation=()"));
    headers.insert("content-security-policy", HeaderValue::from_static(CSP));
    response
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct Call {
    cmd: String,
    #[serde(default)]
    args: Value,
}

async fn invoke<R: Runtime>(State(shared): State<Shared<R>>, headers: HeaderMap, body: Bytes) -> Response {
    let who = match authorize(&headers).await {
        Ok(who) => who,
        Err(denied) => return denied,
    };
    let Ok(call) = serde_json::from_slice::<Call>(&body) else {
        return error(StatusCode::BAD_REQUEST, "That request could not be read.");
    };
    if let Some(reason) = refusal(&call.cmd, &call.args, &who.uid) {
        return error(StatusCode::FORBIDDEN, reason);
    }
    let args = if call.args.is_null() { json!({}) } else { call.args };
    match dispatch(&shared.app, call.cmd, args).await {
        Ok(InvokeResponseBody::Json(text)) => ([(header::CONTENT_TYPE, "application/json")], text).into_response(),
        Ok(InvokeResponseBody::Raw(bytes)) => ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response(),
        Err(value) => (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "error": value }))).into_response(),
    }
}

/// The command, handed to the window's own handler - the same checks, the
/// same arguments, the same answer as a click in the window.
async fn dispatch<R: Runtime>(app: &AppHandle<R>, cmd: String, args: Value) -> Result<InvokeResponseBody, Value> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return Err(json!("TIQR's window on the server is closed."));
    };
    let webview = window.as_ref().clone();
    let url = webview.url().map_err(|e| json!(e.to_string()))?;
    let request = InvokeRequest {
        cmd,
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url,
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: app.invoke_key().to_string(),
    };
    let (tx, rx) = tokio::sync::oneshot::channel::<InvokeResponse>();
    // Off the async threads: a plain command runs where it is handed over.
    tauri::async_runtime::spawn_blocking(move || {
        webview.on_message(
            request,
            Box::new(move |_webview, _cmd, response, _callback, _error| {
                let _ = tx.send(response);
            }),
        );
    });
    match tokio::time::timeout(CALL_TIMEOUT, rx).await {
        Ok(Ok(InvokeResponse::Ok(body))) => Ok(body),
        Ok(Ok(InvokeResponse::Err(e))) => Err(e.0),
        Ok(Err(_)) => Err(json!("TIQR did not answer.")),
        Err(_) => Err(json!("TIQR took too long to answer.")),
    }
}

async fn events_stream<R: Runtime>(State(shared): State<Shared<R>>, headers: HeaderMap) -> Response {
    let who = match authorize(&headers).await {
        Ok(who) => who,
        Err(denied) => return denied,
    };
    // The stream ends with the token; the browser comes back with a new one.
    let left = who.expires_at.saturating_sub(now_secs()).clamp(1, 3600);
    let until = tokio::time::Instant::now() + Duration::from_secs(left);
    let rx = shared.events.subscribe();
    let stream = futures_util::stream::unfold((rx, until), |(mut rx, until)| async move {
        loop {
            match tokio::time::timeout_at(until, rx.recv()).await {
                Err(_) => return None,
                Ok(Ok((name, payload))) => {
                    return Some((Ok::<Event, Infallible>(Event::default().event(name).data(payload)), (rx, until)))
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20))).into_response()
}

async fn asset(method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return error(StatusCode::NOT_FOUND, "No such request.");
    }
    let wanted = if path.is_empty() { "index.html" } else { path };
    let found = find_asset(wanted).or_else(|| if wanted.contains('.') { None } else { find_asset("index.html") });
    match found {
        Some((name, bytes)) => {
            // File names under assets/ carry their content hash.
            let cache = if name.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
            ([(header::CONTENT_TYPE, content_type(name)), (header::CACHE_CONTROL, cache)], bytes).into_response()
        }
        None if WEB_ASSETS.is_empty() => {
            error(StatusCode::SERVICE_UNAVAILABLE, "The web screens are not part of this build of TIQR.")
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn find_asset(name: &str) -> Option<(&'static str, &'static [u8])> {
    WEB_ASSETS.iter().find(|(path, _)| *path == name).copied()
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// What a browser may not ask for.
// ---------------------------------------------------------------------------

const FILES: &str =
    "On the web this would save or read a file on the server, not on this device. Use TIQR on a computer for it.";
const WINDOWS: &str = "This opens a window or a sign-in page on the server's screen. Use TIQR on a computer for it.";
const OWN_WORK: &str = "TIQR's own window on the server does this by itself.";
const SWITCH: &str = "Web access is switched on and off in TIQR on the server itself.";
const PLUGIN: &str = "This is not available on the web.";
const ACCOUNT: &str = "Only the account TIQR has open on the server can be used here.";

const BLOCKED: &[(&[&str], &str)] = &[
    (
        &[
            "backup_database",
            "restore_database",
            "validate_backup_file",
            "export_events_csv",
            "export_events_csv_selected",
            "export_inventory_csv",
            "export_orders_csv",
            "export_orders_csv_selected",
            "export_orders_csv_template",
            "export_sales_csv",
            "export_sales_csv_selected",
            "export_tickets_csv",
            "export_tickets_csv_selected",
            "export_scan_results_csv",
            "import_orders_csv",
            "preview_orders_csv",
            "save_diagnostics",
            "save_png_file",
            "save_mail_attachment",
            "open_mail_attachment",
        ],
        FILES,
    ),
    (
        &[
            "open_price_scanner",
            "close_price_scanner",
            "start_price_scan_run",
            "scan_visible_prices",
            "cancel_price_scan",
            "start_google_sign_in",
            "cancel_google_sign_in",
            "start_firebase_google_sign_in",
            "cancel_firebase_google_sign_in",
            "connect_mail_gmail",
            "cancel_connect_mail",
        ],
        WINDOWS,
    ),
    (
        &[
            "live_sync_take",
            "live_sync_ack",
            "live_sync_apply",
            "live_sync_apply_settings",
            "live_sync_set_cursor",
            "live_sync_report",
            "mail_share_export",
            "mail_share_import",
            "mail_share_cursor",
            "mail_share_follow",
            "cloud_sync_auto",
            "cloud_sync_pull",
            "cloud_sync_restore_revision",
        ],
        OWN_WORK,
    ),
    (&["set_web_access_options"], SWITCH),
];

/// Why a browser may not run `cmd`, or `None` when it may.
fn refusal(cmd: &str, args: &Value, uid: &str) -> Option<&'static str> {
    if cmd.starts_with("plugin:") {
        return Some(PLUGIN);
    }
    if cmd == "switch_active_database" {
        // A browser signed in as the open account opens nothing else.
        return (args.get("uid").and_then(Value::as_str) != Some(uid)).then_some(ACCOUNT);
    }
    BLOCKED.iter().find(|(names, _)| names.contains(&cmd)).map(|(_, why)| *why)
}

// ---------------------------------------------------------------------------
// Sign-in tokens.
// ---------------------------------------------------------------------------

struct Who {
    uid: String,
    expires_at: u64,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
    iat: u64,
    #[serde(default)]
    auth_time: Option<u64>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

async fn authorize(headers: &HeaderMap) -> Result<Who, Response> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string);
    let Some(token) = token else {
        return Err(error(StatusCode::UNAUTHORIZED, "Sign in to use TIQR."));
    };
    let Some(expected) = signed_in() else {
        return Err(error(StatusCode::SERVICE_UNAVAILABLE, "TIQR on the server is not signed in yet."));
    };
    let checked =
        tauri::async_runtime::spawn_blocking(move || verify_token(&token, &expected, now_secs(), &key_for)).await;
    match checked {
        Ok(Ok(who)) => Ok(who),
        // A real sign-in, of someone this TIQR is not open for.
        Ok(Err(Refused::OtherAccount)) => {
            Err(error(StatusCode::FORBIDDEN, "TIQR on the server is open for another account."))
        }
        Ok(Err(Refused::Invalid(why))) => {
            diag::log(Severity::Warning, "web access", &format!("refused a sign-in: {why}"));
            Err(error(StatusCode::UNAUTHORIZED, "Sign in again."))
        }
        Err(_) => Err(error(StatusCode::INTERNAL_SERVER_ERROR, "The sign-in could not be checked.")),
    }
}

#[derive(Debug, PartialEq)]
enum Refused {
    /// Not a valid token of this project: forged, expired, garbled.
    Invalid(String),
    /// A valid token, for an account whose database is not the one open.
    OtherAccount,
}

impl From<String> for Refused {
    fn from(why: String) -> Self {
        Refused::Invalid(why)
    }
}

/// A Firebase ID token, checked the way Firebase documents: RS256 with one of
/// Google's current keys, this project as audience and issuer, not expired,
/// not issued in the future - and for `expected_uid`.
fn verify_token(
    token: &str,
    expected_uid: &str,
    now: u64,
    key_for: &dyn Fn(&str) -> Result<DecodingKey, String>,
) -> Result<Who, Refused> {
    let head = jsonwebtoken::decode_header(token).map_err(|_| "not a sign-in token".to_string())?;
    if head.alg != Algorithm::RS256 {
        return Err(format!("signed with {:?}", head.alg).into());
    }
    let kid = head.kid.ok_or_else(|| "no key id".to_string())?;
    let key = key_for(&kid)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[PROJECT_ID]);
    validation.set_issuer(&[format!("https://securetoken.google.com/{PROJECT_ID}")]);
    validation.set_required_spec_claims(&["exp", "iat", "aud", "iss", "sub"]);
    validation.leeway = 60;
    let claims = jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map_err(|e| format!("token refused: {e}"))?
        .claims;
    if claims.iat > now + 60 || claims.auth_time.is_some_and(|t| t > now + 60) {
        return Err("a token from the future".to_string().into());
    }
    if claims.sub.is_empty() {
        return Err("a token without an account".to_string().into());
    }
    if claims.sub != expected_uid {
        return Err(Refused::OtherAccount);
    }
    Ok(Who { uid: claims.sub, expires_at: claims.exp })
}

struct KeySet {
    keys: HashMap<String, DecodingKey>,
    fresh_until: Instant,
    fetched_at: Instant,
}

static KEYS: Mutex<Option<KeySet>> = Mutex::new(None);

/// Google's key `kid`, from the cache while it is fresh.
fn key_for(kid: &str) -> Result<DecodingKey, String> {
    {
        let cache = KEYS.lock().unwrap();
        if let Some(set) = cache.as_ref() {
            if let Some(key) = set.keys.get(kid) {
                if set.fresh_until > Instant::now() {
                    return Ok(key.clone());
                }
            } else if set.fetched_at.elapsed() < KEY_REFETCH_GAP {
                return Err("an unknown key".to_string());
            }
        }
    }
    let fetched = fetch_keys()?;
    let key = fetched.keys.get(kid).cloned();
    *KEYS.lock().unwrap() = Some(fetched);
    key.ok_or_else(|| "an unknown key".to_string())
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

fn fetch_keys() -> Result<KeySet, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(KEYS_URL).send().map_err(|e| format!("Google's keys could not be fetched: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Google's keys could not be fetched: {}", response.status()));
    }
    let max_age = response
        .headers()
        .get(reqwest::header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .and_then(max_age)
        .unwrap_or(3600)
        .clamp(60, 24 * 3600);
    let set: Jwks = response.json().map_err(|e| format!("Google's keys could not be read: {e}"))?;
    let mut keys = HashMap::new();
    for jwk in set.keys.into_iter().filter(|k| k.kty == "RSA") {
        if let Ok(key) = DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
            keys.insert(jwk.kid, key);
        }
    }
    let now = Instant::now();
    Ok(KeySet { keys, fresh_until: now + Duration::from_secs(max_age), fetched_at: now })
}

fn max_age(cache_control: &str) -> Option<u64> {
    cache_control
        .split(',')
        .filter_map(|part| part.trim().strip_prefix("max-age="))
        .find_map(|v| v.trim().parse().ok())
}

// ---------------------------------------------------------------------------
// Commands - the Settings card on the computer that serves.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAccessStatus {
    pub enabled: bool,
    pub port: u16,
    pub running: bool,
    /// Where the tunnel points: `http://127.0.0.1:<port>`.
    pub address: Option<String>,
    pub signed_in: bool,
    /// False on a build made without the web screens.
    pub screens_included: bool,
    pub error: Option<String>,
}

fn status<R: Runtime>(app: &AppHandle<R>) -> WebAccessStatus {
    let options = read_options(config_path(app).as_ref());
    let error = LAST_ERROR.lock().unwrap().clone();
    let port = RUNNING.lock().unwrap().as_ref().map(|r| r.port);
    let running = port.is_some() && error.is_none();
    WebAccessStatus {
        enabled: options.enabled,
        port: options.port,
        running,
        address: port.filter(|_| running).map(|p| format!("http://127.0.0.1:{p}")),
        signed_in: signed_in().is_some(),
        screens_included: find_asset("index.html").is_some(),
        error,
    }
}

#[tauri::command]
pub fn get_web_access_status(app: AppHandle) -> WebAccessStatus {
    status(&app)
}

#[tauri::command]
pub fn set_web_access_options(app: AppHandle, options: WebAccessOptions) -> AppResult<WebAccessStatus> {
    if options.port < 1024 {
        return Err(AppError::Validation("Choose a port from 1024 to 65535.".to_string()));
    }
    let path = config_path(&app).ok_or_else(|| AppError::Other("The app's data folder could not be found.".into()))?;
    write_options(&path, &options)?;
    apply(&app, options);
    Ok(status(&app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const UID: &str = "owner-uid-123";
    const KID: &str = "test-key-1";

    /// The same openssl route as google_sheets.rs's tests.
    fn keypair() -> (String, String) {
        let private = Command::new("openssl").args(["genrsa", "2048"]).output().expect("openssl must be available");
        assert!(private.status.success());
        let private_pem = String::from_utf8(private.stdout).unwrap();
        let mut child = Command::new("openssl")
            .args(["rsa", "-pubout"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(private_pem.as_bytes()).unwrap();
        let public = child.wait_with_output().unwrap();
        assert!(public.status.success());
        (private_pem, String::from_utf8(public.stdout).unwrap())
    }

    fn token(private_pem: &str, kid: &str, claims: Value) -> String {
        let mut head = jsonwebtoken::Header::new(Algorithm::RS256);
        head.kid = Some(kid.to_string());
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_pem.as_bytes()).unwrap();
        jsonwebtoken::encode(&head, &claims, &key).unwrap()
    }

    fn claims(sub: &str, iat: u64, exp: u64) -> Value {
        json!({
            "iss": format!("https://securetoken.google.com/{PROJECT_ID}"),
            "aud": PROJECT_ID,
            "sub": sub,
            "iat": iat,
            "exp": exp,
            "auth_time": iat,
        })
    }

    #[test]
    fn only_a_current_token_of_the_open_account_for_this_project_gets_in() {
        let (private_pem, public_pem) = keypair();
        let public = DecodingKey::from_rsa_pem(public_pem.as_bytes()).unwrap();
        let keys = move |kid: &str| if kid == KID { Ok(public.clone()) } else { Err("an unknown key".to_string()) };
        let now = now_secs();

        let good = token(&private_pem, KID, claims(UID, now, now + 3600));
        let who = verify_token(&good, UID, now, &keys).expect("a current token of the open account");
        assert_eq!(who.uid, UID);
        assert_eq!(who.expires_at, now + 3600);

        let other = token(&private_pem, KID, claims("someone-else", now, now + 3600));
        assert_eq!(verify_token(&other, UID, now, &keys).err(), Some(Refused::OtherAccount), "another account");

        let expired = token(&private_pem, KID, claims(UID, now - 7200, now - 3600));
        assert!(verify_token(&expired, UID, now, &keys).is_err(), "expired");

        let future = token(&private_pem, KID, claims(UID, now + 3600, now + 7200));
        assert!(verify_token(&future, UID, now, &keys).is_err(), "issued in the future");

        let mut wrong_project = claims(UID, now, now + 3600);
        wrong_project["aud"] = json!("another-project");
        assert!(verify_token(&token(&private_pem, KID, wrong_project), UID, now, &keys).is_err(), "audience");

        let mut wrong_issuer = claims(UID, now, now + 3600);
        wrong_issuer["iss"] = json!("https://securetoken.google.com/another-project");
        assert!(verify_token(&token(&private_pem, KID, wrong_issuer), UID, now, &keys).is_err(), "issuer");

        let unknown_key = token(&private_pem, "another-kid", claims(UID, now, now + 3600));
        assert!(verify_token(&unknown_key, UID, now, &keys).is_err(), "a key Google does not have");

        // Signed by someone else's key under Google's key id.
        let (stranger, _) = keypair();
        let forged = token(&stranger, KID, claims(UID, now, now + 3600));
        assert!(verify_token(&forged, UID, now, &keys).is_err(), "a forged signature");

        // An unsigned token and a shared-secret one never get as far as a key.
        let hs = jsonwebtoken::encode(
            &jsonwebtoken::Header { kid: Some(KID.into()), ..jsonwebtoken::Header::new(Algorithm::HS256) },
            &claims(UID, now, now + 3600),
            &jsonwebtoken::EncodingKey::from_secret(b"guess"),
        )
        .unwrap();
        assert!(verify_token(&hs, UID, now, &keys).is_err(), "HS256");
        assert!(verify_token("not.a.token", UID, now, &keys).is_err(), "garbage");
        let parts: Vec<&str> = good.split('.').collect();
        let none_head = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            br#"{"alg":"none","typ":"JWT","kid":"test-key-1"}"#,
        );
        assert!(verify_token(&format!("{none_head}.{}.", parts[1]), UID, now, &keys).is_err(), "alg none");
    }

    #[test]
    fn files_windows_and_the_servers_own_loops_are_refused_with_a_reason() {
        let none = json!({});
        assert_eq!(refusal("export_sales_csv", &json!({"path": "C:\\x.csv"}), UID), Some(FILES));
        assert_eq!(refusal("restore_database", &json!({"srcPath": "C:\\x"}), UID), Some(FILES));
        assert_eq!(refusal("open_price_scanner", &none, UID), Some(WINDOWS));
        assert_eq!(refusal("connect_mail_gmail", &none, UID), Some(WINDOWS));
        assert_eq!(refusal("live_sync_take", &none, UID), Some(OWN_WORK));
        assert_eq!(refusal("cloud_sync_pull", &none, UID), Some(OWN_WORK));
        assert_eq!(refusal("set_web_access_options", &none, UID), Some(SWITCH));
        assert_eq!(refusal("plugin:dialog|open", &none, UID), Some(PLUGIN));
        assert_eq!(refusal("switch_active_database", &json!({"uid": "someone-else", "legacy": false}), UID), Some(ACCOUNT));
        assert_eq!(refusal("switch_active_database", &json!({"uid": UID, "legacy": true}), UID), None);
        assert_eq!(refusal("list_events", &none, UID), None);
        assert_eq!(refusal("create_sale", &none, UID), None);
        assert_eq!(refusal("get_web_access_status", &none, UID), None);
    }

    #[test]
    fn every_refused_command_is_a_command_this_app_has() {
        // A renamed command would otherwise quietly stop being refused.
        let lib = include_str!("lib.rs");
        for (names, _) in BLOCKED {
            for name in *names {
                assert!(
                    lib.contains(&format!("::{name},")) || lib.contains(&format!("::{name}\n")),
                    "{name} is not registered in lib.rs"
                );
            }
        }
    }

    #[test]
    fn options_are_off_unless_a_readable_file_says_on() {
        let dir = std::env::temp_dir().join(format!("tiqr-web-access-{}", std::process::id()));
        let path = dir.join(CONFIG_FILE);
        assert_eq!(read_options(Some(&path)), WebAccessOptions::default());
        assert!(!read_options(None).enabled);
        write_options(&path, &WebAccessOptions { enabled: true, port: 9000 }).unwrap();
        assert_eq!(read_options(Some(&path)), WebAccessOptions { enabled: true, port: 9000 });
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(read_options(Some(&path)), WebAccessOptions::default(), "a garbled file is off");
        std::fs::write(&path, r#"{"enabled": true}"#).unwrap();
        assert_eq!(read_options(Some(&path)).port, DEFAULT_PORT);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_control_max_age_is_read() {
        assert_eq!(max_age("public, max-age=19856, must-revalidate, no-transform"), Some(19856));
        assert_eq!(max_age("no-cache"), None);
    }

    #[tauri::command]
    fn ping(word: String) -> String {
        format!("pong {word}")
    }

    #[tauri::command]
    fn fails() -> Result<(), AppError> {
        Err(AppError::Validation("That sale is already paid.".into()))
    }

    #[tauri::command]
    fn bytes() -> tauri::ipc::Response {
        tauri::ipc::Response::new(vec![1u8, 2, 3])
    }

    #[tauri::command]
    async fn later(app: tauri::AppHandle<tauri::test::MockRuntime>) -> String {
        use tauri::Emitter;
        let _ = app.emit("mail-sync", json!({ "stored": 2 }));
        "done".to_string()
    }

    fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![ping, fails, bytes, later])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        tauri::WebviewWindowBuilder::new(&app, MAIN_WINDOW, Default::default()).build().unwrap();
        app
    }

    /// The whole road a browser request takes: HTTP on 127.0.0.1, the token,
    /// the refusal list, the window's own command handler, the answer - and
    /// an event the window gets, arriving in the browser's stream.
    #[test]
    fn a_browser_request_reaches_the_windows_command_and_only_with_the_right_token() {
        let (private_pem, public_pem) = keypair();
        {
            let mut keys = HashMap::new();
            keys.insert(KID.to_string(), DecodingKey::from_rsa_pem(public_pem.as_bytes()).unwrap());
            let now = Instant::now();
            *KEYS.lock().unwrap() =
                Some(KeySet { keys, fresh_until: now + Duration::from_secs(3600), fetched_at: now });
        }
        let app = mock_app();
        let port = start(app.handle().clone(), 0).unwrap();
        let base = format!("http://127.0.0.1:{port}");
        let client = reqwest::blocking::Client::new();
        let now = now_secs();
        let good = token(&private_pem, KID, claims(UID, now, now + 3600));
        let call = |token: Option<&str>, body: Value| {
            let mut request = client.post(format!("{base}/api/invoke")).json(&body);
            if let Some(t) = token {
                request = request.bearer_auth(t);
            }
            request.send().unwrap()
        };

        assert_eq!(client.get(format!("{base}/api/health")).send().unwrap().status(), 200);

        // Nobody signed in on the server yet: nobody gets in.
        *SIGNED_IN.lock().unwrap() = None;
        assert_eq!(call(Some(&good), json!({"cmd": "ping", "args": {"word": "x"}})).status(), 503);
        note_signed_in(UID);

        assert_eq!(call(None, json!({"cmd": "ping", "args": {"word": "x"}})).status(), 401);
        assert_eq!(call(Some("not.a.token"), json!({"cmd": "ping", "args": {"word": "x"}})).status(), 401);
        let stranger = token(&private_pem, KID, claims("someone-else", now, now + 3600));
        assert_eq!(call(Some(&stranger), json!({"cmd": "ping", "args": {"word": "x"}})).status(), 403);

        let ok = call(Some(&good), json!({"cmd": "ping", "args": {"word": "hello"}}));
        assert_eq!(ok.status(), 200);
        assert_eq!(ok.headers()["content-security-policy"], CSP);
        assert_eq!(ok.json::<Value>().unwrap(), json!("pong hello"));

        let failed = call(Some(&good), json!({"cmd": "fails"}));
        assert_eq!(failed.status(), 422);
        assert_eq!(failed.json::<Value>().unwrap()["error"], json!("That sale is already paid."));

        let raw = call(Some(&good), json!({"cmd": "bytes"}));
        assert_eq!(raw.headers()["content-type"], "application/octet-stream");
        assert_eq!(raw.bytes().unwrap().to_vec(), vec![1u8, 2, 3]);

        let refused = call(Some(&good), json!({"cmd": "export_sales_csv", "args": {"path": "C:\\x.csv"}}));
        assert_eq!(refused.status(), 403);
        assert_eq!(refused.json::<Value>().unwrap()["error"], json!(FILES));

        // An event the window gets reaches a listening browser.
        let mut stream = client.get(format!("{base}/api/events")).bearer_auth(&good).send().unwrap();
        assert_eq!(stream.headers()["content-type"], "text/event-stream");
        assert_eq!(call(Some(&good), json!({"cmd": "later"})).json::<Value>().unwrap(), json!("done"));
        let mut seen = String::new();
        let mut buf = [0u8; 256];
        while !seen.contains("\n\n") {
            use std::io::Read;
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0, "the stream ended early: {seen:?}");
            seen.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        assert!(seen.contains("event: mail-sync"), "{seen}");
        assert!(seen.contains(r#"data: {"stored":2}"#), "{seen}");
        assert_eq!(client.get(format!("{base}/api/events")).send().unwrap().status(), 401);

        // The screens: a build without them says so instead of a blank page.
        let page = client.get(format!("{base}/")).send().unwrap();
        if WEB_ASSETS.is_empty() {
            assert_eq!(page.status(), 503);
        } else {
            assert_eq!(page.status(), 200);
            assert!(page.headers()["content-type"].to_str().unwrap().starts_with("text/html"));
        }
        assert_eq!(client.post(format!("{base}/")).send().unwrap().status(), 405);

        stop();
    }

    fn serve(router: Router) -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        tauri::async_runtime::spawn(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            let _ = axum::serve(listener, router).await;
        });
        port
    }

    /// Firebase's sign-in pages come through this address: what a page needs
    /// goes out, this address's cookies do not, Firebase's answer comes back
    /// without its cookies - and without TIQR's page rules, which stay on
    /// TIQR's own pages.
    #[test]
    fn firebase_sign_in_pages_come_through_this_address() {
        use std::sync::Arc;
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        // A stand-in for firebaseapp.com.
        let firebase = Router::new().fallback(move |method: Method, uri: Uri, headers: HeaderMap, body: Bytes| {
            let log = log.clone();
            async move {
                let sent = |name: header::HeaderName| headers.get(name).map(|v| v.to_str().unwrap().to_string());
                log.lock().unwrap().push(format!(
                    "{method} {uri} cookie={:?} type={:?} body={}",
                    sent(header::COOKIE),
                    sent(header::CONTENT_TYPE),
                    String::from_utf8_lossy(&body)
                ));
                (
                    [
                        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                        (header::SET_COOKIE, "firebase=1"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    "<html>handler</html>",
                )
            }
        });
        let firebase_port = serve(firebase);
        let app = mock_app();
        let hub = event_hub(app.handle());
        let port = serve(routes(app.handle().clone(), hub, format!("http://127.0.0.1:{firebase_port}")));
        let base = format!("http://127.0.0.1:{port}");
        let client = reqwest::blocking::Client::new();

        // Google's way back, with the query exactly as it came.
        let back = "/__/auth/handler?state=abc&code=4/0Ab&scope=email+https://www.googleapis.com/auth/userinfo.email";
        let page = client.get(format!("{base}{back}")).header(header::COOKIE, "CF_Authorization=secret").send().unwrap();
        assert_eq!(page.status(), 200);
        assert_eq!(page.headers()["content-type"], "text/html; charset=utf-8");
        assert_eq!(page.headers()["cache-control"], "no-cache");
        assert!(page.headers().get(header::SET_COOKIE).is_none(), "no cookies come back");
        assert!(page.headers().get("content-security-policy").is_none(), "Firebase's page, Firebase's rules");
        assert!(page.headers().get("x-frame-options").is_none(), "TIQR's page frames Firebase's frame");
        assert_eq!(page.text().unwrap(), "<html>handler</html>");

        let form = client
            .post(format!("{base}/__/auth/handler"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body("state=abc")
            .send()
            .unwrap();
        assert_eq!(form.status(), 200);
        assert_eq!(client.get(format!("{base}/__/firebase/init.json")).send().unwrap().status(), 200);
        assert_eq!(client.post(format!("{base}/__/firebase/init.json")).send().unwrap().status(), 405);

        // A path that climbs out of the folder is not passed on. (reqwest
        // would tidy the path, so this one goes out as raw HTTP.)
        {
            use std::io::{Read, Write};
            let mut raw = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            raw.write_all(b"GET /__/auth/../../api/health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
            let mut answer = String::new();
            raw.read_to_string(&mut answer).unwrap();
            assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");
        }

        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                format!("GET {back} cookie=None type=None body="),
                "POST /__/auth/handler cookie=None type=Some(\"application/x-www-form-urlencoded\") body=state=abc"
                    .to_string(),
                "GET /__/firebase/init.json cookie=None type=None body=".to_string(),
            ]
        );

        // TIQR's own pages keep their rules.
        let health = client.get(format!("{base}/api/health")).send().unwrap();
        assert_eq!(health.headers()["content-security-policy"], CSP);
        assert_eq!(client.get(format!("{base}/sales")).send().unwrap().headers()["x-frame-options"], "DENY");
    }

    #[test]
    fn firebase_out_of_reach_is_said_plainly() {
        // Nothing listens on port 9 here.
        let port = serve(sign_in_routes("http://127.0.0.1:9".to_string()));
        let answer = reqwest::blocking::get(format!("http://127.0.0.1:{port}/__/auth/iframe")).unwrap();
        assert_eq!(answer.status(), 502);
        assert_eq!(answer.text().unwrap(), SIGN_IN_UNREACHABLE);
    }
}
