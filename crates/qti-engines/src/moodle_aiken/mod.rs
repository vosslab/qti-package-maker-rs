//! Moodle Aiken plain-text writer.
//!
//! Aiken has no image transport.  The writer therefore applies the shared
//! `PlaceholderWarn` policy at the final output boundary and writes its
//! placeholder as text, never as HTML markup (ASVS 1.1.2 and 1.2.1).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qti_core::media::{MediaPolicy, MediaWarning, apply_media_policy, replace_item_images};
use qti_core::{ItemBody, ItemKind, ItemRenderView, number_to_letter, remove_prefix_from_list};

use crate::{EngineError, EngineOptions, RenderHooks, WriteOutcome, Writer, render_bank};

/// Stable engine name used by the registry and diagnostics.
pub(crate) const NAME: &str = "moodle_aiken";
const KINDS: &[ItemKind] = &[ItemKind::Mc];

/// Creates the fixed Moodle Aiken writer for the compile-time registry.
pub fn boxed_writer(_: EngineOptions) -> Box<dyn Writer> {
    Box::new(MoodleAikenWriter)
}

struct MoodleAikenWriter;

impl Writer for MoodleAikenWriter {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> MediaPolicy {
        MediaPolicy::PlaceholderWarn
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
            .unwrap_or_else(|| Path::new("moodle-aiken.txt"))
            .to_path_buf();
        let assets = bank.collect_assets()?;
        let pending_warnings = RefCell::new(BTreeMap::<String, Vec<MediaWarning>>::new());
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &qti_core::Item| {
            let dependencies = assets.dependencies_for(item.crc()).unwrap_or_default();
            let decision = apply_media_policy(
                MediaPolicy::PlaceholderWarn,
                dependencies,
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| media_error(error.to_string()))?;
            pending_warnings
                .borrow_mut()
                .insert(item.crc().to_string(), decision.warnings);
            let placeholders = decision.placeholders;
            replace_item_images(item, |src, _alt| {
                placeholders
                    .get(src)
                    .cloned()
                    .unwrap_or_else(|| src.to_owned())
            })
            .map_err(|error| html_error(error.to_string()))
        };
        let post_render = |item: &qti_core::Item, text| {
            warnings.borrow_mut().extend(
                pending_warnings
                    .borrow_mut()
                    .remove(&item.crc().to_string())
                    .unwrap_or_default(),
            );
            Ok(text)
        };
        let items = render_bank(
            bank,
            KINDS,
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: Some(&post_render),
            },
        )?;
        if items.is_empty() {
            return Ok(WriteOutcome {
                path: None,
                warnings: warnings.into_inner(),
            });
        }
        let document = items.concat();
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

fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let ItemBody::Mc { choices, answer } = item.body() else {
        return Err(EngineError::UnsupportedItemKind {
            engine: NAME,
            kind: item.kind(),
        });
    };
    // Item construction already normalizes these prefixes. Reapplying the shared helper here
    // keeps the writer aligned with frozen Python's Aiken-specific choice rendering.
    let choices = remove_prefix_from_list(choices);

    let mut text = String::with_capacity(
        item.common().question_text.len() + choices.iter().map(String::len).sum::<usize>() + 32,
    );
    text.push_str(&item.common().question_text);
    text.push('\n');
    for (index, choice) in choices.iter().enumerate() {
        let label = number_to_letter(index + 1).map_err(|error| EngineError::InvalidFormat {
            engine: NAME,
            format: "Aiken",
            message: error.to_string(),
        })?;
        text.push(label);
        text.push_str(". ");
        text.push_str(choice);
        text.push('\n');
    }
    let answer_index = choices
        .iter()
        .position(|choice| choice == answer)
        .ok_or_else(|| EngineError::InvalidFormat {
            engine: NAME,
            format: "Aiken",
            message: "correct answer is not present among multiple-choice options".to_owned(),
        })?;
    let answer_label =
        number_to_letter(answer_index + 1).map_err(|error| EngineError::InvalidFormat {
            engine: NAME,
            format: "Aiken",
            message: error.to_string(),
        })?;
    text.push_str("ANSWER: ");
    text.push(answer_label);
    // Frozen Python appends one newline to the answer line and two more to
    // separate Aiken records, leaving three terminal line feeds per record.
    text.push_str("\n\n\n");
    Ok(Some(text))
}

fn media_error(message: String) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format: "media",
        message,
    }
}

fn html_error(message: String) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format: "HTML",
        message,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use qti_core::media::MediaAction;
    use qti_core::{Item, ItemBank, ItemBody, MediaBaseDir};

    use super::{EngineError, EngineOptions, ItemKind, boxed_writer, render_item};

    fn mc(question: &str, choices: &[&str], answer: &str) -> Item {
        Item::new(
            question.to_owned(),
            ItemBody::Mc {
                choices: choices.iter().map(|choice| (*choice).to_owned()).collect(),
                answer: answer.to_owned(),
            },
        )
        .expect("valid MC item")
    }

    #[test]
    fn renders_python_aiken_structure_and_normalized_choice_prefixes() {
        let item = mc(
            "Which letter?",
            &["A. first", "B. second", "C. third"],
            "B. second",
        );
        assert_eq!(
            render_item(&item.render_view()).expect("render"),
            Some("Which letter?\nA. first\nB. second\nC. third\nANSWER: B\n\n\n".to_owned())
        );
    }

    #[test]
    fn preserves_frozen_python_three_terminal_line_feeds() {
        let rendered = render_item(&mc("Question", &["yes", "no"], "yes").render_view())
            .expect("render")
            .expect("MC is rendered");
        assert_eq!(
            rendered.as_bytes(),
            b"Question\nA. yes\nB. no\nANSWER: A\n\n\n"
        );
    }

    #[test]
    fn choice_prefix_rendering_matches_every_frozen_python_prefix_shape() {
        let cases = [
            ("A. first", "B. second", "first", "second"),
            ("A) first", "B) second", "first", "second"),
            ("A: first", "B: second", "first", "second"),
            ("1. first", "2. second", "first", "second"),
            ("1) first", "2) second", "first", "second"),
            ("1: first", "2: second", "first", "second"),
            (
                "<p>A. first</p>",
                "<p>B. second</p>",
                "<p>first</p>",
                "<p>second</p>",
            ),
        ];
        for (first, second, expected_first, expected_second) in cases {
            let item = mc("Question", &[first, second], first);
            assert_eq!(
                render_item(&item.render_view()).expect("Aiken rendering"),
                Some(format!(
                    "Question\nA. {expected_first}\nB. {expected_second}\nANSWER: A\n\n\n"
                ))
            );
        }
    }

    #[test]
    fn skips_non_mc_items_through_the_shared_writer_contract() {
        let directory = tempfile::tempdir().expect("output directory");
        fs::write(directory.path().join("unused.png"), b"unused image").expect("unused image");
        let mut bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(directory.path()));
        bank.add_item(mc("Question", &["yes", "no"], "yes"))
            .expect("MC item");
        bank.add_item(
            Item::new(
                "Order these".to_owned(),
                ItemBody::Order {
                    answers: vec![
                        "<img src=\"unused.png\"/>one".to_owned(),
                        "two".to_owned(),
                        "three".to_owned(),
                    ],
                },
            )
            .expect("order item"),
        )
        .expect("order bank item");
        let output = directory.path().join("items.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("writer skips unsupported item as Python does");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert!(
            outcome.warnings.is_empty(),
            "unsupported items must not emit media policy warnings"
        );
        assert_eq!(
            fs::read_to_string(output).expect("Aiken output"),
            "Question\nA. yes\nB. no\nANSWER: A\n\n\n"
        );
    }

    #[test]
    fn direct_non_mc_render_has_a_typed_unsupported_kind_error() {
        let item = Item::new(
            "Order these".to_owned(),
            ItemBody::Order {
                answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            },
        )
        .expect("order item");
        assert!(matches!(
            render_item(&item.render_view()),
            Err(EngineError::UnsupportedItemKind {
                engine: "moodle_aiken",
                kind: ItemKind::Order,
            })
        ));
    }

    #[test]
    fn accepts_frozen_rdkit_canvas_and_table_markup() {
        let mut bank = ItemBank::new(false);
        // This is the minimal shape from the real corpus repro:
        // bbq-parity-corpus-38c8252b75fc0f88-moodle_aiken-questions.txt.
        // The pinned Python writer's is_valid_content returns True before its
        // dormant RDKit/table checks, so it carries this markup as Aiken text.
        let rdkit = "<script src=\"https://unpkg.com/@rdkit/rdkit/dist/RDKit_minimal.js\"></script><canvas id=\"canvas_alanine_1dd2\"></canvas><script>initRDKitModule()</script>";
        bank.add_item(mc(rdkit, &["yes", "no"], "yes"))
            .expect("item");
        let directory = tempfile::tempdir().expect("output directory");
        let output = directory.path().join("items.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("pinned Aiken behavior accepts canvas markup");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert_eq!(
            fs::read_to_string(&output).expect("Aiken output"),
            format!("{rdkit}\nA. yes\nB. no\nANSWER: A\n\n\n")
        );

        let mut table_bank = ItemBank::new(false);
        let table_item = mc(
            "<table><tr><td>cell</td></tr></table>",
            &["yes", "no"],
            "yes",
        );
        table_bank.add_item(table_item).expect("table item");
        let table_output = directory.path().join("table-items.txt");
        let table_outcome = boxed_writer(EngineOptions::default())
            .save_package(&table_bank, Some(&table_output))
            .expect("pinned Aiken behavior accepts table markup");
        assert_eq!(table_outcome.path.as_deref(), Some(table_output.as_path()));
        assert!(
            fs::read_to_string(table_output)
                .expect("Aiken table output")
                .contains("<table><tr><td>cell</td></tr></table>")
        );
    }

    #[test]
    fn substitutes_local_remote_and_data_images_as_text_with_escaped_attributes() {
        let directory = tempfile::tempdir().expect("media directory");
        fs::write(
            directory.path().join("local.png"),
            b"not decoded by placeholder policy",
        )
        .expect("local image");
        let mut bank =
            ItemBank::with_media_base_dir(false, MediaBaseDir::external(directory.path()));
        bank.add_item(mc(
            "<p><img alt=\"&lt;ignored&gt;\" src=\"local.png\"/> <img src=\"https://example.test/remote.png?x=1\" alt=\"remote\"/> <img src=\"data:image/png;base64,AA==\" alt=\"embedded\"/></p>",
            &["yes", "no"],
            "yes",
        ))
        .expect("item");
        let output = directory.path().join("items.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("Aiken output");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert_eq!(outcome.warnings.len(), 3);
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| (warning.src.as_str(), warning.action))
                .collect::<Vec<_>>(),
            vec![
                ("local.png", MediaAction::Substituted),
                (
                    "https://example.test/remote.png?x=1",
                    MediaAction::Substituted,
                ),
                ("data:image/png;base64,AA==", MediaAction::Substituted),
            ]
        );
        let text = fs::read_to_string(output).expect("Aiken output");
        assert!(text.contains("[image: local.png]"));
        assert!(text.contains("[image: remote.png]"));
        assert!(text.contains("[image: embedded image]"));
        assert!(!text.contains("<img"));
        assert!(!text.contains("ignored"));
        assert!(
            bank.get(0)
                .expect("source item")
                .common()
                .question_text
                .contains("<img")
        );
    }

    #[test]
    fn refuses_aiken_labels_beyond_z() {
        let choices = (0..27)
            .map(|index| format!("choice {index}"))
            .collect::<Vec<_>>();
        let item = Item::new(
            "Too many choices".to_owned(),
            ItemBody::Mc {
                answer: choices[0].clone(),
                choices,
            },
        )
        .expect("valid core item");
        assert!(matches!(
            render_item(&item.render_view()),
            Err(EngineError::InvalidFormat {
                engine: "moodle_aiken",
                format: "Aiken",
                ..
            })
        ));
    }

    #[test]
    fn unsupported_only_bank_has_no_output_path_or_file() {
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "Order these".to_owned(),
                ItemBody::Order {
                    answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                },
            )
            .expect("order item"),
        )
        .expect("bank item");
        let directory = tempfile::tempdir().expect("output directory");
        let output = directory.path().join("items.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("unsupported item is skipped");
        assert_eq!(outcome.path, None);
        assert!(outcome.warnings.is_empty());
        assert!(!output.exists());
    }
}
