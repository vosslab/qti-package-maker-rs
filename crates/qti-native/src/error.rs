//! Structured failures at the native host boundary.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error("could not access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid native input or output: {0}")]
    Invalid(String),
    #[error(transparent)]
    Engine(#[from] qti_engines::EngineError),
    #[error("could not obtain the host local civil date: {0}")]
    LocalDate(String),
}

pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> NativeError {
    NativeError::Io {
        path: path.to_path_buf(),
        source,
    }
}
