//! Command-line boundaries for the Rust QTI package maker.
//!
//! The public library API keeps argument parsing, bank loading, conversion, and
//! output dispatch testable without spawning a process.  The two binaries are
//! intentionally thin process-exit adapters.

mod app;
mod error;

pub use app::{
    BbqConverterArgs, Cli, ConverterReport, PackageMakerArgs, PackageMakerCommand,
    package_check_failed, run_bbq_converter, run_bbq_converter_from, run_package_maker,
    run_package_maker_from,
};
pub use error::CliError;
