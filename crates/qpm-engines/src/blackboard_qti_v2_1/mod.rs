//! Blackboard Learn QTI 2.1 content-package writer.
//!
//! Each assessment item is a distinct XML resource below `qti21_items/`; local images are
//! copied to the ZIP root and every image reference is rewritten relative to that directory.

mod fragment;
mod item_xml;
mod writer;

#[cfg(test)]
mod tests;

pub(crate) use writer::NAME;
pub use writer::boxed_writer;
