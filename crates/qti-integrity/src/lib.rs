//! Cross-reference integrity checks for finished QTI and Blackboard packages.
//!
//! The crate deliberately depends on neither the item model nor any writer. It
//! checks a ZIP or extracted package exactly as an LMS receives it.

mod checker;
mod input;
mod types;

pub use crate::checker::check_entries;
pub use crate::input::{check_package, read_zip_entries, validate_entries};
pub use crate::types::{Provenance, Severity, Violation};
