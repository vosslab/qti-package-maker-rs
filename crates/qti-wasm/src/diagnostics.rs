//! One projection of shared failures and warnings into transport diagnostics.

use qti_core::BankError;
use qti_engines::{EngineError, ReadLocation, ReadWarning};
use qti_integrity::{Provenance, Severity, Violation};

use crate::{Diagnostic, IntegrityFinding, Warning};

impl Diagnostic {
    pub(crate) fn request(category: &str, message: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            message: message.into(),
            format: None,
            item: None,
            source: None,
            logical_name: None,
        }
    }
}

pub(crate) fn engine_error(error: &EngineError) -> Diagnostic {
    let mut diagnostic = Diagnostic::request("conversion", error.to_string());
    match error {
        EngineError::Context {
            engine,
            source_name,
            source,
        } => {
            diagnostic = engine_error(source);
            diagnostic.message = error.to_string();
            diagnostic.format.get_or_insert_with(|| (*engine).into());
            diagnostic.logical_name = source_name.clone().or(diagnostic.logical_name);
            if diagnostic.source.is_none() {
                diagnostic.source = source_name.clone();
            }
        }
        EngineError::UnknownEngine { name } => {
            diagnostic.category = "unknownFormat".into();
            diagnostic.format = Some(name.clone());
        }
        EngineError::UnsupportedDirection { engine, .. } => {
            diagnostic.category = "unsupportedDirection".into();
            diagnostic.format = Some((*engine).into());
        }
        EngineError::UnsupportedItemKind { engine, .. } => {
            diagnostic.category = "unsupportedItemKind".into();
            diagnostic.format = Some((*engine).into());
        }
        EngineError::InvalidFormat { engine, .. } => {
            diagnostic.category = "invalidFormat".into();
            diagnostic.format = Some((*engine).into());
        }
        EngineError::Bank(bank) => {
            diagnostic.category = "bank".into();
            if let BankError::CollectAsset { item_crc, src, .. } = bank {
                diagnostic.category = "media".into();
                diagnostic.item = Some(item_crc.to_string());
                diagnostic.source = src.clone();
            }
        }
        EngineError::Validation(_) => diagnostic.category = "validation".into(),
        EngineError::Manifest(_) => diagnostic.category = "manifest".into(),
        EngineError::Zip(_) => diagnostic.category = "archive".into(),
    }
    diagnostic
}

pub(crate) fn read_warning(warning: ReadWarning, format: &str, input: &str) -> Warning {
    let (category, item, source) = match warning.location {
        ReadLocation::Input => ("input", None, input.into()),
        ReadLocation::Line { line } => ("line", Some(line.to_string()), input.into()),
        ReadLocation::Block { number } => ("block", Some(number.to_string()), input.into()),
        ReadLocation::ArchiveEntry { name } => ("archiveEntry", None, name),
        ReadLocation::PoolItem { resource, number } => {
            ("poolItem", Some(number.to_string()), resource)
        }
        ReadLocation::MediaToken { resource, token } => ("mediaToken", Some(token), resource),
    };
    Warning {
        stage: "read".into(),
        category: category.into(),
        message: warning.message,
        format: Some(format.into()),
        item,
        source: Some(source),
    }
}

pub(crate) fn write_warning(warning: qti_engines::MediaWarning) -> Warning {
    Warning {
        stage: "write".into(),
        category: "media".into(),
        message: warning.reason,
        format: Some(warning.engine_name),
        item: Some(warning.item_crc),
        source: Some(warning.src),
    }
}

pub(crate) fn finding(violation: Violation) -> IntegrityFinding {
    IntegrityFinding {
        code: violation.code.into(),
        severity: match violation.severity {
            Severity::Error => "error",
            Severity::Advisory => "advisory",
        }
        .into(),
        provenance: match violation.provenance {
            Provenance::FormatRequirement => "formatRequirement",
            Provenance::BlackboardImportFailure => "blackboardImportFailure",
            Provenance::PythonParityAdvisory => "pythonParityAdvisory",
            Provenance::SafeInputHandling => "safeInputHandling",
        }
        .into(),
        path: violation.path,
        message: violation.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qti_engines::{DocumentMetadata, ReadInput, WriteContext, read_bank, write_bank};

    #[test]
    fn read_failures_project_logical_input_without_losing_category() {
        let entries = qti_core::EntryMap::new();
        for input in [
            ReadInput::File {
                name: "broken.zip",
                bytes: b"not a ZIP archive",
            },
            ReadInput::Archive {
                name: "pool.zip",
                entries: &entries,
            },
        ] {
            let error = read_bank("blackboard_export_zip", input, true, None)
                .expect_err("invalid Blackboard package");
            let diagnostic = engine_error(&error);
            assert_eq!(diagnostic.category, "invalidFormat");
            assert_eq!(diagnostic.format.as_deref(), Some("blackboard_export_zip"));
            assert_eq!(diagnostic.logical_name.as_deref(), Some(input.name()));
            assert_eq!(diagnostic.source.as_deref(), Some(input.name()));
            assert!(!diagnostic.message.is_empty());
        }
    }

    #[test]
    fn media_failures_preserve_authored_source_item_and_logical_output() {
        let read = read_bank(
            "bbq_text_upload",
            ReadInput::File {
                name: "quiz.txt",
                bytes: b"MC\t<img src='figure.png'/> Which base?\tA\tcorrect\tG\tincorrect\n",
            },
            false,
            None,
        )
        .expect("valid item");
        let context = WriteContext::new(
            "quiz-output.zip",
            DocumentMetadata {
                title: "Genetics".into(),
                date: "2026-10-08".into(),
            },
            0,
        )
        .expect("valid output context");
        let error = write_bank(
            "canvas_qti_v1_2",
            &read.bank,
            &qti_core::MemoryAssets::new(),
            &context,
        )
        .expect_err("missing media payload");
        let diagnostic = engine_error(&error);
        assert_eq!(diagnostic.category, "media");
        assert_eq!(diagnostic.format.as_deref(), Some("canvas_qti_v1_2"));
        assert_eq!(diagnostic.source.as_deref(), Some("figure.png"));
        assert_eq!(
            diagnostic.item,
            Some(read.bank.get(0).expect("first item").crc().to_string())
        );
        assert_eq!(diagnostic.logical_name.as_deref(), Some("quiz-output.zip"));
        let serialized = serde_json::to_value(diagnostic).expect("diagnostic transport");
        assert_eq!(serialized["logicalName"], "quiz-output.zip");
        assert_eq!(serialized["source"], "figure.png");
    }
}
