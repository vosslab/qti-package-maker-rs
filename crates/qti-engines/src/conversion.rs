//! Shared read, limit, and write orchestration for every host adapter.

use std::borrow::Cow;

use qti_core::media::{MediaError, MediaWarning};
use qti_core::{AssetSource, ItemBank, MemoryAssets};

use crate::{
    EngineError, ReadInput, ReadOutcome, ReadWarning, WriteArtifact, WriteContext, WriteOutcome,
    engine,
};

/// Borrowed inputs and explicit options for one portable conversion.
pub struct ConversionRequest<'a> {
    /// Registered input format name.
    pub input_format: &'a str,
    /// Registered output format name.
    pub output_format: &'a str,
    /// Document bytes or decoded package entries.
    pub input: ReadInput<'a>,
    /// Caller-provided assets, used when the reader did not recover a source.
    pub assets: &'a dyn AssetSource,
    /// Whether the reader accepts more than one item kind.
    pub allow_mixed: bool,
    /// Retain at most this many items, in input order.
    pub max_items: Option<usize>,
    /// Host-resolved output name, labels, and deterministic shuffle seed.
    pub context: &'a WriteContext,
}

/// Owned output and diagnostics from one shared conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionOutcome {
    /// Completed artifact, absent when no item rendered.
    pub artifact: Option<WriteArtifact>,
    /// Reader diagnostics in input order, presented before writer diagnostics.
    pub read_warnings: Vec<ReadWarning>,
    /// Writer diagnostics in render order.
    pub write_warnings: Vec<MediaWarning>,
    /// Validated item count after the requested limit.
    pub item_count: usize,
}

/// Reads through the registered reader and applies the shared item limit.
///
/// # Errors
/// Returns an unknown format, unsupported input direction, or contextual reader error.
pub fn read_bank(
    format: &str,
    input: ReadInput<'_>,
    allow_mixed: bool,
    max_items: Option<usize>,
) -> Result<ReadOutcome, EngineError> {
    let entry = registered(format)?;
    let factory = entry.make_reader.ok_or_else(|| {
        contextualize(
            EngineError::UnsupportedDirection {
                engine: entry.name,
                direction: "reading",
            },
            entry.name,
            input.name(),
        )
    })?;
    let mut outcome = factory()
        .read_items(input, allow_mixed)
        .map_err(|error| contextualize(error, entry.name, input.name()))?;
    if let Some(limit) = max_items {
        outcome.bank.trim_to(limit);
    }
    Ok(outcome)
}

/// Writes through the registered writer without host persistence.
///
/// # Errors
/// Returns an unknown format, unsupported output direction, or contextual writer error.
pub fn write_bank(
    format: &str,
    bank: &ItemBank,
    assets: &dyn AssetSource,
    context: &WriteContext,
) -> Result<WriteOutcome, EngineError> {
    let entry = registered(format)?;
    let factory = entry.make_writer.ok_or_else(|| {
        contextualize(
            EngineError::UnsupportedDirection {
                engine: entry.name,
                direction: "writing",
            },
            entry.name,
            context.output_name(),
        )
    })?;
    factory()
        .write_package(bank, assets, context)
        .map_err(|error| contextualize(error, entry.name, context.output_name()))
}

/// Converts through the same read and write phases used by native adapters.
///
/// Reader-recovered asset bytes take precedence over external assets. A missing
/// recovered source is resolved by the caller's provider; other failures propagate.
///
/// # Errors
/// Returns the reader or writer error with its available engine and source context.
pub fn convert(request: ConversionRequest<'_>) -> Result<ConversionOutcome, EngineError> {
    // Resolve the output format before work so unknown output names cannot consume input.
    registered(request.output_format)?;
    let read = read_bank(
        request.input_format,
        request.input,
        request.allow_mixed,
        request.max_items,
    )?;
    let assets = AssetOverlay {
        memory: &read.assets,
        fallback: request.assets,
    };
    let write = write_bank(request.output_format, &read.bank, &assets, request.context)?;
    Ok(ConversionOutcome {
        artifact: write.artifact,
        read_warnings: read.warnings,
        write_warnings: write.warnings,
        item_count: read.bank.len(),
    })
}

/// Borrows recovered media first and uses another provider for absent sources.
pub struct AssetOverlay<'a> {
    /// Reader-recovered or native-rendered bytes with authoritative source names.
    pub memory: &'a MemoryAssets,
    /// Provider consulted only when the memory store does not contain a source.
    pub fallback: &'a dyn AssetSource,
}

impl AssetSource for AssetOverlay<'_> {
    fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
        match self.memory.get(src) {
            Some(bytes) => Ok(Cow::Borrowed(bytes)),
            None => self.fallback.read(src),
        }
    }
}

fn registered(format: &str) -> Result<&'static crate::EngineEntry, EngineError> {
    engine(format).ok_or_else(|| EngineError::UnknownEngine {
        name: format.to_owned(),
    })
}

fn contextualize(error: EngineError, engine: &'static str, name: &str) -> EngineError {
    match error {
        EngineError::Context {
            engine,
            source_name,
            source,
        } => EngineError::Context {
            engine,
            source_name: source_name.or_else(|| Some(name.to_owned())),
            source,
        },
        error => EngineError::Context {
            engine,
            source_name: Some(name.to_owned()),
            source: Box::new(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentMetadata;

    fn context(name: &str) -> WriteContext {
        WriteContext::new(
            name,
            DocumentMetadata {
                title: "Genetics".into(),
                date: "2026-10-08".into(),
            },
            42,
        )
        .expect("valid context")
    }

    #[test]
    fn conversion_limits_in_input_order_and_preserves_reader_warnings() {
        let input = b"MC\tFirst\ta\tcorrect\tb\tincorrect\nINVALID\tbad\nMC\tSecond\ta\tcorrect\tb\tincorrect\n";
        let assets = MemoryAssets::new();
        let result = convert(ConversionRequest {
            input_format: "bbq_text_upload",
            output_format: "bbq_text_upload",
            input: ReadInput::File {
                name: "questions.txt",
                bytes: input,
            },
            assets: &assets,
            allow_mixed: true,
            max_items: Some(1),
            context: &context("output.txt"),
        })
        .expect("conversion");
        assert_eq!(result.item_count, 1);
        assert_eq!(result.read_warnings.len(), 1);
        let WriteArtifact::File {
            primary,
            companions,
        } = result.artifact.expect("output")
        else {
            panic!("file artifact");
        };
        assert_eq!(primary.name(), "output.txt");
        assert!(companions.is_empty());
        let text = std::str::from_utf8(primary.bytes()).expect("UTF-8");
        assert!(text.contains("First"));
        assert!(!text.contains("Second"));
    }

    #[test]
    fn zero_limit_preserves_writer_empty_output_behavior() {
        let assets = MemoryAssets::new();
        let result = convert(ConversionRequest {
            input_format: "bbq_text_upload",
            output_format: "bbq_text_upload",
            input: ReadInput::File {
                name: "questions.txt",
                bytes: b"MC\tFirst\ta\tcorrect\tb\tincorrect\n",
            },
            assets: &assets,
            allow_mixed: false,
            max_items: Some(0),
            context: &context("output.txt"),
        })
        .expect("conversion");
        assert_eq!(result.item_count, 0);
        let WriteArtifact::File {
            primary,
            companions,
        } = result.artifact.expect("empty BBQ document")
        else {
            panic!("file artifact");
        };
        assert!(primary.bytes().is_empty());
        assert!(companions.is_empty());
    }

    #[test]
    fn recovered_assets_are_authoritative_and_missing_sources_use_caller() {
        let mut recovered = MemoryAssets::new();
        recovered
            .insert("image.png", vec![1])
            .expect("recovered image");
        let mut supplied = MemoryAssets::new();
        supplied.insert("image.png", vec![2]).expect("caller image");
        supplied
            .insert("other.png", vec![3])
            .expect("caller companion");
        let assets = AssetOverlay {
            memory: &recovered,
            fallback: &supplied,
        };
        assert_eq!(assets.read("image.png").expect("recovered").as_ref(), &[1]);
        assert_eq!(assets.read("other.png").expect("supplied").as_ref(), &[3]);
        assert!(matches!(
            assets.read("missing.png"),
            Err(MediaError::MissingAsset { .. })
        ));
    }

    #[test]
    fn format_and_input_failures_retain_actionable_context() {
        assert!(
            matches!(read_bank("unknown", ReadInput::File { name: "input", bytes: &[] }, false, None),
            Err(EngineError::UnknownEngine { name }) if name == "unknown")
        );
        let error = read_bank(
            "html_selftest",
            ReadInput::File {
                name: "input",
                bytes: &[],
            },
            false,
            None,
        )
        .expect_err("writer-only format");
        let EngineError::Context {
            source_name,
            source,
            ..
        } = error
        else {
            panic!("logical input context is required");
        };
        assert_eq!(source_name.as_deref(), Some("input"));
        assert!(matches!(
            *source,
            EngineError::UnsupportedDirection {
                engine: "html_selftest",
                ..
            }
        ));
        let error = read_bank(
            "bbq_text_upload",
            ReadInput::File {
                name: "input.txt",
                bytes: &[255],
            },
            false,
            None,
        )
        .expect_err("invalid text")
        .to_string();
        assert!(error.contains("bbq_text_upload"));
        assert!(error.contains("input.txt"));
        assert!(
            WriteContext::new(
                "../escape.txt",
                DocumentMetadata {
                    title: "Title".into(),
                    date: "2026-10-08".into()
                },
                0
            )
            .is_err()
        );

        let entries = qti_core::EntryMap::new();
        let archive = ReadInput::Archive {
            name: "pool.zip",
            entries: &entries,
        };
        let error = archive
            .text("bbq_text_upload")
            .expect_err("archive is not text");
        assert!(error.to_string().contains("pool.zip"));
        assert!(std::ptr::eq(
            archive
                .archive_entries("blackboard_export_zip")
                .expect("archive"),
            &entries
        ));
        let file = ReadInput::File {
            name: "questions.txt",
            bytes: &[],
        };
        assert!(
            file.archive_entries("blackboard_export_zip")
                .expect_err("file is not archive")
                .to_string()
                .contains("questions.txt")
        );
    }

    #[test]
    fn blackboard_read_failures_retain_logical_input_and_typed_cause() {
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
            let EngineError::Context {
                engine,
                source_name,
                source,
            } = error
            else {
                panic!("logical input context is required");
            };
            assert_eq!(engine, "blackboard_export_zip");
            assert_eq!(source_name.as_deref(), Some(input.name()));
            assert!(matches!(
                *source,
                EngineError::InvalidFormat {
                    engine: "blackboard_export_zip",
                    ..
                }
            ));
        }
    }
}
