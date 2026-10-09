//! Print-oriented exam YAML writer.
//!
//! This intentionally lossy export mirrors Python's `exam_yaml` engine.  It
//! preserves student-facing statements and choices, while deliberately omitting
//! answer keys, scores, tolerances, and ordering semantics.

use std::cell::RefCell;

use qti_core::media::{MediaPolicy, apply_media_policy};
use qti_core::{AssetSource, Item, ItemBody, ItemKind, ItemRenderView, NamedFile};
use serde::Serialize;

use crate::{
    EngineError, RenderHooks, WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "exam_yaml";

/// Creates the stateless exam writer for the registry.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(ExamYamlWriter)
}

/// Projects the bank into print-oriented YAML using the supplied document metadata.
pub struct ExamYamlWriter;

impl Writer for ExamYamlWriter {
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
        bank: &qti_core::ItemBank,
        _source: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        // Reference-only YAML classifies sources without requesting local payloads.
        // Policy warnings describe the questions emitted into the document.
        let assets = bank.inspect_assets()?;
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
            self.supported_kinds(),
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;
        if questions.is_empty() {
            return Ok(WriteOutcome {
                artifact: None,
                warnings: warnings.into_inner(),
            });
        }
        let document = ExamDocument {
            title: &context.document.title,
            date: &context.document.date,
            sections: vec![ExamSection {
                heading: &context.document.title,
                questions,
            }],
        };
        let yaml =
            serde_yaml_ng::to_string(&document).map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "YAML",
                message: error.to_string(),
            })?;
        let primary = NamedFile::new(context.output_name(), yaml.into_bytes())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary,
                companions: Vec::new(),
            }),
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
    fn context(name: &str) -> crate::WriteContext {
        crate::WriteContext::new(
            name.to_owned(),
            crate::DocumentMetadata {
                title: "Exam".to_owned(),
                date: "2026-09-30".to_owned(),
            },
            0,
        )
        .expect("valid context")
    }

    fn primary(outcome: &crate::WriteOutcome) -> &qti_core::NamedFile {
        match outcome.artifact.as_ref().expect("file artifact") {
            crate::WriteArtifact::File { primary, .. } => primary,
            crate::WriteArtifact::Directory { .. } => panic!("expected file artifact"),
        }
    }

    fn document(outcome: &crate::WriteOutcome) -> &str {
        std::str::from_utf8(primary(outcome).bytes()).expect("UTF-8 document")
    }

    use std::collections::BTreeMap;

    use super::{ExamYamlWriter, NAME, Writer, render_item};
    use crate::DocumentMetadata;
    use qti_core::MemoryAssets;
    use qti_core::media::MediaPolicy;
    use qti_core::{Item, ItemBank, ItemBody};

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

        let write_context = crate::WriteContext::new(
            "exam.yaml".to_owned(),
            DocumentMetadata {
                title: "Biology 301".to_owned(),
                date: "2026-09-30".to_owned(),
            },
            0,
        )
        .expect("context");
        let outcome = super::boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &write_context)
            .expect("write YAML");
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(document(&outcome)).expect("parse YAML");
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
        assert!(!document(&outcome).contains("tolerance"));
    }

    #[test]
    fn keeps_local_remote_and_data_image_sources_verbatim_after_validation() {
        let mut bank = ItemBank::new(true);
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
        let output = "media.yaml";
        let outcome = ExamYamlWriter
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("reference policy keeps all sources");
        assert_eq!(primary(&outcome).name(), output);
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
        let written = document(&outcome);
        assert!(written.contains("figure.png"));
        assert!(written.contains("https://example.test/remote.png"));
        assert!(written.contains("data:image/png;base64,iVBORw0KGgo="));
    }

    #[test]
    fn validates_reference_media_in_every_projected_and_lossy_item_field() {
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
        let mut bank = ItemBank::new(true);
        for (statement, body) in item_bodies {
            bank.add_item(Item::new(statement, body).expect("valid item"))
                .expect("add item");
        }

        let assets = bank.inspect_assets().expect("reference assets");
        assert_eq!(assets.assets().len(), 3);
        for item in bank.iter_ordered() {
            assert!(
                assets.dependencies_for(item.crc()).is_some(),
                "{} should have a validated reference image",
                item.crc()
            );
        }
        let output = "all-fields.yaml";
        let outcome = ExamYamlWriter
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("reference writer handles all kinds");
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(document(&outcome)).expect("parse YAML");
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
    fn declares_reference_warn_policy() {
        let writer = ExamYamlWriter;
        assert_eq!(writer.name(), NAME);
        assert_eq!(writer.media_policy(), MediaPolicy::ReferenceWarn);
    }

    #[test]
    fn skips_output_for_an_empty_exam() {
        let output = "empty.yaml";
        let outcome = ExamYamlWriter
            .write_package(&ItemBank::new(true), &MemoryAssets::new(), &context(output))
            .expect("empty exam is a valid no-output result");

        assert!(outcome.artifact.is_none());
        assert!(outcome.warnings.is_empty());
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
