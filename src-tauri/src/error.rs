use serde::Serialize;

/// Single application error type returned by every Tauri command.
/// Serialized as a plain string so the frontend can show it directly.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Db(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
    /// A call to an external service (currently only Google Sheets) failed
    /// or was rejected - network error, Google returned a non-2xx status,
    /// or the app has no service account credentials embedded in this build.
    /// Kept distinct from `Other` so the frontend can, if useful, show sync
    /// failures differently from ordinary validation/db errors.
    #[error("{0}")]
    External(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint failed") {
            AppError::Validation(format!("Duplicate value: {msg}"))
        } else if msg.contains("FOREIGN KEY constraint failed") {
            AppError::Validation(
                "This action is blocked because other records still reference it.".into(),
            )
        } else if msg.contains("CHECK constraint failed") {
            AppError::Validation(format!("Invalid value: {msg}"))
        } else {
            AppError::Db(msg)
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(e: csv::Error) -> Self {
        AppError::Other(format!("CSV error: {e}"))
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
