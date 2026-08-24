use std::path::PathBuf;

/// Errors emitted by deterministic file-authority operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsafe path: {0}")]
    UnsafePath(String),
    #[error("path already exists: {}", .0.display())]
    Collision(PathBuf),
    #[error("source changed (expected {expected}, found {actual})")]
    Conflict { expected: String, actual: String },
    #[error("no vault is selected")]
    NoVaultSelected,
    #[error("unknown vault: {0}")]
    UnknownVault(String),
    #[error("invalid vault name: {0}")]
    InvalidVaultName(String),
    #[error("invalid UTF-8 note content: {}", .0.display())]
    InvalidUtf8(PathBuf),
    #[error("invalid trash entry: {0}")]
    InvalidTrashEntry(String),
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("registry JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
