//! Maps validated QPM display items to PLE Native JSON source documents.

use std::collections::{BTreeMap, HashSet};

use qti_core::{ItemBody, ItemRenderView};

use super::source::{
    Blank, Choice, Match, MatchMode, MatchingChoice, MatchingPrompt, OrderingItem, Response,
    SourceDocument, Tolerance,
};
use crate::EngineError;

const ENGINE: &str = "ple_native_json";
const FORMAT: &str = "PLE Native JSON";
const MATCH_MODE: MatchMode = MatchMode::Normalized;
const MAX_LENGTH: u32 = 16_384;

/// Map one validated, media-rewritten display item without changing its identity.
pub(super) fn map_item(item: &ItemRenderView) -> Result<SourceDocument, EngineError> {
    let mut prompt = item.common().question_text.clone();
    let response = match item.body() {
        ItemBody::Mc { choices, answer } => {
            let correct_index = choices
                .iter()
                .position(|choice| choice == answer)
                .ok_or_else(|| invalid(item, "MC answer is absent from its choices"))?;
            Response::SingleChoice {
                choices: choices_with_ids(choices),
                correct_choice: id("choice", correct_index),
            }
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            let correct_choices = choices
                .iter()
                .enumerate()
                .filter(|(_, choice)| answers.contains(choice))
                .map(|(index, _)| id("choice", index))
                .collect();
            // Item validation already requires every answer to occur among the choices. Check
            // again at this conversion boundary so malformed derived views never lose an answer.
            if answers.iter().any(|answer| !choices.contains(answer)) {
                return Err(invalid(item, "MA answer is absent from its choices"));
            }
            Response::MultipleAnswer {
                choices: choices_with_ids(choices),
                correct_choices,
            }
        }
        ItemBody::Fib { answers } => Response::FillIn {
            answers: unique_answers(answers),
            match_mode: MATCH_MODE,
            max_length: MAX_LENGTH,
        },
        ItemBody::MultiFib { answers } => Response::MultiFillIn {
            blanks: blanks_by_appearance(item, &prompt, answers)?,
        },
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            if *tolerance_message {
                prompt.push_str(&format!(
                    "<p>Answer must be within &plusmn;{tolerance}.</p>"
                ));
            }
            Response::Numeric {
                answer: *answer,
                tolerance: Tolerance::Absolute {
                    epsilon: *tolerance,
                },
            }
        }
        ItemBody::Match { prompts, choices } => Response::Matching {
            prompts: prompts
                .iter()
                .enumerate()
                .map(|(index, text)| MatchingPrompt {
                    id: id("prompt", index),
                    text: text.clone(),
                })
                .collect(),
            choices: choices
                .iter()
                .enumerate()
                .map(|(index, text)| MatchingChoice {
                    id: id("choice", index),
                    text: text.clone(),
                })
                .collect(),
            matches: (0..prompts.len())
                .map(|index| Match {
                    prompt: id("prompt", index),
                    choice: id("choice", index),
                })
                .collect(),
        },
        ItemBody::Order { answers } => Response::Ordering {
            items: answers
                .iter()
                .enumerate()
                .map(|(index, text)| OrderingItem {
                    id: id("item", index),
                    text: text.clone(),
                })
                .collect(),
            correct_order: (0..answers.len()).map(|index| id("item", index)).collect(),
        },
    };

    Ok(SourceDocument {
        format: "pleQuestionJson".to_owned(),
        prompt,
        response,
        external_resources: Vec::new(),
    })
}

fn choices_with_ids(choices: &[String]) -> Vec<Choice> {
    choices
        .iter()
        .enumerate()
        .map(|(index, text)| Choice {
            id: id("choice", index),
            text: text.clone(),
        })
        .collect()
}

fn blanks_by_appearance(
    item: &ItemRenderView,
    prompt: &str,
    answers: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<Blank>, EngineError> {
    let mut positions = answers
        .iter()
        .map(|(key, values)| {
            let marker = format!("[{key}]");
            let position = prompt
                .find(&marker)
                .ok_or_else(|| invalid(item, format!("MULTI_FIB marker {marker:?} is absent")))?;
            Ok((position, key, values))
        })
        .collect::<Result<Vec<_>, EngineError>>()?;
    positions.sort_by_key(|(position, _, _)| *position);

    Ok(positions
        .into_iter()
        .enumerate()
        .map(|(index, (_, key, values))| Blank {
            id: id("blank", index),
            label: escape_html(&format!("[{key}]")),
            answers: unique_answers(values),
            match_mode: MATCH_MODE,
            max_length: MAX_LENGTH,
        })
        .collect())
}

fn unique_answers(answers: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    answers
        .iter()
        .filter(|answer| seen.insert(answer.as_str()))
        .cloned()
        .collect()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn id(prefix: &str, zero_based_index: usize) -> String {
    format!("{prefix}-{}", zero_based_index + 1)
}

fn invalid(item: &ItemRenderView, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: ENGINE,
        format: FORMAT,
        message: format!("item {}: {}", item.common().item_number, message.into()),
    }
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
