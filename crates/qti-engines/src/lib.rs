//! Format readers, writers, and the compile-time engine registry.

pub mod bbq_text_upload;
pub mod blackboard_export_zip;
pub mod blackboard_qti_v2_1;
pub mod canvas_qti_v1_2;
mod conversion;
pub mod error;
pub mod exam_yaml;
pub mod html_selftest;
pub mod human_readable;
pub mod moodle_aiken;
pub mod okla_chrst_bqgen;
pub mod ple_native_json;
pub mod registry;
pub mod text2qti;
pub mod traits;

pub use conversion::{
    AssetOverlay, ConversionOutcome, ConversionRequest, convert, read_bank, write_bank,
};
pub use error::EngineError;
pub use qti_core::media::MediaWarning;
pub use registry::{DocumentMetadata, ENGINES, EngineEntry, engine};
pub use traits::{
    ReadInput, ReadLocation, ReadOutcome, ReadWarning, Reader, RenderHooks, WriteArtifact,
    WriteContext, WriteOutcome, Writer, render_bank,
};
