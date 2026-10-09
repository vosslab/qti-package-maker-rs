//! Blackboard BBQ text reader and writer.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qti_core::media::{MediaPolicy, apply_media_policy};
use qti_core::{AssetSource, Item, ItemBank, ItemBody, ItemKind, MemoryAssets, NamedFile};

use crate::{
    EngineError, ReadInput, ReadLocation, ReadOutcome, ReadWarning, Reader, RenderHooks,
    WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "bbq_text_upload";

/// Creates the writer for the fixed registry entry.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(BbqWriter)
}
/// Creates the reader for the fixed registry entry.
pub fn boxed_reader() -> Box<dyn Reader> {
    Box::new(BbqReader)
}

struct BbqWriter;
impl Writer for BbqWriter {
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
        _source: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        // Reference-only output classifies authored sources without loading local payloads.
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
        let lines = render_bank(
            bank,
            self.supported_kinds(),
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;
        let primary = NamedFile::new(context.output_name(), lines.concat().into_bytes())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary,
                companions: Vec::new(),
            }),
            warnings: warnings.into_inner(),
        })
    }
}

struct BbqReader;
impl Reader for BbqReader {
    fn name(&self) -> &'static str {
        NAME
    }
    fn read_items(
        &self,
        input: ReadInput<'_>,
        allow_mixed: bool,
    ) -> Result<ReadOutcome, EngineError> {
        let text = input.text(NAME)?;
        let mut bank = ItemBank::new(allow_mixed);
        let mut warnings = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            if line.trim().is_empty() {
                warnings.push(ReadWarning {
                    location: ReadLocation::Line { line: line_number },
                    message: "blank line skipped".to_owned(),
                });
                continue;
            }
            match parse_line(line) {
                Ok(item) => match bank.add_item(item) {
                    Ok(_) => {}
                    Err(error) => warnings.push(ReadWarning {
                        location: ReadLocation::Line { line: line_number },
                        message: format!("line skipped: {error}"),
                    }),
                },
                Err(message) => warnings.push(ReadWarning {
                    location: ReadLocation::Line { line: line_number },
                    message: format!("line skipped: {message}"),
                }),
            }
        }
        Ok(ReadOutcome {
            bank,
            assets: MemoryAssets::new(),
            warnings,
        })
    }
}

fn render_item(item: &qti_core::ItemRenderView) -> Result<Option<String>, EngineError> {
    let question = format!(
        "<p>{}</p> {}",
        item.crc(),
        clean(&item.common().question_text)
    );
    let line = match item.body() {
        ItemBody::Mc { choices, answer } => pair_line("MC", question, choices, |choice| {
            if choice == answer {
                "Correct".to_owned()
            } else {
                "Incorrect".to_owned()
            }
        }),
        ItemBody::Ma {
            choices, answers, ..
        } => pair_line("MA", question, choices, |choice| {
            if answers.contains(choice) {
                "Correct".to_owned()
            } else {
                "Incorrect".to_owned()
            }
        }),
        ItemBody::Match { prompts, choices } => {
            let mut fields = vec!["MAT".to_owned(), question];
            for (prompt, choice) in prompts.iter().zip(choices) {
                fields.push(clean(prompt));
                fields.push(clean(choice));
            }
            fields.join("\t")
        }
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            let mut prompt = question;
            if *tolerance_message {
                prompt.push_str(&numeric_note(*answer, *tolerance));
            }
            format!("NUM\t{prompt}\t{answer:.8}\t{tolerance:.8}")
        }
        ItemBody::Fib { answers } => join_answers("FIB", question, answers),
        ItemBody::MultiFib { answers } => {
            let mut fields = vec!["FIB_PLUS".to_owned(), question];
            for (key, values) in answers {
                fields.push(key.clone());
                fields.extend(values.iter().map(|answer| clean(answer)));
                fields.push(String::new());
            }
            fields.join("\t")
        }
        ItemBody::Order { answers } => join_answers("ORD", question, answers),
    };
    Ok(Some(format!("{line}\n")))
}

fn pair_line(
    header: &str,
    question: String,
    choices: &[String],
    status: impl Fn(&String) -> String,
) -> String {
    let mut fields = vec![header.to_owned(), question];
    for (index, choice) in choices.iter().enumerate() {
        fields.push(insert_choice_prefix(&clean(choice), index));
        fields.push(status(choice));
    }
    fields.join("\t")
}
fn join_answers(header: &str, question: String, answers: &[String]) -> String {
    let mut fields = vec![header.to_owned(), question];
    fields.extend(answers.iter().map(|answer| clean(answer)));
    fields.join("\t")
}
fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn insert_choice_prefix(value: &str, index: usize) -> String {
    let prefix = char::from(b'A' + u8::try_from(index).unwrap_or(25));
    if let Some(position) = value.to_ascii_lowercase().find("<div")
        && let Some(end) = value[position..].find('>')
    {
        let end = position + end + 1;
        return format!("{}{}{}. {}", &value[..end], "", prefix, &value[end..]);
    }
    format!("{prefix}. {value}")
}
fn numeric_note(answer: f64, tolerance: f64) -> String {
    if answer == 0.0 {
        format!(
            "<p><i>Note: answers need to be within {tolerance:.8} of the correct number to be correct.</i></p>"
        )
    } else {
        format!(
            "<p><i>Note: answers need to be within {}&percnt; of the correct number to be correct.</i></p>",
            (tolerance / answer * 100.0).ceil()
        )
    }
}

fn parse_line(line: &str) -> Result<Item, String> {
    let parts = line.trim().split('\t').collect::<Vec<_>>();
    let header = parts.first().copied().unwrap_or_default().trim();
    if parts.len() < 2 {
        return Err("missing question text".to_owned());
    }
    let question = strip_written_crc(parts[1]);
    let body = match header {
        "MC" | "MA" => {
            if parts.len() < 4 || (parts.len() - 2) % 2 != 0 {
                return Err("choice/status fields must occur in pairs".to_owned());
            }
            let (choice_pairs, []) = parts[2..].as_chunks::<2>() else {
                return Err("choice/status fields must occur in pairs".to_owned());
            };
            let choices = choice_pairs
                .iter()
                .map(|part| strip_choice_prefix(part[0]))
                .collect::<Vec<_>>();
            let correct = parts[3..]
                .iter()
                .step_by(2)
                .enumerate()
                .filter_map(|(index, status)| {
                    status
                        .trim()
                        .eq_ignore_ascii_case("correct")
                        .then_some(choices[index].clone())
                })
                .collect::<Vec<_>>();
            if header == "MC" {
                let answer = correct
                    .first()
                    .cloned()
                    .ok_or_else(|| "MC has no Correct choice".to_owned())?;
                ItemBody::Mc { choices, answer }
            } else {
                ItemBody::Ma {
                    choices,
                    answers: correct,
                    // BBQ carries choices and correct statuses, but no MA grading options.
                    // Match the frozen Python MA constructor for the omitted fields.
                    min_answers_required: 1,
                    allow_all_correct: true,
                }
            }
        }
        "MAT" => {
            if (parts.len() - 2) % 2 != 0 {
                return Err("matching fields must occur in pairs".to_owned());
            }
            let (pairs, []) = parts[2..].as_chunks::<2>() else {
                return Err("matching fields must occur in pairs".to_owned());
            };
            ItemBody::Match {
                prompts: pairs.iter().map(|pair| pair[0].trim().to_owned()).collect(),
                choices: pairs.iter().map(|pair| pair[1].trim().to_owned()).collect(),
            }
        }
        "NUM" => {
            let answer = parts
                .get(2)
                .ok_or_else(|| "NUM lacks answer".to_owned())?
                .trim()
                .parse::<f64>()
                .map_err(|_| "invalid NUM answer".to_owned())?;
            let tolerance = parts
                .get(3)
                .filter(|value| !value.trim().is_empty())
                .map_or(Ok(0.0), |value| {
                    value
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "invalid NUM tolerance".to_owned())
                })?;
            ItemBody::Num {
                answer,
                tolerance,
                tolerance_message: true,
            }
        }
        "FIB" => ItemBody::Fib {
            answers: parts[2..]
                .iter()
                .map(|value| value.trim().to_owned())
                .collect(),
        },
        "ORD" => ItemBody::Order {
            answers: parts[2..]
                .iter()
                .map(|value| value.trim().to_owned())
                .collect(),
        },
        "FIB_PLUS" => ItemBody::MultiFib {
            answers: parse_multi_fib(&parts[2..])?,
        },
        _ => return Err(format!("unsupported question type '{header}'")),
    };
    Item::new(question, body).map_err(|error| error.to_string())
}

fn parse_multi_fib(fields: &[&str]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut values = BTreeMap::new();
    let mut index = 0;
    while index < fields.len() {
        if fields[index].trim().is_empty() {
            index += 1;
            continue;
        }
        let key = fields[index].trim().to_owned();
        index += 1;
        let start = index;
        while index < fields.len() && !fields[index].trim().is_empty() {
            index += 1;
        }
        if start == index {
            return Err(format!("FIB_PLUS blank '{key}' has no answers"));
        }
        values.insert(
            key,
            fields[start..index]
                .iter()
                .map(|value| value.trim().to_owned())
                .collect(),
        );
    }
    Ok(values)
}
fn strip_written_crc(question: &str) -> String {
    let trimmed = question.trim();
    if let Some(rest) = trimmed.strip_prefix("<p>")
        && let Some(close) = rest.find("</p>")
    {
        let candidate = &rest[..close];
        if candidate.len() == 9
            && candidate.as_bytes().get(4) == Some(&b'_')
            && candidate
                .chars()
                .all(|value| value == '_' || value.is_ascii_hexdigit())
        {
            return rest[close + 4..].trim().to_owned();
        }
    }
    trimmed.to_owned()
}

fn strip_choice_prefix(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 3
        && value.as_bytes()[0].is_ascii_uppercase()
        && value.as_bytes()[1] == b'.'
        && value.as_bytes()[2] == b' '
    {
        return value[3..].to_owned();
    }
    value.to_owned()
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

    use super::{boxed_reader, boxed_writer};
    use super::{parse_line, render_item};
    use qti_core::MemoryAssets;
    use qti_core::{Item, ItemBank, ItemBody, ItemFingerprint};
    use std::collections::BTreeMap;
    #[test]
    fn bbq_mc_round_trip_keeps_normalized_item_shape() {
        let item = Item::new(
            "What?".into(),
            ItemBody::Mc {
                choices: vec!["one".into(), "two".into()],
                answer: "two".into(),
            },
        )
        .expect("item");
        let encoded = render_item(&item.render_view())
            .expect("render")
            .expect("text");
        let decoded = parse_line(encoded.trim()).expect("read");
        assert_eq!(
            ItemFingerprint::new(&decoded, Vec::new()).expect("decoded fingerprint"),
            ItemFingerprint::new(&item, Vec::new()).expect("source fingerprint")
        );
    }

    #[test]
    fn every_non_numeric_bbq_shape_round_trips_by_fingerprint() {
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
                    min_answers_required: 1,
                    allow_all_correct: true,
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
                "FIB".into(),
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
        for source in items {
            let source = source.expect("item");
            let text = render_item(&source.render_view())
                .expect("render")
                .expect("text");
            let restored = parse_line(text.trim()).expect("restore");
            assert_eq!(
                ItemFingerprint::new(&restored, Vec::new()).expect("restored"),
                ItemFingerprint::new(&source, Vec::new()).expect("source")
            );
        }
    }

    #[test]
    fn reader_preserves_numeric_tolerance_note_for_python_compatible_crc() {
        let question = "What is the concentration? <p><i>Note: answers need to be within 5&percnt; of the correct number to be correct.</i></p>";
        let decoded = parse_line(&format!("NUM\t{question}\t2.00000000\t0.10000000"))
            .expect("NUM line parses");

        assert_eq!(decoded.common().question_text, question);
        let expected = Item::new(
            question.to_owned(),
            ItemBody::Num {
                answer: 2.0,
                tolerance: 0.1,
                tolerance_message: true,
            },
        )
        .expect("expected NUM item");
        assert_eq!(decoded.crc(), expected.crc());
    }

    #[test]
    fn reader_uses_python_defaults_for_omitted_ma_grading_options() {
        let decoded = parse_line(
            "MA\tChoose all correct answers\tA. yes\tCorrect\tB. also yes\tCorrect\tC. no\tIncorrect",
        )
        .expect("MA line parses");

        assert!(matches!(
            decoded.body(),
            ItemBody::Ma {
                min_answers_required: 1,
                allow_all_correct: true,
                ..
            }
        ));
    }

    #[test]
    fn numeric_writer_and_reader_retain_an_existing_tolerance_note() {
        let note = "<p><i>Note: answers need to be within 5&percnt; of the correct number to be correct.</i></p>";
        let source = Item::new(
            format!("Question {note}"),
            ItemBody::Num {
                answer: 2.0,
                tolerance: 0.1,
                tolerance_message: true,
            },
        )
        .expect("source NUM item");
        let encoded = render_item(&source.render_view())
            .expect("render")
            .expect("text");
        let decoded = parse_line(encoded.trim()).expect("read");

        assert_eq!(decoded.common().question_text.matches(note).count(), 2);
    }

    #[test]
    fn reader_returns_empty_assets_and_ordered_line_warnings() {
        let input = "questions.txt";
        let input_text =
            "\nINVALID\tbad\nMC\tQuestion\ta\tCorrect\tb\tIncorrect\nFIB\tOther\tanswer\n";
        let outcome = boxed_reader()
            .read_items(
                crate::ReadInput::File {
                    name: input,
                    bytes: input_text.as_bytes(),
                },
                false,
            )
            .expect("read");
        assert!(outcome.assets.entries().is_empty());
        assert_eq!(outcome.bank.len(), 1);
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| &warning.location)
                .collect::<Vec<_>>(),
            vec![
                &crate::ReadLocation::Line { line: 1 },
                &crate::ReadLocation::Line { line: 2 },
                &crate::ReadLocation::Line { line: 4 }
            ]
        );
    }

    #[test]
    fn writer_returns_reference_warning_in_item_and_source_order() {
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "First <img src=\"https://example.test/one.png\" /> then <img src=\"https://example.test/two.png\" />".into(),
                ItemBody::Fib { answers: vec!["yes".into()] },
            )
            .expect("item"),
        )
        .expect("bank item");
        let output = "questions.txt";
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("write BBQ");
        assert_eq!(primary(&outcome).name(), output);
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| warning.src.as_str())
                .collect::<Vec<_>>(),
            [
                "https://example.test/one.png",
                "https://example.test/two.png"
            ]
        );
    }

    #[test]
    fn empty_bank_creates_an_empty_bbq_upload_file() {
        let output = "questions.txt";
        let outcome = boxed_writer()
            .write_package(
                &ItemBank::new(false),
                &MemoryAssets::new(),
                &context(output),
            )
            .expect("write empty BBQ");
        assert_eq!(primary(&outcome).name(), output);
        assert!(outcome.warnings.is_empty());
        assert_eq!(document(&outcome), "");
    }
}
