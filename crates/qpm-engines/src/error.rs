//! Typed fatal errors at an engine boundary.

use qpm_core::{BankError, ItemKind, ManifestError, ValidationError, ZipError};
use thiserror::Error;

/// Retain validation types until the reader chooses whether an item is recoverable.
#[derive(Debug, Error)]
pub(crate) enum ReadItemError {
    #[error("{0}")]
    Syntax(String),
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

impl From<String> for ReadItemError {
    fn from(message: String) -> Self {
        Self::Syntax(message)
    }
}

impl ReadItemError {
    pub(crate) fn recoverable_message(self) -> Result<String, EngineError> {
        match self {
            Self::Validation(error @ ValidationError::InvalidInlineCss { .. }) => Err(error.into()),
            error => Ok(error.to_string()),
        }
    }
}

/// A fatal failure from a named input or output engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The requested format is absent from the shared registry.
    #[error("unknown engine '{name}'")]
    UnknownEngine {
        /// Caller-provided format name.
        name: String,
    },
    /// A registered format cannot perform the requested operation.
    #[error("engine '{engine}' does not support {direction}")]
    UnsupportedDirection {
        /// Registry engine name.
        engine: &'static str,
        /// Reader or writer operation.
        direction: &'static str,
    },
    /// Logical input or output provenance around a portable engine failure.
    #[error("engine '{engine}' failed for {source_name:?}: {source}")]
    Context {
        /// Registry engine name.
        engine: &'static str,
        /// Logical input or output name, independent of item or media provenance.
        source_name: Option<String>,
        /// Underlying portable failure.
        #[source]
        source: Box<EngineError>,
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
