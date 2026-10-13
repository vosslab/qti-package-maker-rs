//! Blackboard pool package reader split by discovery, item types, and media transport.

mod discovery;
mod media_csfiles;
mod media_hotspot;
mod types_core;
mod types_extra;

#[cfg(test)]
mod reader_tests;

pub(super) use discovery::read_package;
