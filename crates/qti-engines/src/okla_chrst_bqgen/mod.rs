//! Oklahoma Christian BQGen plain-text reader and writer.
//!
//! This module follows the certified Python BQGen grammar.  Its writer represents MC, MA,
//! MATCH, and FIB items; the shared rendering loop silently skips the other core kinds because
//! Python's corresponding writer functions return `None`.  The reader accepts only the same
//! three header families and deliberately ignores unknown blocks, as the Python reader does.

use std::cell::RefCell;

use qti_core::media::{MediaPolicy, apply_media_policy, replace_item_images};
use qti_core::{
    AssetSource, Item, ItemBank, ItemBody, ItemKind, ItemRenderView, MemoryAssets, NamedFile,
};
use regex::Regex;

use crate::{
    EngineError, ReadInput, ReadLocation, ReadOutcome, ReadWarning, Reader, RenderHooks,
    WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "okla_chrst_bqgen";

/// Creates the BQGen writer used by the static engine registry.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(BqgenWriter)
}

/// Creates the BQGen reader used by the static engine registry.
pub fn boxed_reader() -> Box<dyn Reader> {
    Box::new(BqgenReader)
}

struct BqgenWriter;

impl Writer for BqgenWriter {
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
        // Inspect only items this document can emit; placeholders need no payload reads.
        let mut visible = qti_core::ItemBank::new(true);
        for item in bank
            .iter_ordered()
            .filter(|item| self.supported_kinds().contains(&item.kind()))
        {
            visible.add_item(item.clone())?;
        }
        let assets = visible.inspect_assets()?;
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &Item| {
            let dependencies = assets.dependencies_for(item.crc()).unwrap_or_default();
            // Classify authored references without loading payloads this format cannot transport.
            // The format has no image channel, so the source item's presentation copy receives
            // readable text while the validated source item and its CRC remain untouched.
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
            warnings
                .borrow_mut()
                .extend(decision.warnings.iter().cloned());
            replace_item_images(item, |src, _alt| {
                decision
                    .placeholders
                    .get(src)
                    .cloned()
                    .unwrap_or_else(|| src.to_owned())
            })
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "HTML",
                message: error.to_string(),
            })
        };
        let rendered = render_bank(
            bank,
            self.supported_kinds(),
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;
        if rendered.is_empty() {
            return Ok(WriteOutcome {
                artifact: None,
                warnings: warnings.into_inner(),
            });
        }
        let primary = NamedFile::new(context.output_name(), rendered.concat().into_bytes())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary,
                companions: Vec::new(),
            }),
            warnings: warnings.into_inner(),
        })
    }
}

struct BqgenReader;

impl Reader for BqgenReader {
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
        for (number, block) in split_blocks(text).into_iter().enumerate() {
            let Some(item) = parse_block(&block) else {
                // The Python reader intentionally drops unknown or malformed blocks.  Retain a
                // located warning in Rust's richer ReadOutcome without changing valid records.
                warnings.push(ReadWarning {
                    location: ReadLocation::Block { number: number + 1 },
                    message: "unrecognized BQGen block skipped".to_owned(),
                });
                continue;
            };
            if let Err(error) = bank.add_item(item) {
                warnings.push(ReadWarning {
                    location: ReadLocation::Block { number: number + 1 },
                    message: format!("block skipped: {error}"),
                });
            }
        }
        Ok(ReadOutcome {
            bank,
            assets: MemoryAssets::new(),
            warnings,
        })
    }
}

fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let number = item.common().item_number;
    let text = match item.body() {
        ItemBody::Mc { choices, answer } => format!(
            "{number}. {}\n{}",
            item.common().question_text,
            render_choices(choices, std::slice::from_ref(answer))?
        ),
        ItemBody::Ma {
            choices, answers, ..
        } => format!(
            "{number}. {}\n{}",
            item.common().question_text,
            render_choices(choices, answers)?
        ),
        ItemBody::Match { prompts, choices } => {
            let mut output = format!("match {number}. {}\n", item.common().question_text);
            for (index, (prompt, choice)) in prompts.iter().zip(choices).enumerate() {
                output.push_str(&format!("{}) {prompt}/{choice}\n", bqgen_letter(index)?));
            }
            output.push('\n');
            output
        }
        ItemBody::Fib { answers } => {
            let mut output = format!("blank {number}. {}\n", item.common().question_text);
            for (index, answer) in answers.iter().enumerate() {
                output.push_str(&format!("*{}. {answer}\n", bqgen_letter(index)?));
            }
            output.push('\n');
            output
        }
        ItemBody::Num { .. } | ItemBody::MultiFib { .. } | ItemBody::Order { .. } => {
            return Ok(None);
        }
    };
    Ok(Some(text))
}

fn render_choices(choices: &[String], answers: &[String]) -> Result<String, EngineError> {
    let mut output = String::new();
    for (index, choice) in choices.iter().enumerate() {
        let marker = if answers.contains(choice) { "*" } else { "" };
        output.push_str(&format!("{marker}{}) {choice}\n", bqgen_letter(index)?));
    }
    output.push('\n');
    Ok(output)
}

fn bqgen_letter(index: usize) -> Result<char, EngineError> {
    u8::try_from(index)
        .ok()
        .and_then(|index| b'a'.checked_add(index))
        .filter(|letter| letter.is_ascii_lowercase())
        .map(char::from)
        .ok_or_else(|| EngineError::InvalidFormat {
            engine: NAME,
            format: "BQGen",
            message: "BQGen supports at most 26 answer choices".to_owned(),
        })
}

fn split_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line.trim_end().to_owned());
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn parse_block(block: &str) -> Option<Item> {
    let header = block.lines().next()?.trim().to_ascii_lowercase();
    if header.starts_with("match") {
        parse_match(block)
    } else if header.starts_with("blank") {
        parse_fib(block)
    } else if Regex::new(r"^\d+\.")
        .expect("static expression")
        .is_match(&header)
    {
        parse_mc_ma(block)
    } else {
        None
    }
}

fn parse_mc_ma(block: &str) -> Option<Item> {
    let lines = block.lines().collect::<Vec<_>>();
    let question = numbered_stem(lines.first()?)?;
    let mut choices = Vec::new();
    let mut correct = Vec::new();
    for line in &lines[1..] {
        let Some((is_correct, text)) = parse_choice_line(line) else {
            continue;
        };
        choices.push(text.to_owned());
        if is_correct {
            correct.push(text.to_owned());
        }
    }
    if choices.is_empty() {
        return None;
    }
    let body = if correct.len() == 1 {
        ItemBody::Mc {
            choices,
            answer: correct.pop()?,
        }
    } else {
        ItemBody::Ma {
            choices,
            answers: correct,
            // BQGen carries no MA grading options. The Python reader delegates to the MA
            // constructor, so retain that constructor's defaults for omitted fields.
            min_answers_required: 1,
            allow_all_correct: true,
        }
    };
    Item::new(question, body).ok()
}

fn parse_fib(block: &str) -> Option<Item> {
    let lines = block.lines().collect::<Vec<_>>();
    let question = prefixed_stem(lines.first()?, "blank")?;
    let mut answers = Vec::new();
    for line in &lines[1..] {
        if let Some((_, text)) = parse_choice_line(line) {
            answers.push(text.to_owned());
        } else if !line.trim().is_empty() {
            answers.push(line.trim().to_owned());
        }
    }
    (!answers.is_empty())
        .then(|| Item::new(question, ItemBody::Fib { answers }).ok())
        .flatten()
}

fn parse_match(block: &str) -> Option<Item> {
    let lines = block.lines().collect::<Vec<_>>();
    let question = prefixed_stem(lines.first()?, "match")?;
    let mut prompts = Vec::new();
    let mut choices = Vec::new();
    for line in &lines[1..] {
        let Some((_, text)) = parse_choice_line(line) else {
            continue;
        };
        let Some((prompt, choice)) = text.split_once('/') else {
            continue;
        };
        prompts.push(prompt.trim().to_owned());
        choices.push(choice.trim().to_owned());
    }
    (!prompts.is_empty())
        .then(|| Item::new(question, ItemBody::Match { prompts, choices }).ok())
        .flatten()
}

fn parse_choice_line(line: &str) -> Option<(bool, &str)> {
    let line = line.trim();
    let (correct, line) = line
        .strip_prefix('*')
        .map_or((false, line), |line| (true, line));
    let mut characters = line.chars();
    characters.next()?.is_ascii_alphabetic().then_some(())?;
    let remainder = characters.as_str();
    let remainder = remainder
        .strip_prefix(')')
        .or_else(|| remainder.strip_prefix('.'))
        .unwrap_or(remainder);
    let text = remainder.trim();
    (!text.is_empty()).then_some((correct, text))
}

fn numbered_stem(header: &str) -> Option<String> {
    let digits = header.chars().take_while(char::is_ascii_digit).count();
    (digits > 0 && header.as_bytes().get(digits) == Some(&b'.'))
        .then(|| header[digits + 1..].trim().to_owned())
}

fn prefixed_stem(header: &str, prefix: &str) -> Option<String> {
    let rest = header
        .get(..prefix.len())?
        .eq_ignore_ascii_case(prefix)
        .then(|| &header[prefix.len()..])?;
    let rest = rest.trim_start();
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if rest.as_bytes().get(digits) == Some(&b'.') {
        return Some(rest[digits + 1..].trim().to_owned());
    }
    Some(header.trim().to_owned())
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

    use super::{NAME, boxed_reader, boxed_writer, parse_block, render_item};
    use qti_core::MemoryAssets;
    use qti_core::media::{MediaAction, MediaPolicy, apply_media_policy};
    use qti_core::{Item, ItemBank, ItemBody, ItemFingerprint};

    fn supported_items() -> Vec<Item> {
        vec![
            Item::new(
                "MC question".into(),
                ItemBody::Mc {
                    choices: vec!["a".into(), "b".into(), "c".into()],
                    answer: "b".into(),
                },
            ),
            Item::new(
                "MA question".into(),
                ItemBody::Ma {
                    choices: vec!["a".into(), "b".into(), "c".into()],
                    answers: vec!["a".into(), "b".into()],
                    min_answers_required: 1,
                    allow_all_correct: true,
                },
            ),
            Item::new(
                "MATCH question".into(),
                ItemBody::Match {
                    prompts: vec!["p1".into(), "p2".into()],
                    choices: vec!["c1".into(), "c2".into()],
                },
            ),
            Item::new(
                "FIB question".into(),
                ItemBody::Fib {
                    answers: vec!["one".into(), "two".into()],
                },
            ),
        ]
        .into_iter()
        .map(|item| item.expect("valid test item"))
        .collect()
    }

    #[test]
    fn supported_shapes_round_trip_by_fingerprint() {
        let mut bank = ItemBank::new(true);
        for item in supported_items() {
            bank.add_item(item).expect("bank item");
        }
        let output = "items.txt";
        let write_outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("write BQGen");
        assert!(write_outcome.warnings.is_empty());
        let outcome = boxed_reader()
            .read_items(
                crate::ReadInput::File {
                    name: primary(&write_outcome).name(),
                    bytes: primary(&write_outcome).bytes(),
                },
                true,
            )
            .expect("read BQGen");
        assert!(outcome.warnings.is_empty());
        assert_eq!(outcome.bank.len(), bank.len());
        for (expected, actual) in bank.iter_ordered().zip(outcome.bank.iter_ordered()) {
            assert_eq!(
                ItemFingerprint::new(expected, Vec::new()).expect("expected fingerprint"),
                ItemFingerprint::new(actual, Vec::new()).expect("actual fingerprint")
            );
        }
    }

    #[test]
    fn writer_skips_python_unsupported_kinds() {
        let mut bank = ItemBank::new(true);
        for item in supported_items() {
            bank.add_item(item).expect("supported item");
        }
        bank.add_item(
            Item::new(
                "NUM".into(),
                ItemBody::Num {
                    answer: 1.0,
                    tolerance: 0.0,
                    tolerance_message: false,
                },
            )
            .expect("NUM"),
        )
        .expect("NUM bank item");
        bank.add_item(
            Item::new(
                "ORDER".into(),
                ItemBody::Order {
                    answers: vec!["one".into(), "two".into(), "three".into()],
                },
            )
            .expect("ORDER"),
        )
        .expect("ORDER bank item");
        assert!(
            !crate::engine(NAME)
                .expect("registry")
                .supported_kinds
                .contains(&qti_core::ItemKind::Num)
        );
        assert!(
            !crate::engine(NAME)
                .expect("registry")
                .supported_kinds
                .contains(&qti_core::ItemKind::Order)
        );
        let texts = bank
            .iter_ordered()
            .filter_map(|item| render_item(&item.render_view()).expect("render"))
            .collect::<Vec<_>>();
        assert_eq!(texts.len(), 4);
    }

    #[test]
    fn unsupported_only_bank_creates_no_requested_output() {
        let output = "must-not-exist.txt";
        let mut bank = ItemBank::new(true);
        bank.add_item(
            Item::new(
                "NUM question".into(),
                ItemBody::Num {
                    answer: 1.0,
                    tolerance: 0.0,
                    tolerance_message: false,
                },
            )
            .expect("NUM item"),
        )
        .expect("NUM bank item");
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("skip-only bank");
        assert!(outcome.artifact.is_none());
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn reader_preserves_python_unknown_block_recovery_with_location() {
        let input = "input.txt";
        let input_text = "essay 1. Unknown format\nprose\n\n1. Known\n*a) yes\nb) no\n";
        let outcome = boxed_reader()
            .read_items(
                crate::ReadInput::File {
                    name: input,
                    bytes: input_text.as_bytes(),
                },
                false,
            )
            .expect("reader returns valid records");
        assert_eq!(outcome.bank.len(), 1);
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].location,
            crate::ReadLocation::Block { number: 1 }
        );
        assert!(parse_block("essay 1. Unknown\nprose").is_none());
    }

    #[test]
    fn reader_uses_python_defaults_for_omitted_ma_grading_options() {
        let decoded = parse_block("1. Choose all correct answers\n*a) yes\n*b) also yes\nc) no")
            .expect("MA block parses");

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
    fn placeholder_warn_substitutes_local_remote_and_data_images() {
        let mut bank = ItemBank::new(true);
        let local = Item::new(
            "<img src=\"figure.png\"/>".into(),
            ItemBody::Mc {
                choices: vec!["yes".into(), "no".into(), "maybe".into()],
                answer: "yes".into(),
            },
        )
        .expect("local item");
        let remote = Item::new(
            "<img src=\"https://example.test/figure.png\"/>".into(),
            ItemBody::Ma {
                choices: vec!["yes".into(), "no".into(), "maybe".into()],
                answers: vec!["yes".into()],
                min_answers_required: 0,
                allow_all_correct: false,
            },
        )
        .expect("remote item");
        let data = Item::new(
            "<img src=\"data:image/png;base64,iVBORw0KGgo=\"/>".into(),
            ItemBody::Match {
                prompts: vec!["one".into(), "two".into()],
                choices: vec!["one".into(), "two".into()],
            },
        )
        .expect("data item");
        let fib = Item::new(
            "FIB <img src=\"figure.png\" alt=\"figure alt\"/>".into(),
            ItemBody::Fib {
                answers: vec!["x".into()],
            },
        )
        .expect("FIB item");
        for item in [local, remote, data, fib] {
            bank.add_item(item).expect("bank item");
        }
        bank.add_item(
            Item::new(
                "NUM skipped <img src=\"figure.png\"/>".into(),
                ItemBody::Num {
                    answer: 1.0,
                    tolerance: 0.0,
                    tolerance_message: false,
                },
            )
            .expect("NUM item"),
        )
        .expect("NUM bank item");
        let assets = bank.inspect_assets().expect("assets");
        for item in bank.iter_ordered() {
            let decision = apply_media_policy(
                MediaPolicy::PlaceholderWarn,
                assets.dependencies_for(item.crc()).expect("dependency"),
                NAME,
                &item.crc().to_string(),
            )
            .expect("placeholder policy");
            assert_eq!(decision.warnings.len(), 1);
            assert_eq!(decision.warnings[0].action, MediaAction::Substituted);
        }
        let output = "references.txt";
        let write_outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &context(output))
            .expect("writer");
        let text = document(&write_outcome);
        assert_eq!(text.matches("[image: figure.png]").count(), 3);
        assert!(text.contains("[image: embedded image]"));
        assert!(!text.contains("<img"));
        assert!(!text.contains("https://example.test/figure.png"));
        assert!(!text.contains("data:image/png;base64,iVBORw0KGgo="));
        assert!(
            bank.get(3)
                .expect("source item")
                .common()
                .question_text
                .contains("alt=\"figure alt\"")
        );
        assert_eq!(write_outcome.warnings.len(), 4);
        assert_eq!(
            write_outcome
                .warnings
                .iter()
                .map(|warning| warning.src.as_str())
                .collect::<Vec<_>>(),
            [
                "figure.png",
                "https://example.test/figure.png",
                "data:image/png;base64,iVBORw0KGgo=",
                "figure.png",
            ]
        );
    }

    #[test]
    fn writer_has_declared_name_and_policy() {
        let writer = boxed_writer();
        assert_eq!(writer.name(), NAME);
        assert_eq!(writer.media_policy(), MediaPolicy::PlaceholderWarn);
    }
}
