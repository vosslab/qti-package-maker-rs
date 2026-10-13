//! Reader and writer for text2qti's compact Markdown-like question format.
//!
//! The supported item shapes are MC, MA, NUM, and FIB.  The format has no
//! representation for matching, multi-blank, or ordering items.

mod media;
mod parser;
mod reader;
#[cfg(test)]
mod tests;
mod writer;

use crate::{Reader, Writer};
use reader::Text2QtiReader;
use writer::Text2QtiWriter;

pub(crate) const NAME: &str = "text2qti";

/// Creates this engine's text writer for the common registry factory.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(Text2QtiWriter)
}

/// Creates this engine's text reader for the common registry factory.
pub fn boxed_reader() -> Box<dyn Reader> {
    Box::new(Text2QtiReader)
}
