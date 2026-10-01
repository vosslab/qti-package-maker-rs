//! Typed failures at the command-line boundary.

use std::path::PathBuf;

use thiserror::Error;

/// A diagnostic suitable for a command-line user, with a stable exit category.
#[derive(Debug, Error)]
pub enum CliError {
    /// Clap accepted the syntax but the selected combination violates the legacy contract.
    #[error("invalid command line: {message}")]
    Arguments { message: String },
    /// The legacy BBQ filename convention is required to derive package names.
    #[error("input filename '{path}' does not match expected pattern 'bbq-<name>-questions.txt'")]
    InputName { path: PathBuf },
    /// The requested reader is not available in the static registry.
    #[error("engine '{engine}' cannot read input")]
    ReaderUnavailable { engine: String },
    /// Engine resolution found no exact or unique-prefix match.
    #[error("unknown engine '{requested}'; available engines: {candidates}")]
    UnknownEngine {
        requested: String,
        candidates: String,
    },
    /// An engine prefix matches more than one registry entry.
    #[error("ambiguous engine '{requested}'; candidates: {candidates}")]
    AmbiguousEngine {
        requested: String,
        candidates: String,
    },
    /// A selected engine has no writer factory.
    #[error("engine '{engine}' cannot write output")]
    WriterUnavailable { engine: String },
    /// Reading a package failed at the engine boundary.
    #[error(transparent)]
    Engine(#[from] qti_engines::EngineError),
    /// The native, static HTML-to-image conversion rejected the bank.
    #[error(transparent)]
    HtmlToImage(#[from] qti_engines::html_to_image::ConversionError),
    /// The operating system did not provide a local civil time.
    #[error("could not obtain the host local civil date: {message}")]
    LocalDate { message: String },
}

impl CliError {
    /// Exit status used by thin binary adapters.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Arguments { .. }
            | Self::InputName { .. }
            | Self::UnknownEngine { .. }
            | Self::AmbiguousEngine { .. }
            | Self::ReaderUnavailable { .. }
            | Self::WriterUnavailable { .. } => 2,
            Self::Engine(_) | Self::HtmlToImage(_) | Self::LocalDate { .. } => 1,
        }
    }
}
