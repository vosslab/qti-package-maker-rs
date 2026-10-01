//! Reader and writer for text2qti's compact Markdown-like question format.
//!
//! The supported item shapes are MC, MA, NUM, and FIB.  The format has no
//! representation for matching, multi-blank, or ordering items.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use lol_html::{RewriteStrSettings, element, rewrite_str};
use qti_core::media::{AssetKind, MediaPolicy, MediaWarning, apply_media_policy};
use qti_core::{Item, ItemBank, ItemBody, ItemKind, ItemRenderView, MediaBaseDir};
use regex::Regex;

use crate::{
    EngineError, EngineOptions, ReadLocation, ReadOutcome, ReadWarning, Reader, RenderHooks,
    WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "text2qti";
const KINDS: &[ItemKind] = &[ItemKind::Mc, ItemKind::Ma, ItemKind::Num, ItemKind::Fib];

/// Creates this engine's text writer for the common registry factory.
pub fn boxed_writer(_: EngineOptions) -> Box<dyn Writer> {
    Box::new(Text2QtiWriter)
}

/// Creates this engine's text reader for the common registry factory.
pub fn boxed_reader(_: EngineOptions) -> Box<dyn Reader> {
    Box::new(Text2QtiReader)
}

struct Text2QtiWriter;

impl Writer for Text2QtiWriter {
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
        bank: &ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError> {
        ensure_supported(bank)?;
        let collected = bank.collect_assets()?;
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
                KINDS,
                render_item,
                RenderHooks {
                    pre_render: None,
                    post_render: Some(&post_render),
                },
            )?
        };
        let text = rendered.join("\n");
        let output = output
            .unwrap_or_else(|| Path::new("text2qti-package.txt"))
            .to_path_buf();

        // Read all source bytes before changing the destination. This keeps malformed or
        // missing authored media from leaving a new text file behind (ASVS 2.2.1).
        let copies = local_copies(&collected)?;
        copy_media(&output, &copies)?;
        fs::write(&output, text).map_err(|source| EngineError::Io {
            engine: NAME,
            path: output.clone(),
            source,
        })?;
        Ok(WriteOutcome {
            path: Some(output),
            warnings: rendered_warnings.into_inner(),
        })
    }
}

fn ensure_supported(bank: &ItemBank) -> Result<(), EngineError> {
    bank.iter_ordered()
        .find(|item| !KINDS.contains(&item.kind()))
        .map_or(Ok(()), |item| {
            Err(EngineError::UnsupportedItemKind {
                engine: NAME,
                kind: item.kind(),
            })
        })
}

struct Text2QtiReader;

impl Reader for Text2QtiReader {
    fn name(&self) -> &'static str {
        NAME
    }

    fn read_items(&self, input: &Path, allow_mixed: bool) -> Result<ReadOutcome, EngineError> {
        let text = fs::read_to_string(input).map_err(|source| EngineError::Io {
            engine: NAME,
            path: input.to_path_buf(),
            source,
        })?;
        // The reader recognizes only this engine's Markdown image form and restores it once at
        // the input boundary. Attribute encoding occurs here, immediately before HTML creation
        // (ASVS 1.1.1, 1.1.2, 1.2.1).
        let restored = restore_markdown_images(&text);
        let parent = input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let mut bank = ItemBank::with_media_base_dir(allow_mixed, MediaBaseDir::external(parent));
        let mut warnings = Vec::new();
        for (number, block) in split_questions(&restored).into_iter().enumerate() {
            match parse_block(&block) {
                Ok(Some(item)) => match bank.add_item(item) {
                    Ok(_) => {}
                    Err(error) => warnings.push(block_warning(number + 1, error.to_string())),
                },
                Ok(None) => warnings.push(block_warning(
                    number + 1,
                    "unrecognized question block skipped".to_owned(),
                )),
                Err(message) => warnings.push(block_warning(number + 1, message)),
            }
        }
        Ok(ReadOutcome { bank, warnings })
    }
}

fn block_warning(number: usize, message: String) -> ReadWarning {
    ReadWarning {
        location: ReadLocation::Block { number },
        message,
    }
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

struct MarkdownMedia {
    targets: BTreeMap<qti_core::ItemCrc, BTreeMap<String, String>>,
    warnings: BTreeMap<qti_core::ItemCrc, Vec<MediaWarning>>,
}

fn markdown_media(
    collected: &qti_core::CollectedAssets,
    bank: &ItemBank,
) -> Result<MarkdownMedia, EngineError> {
    let mut targets = BTreeMap::new();
    let mut warnings = BTreeMap::new();
    for item in bank.iter_ordered() {
        let dependencies = collected.dependencies_for(item.crc()).unwrap_or_default();
        let decision = apply_media_policy(
            MediaPolicy::ReferenceWarn,
            dependencies,
            NAME,
            &item.crc().to_string(),
        )
        .map_err(|error| EngineError::InvalidFormat {
            engine: NAME,
            format: "media",
            message: error.to_string(),
        })?;
        let item_targets = dependencies
            .iter()
            .map(|asset| {
                let target =
                    if asset.kind == AssetKind::Local {
                        let name = asset.output_name.as_deref().ok_or_else(|| {
                            EngineError::InvalidFormat {
                                engine: NAME,
                                format: "media",
                                message: format!("asset '{}' has no output name", asset.src),
                            }
                        })?;
                        format!("media/{name}")
                    } else {
                        asset.src.clone()
                    };
                Ok((asset.src.clone(), target))
            })
            .collect::<Result<BTreeMap<_, _>, EngineError>>()?;
        targets.insert(*item.crc(), item_targets);
        warnings.insert(*item.crc(), decision.warnings);
    }
    Ok(MarkdownMedia { targets, warnings })
}

fn markdown_images(text: &str, targets: &BTreeMap<String, String>) -> Result<String, EngineError> {
    rewrite_str(
        text,
        RewriteStrSettings::new().append_element_content_handler(element!("img[src]", |element| {
            let source = element
                .get_attribute("src")
                .expect("img[src] selector guarantees a source attribute");
            let target = targets.get(&source).unwrap_or(&source);
            let alt = element.get_attribute("alt").unwrap_or_default();
            // The replacement is text2qti syntax, not author-supplied markup. The reader
            // context-encodes its two fields before recreating HTML (ASVS 1.2.1).
            element.replace(
                &markdown_image(&alt, target),
                lol_html::html_content::ContentType::Html,
            );
            Ok(())
        })),
    )
    .map_err(|error| EngineError::InvalidFormat {
        engine: NAME,
        format: "HTML",
        message: error.to_string(),
    })
}

fn markdown_image(alt: &str, target: &str) -> String {
    format!("![{alt}]({target})")
}

fn local_copies(
    collected: &qti_core::CollectedAssets,
) -> Result<Vec<(String, Vec<u8>)>, EngineError> {
    collected
        .assets()
        .iter()
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| {
            let name = asset
                .output_name
                .clone()
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "media",
                    message: format!("asset '{}' has no output name", asset.src),
                })?;
            let bytes = asset
                .read_bytes()
                .map_err(|error| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "media",
                    message: error.to_string(),
                })?;
            Ok((name, bytes))
        })
        .collect()
}

fn copy_media(output: &Path, copies: &[(String, Vec<u8>)]) -> Result<(), EngineError> {
    if copies.is_empty() {
        return Ok(());
    }
    let directory = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("media");
    fs::create_dir_all(&directory).map_err(|source| EngineError::Io {
        engine: NAME,
        path: directory.clone(),
        source,
    })?;
    for (name, bytes) in copies {
        let Some(leaf) = Path::new(name).file_name() else {
            return Err(EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: format!("unsafe generated media name '{name}'"),
            });
        };
        let path = directory.join(leaf);
        fs::write(&path, bytes).map_err(|source| EngineError::Io {
            engine: NAME,
            path,
            source,
        })?;
    }
    Ok(())
}

fn split_questions(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut previous_blank = true;
    for raw_line in text.trim().lines() {
        let line = raw_line.trim_end();
        if question_start(line) && previous_blank && !current.is_empty() {
            blocks.push(current.join("\n").trim().to_owned());
            current.clear();
        }
        current.push(line);
        previous_blank = line.is_empty();
    }
    if !current.is_empty() {
        blocks.push(current.join("\n").trim().to_owned());
    }
    blocks
}

fn question_start(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut position = 0;
    while bytes.get(position).is_some_and(u8::is_ascii_digit) {
        position += 1;
    }
    position > 0 && bytes.get(position) == Some(&b'.') && bytes.get(position + 1) == Some(&b' ')
}

fn parse_block(block: &str) -> Result<Option<Item>, String> {
    let lines = block.lines().map(str::trim_end).collect::<Vec<_>>();
    let Some(number_end) = question_number_end(lines.first().copied().unwrap_or_default()) else {
        return Ok(None);
    };
    if count_ma(&lines) >= 3 {
        parse_ma(&lines, number_end).map(Some)
    } else if count_mc(&lines) >= 2 {
        parse_mc(&lines, number_end).map(Some)
    } else if lines.iter().any(|line| line.starts_with('=')) {
        parse_num(&lines, number_end).map(Some)
    } else if lines.iter().any(|line| line.starts_with("* ")) {
        parse_fib(&lines, number_end).map(Some)
    } else {
        Ok(None)
    }
}

fn question_number_end(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut position = 0;
    while bytes.get(position).is_some_and(u8::is_ascii_digit) {
        position += 1;
    }
    (position > 0 && bytes.get(position) == Some(&b'.') && bytes.get(position + 1) == Some(&b' '))
        .then_some(position + 2)
}

fn count_mc(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| mc_choice(line).is_some())
        .count()
}

fn count_ma(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| ma_choice(line).is_some())
        .count()
}

fn mc_choice(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim_start();
    let (correct, rest) = trimmed
        .strip_prefix('*')
        .map_or((false, trimmed), |rest| (true, rest));
    let bytes = rest.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b')'
        && bytes[2].is_ascii_whitespace())
    .then(|| (correct, rest[3..].trim().to_owned()))
}

fn ma_choice(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim();
    let content = trimmed
        .strip_prefix("[*] ")
        .map(|text| (true, text))
        .or_else(|| trimmed.strip_prefix("[ ] ").map(|text| (false, text)))?;
    Some((content.0, content.1.trim().to_owned()))
}

fn stem(lines: &[&str], end: usize, answer_start: usize) -> String {
    let mut pieces = Vec::new();
    if let Some(first) = lines.first() {
        pieces.push(first[end..].trim());
    }
    pieces.extend(
        lines
            .iter()
            .take(answer_start)
            .skip(1)
            .map(|line| line.trim()),
    );
    pieces
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_mc(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| mc_choice(line).is_some())
        .ok_or_else(|| "MC has no choices".to_owned())?;
    let question = stem(lines, number_end, start);
    let mut choices = Vec::new();
    let mut answer = None;
    let mut current = None::<(bool, String)>;
    for line in &lines[start..] {
        if let Some(next) = mc_choice(line) {
            if let Some((correct, choice)) = current.take() {
                if correct {
                    if answer.is_some() {
                        return Err("MC has more than one correct choice".to_owned());
                    }
                    answer = Some(choice.clone());
                }
                choices.push(choice);
            }
            current = Some(next);
        } else if !feedback_line(line)
            && let Some((_, choice)) = &mut current
        {
            append_line(choice, line);
        }
    }
    if let Some((correct, choice)) = current {
        if correct {
            if answer.is_some() {
                return Err("MC has more than one correct choice".to_owned());
            }
            answer = Some(choice.clone());
        }
        choices.push(choice);
    }
    Item::new(
        question,
        ItemBody::Mc {
            choices,
            answer: answer.ok_or_else(|| "MC has no correct choice".to_owned())?,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_ma(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| ma_choice(line).is_some())
        .ok_or_else(|| "MA has no choices".to_owned())?;
    let question = stem(lines, number_end, start);
    let mut choices = Vec::new();
    let mut answers = Vec::new();
    let mut current = None::<(bool, String)>;
    for line in &lines[start..] {
        if let Some(next) = ma_choice(line) {
            if let Some((correct, choice)) = current.take() {
                if correct {
                    answers.push(choice.clone());
                }
                choices.push(choice);
            }
            current = Some(next);
        } else if !feedback_line(line)
            && let Some((_, choice)) = &mut current
        {
            append_line(choice, line);
        }
    }
    if let Some((correct, choice)) = current {
        if correct {
            answers.push(choice.clone());
        }
        choices.push(choice);
    }
    Item::new(
        question,
        ItemBody::Ma {
            choices,
            answers,
            // text2qti carries choices and answer markers but no MA grading options.
            // The frozen reader delegates omitted fields to the MA constructor defaults.
            min_answers_required: 1,
            allow_all_correct: true,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_num(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| line.starts_with('='))
        .ok_or_else(|| "NUM lacks answer line".to_owned())?;
    let question = stem(lines, number_end, start);
    let value = lines[start].trim_start_matches('=').trim();
    let (answer, tolerance) = if let Some(range) = value
        .strip_prefix('[')
        .and_then(|part| part.strip_suffix(']'))
    {
        let (low, high) = range
            .split_once(',')
            .ok_or_else(|| "invalid NUM range".to_owned())?;
        let low = parse_number(low)?;
        let high = parse_number(high)?;
        ((low + high) / 2.0, (high - low) / 2.0)
    } else if let Some((answer, tolerance)) = value.split_once("+-") {
        (parse_number(answer)?, parse_number(tolerance)?)
    } else {
        (parse_number(value)?, 0.0)
    };
    Item::new(
        question,
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message: true,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_number(value: &str) -> Result<f64, String> {
    let number = value
        .trim()
        .replace('_', "")
        .parse::<f64>()
        .map_err(|_| "invalid NUM answer".to_owned())?;
    number
        .is_finite()
        .then_some(number)
        .ok_or_else(|| "NUM values must be finite".to_owned())
}

fn parse_fib(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| line.starts_with("* "))
        .ok_or_else(|| "FIB has no answers".to_owned())?;
    let question = stem(lines, number_end, start);
    let answers = lines[start..]
        .iter()
        .filter_map(|line| {
            line.strip_prefix("* ")
                .map(|answer| answer.trim().to_owned())
        })
        .collect();
    Item::new(question, ItemBody::Fib { answers }).map_err(|error| error.to_string())
}

fn feedback_line(line: &str) -> bool {
    ["... ", "+ ", "- "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

fn append_line(target: &mut String, line: &str) {
    let text = line.trim();
    if !text.is_empty() {
        if !target.is_empty() {
            target.push(' ');
        }
        target.push_str(text);
    }
}

fn restore_markdown_images(text: &str) -> String {
    // The grammar intentionally accepts the form emitted by this writer. It does not interpret
    // arbitrary Markdown, execute links, or fetch remote resources (ASVS 1.3.5, 1.3.6).
    let pattern = Regex::new(r"!\[([^\]]*)\]\(([^)]*)\)").expect("constant Markdown image regex");
    pattern
        .replace_all(text, |captures: &regex::Captures<'_>| {
            format!(
                "<img src=\"{}\" alt=\"{}\"/>",
                html_attribute(&unescape_markdown(&captures[2])),
                html_attribute(&unescape_markdown(&captures[1]))
            )
        })
        .into_owned()
}

fn unescape_markdown(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            result.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{boxed_reader, boxed_writer, parse_block, restore_markdown_images};
    use crate::{EngineError, EngineOptions, ReadLocation};
    use qti_core::media::resolve_item_media_refs;
    use qti_core::{Item, ItemBank, ItemBody, ItemFingerprint, MediaBaseDir};

    fn every_supported_item() -> ItemBank {
        let mut bank = ItemBank::new(true);
        for item in [
            Item::new(
                "MC stem".to_owned(),
                ItemBody::Mc {
                    choices: vec!["first".to_owned(), "second".to_owned()],
                    answer: "second".to_owned(),
                },
            ),
            Item::new(
                "MA stem".to_owned(),
                ItemBody::Ma {
                    choices: vec!["first".to_owned(), "second".to_owned(), "third".to_owned()],
                    answers: vec!["first".to_owned(), "third".to_owned()],
                    min_answers_required: 1,
                    allow_all_correct: true,
                },
            ),
            Item::new(
                "NUM stem".to_owned(),
                ItemBody::Num {
                    answer: 2.5,
                    tolerance: 0.01,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "FIB stem".to_owned(),
                ItemBody::Fib {
                    answers: vec!["alpha".to_owned(), "beta".to_owned()],
                },
            ),
        ] {
            bank.add_item(item.expect("valid item")).expect("add item");
        }
        bank
    }

    #[test]
    fn every_python_supported_shape_round_trips_by_fingerprint_and_order() {
        let directory = tempfile::tempdir().expect("temporary output");
        let output = directory.path().join("questions.txt");
        let bank = every_supported_item();
        boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("write");
        let restored = boxed_reader(EngineOptions::default())
            .read_items(&output, true)
            .expect("read");
        assert!(restored.warnings.is_empty());
        assert_eq!(restored.bank.len(), bank.len());
        for (source, round_trip) in bank.iter_ordered().zip(restored.bank.iter_ordered()) {
            assert_eq!(source.kind(), round_trip.kind());
            assert_eq!(
                ItemFingerprint::new(source, Vec::new()).expect("source fingerprint"),
                ItemFingerprint::new(round_trip, Vec::new()).expect("restored fingerprint")
            );
        }
    }

    #[test]
    fn writer_uses_text2qti_markers_and_blank_block_separators() {
        let directory = tempfile::tempdir().expect("temporary output");
        let output = directory.path().join("questions.txt");
        boxed_writer(EngineOptions::default())
            .save_package(&every_supported_item(), Some(&output))
            .expect("write");
        assert_eq!(
            std::fs::read_to_string(output).expect("text"),
            "1. MC stem\nA) first\n*B) second\n\n2. MA stem\n[*] first\n[ ] second\n[*] third\n\n3. NUM stem\n= 2.5 +- 0.01\n\n4. FIB stem\n* alpha\n* beta\n"
        );
    }

    #[test]
    fn writer_returns_typed_unsupported_errors() {
        let cases = [
            (
                "MATCH stem",
                ItemBody::Match {
                    prompts: vec!["prompt one".to_owned(), "prompt two".to_owned()],
                    choices: vec!["choice one".to_owned(), "choice two".to_owned()],
                },
            ),
            (
                "Fill [blank] now",
                ItemBody::MultiFib {
                    answers: [("blank".to_owned(), vec!["answer".to_owned()])]
                        .into_iter()
                        .collect(),
                },
            ),
            (
                "ORDER stem",
                ItemBody::Order {
                    answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                },
            ),
        ];
        for (question, body) in cases {
            let mut bank = ItemBank::new(true);
            let item = Item::new(question.to_owned(), body).expect("valid source item");
            bank.add_item(item).expect("add item");
            let directory = tempfile::tempdir().expect("output directory");
            let error = boxed_writer(EngineOptions::default())
                .save_package(&bank, Some(&directory.path().join("questions.txt")))
                .expect_err("unsupported");
            assert!(matches!(
                error,
                EngineError::UnsupportedItemKind {
                    engine: super::NAME,
                    ..
                }
            ));
        }
    }

    #[test]
    fn reader_discards_feedback_and_retains_block_warning_locations() {
        let directory = tempfile::tempdir().expect("temporary input");
        let input = directory.path().join("questions.txt");
        std::fs::write(&input, "1. First question\na) wrong\n... ignored feedback\n*b) right\n+ also ignored\n\n2. broken\n\n3. Last\n* answer\n").expect("input");
        let outcome = boxed_reader(EngineOptions::default())
            .read_items(&input, true)
            .expect("read");
        assert_eq!(outcome.bank.len(), 2);
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].location,
            ReadLocation::Block { number: 2 }
        );
        assert_eq!(
            outcome.bank.get(0).expect("MC").common().question_text,
            "First question"
        );
    }

    #[test]
    fn reader_uses_python_defaults_for_omitted_ma_grading_options() {
        let parsed = parse_block("1. Choose all correct answers\n[*] first\n[*] second\n[ ] third")
            .expect("MA block parses")
            .expect("MA block is recognized");

        assert!(matches!(
            parsed.body(),
            ItemBody::Ma {
                min_answers_required: 1,
                allow_all_correct: true,
                ..
            }
        ));
    }

    #[test]
    fn local_remote_and_data_media_keep_the_contract_end_to_end() {
        let directory = tempfile::tempdir().expect("temporary media");
        let png = [137, 80, 78, 71, 13, 10, 26, 10];
        std::fs::write(directory.path().join("local.png"), png).expect("local image");
        let mut bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(directory.path()));
        bank.add_item(Item::new("<img src=\"local.png\" alt=\"local figure\"/><img src=\"https://example.test/remote.png\" alt=\"remote\"/><img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"inline\"/>".to_owned(), ItemBody::Fib { answers: vec!["yes".to_owned()] }).expect("valid item")).expect("add");
        let output = directory.path().join("questions.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&output))
            .expect("write");
        assert_eq!(outcome.path, Some(output.clone()));
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| warning.src.as_str())
                .collect::<Vec<_>>(),
            [
                "local.png",
                "https://example.test/remote.png",
                "data:image/png;base64,iVBORw0KGgo="
            ]
        );
        let written = std::fs::read_to_string(&output).expect("text");
        assert!(written.contains("![local figure](media/local.png)"));
        assert!(written.contains("![remote](https://example.test/remote.png)"));
        assert!(written.contains("![inline](data:image/png;base64,iVBORw0KGgo=)"));
        assert_eq!(
            std::fs::read(directory.path().join("media/local.png")).expect("copied"),
            png
        );
        let restored = boxed_reader(EngineOptions::default())
            .read_items(&output, true)
            .expect("read");
        let source = bank.get(0).expect("source");
        let round_trip = restored.bank.get(0).expect("round trip");
        let source_refs =
            resolve_item_media_refs(source, directory.path()).expect("source media refs");
        let restored_refs =
            resolve_item_media_refs(round_trip, directory.path()).expect("restored media refs");
        assert_eq!(
            ItemFingerprint::new(source, source_refs).expect("source fingerprint"),
            ItemFingerprint::new(round_trip, restored_refs).expect("restored fingerprint")
        );
    }

    #[test]
    fn write_outcome_warnings_follow_rendered_item_order() {
        let directory = tempfile::tempdir().expect("temporary output");
        let mut bank = ItemBank::new(true);
        let first = Item::new(
            "<img src=\"https://example.test/first.png\"/> first".to_owned(),
            ItemBody::Fib {
                answers: vec!["first".to_owned()],
            },
        )
        .expect("first item");
        let first_crc = first.crc().to_string();
        bank.add_item(first).expect("add first");
        let second = Item::new(
            "<img src=\"https://example.test/second.png\"/> second".to_owned(),
            ItemBody::Fib {
                answers: vec!["second".to_owned()],
            },
        )
        .expect("second item");
        let second_crc = second.crc().to_string();
        bank.add_item(second).expect("add second");

        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&directory.path().join("questions.txt")))
            .expect("write");
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| warning.item_crc.as_str())
                .collect::<Vec<_>>(),
            [first_crc.as_str(), second_crc.as_str()]
        );
    }

    #[test]
    fn empty_bank_preserves_python_empty_output_behavior() {
        let directory = tempfile::tempdir().expect("temporary output");
        let output = directory.path().join("questions.txt");
        let outcome = boxed_writer(EngineOptions::default())
            .save_package(&ItemBank::new(true), Some(&output))
            .expect("empty bank");
        assert_eq!(outcome.path, Some(output.clone()));
        assert!(outcome.warnings.is_empty());
        assert_eq!(std::fs::read_to_string(output).expect("empty output"), "");
    }

    #[test]
    fn markdown_restoration_encodes_attributes_before_creating_html() {
        let restored =
            restore_markdown_images("1. Stem ![x\" onerror=\"boom](image.png)\n* answer");
        assert!(restored.contains("alt=\"x&quot; onerror=&quot;boom\""));
        assert!(parse_block(&restored).expect("parse").is_some());
    }
}
