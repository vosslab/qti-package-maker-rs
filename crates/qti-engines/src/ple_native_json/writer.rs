//! Object-safe writer adapter for the PLE Native JSON directory export.

use qti_core::media::{AssetSource, MediaPolicy};
use qti_core::{EntryMap, ItemBank, ItemKind};

use super::export_bank;
use crate::{EngineError, WriteArtifact, WriteContext, WriteOutcome, Writer};

/// Creates the registry writer; document values are supplied at invocation.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(PleNativeJsonWriter)
}

struct PleNativeJsonWriter;

impl Writer for PleNativeJsonWriter {
    fn name(&self) -> &'static str {
        "ple_native_json"
    }

    fn media_policy(&self) -> MediaPolicy {
        crate::engine(self.name())
            .expect("registered PLE engine")
            .media_policy
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(self.name())
            .expect("registered PLE engine")
            .supported_kinds
    }

    fn write_package(
        &self,
        bank: &ItemBank,
        assets: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        let export = export_bank(bank, assets)?;
        if export.questions.is_empty() {
            return Ok(WriteOutcome {
                artifact: None,
                warnings: export.warnings,
            });
        }
        let mut entries = EntryMap::new();
        for question in export.questions {
            entries.insert(
                format!("item_{:05}.json", question.item_number),
                question.source_json.into_bytes(),
            );
            for file in question.files {
                let name = file.name().to_owned();
                match entries.entry(name.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(file.into_parts().1);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get().as_slice() != file.bytes() =>
                    {
                        return Err(EngineError::InvalidFormat {
                            engine: self.name(),
                            format: "PLE Native JSON",
                            message: format!("associated file {name:?} has conflicting payloads"),
                        });
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {}
                }
            }
        }
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::Directory {
                name: context.output_name().to_owned(),
                entries,
            }),
            warnings: export.warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use qti_core::media::MemoryAssets;
    use qti_core::{Item, ItemBank, ItemBody};

    use super::boxed_writer;
    use crate::{DocumentMetadata, WriteArtifact, WriteContext};

    fn context() -> WriteContext {
        WriteContext::new(
            "exports/course.ple",
            DocumentMetadata {
                title: "Course".into(),
                date: "2026-10-08".into(),
            },
            0,
        )
        .expect("context")
    }

    #[test]
    fn directory_contains_exact_question_documents_and_shared_media_without_host_manifest() {
        let mut assets = MemoryAssets::new();
        assets
            .insert("figure.png", b"figure bytes".to_vec())
            .expect("asset");
        let mut bank = ItemBank::new(true);
        for prompt in [
            "First <img src='figure.png'/>",
            "Second <img src='figure.png'/>",
        ] {
            bank.add_item(
                Item::new(
                    prompt.into(),
                    ItemBody::Fib {
                        answers: vec!["yes".into()],
                    },
                )
                .expect("item"),
            )
            .expect("add");
        }
        let export = super::export_bank(&bank, &assets).expect("export");
        let result = boxed_writer()
            .write_package(&bank, &assets, &context())
            .expect("write");
        let Some(WriteArtifact::Directory { name, entries }) = result.artifact else {
            panic!("PLE directory");
        };
        assert_eq!(name, "exports/course.ple");
        assert_eq!(
            entries.keys().map(String::as_str).collect::<Vec<_>>(),
            ["item_00001.json", "item_00002.json", "media/figure.png"]
        );
        for (index, question) in export.questions.iter().enumerate() {
            assert_eq!(
                entries[&format!("item_{:05}.json", index + 1)],
                question.source_json.as_bytes()
            );
        }
        assert_eq!(entries["media/figure.png"], b"figure bytes");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn empty_bank_returns_no_artifact() {
        let result = boxed_writer()
            .write_package(&ItemBank::new(true), &MemoryAssets::new(), &context())
            .expect("empty output");
        assert!(result.artifact.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn directory_emits_one_payload_for_large_shared_image_fanout() {
        let payload = (0..1024 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let mut assets = MemoryAssets::new();
        assets.insert("shared.png", payload.clone()).expect("image");
        let mut bank = ItemBank::new(true);
        for index in 0..160 {
            bank.add_item(
                Item::new(
                    format!("Question {index} <img src='shared.png'/>"),
                    ItemBody::Fib {
                        answers: vec!["yes".into()],
                    },
                )
                .expect("item"),
            )
            .expect("unique item");
        }
        let result = boxed_writer()
            .write_package(&bank, &assets, &context())
            .expect("write");
        let Some(WriteArtifact::Directory { entries, .. }) = result.artifact else {
            panic!("PLE directory");
        };
        assert_eq!(entries.len(), 161);
        assert_eq!(entries["media/shared.png"], payload);
        assert_eq!(
            entries
                .keys()
                .filter(|name| name.starts_with("media/"))
                .count(),
            1
        );
    }
}
