//! Object-safe writer adapter for the PLE Native JSON directory export.

use std::path::Path;

use qti_core::media::MediaPolicy;
use qti_core::{ItemBank, ItemKind};

use super::{export_bank, output};
use crate::{EngineError, EngineOptions, WriteOutcome, Writer};

const KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Fib,
    ItemKind::MultiFib,
    ItemKind::Num,
    ItemKind::Match,
    ItemKind::Order,
];

/// Creates the registry writer; this format has no document-level options.
pub fn boxed_writer(_: EngineOptions) -> Box<dyn Writer> {
    Box::new(PleNativeJsonWriter)
}

struct PleNativeJsonWriter;

impl Writer for PleNativeJsonWriter {
    fn name(&self) -> &'static str {
        "ple_native_json"
    }

    fn media_policy(&self) -> MediaPolicy {
        MediaPolicy::Package
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        KINDS
    }

    fn save_package(
        &self,
        bank: &ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError> {
        let export = export_bank(bank)?;
        if export.questions.is_empty() {
            return Ok(WriteOutcome {
                path: None,
                warnings: export.warnings,
            });
        }
        let path = output.unwrap_or_else(|| Path::new("ple"));
        output::write_export(&export, path)?;
        Ok(WriteOutcome {
            path: Some(path.to_path_buf()),
            warnings: export.warnings,
        })
    }
}
