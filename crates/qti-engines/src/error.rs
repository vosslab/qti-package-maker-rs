//! Typed fatal errors at an engine boundary.

use std::path::PathBuf;

use qti_core::{BankError, ItemKind, ManifestError, ValidationError, ZipError};
use thiserror::Error;

/// A fatal failure from a named input or output engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The operating system could not read or write an engine path.
    #[error("engine '{engine}' could not access {path}: {source}")]
    Io {
        /// Registry engine name.
        engine: &'static str,
        /// Input or output path.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// An item kind has no representation in the selected format.
    #[error("engine '{engine}' does not support item kind {kind:?}")]
    UnsupportedItemKind {
        /// Registry engine name.
        engine: &'static str,
        /// Unsupported item kind.
        kind: ItemKind,
    },
    /// The writer could not collect or rewrite source media.
    #[error(transparent)]
    Bank(#[from] BankError),
    /// An item parsed from a reader input violated the core contract.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// Package manifest construction failed.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// ZIP preflight, encoding, or persistence failed.
    #[error(transparent)]
    Zip(#[from] ZipError),
    /// The format's input or generated XML shape was invalid.
    #[error("engine '{engine}' invalid {format} data: {message}")]
    InvalidFormat {
        /// Registry engine name.
        engine: &'static str,
        /// Format component such as XML or BBQ text.
        format: &'static str,
        /// Specific diagnostic safe for the caller.
        message: String,
    },
}
