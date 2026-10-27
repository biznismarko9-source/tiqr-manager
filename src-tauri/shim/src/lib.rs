//! TIQR in a phone's browser: the few parts of Tauri that the shared command
//! code uses, for the WebAssembly build.
//!
//! The phone build compiles the same command modules as the desktop app. They
//! say `use tauri::State;`, `#[tauri::command]`, `app.state::<AppState>()`;
//! on WebAssembly `tauri` is this crate (lib.rs: `extern crate ... as
//! tauri`), so those lines mean the same thing in a browser:
//!
//! * the app's state is kept here for the life of the page (`Manager`),
//! * an event goes to the page through a hook the phone build sets
//!   (`Emitter`, `set_emit_hook`),
//! * "run this off the window's thread" runs it right away - the phone build
//!   already runs in a worker, away from the screen (`async_runtime`),
//! * a command is called by name with JSON arguments (`ipc`, and the
//!   `command` macro that writes each command's caller).
//!
//! Nothing else of Tauri is here on purpose: a module that needs more (a
//! window, a file dialog, a plugin) is a computer's, and is not compiled into
//! the phone build.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

pub use tiqr_tauri_shim_macros::{command, generate_handler};
/// As Tauri's: the `url` crate's.
pub use url::Url;

/// Tauri's error, as far as the shared code looks at one: a message.
#[derive(Debug, Clone)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A piece of the app's state, borrowed for as long as the page lives.
pub struct State<'r, T: Send + Sync + 'static>(&'r T);

impl<'r, T: Send + Sync + 'static> State<'r, T> {
    pub fn inner(&self) -> &'r T {
        self.0
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: Send + Sync + 'static> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        State(self.0)
    }
}

type Managed = HashMap<TypeId, &'static (dyn Any + Send + Sync)>;
static MANAGED: Mutex<Option<Managed>> = Mutex::new(None);

type EmitHook = Box<dyn Fn(&str, String) + Send + Sync>;
static EMIT: Mutex<Option<EmitHook>> = Mutex::new(None);

static DATA_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

static PACKAGE: Mutex<Option<PackageInfo>> = Mutex::new(None);

/// The app, as the shared code holds it: one per page.
#[derive(Clone, Default)]
pub struct AppHandle {
    _one: (),
}

impl AppHandle {
    pub fn new() -> Self {
        AppHandle { _one: () }
    }

    pub fn path(&self) -> PathResolver {
        PathResolver
    }

    /// The app's name and version, as the phone build set them
    /// (`set_package_info`) - this crate's own would be the stand-in's.
    pub fn package_info(&self) -> PackageInfo {
        PACKAGE.lock().unwrap().clone().unwrap_or_default()
    }
}

/// What `AppHandle::package_info` answers.
#[derive(Clone, Debug, Default)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

pub fn set_package_info(name: &str, version: &str) {
    *PACKAGE.lock().unwrap() = Some(PackageInfo { name: name.to_string(), version: version.to_string() });
}

/// Where the app keeps its files - inside the browser's own storage for this
/// site (the phone build's database file system).
pub struct PathResolver;

impl PathResolver {
    pub fn app_data_dir(&self) -> Result<PathBuf> {
        Ok(DATA_DIR.lock().unwrap().clone().unwrap_or_else(|| PathBuf::from("/tiqr")))
    }

    pub fn app_local_data_dir(&self) -> Result<PathBuf> {
        self.app_data_dir()
    }

    pub fn app_config_dir(&self) -> Result<PathBuf> {
        self.app_data_dir()
    }

    pub fn app_log_dir(&self) -> Result<PathBuf> {
        Ok(self.app_data_dir()?.join("logs"))
    }
}

/// Sets the folder `PathResolver::app_data_dir` answers with.
pub fn set_data_dir(dir: PathBuf) {
    *DATA_DIR.lock().unwrap() = Some(dir);
}

pub trait Manager {
    fn state<T: Send + Sync + 'static>(&self) -> State<'static, T>;
    fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'static, T>>;
    fn manage<T: Send + Sync + 'static>(&self, value: T) -> bool;
}

impl Manager for AppHandle {
    /// Like Tauri: asking for state that was never managed is a programming
    /// error, and says which.
    fn state<T: Send + Sync + 'static>(&self) -> State<'static, T> {
        self.try_state::<T>()
            .unwrap_or_else(|| panic!("state not managed: {}", std::any::type_name::<T>()))
    }

    fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'static, T>> {
        let managed = MANAGED.lock().unwrap();
        let value = *managed.as_ref()?.get(&TypeId::of::<T>())?;
        value.downcast_ref::<T>().map(State)
    }

    /// Kept for the life of the page, as in Tauri. A second value of the same
    /// type is refused, as in Tauri.
    fn manage<T: Send + Sync + 'static>(&self, value: T) -> bool {
        let mut managed = MANAGED.lock().unwrap();
        let map = managed.get_or_insert_with(HashMap::new);
        if map.contains_key(&TypeId::of::<T>()) {
            return false;
        }
        let leaked: &'static T = Box::leak(Box::new(value));
        map.insert(TypeId::of::<T>(), leaked);
        true
    }
}

pub trait Emitter {
    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> Result<()>;
}

impl Emitter for AppHandle {
    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> Result<()> {
        let text = serde_json::to_string(&payload).map_err(|e| Error(e.to_string()))?;
        if let Some(hook) = EMIT.lock().unwrap().as_ref() {
            hook(event, text);
        }
        Ok(())
    }
}

/// Where `emit` sends an event: the phone build hands it to the page.
pub fn set_emit_hook(hook: impl Fn(&str, String) + Send + Sync + 'static) {
    *EMIT.lock().unwrap() = Some(Box::new(hook));
}

pub mod async_runtime {
    /// Runs `f` now. The phone build already runs in a worker, away from the
    /// screen, which is what the desktop app uses a separate thread for.
    pub fn spawn_blocking<F, R>(f: F) -> std::future::Ready<super::Result<R>>
    where
        F: FnOnce() -> R,
    {
        std::future::ready(Ok(f()))
    }
}

pub mod ipc {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::Value;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// Raw bytes as a command's answer (a file, a picture).
    pub struct Response(Vec<u8>);

    impl Response {
        pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
            Response(bytes.into())
        }
    }

    /// What a command answered.
    pub enum Reply {
        Json(Value),
        Bytes(Vec<u8>),
    }

    impl Reply {
        pub fn json<T: Serialize>(value: &T) -> Result<Reply, Value> {
            serde_json::to_value(value)
                .map(Reply::Json)
                .map_err(|e| Value::String(format!("The answer could not be written: {e}")))
        }

        pub fn bytes(response: Response) -> Reply {
            Reply::Bytes(response.0)
        }
    }

    /// One argument, by its name, as Tauri reads it.
    pub fn arg<T: DeserializeOwned>(args: &Value, key: &str, cmd: &str) -> Result<T, Value> {
        let value = args.get(key).cloned().unwrap_or(Value::Null);
        serde_json::from_value(value)
            .map_err(|e| Value::String(format!("invalid args `{key}` for command `{cmd}`: {e}")))
    }

    /// A command's error, as the screen reads it - the same JSON Tauri sends.
    pub fn error_value<E: Serialize>(error: &E) -> Value {
        serde_json::to_value(error).unwrap_or_else(|e| Value::String(e.to_string()))
    }

    /// A command's answer: ready now, or later for a command that waits.
    pub enum Pending {
        Ready(Option<Result<Reply, Value>>),
        Later(Pin<Box<dyn Future<Output = Result<Reply, Value>>>>),
    }

    impl Pending {
        pub fn ready(result: Result<Reply, Value>) -> Self {
            Pending::Ready(Some(result))
        }

        pub fn later(future: impl Future<Output = Result<Reply, Value>> + 'static) -> Self {
            Pending::Later(Box::pin(future))
        }
    }

    impl Future for Pending {
        type Output = Result<Reply, Value>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.get_mut() {
                Pending::Ready(slot) => {
                    Poll::Ready(slot.take().unwrap_or_else(|| Err(Value::String("answered twice".into()))))
                }
                Pending::Later(future) => future.as_mut().poll(cx),
            }
        }
    }
}
