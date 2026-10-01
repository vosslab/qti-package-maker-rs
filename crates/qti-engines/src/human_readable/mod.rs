//! Human-readable HTML writer.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qti_core::media::{MediaPolicy, apply_media_policy, replace_item_images};
use qti_core::{ItemBody, ItemKind, ItemRenderView, make_question_pretty};

use crate::{EngineError, EngineOptions, RenderHooks, WriteOutcome, Writer, render_bank};

pub(crate) const NAME: &str = "human_readable";
const KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
    ItemKind::Order,
];

/// Creates the fixed human-readable writer for the registry.
pub fn boxed_writer(_: EngineOptions) -> Box<dyn Writer> {
    Box::new(HumanReadableWriter)
}

struct HumanReadableWriter;

impl Writer for HumanReadableWriter {
    fn name(&self) -> &'static str {
        NAME
    }
    fn media_policy(&self) -> MediaPolicy {
        // The frozen engine declaration follows the Python registry.  Rendering still
        // replaces image elements with reader-facing text below, because this writer
        // has no binary output package in which to retain a reference.
        MediaPolicy::ReferenceWarn
    }
    fn supported_kinds(&self) -> &'static [ItemKind] {
        KINDS
    }

    fn save_package(
        &self,
        bank: &qti_core::ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError> {
        let output = output
            .unwrap_or_else(|| Path::new("human-readable.html"))
            .to_path_buf();
        let assets = bank.collect_assets()?;
        let pending_warnings = RefCell::new(BTreeMap::new());
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &qti_core::Item| {
            let dependencies = assets.dependencies_for(item.crc()).unwrap_or_default();
            let decision = apply_media_policy(
                MediaPolicy::PlaceholderWarn,
                dependencies,
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: error.to_string(),
            })?;
            pending_warnings
                .borrow_mut()
                .insert(item.crc().to_string(), decision.warnings);
            let by_src = dependencies
                .iter()
                .map(|asset| (asset.src.as_str(), asset))
                .collect::<BTreeMap<_, _>>();
            replace_item_images(item, |src, alt| {
                by_src.get(src).map_or_else(
                    || src.to_owned(),
                    |asset| image_description(asset, decision.placeholders.get(&asset.src), alt),
                )
            })
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "HTML",
                message: error.to_string(),
            })
        };
        let lines = render_bank(
            bank,
            KINDS,
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: Some(&|item, line| {
                    if let Some(item_warnings) = pending_warnings
                        .borrow_mut()
                        .remove(&item.crc().to_string())
                    {
                        warnings.borrow_mut().extend(item_warnings);
                    }
                    Ok(line)
                }),
            },
        )?;
        if lines.is_empty() {
            return Ok(WriteOutcome {
                path: None,
                warnings: warnings.into_inner(),
            });
        }
        let mut document = String::from(
            "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"UTF-8\">\n<style>body { background: white; color: black; } @media (prefers-color-scheme: dark) { body { background: #121212; color: #e0e0e0; } }</style>\n</head>\n<body>\n<pre>\n",
        );
        for (number, line) in lines.iter().enumerate() {
            document.push_str(&format!("{}. {line}", number + 1));
        }
        document.push_str("</pre>\n</body>\n</html>\n");
        fs::write(&output, document).map_err(|source| EngineError::Io {
            engine: NAME,
            path: output.clone(),
            source,
        })?;
        Ok(WriteOutcome {
            path: Some(output),
            warnings: warnings.into_inner(),
        })
    }
}

fn image_description(
    asset: &qti_core::media::MediaAsset,
    placeholder: Option<&String>,
    alt_text: Option<&str>,
) -> String {
    let mut result = placeholder
        .cloned()
        .unwrap_or_else(|| "[image: embedded image]".to_owned());
    if alt_text.is_some_and(|text| !text.is_empty()) {
        result.push_str(" (alt: ");
        result.push_str(alt_text.expect("checked above"));
        result.push(')');
    }
    result.push_str(" (source: ");
    result.push_str(&asset.src);
    result.push(')');
    result
}

fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    if has_unrenderable_content(item) {
        return Ok(None);
    }
    let question = pretty(&item.common().question_text);
    let question = if matches!(item.body(), ItemBody::Fib { .. }) {
        question.replace("____", "[____]")
    } else {
        question
    };
    let mut output = format!("{question}\n");
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            for (index, choice) in choices.iter().enumerate() {
                output.push_str(&format!(
                    "- [{}] {}. {}\n",
                    if choice == answer { "*" } else { " " },
                    letter(index),
                    pretty(choice)
                ));
            }
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            for (index, choice) in choices.iter().enumerate() {
                output.push_str(&format!(
                    "- [{}] {}. {}\n",
                    if answers.contains(choice) { "*" } else { " " },
                    letter(index),
                    pretty(choice)
                ));
            }
        }
        ItemBody::Match { prompts, choices } => {
            for (index, (prompt, choice)) in prompts.iter().zip(choices).enumerate() {
                output.push_str(&format!(
                    "- {}. {} / {}. {}\n",
                    index + 1,
                    pretty(prompt),
                    letter(index),
                    pretty(choice)
                ));
            }
        }
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            if *tolerance_message {
                output.push_str(&format!(
                    "(Note: Answer must be within &pm;{tolerance} of the correct value)\n"
                ));
            }
            output.push_str(&format!("- Answer: [____] (Correct: {answer:.3})\n"));
        }
        ItemBody::Fib { answers } => {
            for (index, answer) in answers.iter().enumerate() {
                output.push_str(&format!(
                    "- Answer: [{}] {}\n",
                    lower_letter(index),
                    pretty(answer)
                ));
            }
        }
        ItemBody::MultiFib { answers } => {
            for (index, (key, values)) in answers.iter().enumerate() {
                output.push_str(&format!("Blank {}. {key}:\n", index + 1));
                for (answer_index, answer) in values.iter().enumerate() {
                    output.push_str(&format!(
                        "- [{}] {}\n",
                        lower_letter(answer_index),
                        pretty(answer)
                    ));
                }
            }
        }
        ItemBody::Order { answers } => {
            for (index, answer) in answers.iter().enumerate() {
                output.push_str(&format!(
                    "- [{}] [____] (Correct: {})\n",
                    index + 1,
                    pretty(answer)
                ));
            }
        }
    }
    output.push('\n');
    Ok(Some(output))
}

/// Mirrors the frozen human-readable writer's per-kind source checks.  In particular, MC/MA
/// choices and MATCH prompts/choices can contain RDKit canvases even when the question stem does
/// not.  Such an item yields no human-readable record, and an all-skipped bank yields no file.
fn has_unrenderable_content(item: &ItemRenderView) -> bool {
    let invalid = |content: &str| {
        let content = content.to_ascii_lowercase();
        content.contains("<mathml") || content.contains("rdkit")
    };
    if invalid(&item.common().question_text) {
        return true;
    }
    match item.body() {
        ItemBody::Mc { choices, .. } | ItemBody::Ma { choices, .. } => {
            choices.iter().any(|choice| invalid(choice))
        }
        ItemBody::Match { prompts, choices } => prompts
            .iter()
            .chain(choices)
            .any(|content| invalid(content)),
        // The frozen NUM and MULTI_FIB renderers validate their question text only.
        ItemBody::Num { .. } | ItemBody::MultiFib { .. } => false,
        ItemBody::Fib { answers } | ItemBody::Order { answers } => {
            answers.iter().any(|answer| invalid(answer))
        }
    }
}

fn pretty(html: &str) -> String {
    make_question_pretty(html)
}
fn letter(index: usize) -> char {
    (b'A' + u8::try_from(index).unwrap_or(25)) as char
}
fn lower_letter(index: usize) -> char {
    (b'a' + u8::try_from(index).unwrap_or(25)) as char
}

#[cfg(test)]
mod tests {
    use super::{KINDS, boxed_writer, pretty, render_item};
    use crate::EngineOptions;
    use qti_core::{Item, ItemBank, ItemBody, MediaBaseDir};
    use std::collections::BTreeMap;
    #[test]
    fn renders_multiple_choice_with_correct_mark() {
        let item = Item::new(
            "What?".into(),
            ItemBody::Mc {
                choices: vec!["first".into(), "second".into()],
                answer: "second".into(),
            },
        )
        .expect("item");
        let rendered = render_item(&item.render_view())
            .expect("render")
            .expect("supported");
        assert!(KINDS.contains(&item.kind()));
        assert!(rendered.contains("[ ] A. first"));
        assert!(rendered.contains("[*] B. second"));
    }

    #[test]
    fn renders_nested_authored_choice_labels_once() {
        let choices = ['A', 'B']
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                format!("<table><tr><td><div>{label}. box plot {index}</div></td></tr></table>")
            })
            .collect::<Vec<_>>();
        let item = Item::new(
            "Which box plot?".into(),
            ItemBody::Mc {
                answer: choices[0].clone(),
                choices,
            },
        )
        .expect("item");

        let rendered = render_item(&item.render_view())
            .expect("render")
            .expect("supported");
        assert!(rendered.contains("- [*] A. \0"), "rendered: {rendered:?}");
        assert!(rendered.contains("- [ ] B. \0"), "rendered: {rendered:?}");
        assert!(!rendered.contains("A. \0\nA."), "rendered: {rendered:?}");
        assert!(!rendered.contains("B. \0\nB."), "rendered: {rendered:?}");
    }

    #[test]
    fn pretty_preserves_source_word_boundaries_and_ascii_table_structure() {
        let rendered = pretty(
            "<p>Which protein?</p><p>Select the best answer.</p><table border=\"0\"><tr><th>Name</th><th>Mass</th></tr><tr><td>alpha</td><td>22</td></tr></table>",
        );

        assert!(rendered.starts_with("Which protein?\nSelect the best answer."));
        assert!(
            rendered.contains("\0\nName     Mass\nalpha      22"),
            "rendered: {rendered:?}"
        );
    }

    #[test]
    fn declares_the_frozen_reference_warn_policy() {
        assert_eq!(
            boxed_writer(EngineOptions::default()).media_policy(),
            qti_core::media::MediaPolicy::ReferenceWarn
        );
    }

    #[test]
    fn renders_every_supported_kind() {
        let mut multi = BTreeMap::new();
        multi.insert("blank".to_owned(), vec!["value".to_owned()]);
        let items = vec![
            Item::new(
                "MULTIPLE".into(),
                ItemBody::Mc {
                    choices: vec!["a".into(), "b".into(), "c".into()],
                    answer: "a".into(),
                },
            ),
            Item::new(
                "MULTI ANSWER".into(),
                ItemBody::Ma {
                    choices: vec!["a".into(), "b".into(), "c".into()],
                    answers: vec!["a".into()],
                    min_answers_required: 0,
                    allow_all_correct: false,
                },
            ),
            Item::new(
                "MATCH".into(),
                ItemBody::Match {
                    prompts: vec!["p1".into(), "p2".into()],
                    choices: vec!["c1".into(), "c2".into()],
                },
            ),
            Item::new(
                "NUM".into(),
                ItemBody::Num {
                    answer: 2.0,
                    tolerance: 0.1,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "The hereditary material is ____.".into(),
                ItemBody::Fib {
                    answers: vec!["value".into()],
                },
            ),
            Item::new(
                "MULTI [blank]".into(),
                ItemBody::MultiFib { answers: multi },
            ),
            Item::new(
                "ORDER".into(),
                ItemBody::Order {
                    answers: vec!["one".into(), "two".into(), "three".into()],
                },
            ),
        ];
        for item in items {
            let item = item.expect("valid item");
            assert!(KINDS.contains(&item.kind()));
            let rendered = render_item(&item.render_view())
                .expect("render")
                .expect("content");
            if matches!(item.body(), ItemBody::Fib { .. }) {
                assert!(rendered.starts_with("The hereditary material is [____].\n"));
            }
        }
    }

    #[test]
    fn writer_substitutes_placeholder_media_without_mutating_source() {
        let directory = tempfile::tempdir().expect("media directory");
        std::fs::write(
            directory.path().join("figure.png"),
            b"not decoded by this policy",
        )
        .expect("image");
        let mut bank =
            ItemBank::with_media_base_dir(false, MediaBaseDir::external(directory.path()));
        let item = Item::new(
            "Look <img src=\"figure.png\" alt=\"plot\" />".into(),
            ItemBody::Fib {
                answers: vec!["yes".into()],
            },
        )
        .expect("item");
        bank.add_item(item).expect("bank item");
        let output = directory.path().join("human.html");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("write human page");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].src, "figure.png");
        let page = std::fs::read_to_string(output).expect("page");
        assert!(page.contains("[image: figure.png] (alt: plot) (source: figure.png)"));
        assert!(
            bank.get(0)
                .expect("source item")
                .common()
                .question_text
                .contains("<img")
        );
    }

    #[test]
    fn zero_rendered_items_creates_no_human_readable_document() {
        let directory = tempfile::tempdir().expect("output directory");
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "<mathml>not printable</mathml>".into(),
                ItemBody::Fib {
                    answers: vec!["answer".into()],
                },
            )
            .expect("item"),
        )
        .expect("bank item");
        let output = directory.path().join("skipped.html");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("skip document");
        assert_eq!(outcome.path, None);
        assert!(outcome.warnings.is_empty());
        assert!(!output.exists());
    }

    #[test]
    fn rdkit_choice_skips_a_multiple_choice_item_and_its_document() {
        let directory = tempfile::tempdir().expect("output directory");
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "Which structure is correct?".into(),
                ItemBody::Mc {
                    choices: vec![
                        "<canvas></canvas><script>RDKitModule.get_mol('C')</script>".into(),
                        "plain choice".into(),
                    ],
                    answer: "plain choice".into(),
                },
            )
            .expect("item"),
        )
        .expect("bank item");
        let output = directory.path().join("skipped.html");

        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("skip document");

        assert_eq!(outcome.path, None);
        assert!(outcome.warnings.is_empty());
        assert!(!output.exists());
    }
}
