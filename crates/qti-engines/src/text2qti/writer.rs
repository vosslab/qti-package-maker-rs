//! Text output and item rendering for text2qti.

use std::cell::RefCell;

use qti_core::media::MediaPolicy;
use qti_core::{AssetSource, Item, ItemBank, ItemBody, ItemKind, ItemRenderView, NamedFile};

use super::NAME;
use super::media::{local_copies, markdown_images, markdown_media};
use crate::{
    EngineError, RenderHooks, WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(super) struct Text2QtiWriter;

impl Writer for Text2QtiWriter {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> MediaPolicy {
        crate::engine(NAME).expect("registered engine").media_policy
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(NAME)
            .expect("registered engine")
            .supported_kinds
    }

    fn write_package(
        &self,
        bank: &ItemBank,
        source: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        ensure_supported(bank, self.supported_kinds())?;
        let collected = bank.collect_assets(source)?;
        let media = markdown_media(&collected, bank)?;
        let rendered_warnings = RefCell::new(Vec::new());
        let rendered = {
            let post_render = |item: &Item, text: String| {
                // `post_render` runs only for an item that emitted output, so this keeps
                // policy diagnostics in the same order as the completed document.
                rendered_warnings.borrow_mut().extend(
                    media
                        .warnings
                        .get(item.crc())
                        .into_iter()
                        .flatten()
                        .cloned(),
                );
                let by_source = media.targets.get(item.crc()).cloned().unwrap_or_default();
                markdown_images(&text, &by_source)
            };
            render_bank(
                bank,
                self.supported_kinds(),
                render_item,
                RenderHooks {
                    pre_render: None,
                    post_render: Some(&post_render),
                },
            )?
        };
        let text = rendered.join("\n");
        let companions = local_copies(&collected)?
            .into_iter()
            .map(|(name, bytes)| NamedFile::new(format!("media/{name}"), bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let primary = NamedFile::new(context.output_name(), text.into_bytes())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary,
                companions,
            }),
            warnings: rendered_warnings.into_inner(),
        })
    }
}

fn ensure_supported(bank: &ItemBank, kinds: &[ItemKind]) -> Result<(), EngineError> {
    bank.iter_ordered()
        .find(|item| !kinds.contains(&item.kind()))
        .map_or(Ok(()), |item| {
            Err(EngineError::UnsupportedItemKind {
                engine: NAME,
                kind: item.kind(),
            })
        })
}
fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let mut lines = vec![format!(
        "{}. {}",
        item.common().item_number,
        item.common().question_text
    )];
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            for (index, choice) in choices.iter().enumerate() {
                let marker = if choice == answer { "*" } else { "" };
                lines.push(format!("{marker}{}) {choice}", letter(index)?));
            }
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            for choice in choices {
                let marker = if answers.contains(choice) {
                    "[*]"
                } else {
                    "[ ]"
                };
                lines.push(format!("{marker} {choice}"));
            }
        }
        ItemBody::Num {
            answer, tolerance, ..
        } => lines.push(format!("= {answer} +- {tolerance}")),
        ItemBody::Fib { answers } => {
            lines.extend(answers.iter().map(|answer| format!("* {answer}")));
        }
        ItemBody::Match { .. } | ItemBody::MultiFib { .. } | ItemBody::Order { .. } => {
            return Err(EngineError::UnsupportedItemKind {
                engine: NAME,
                kind: item.kind(),
            });
        }
    }
    Ok(Some(format!("{}\n", lines.join("\n"))))
}

fn letter(index: usize) -> Result<char, EngineError> {
    u8::try_from(index)
        .ok()
        .and_then(|index| b'A'.checked_add(index))
        .filter(u8::is_ascii_uppercase)
        .map(char::from)
        .ok_or_else(|| EngineError::InvalidFormat {
            engine: NAME,
            format: "text2qti",
            message: "MC supports at most 26 choices".to_owned(),
        })
}
