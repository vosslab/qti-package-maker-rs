//! Print-oriented exam YAML writer.
//!
//! This intentionally lossy export mirrors Python's `exam_yaml` engine.  It
//! preserves student-facing statements and choices, while deliberately omitting
//! answer keys, scores, tolerances, and ordering semantics.

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use qti_core::media::{MediaPolicy, apply_media_policy};
use qti_core::{Item, ItemBody, ItemKind, ItemRenderView};
use serde::Serialize;

use crate::{EngineError, EngineOptions, RenderHooks, WriteOutcome, Writer, render_bank};

pub(crate) const NAME: &str = "exam_yaml";
const KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
    ItemKind::Order,
];

/// Creates the registry writer from conversion-run document metadata.
pub fn boxed_writer(options: EngineOptions) -> Box<dyn Writer> {
    Box::new(ExamYamlWriter::new(
        options.document.title,
        options.document.date,
    ))
}

/// A writer whose title and date are fixed when it is constructed.
///
/// Owning these strings keeps the object-safe [`Writer`] boundary independent
/// of borrowed CLI arguments.  The registry uses the conventional `exam`
/// title; callers needing a course-specific document can construct this type.
pub struct ExamYamlWriter {
    title: String,
    date: String,
}

impl ExamYamlWriter {
    /// Creates a writer with explicit document metadata.
    #[must_use]
    pub fn new(title: impl Into<String>, date: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            date: date.into(),
        }
    }
}

impl Writer for ExamYamlWriter {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> MediaPolicy {
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
        // Resolve before rendering or opening the output.  ReferenceWarn keeps
        // each authored src unchanged, but still rejects invalid local sources
        // through the core resolver.  The render hook records warnings only
        // for questions that actually appear in the completed document.
        let assets = bank.collect_assets()?;
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &Item| {
            let decision = apply_media_policy(
                MediaPolicy::ReferenceWarn,
                assets.dependencies_for(item.crc()).unwrap_or_default(),
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: error.to_string(),
            })?;
            warnings.borrow_mut().extend(decision.warnings);
            Ok(item.render_view())
        };

        let questions = render_bank(
            bank,
            KINDS,
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;
        if questions.is_empty() {
            return Ok(WriteOutcome {
                path: None,
                warnings: warnings.into_inner(),
            });
        }
        let output = output
            .unwrap_or_else(|| Path::new("exam.yaml"))
            .to_path_buf();
        let document = ExamDocument {
            title: &self.title,
            date: &self.date,
            sections: vec![ExamSection {
                heading: &self.title,
                questions,
            }],
        };
        let yaml =
            serde_yaml_ng::to_string(&document).map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "YAML",
                message: error.to_string(),
            })?;
        fs::write(&output, yaml).map_err(|source| EngineError::Io {
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

#[derive(Serialize)]
struct ExamDocument<'a> {
    title: &'a str,
    date: &'a str,
    sections: Vec<ExamSection<'a>>,
}

#[derive(Serialize)]
struct ExamSection<'a> {
    heading: &'a str,
    questions: Vec<ExamQuestion>,
}

/// The precise seven-shape projection accepted by downstream exam tooling.
///
/// This is deliberately separate from `ItemBody`: serializing `ItemBody`
/// would leak answer-bearing fields that Python's print-oriented export drops.
#[derive(Serialize)]
#[serde(untagged)]
enum ExamQuestion {
    Choices {
        statement: String,
        choices: Vec<String>,
    },
    Match {
        statement: String,
        table: MatchTable,
    },
    Statement {
        statement: String,
    },
}

#[derive(Serialize)]
struct MatchTable {
    columns: [&'static str; 2],
    rows: Vec<[String; 2]>,
}

fn render_item(item: &ItemRenderView) -> Result<Option<ExamQuestion>, EngineError> {
    let statement = item.common().question_text.clone();
    let question = match item.body() {
        ItemBody::Mc { choices, .. } | ItemBody::Ma { choices, .. } => ExamQuestion::Choices {
            statement,
            choices: choices.clone(),
        },
        ItemBody::Match { prompts, choices } => ExamQuestion::Match {
            statement,
            table: MatchTable {
                columns: ["Prompt", "Answer"],
                rows: prompts
                    .iter()
                    .zip(choices)
                    .map(|(prompt, choice)| [prompt.clone(), choice.clone()])
                    .collect(),
            },
        },
        ItemBody::Num { .. } | ItemBody::Fib { .. } | ItemBody::MultiFib { .. } => {
            ExamQuestion::Statement { statement }
        }
        ItemBody::Order { answers } => ExamQuestion::Choices {
            statement,
            choices: answers.clone(),
        },
    };
    Ok(Some(question))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{ExamYamlWriter, KINDS, NAME, Writer, render_item};
    use crate::{DocumentMetadata, EngineOptions};
    use qti_core::media::MediaPolicy;
    use qti_core::{Item, ItemBank, ItemBody, MediaBaseDir};
    use tempfile::tempdir;

    #[test]
    fn projects_all_seven_item_shapes_without_answer_fields() {
        let mut multi = BTreeMap::new();
        multi.insert(
            "blank_one".to_owned(),
            vec!["alpha".to_owned(), "beta".to_owned()],
        );
        let items = vec![
            Item::new(
                "MC stem".to_owned(),
                ItemBody::Mc {
                    choices: vec![
                        "A. first".to_owned(),
                        "B. second".to_owned(),
                        "C. third".to_owned(),
                    ],
                    answer: "A. first".to_owned(),
                },
            ),
            Item::new(
                "MA stem".to_owned(),
                ItemBody::Ma {
                    choices: vec![
                        "A. one".to_owned(),
                        "B. two".to_owned(),
                        "C. three".to_owned(),
                    ],
                    answers: vec!["A. one".to_owned()],
                    min_answers_required: 1,
                    allow_all_correct: false,
                },
            ),
            Item::new(
                "MATCH stem".to_owned(),
                ItemBody::Match {
                    prompts: vec!["prompt 1".to_owned(), "prompt 2".to_owned()],
                    choices: vec!["choice 1".to_owned(), "choice 2".to_owned()],
                },
            ),
            Item::new(
                "NUM stem".to_owned(),
                ItemBody::Num {
                    answer: 9.5,
                    tolerance: 0.2,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "FIB stem".to_owned(),
                ItemBody::Fib {
                    answers: vec!["answer".to_owned()],
                },
            ),
            Item::new(
                "MULTI_FIB [blank_one] stem".to_owned(),
                ItemBody::MultiFib { answers: multi },
            ),
            Item::new(
                "ORDER stem".to_owned(),
                ItemBody::Order {
                    answers: vec![
                        "A. first".to_owned(),
                        "B. second".to_owned(),
                        "C. third".to_owned(),
                    ],
                },
            ),
        ];
        let mut bank = ItemBank::new(true);
        for item in items {
            bank.add_item(item.expect("valid item")).expect("add item");
        }

        let temporary = tempdir().expect("temporary output");
        let path = temporary.path().join("exam.yaml");
        super::boxed_writer(EngineOptions {
            html_to_image: false,
            document: DocumentMetadata {
                title: "Biology 301".to_owned(),
                date: "2026-09-30".to_owned(),
            },
        })
        .save_package(&bank, Some(&path))
        .expect("write YAML");
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&std::fs::read_to_string(path).expect("read YAML"))
                .expect("parse YAML");
        let questions = value["sections"][0]["questions"]
            .as_sequence()
            .expect("questions");

        assert_eq!(questions.len(), 7);
        assert_eq!(value["title"].as_str(), Some("Biology 301"));
        assert_eq!(value["date"].as_str(), Some("2026-09-30"));
        assert_eq!(
            questions[0]["choices"].as_sequence().expect("MC choices")[0],
            "first"
        );
        assert_eq!(
            questions[1]["choices"].as_sequence().expect("MA choices")[1],
            "two"
        );
        assert_eq!(questions[2]["table"]["columns"][0], "Prompt");
        assert_eq!(questions[2]["table"]["rows"][1][1], "choice 2");
        for index in [3, 4, 5] {
            assert_eq!(questions[index].as_mapping().expect("question").len(), 1);
        }
        assert_eq!(questions[6]["choices"][0], "first");
        assert!(
            !std::fs::read_to_string(temporary.path().join("exam.yaml"))
                .expect("read YAML")
                .contains("tolerance")
        );
    }

    #[test]
    fn keeps_local_remote_and_data_image_sources_verbatim_after_validation() {
        let temporary = tempdir().expect("temporary media root");
        let image = temporary.path().join("figure.png");
        std::fs::write(
            &image,
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248,
                207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68,
                174, 66, 96, 130,
            ],
        )
        .expect("image bytes");
        let mut bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(temporary.path()));
        let statement = concat!(
            "<p>Images</p><img src=\"figure.png\" alt=\"local\"/>",
            "<img src=\"https://example.test/remote.png\" alt=\"remote\"/>",
            "<img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"data\"/>"
        );
        bank.add_item(
            Item::new(
                statement.to_owned(),
                ItemBody::Mc {
                    choices: vec!["A. correct".to_owned(), "B. other".to_owned()],
                    answer: "A. correct".to_owned(),
                },
            )
            .expect("item"),
        )
        .expect("add item");
        let output = temporary.path().join("media.yaml");
        let outcome = ExamYamlWriter::new("Media", "2026-09-30")
            .save_package(&bank, Some(&output))
            .expect("reference policy keeps all sources");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| warning.src.as_str())
                .collect::<Vec<_>>(),
            [
                "figure.png",
                "https://example.test/remote.png",
                "data:image/png;base64,iVBORw0KGgo=",
            ]
        );
        let written = std::fs::read_to_string(output).expect("YAML");
        assert!(written.contains("figure.png"));
        assert!(written.contains("https://example.test/remote.png"));
        assert!(written.contains("data:image/png;base64,iVBORw0KGgo="));
    }

    #[test]
    fn validates_reference_media_in_every_projected_and_lossy_item_field() {
        let temporary = tempdir().expect("temporary media root");
        std::fs::write(
            temporary.path().join("local.png"),
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248,
                207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68,
                174, 66, 96, 130,
            ],
        )
        .expect("local image");
        let local = "<img src=\"local.png\" alt=\"local\"/>";
        let remote = "<img src=\"https://example.test/remote.png\" alt=\"remote\"/>";
        let data = "<img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"data\"/>";
        let mut multi = BTreeMap::new();
        multi.insert("blank".to_owned(), vec![local.to_owned()]);
        let item_bodies = vec![
            (
                format!("MC statement {remote}"),
                ItemBody::Mc {
                    choices: vec![local.to_owned(), "second".to_owned()],
                    answer: local.to_owned(),
                },
            ),
            (
                format!("MA statement {local}"),
                ItemBody::Ma {
                    choices: vec![data.to_owned(), "second".to_owned(), "third".to_owned()],
                    answers: vec![data.to_owned()],
                    min_answers_required: 1,
                    allow_all_correct: false,
                },
            ),
            (
                format!("MATCH statement {data}"),
                ItemBody::Match {
                    prompts: vec![remote.to_owned(), "prompt two".to_owned()],
                    choices: vec![local.to_owned(), "choice two".to_owned()],
                },
            ),
            (
                format!("NUM statement {remote}"),
                ItemBody::Num {
                    answer: 3.0,
                    tolerance: 0.0,
                    tolerance_message: false,
                },
            ),
            (
                "FIB statement".to_owned(),
                ItemBody::Fib {
                    answers: vec![data.to_owned()],
                },
            ),
            (
                "MULTI_FIB [blank] statement".to_owned(),
                ItemBody::MultiFib { answers: multi },
            ),
            (
                format!("ORDER statement {local}"),
                ItemBody::Order {
                    answers: vec![remote.to_owned(), "second".to_owned(), "third".to_owned()],
                },
            ),
        ];
        let mut bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(temporary.path()));
        for (statement, body) in item_bodies {
            bank.add_item(Item::new(statement, body).expect("valid item"))
                .expect("add item");
        }

        let assets = bank.collect_assets().expect("reference assets");
        assert_eq!(assets.assets().len(), 3);
        for item in bank.iter_ordered() {
            assert!(
                assets.dependencies_for(item.crc()).is_some(),
                "{} should have a validated reference image",
                item.crc()
            );
        }
        let output = temporary.path().join("all-fields.yaml");
        ExamYamlWriter::new("Media fields", "2026-09-30")
            .save_package(&bank, Some(&output))
            .expect("reference writer handles all kinds");
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&std::fs::read_to_string(output).expect("YAML"))
                .expect("parse YAML");
        let questions = value["sections"][0]["questions"]
            .as_sequence()
            .expect("questions");
        assert!(
            questions[0]["choices"][0]
                .as_str()
                .expect("MC choice")
                .contains("local.png")
        );
        assert!(
            questions[1]["choices"][0]
                .as_str()
                .expect("MA choice")
                .contains("data:image/png")
        );
        assert!(
            questions[2]["table"]["rows"][0][0]
                .as_str()
                .expect("MATCH prompt")
                .contains("https://example.test/remote.png")
        );
        assert!(questions[4].get("choices").is_none());
        assert!(questions[5].get("choices").is_none());
        assert!(
            questions[6]["choices"][0]
                .as_str()
                .expect("ORDER choice")
                .contains("https://example.test/remote.png")
        );
    }

    #[test]
    fn declares_reference_warn_and_supports_every_python_kind() {
        let writer = ExamYamlWriter::new("Exam", "2026-09-30");
        assert_eq!(writer.name(), NAME);
        assert_eq!(writer.media_policy(), MediaPolicy::ReferenceWarn);
        assert_eq!(writer.supported_kinds(), KINDS);
    }

    #[test]
    fn skips_output_for_an_empty_exam() {
        let temporary = tempdir().expect("temporary output");
        let output = temporary.path().join("empty.yaml");
        let outcome = ExamYamlWriter::new("Empty", "2026-09-30")
            .save_package(&ItemBank::new(true), Some(&output))
            .expect("empty exam is a valid no-output result");

        assert_eq!(outcome.path, None);
        assert!(outcome.warnings.is_empty());
        assert!(!output.exists());
    }

    #[test]
    fn render_item_uses_normalized_student_facing_values() {
        let item = Item::new(
            "Stem".to_owned(),
            ItemBody::Mc {
                choices: vec!["A. first".to_owned(), "B. second".to_owned()],
                answer: "A. first".to_owned(),
            },
        )
        .expect("item");
        let encoded = serde_yaml_ng::to_string(
            &render_item(&item.render_view())
                .expect("render")
                .expect("question"),
        )
        .expect("serialize question");
        assert!(encoded.contains("first"));
        assert!(!encoded.contains("A. first"));
    }
}
