//! Item validation, including safe XML well-formedness checks for HTML-bearing fields.

use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use regex::Regex;
use thiserror::Error;

use crate::crc::CrcError;
use crate::item::ItemBody;

/// Reasons an assessment item cannot enter the validated domain model.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ValidationError {
    /// A required text field is blank or shorter than its required length.
    #[error("{field} must contain at least {minimum} non-whitespace characters; found {found}")]
    EmptyField {
        field: &'static str,
        minimum: usize,
        found: usize,
    },
    /// A list lacks enough items for its item type.
    #[error("{field} must have at least {minimum} items; found {found}")]
    TooFewItems {
        field: &'static str,
        minimum: usize,
        found: usize,
    },
    /// A list contains duplicate strings.
    #[error("{field} cannot contain duplicate items")]
    DuplicateItems { field: &'static str },
    /// A declared answer is not available among the choices.
    #[error("one or more correct answers are not in choices")]
    AnswerAbsentFromChoices,
    /// Multiple-answer items that require a distractor have none.
    #[error("multiple-answer item must contain at least one non-answer choice")]
    AllCorrectNotAllowed,
    /// A MULTI_FIB answer key has no corresponding bracketed question placeholder.
    #[error("MULTI_FIB key [{key}] is absent from question text")]
    MultiFibKeyAbsent { key: String },
    /// Matching items have more prompts than available choices.
    #[error("MATCH has {prompts} prompts but only {choices} choices")]
    PromptsExceedChoices { prompts: usize, choices: usize },
    /// A numeric tolerance is below zero.
    #[error("numeric tolerance must be non-negative; found {tolerance}")]
    NegativeTolerance { tolerance: f64 },
    /// A numeric answer is `NaN` or infinite.
    #[error("numeric answer must be finite")]
    NonFiniteAnswer,
    /// A numeric tolerance is `NaN` or infinite.
    #[error("numeric tolerance must be finite")]
    NonFiniteTolerance,
    /// An HTML-bearing field cannot be parsed as XML after the Python-compatible cleanup pass.
    #[error("{field} is not well-formed XML-compatible HTML: {reason}")]
    MalformedHtml { field: &'static str, reason: String },
    /// ASCII-only CRC generation rejected item identity input.
    #[error(transparent)]
    Crc(#[from] CrcError),
}

/// Validates a fully normalized item body and question stem.
pub fn validate_item(question_text: &str, body: &ItemBody) -> Result<(), ValidationError> {
    validate_string(question_text, "question_text", 3)?;
    match body {
        ItemBody::Mc { choices, answer } => {
            validate_list(choices, "choices", 2)?;
            validate_string(answer, "answer", 1)?;
            if !choices.contains(answer) {
                return Err(ValidationError::AnswerAbsentFromChoices);
            }
        }
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => {
            validate_list(choices, "choices", 3)?;
            validate_list(answers, "answers", *min_answers_required)?;
            if !*allow_all_correct
                && choices.iter().collect::<std::collections::BTreeSet<_>>()
                    == answers.iter().collect::<std::collections::BTreeSet<_>>()
            {
                return Err(ValidationError::AllCorrectNotAllowed);
            }
            if answers.iter().any(|answer| !choices.contains(answer)) {
                return Err(ValidationError::AnswerAbsentFromChoices);
            }
        }
        ItemBody::Match { prompts, choices } => {
            validate_list(prompts, "prompts", 2)?;
            validate_list(choices, "choices", 2)?;
            if prompts.len() > choices.len() {
                return Err(ValidationError::PromptsExceedChoices {
                    prompts: prompts.len(),
                    choices: choices.len(),
                });
            }
        }
        ItemBody::Num {
            answer, tolerance, ..
        } => {
            if !answer.is_finite() {
                return Err(ValidationError::NonFiniteAnswer);
            }
            if !tolerance.is_finite() {
                return Err(ValidationError::NonFiniteTolerance);
            }
            if *tolerance < 0.0 {
                return Err(ValidationError::NegativeTolerance {
                    tolerance: *tolerance,
                });
            }
        }
        ItemBody::Fib { answers } => validate_list(answers, "answers", 1)?,
        ItemBody::MultiFib { answers } => {
            if answers.is_empty() {
                return Err(ValidationError::TooFewItems {
                    field: "answers",
                    minimum: 1,
                    found: 0,
                });
            }
            for (key, values) in answers {
                if !question_text.contains(&format!("[{key}]")) {
                    return Err(ValidationError::MultiFibKeyAbsent { key: key.clone() });
                }
                if values.is_empty() {
                    return Err(ValidationError::TooFewItems {
                        field: "MULTI_FIB answer values",
                        minimum: 1,
                        found: 0,
                    });
                }
                for value in values {
                    validate_string(value, "MULTI_FIB answer values", 1)?;
                }
            }
        }
        ItemBody::Order { answers } => validate_list(answers, "answers", 3)?,
    }
    Ok(())
}

/// Applies the Python validator's conservative cleanup before XML parsing.
#[must_use]
pub fn clean_html_for_xml(html: &str) -> String {
    let script =
        Regex::new(r"(?is)<script\b[^>]*>.*?</script>").expect("static script regex is valid");
    let script_open = Regex::new(r"(?i)<script[^>]*>").expect("static script-open regex is valid");
    let numeric_attribute = Regex::new(
        r#"(?i)(\b(?:colspan|rowspan|width|height|size)\s*=\s*)(\d+)([^\"A-Za-z0-9_]|$)"#,
    )
    .expect("static numeric-attribute regex is valid");
    let entity = Regex::new(r"&[#a-zA-Z0-9]+;").expect("static entity regex is valid");
    let href_query = Regex::new(r#"(?i)(href=['\"])(https?://[^'\"]+?)\?.*?(['\"])"#)
        .expect("static href regex is valid");
    let smiles = Regex::new(r#"(?i)smiles=\"[^\"]*?\""#).expect("static smiles regex is valid");

    let html = script.replace_all(html, "<script></script>");
    let html = script_open.replace_all(&html, "<script>");
    let html = numeric_attribute.replace_all(&html, "$1\"$2\"$3");
    let html = entity.replace_all(&html, "");
    let html = href_query.replace_all(&html, "$1$2$3");
    smiles.replace_all(&html, "smiles=\"\"").trim().to_owned()
}

/// Checks XML well-formedness without loading entities or executing content.
///
/// DTD declarations are rejected before entity expansion. This keeps item validation a pure parse
/// operation and prevents external-entity access (ASVS 1.5.1 and 2.2.1).
pub fn validate_html(html: &str) -> Result<(), String> {
    let cleaned = clean_html_for_xml(html);
    if cleaned
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\t' | '\n' | '\r'))
    {
        return Err("XML 1.0 forbids this control character".to_owned());
    }
    let wrapped = format!("<root><cleaned>{cleaned}</cleaned></root>");
    let mut reader = Reader::from_str(&wrapped);
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = true;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return Ok(()),
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                for attribute in event.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|error| error.to_string())?;
                    attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|error| error.to_string())?;
                }
            }
            Ok(Event::Text(event)) => {
                let decoded = event.xml_content(XmlVersion::Implicit1_0);
                unescape(&decoded).map_err(|error| error.to_string())?;
            }
            Ok(Event::DocType(_)) => return Err("DTD declarations are not allowed".to_owned()),
            Ok(_) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn validate_string(
    value: &str,
    field: &'static str,
    minimum: usize,
) -> Result<(), ValidationError> {
    let found = value.trim().len();
    if found < minimum {
        return Err(ValidationError::EmptyField {
            field,
            minimum,
            found,
        });
    }
    validate_html(value).map_err(|reason| ValidationError::MalformedHtml { field, reason })
}

fn validate_list(
    values: &[String],
    field: &'static str,
    minimum: usize,
) -> Result<(), ValidationError> {
    if values.len() < minimum {
        return Err(ValidationError::TooFewItems {
            field,
            minimum,
            found: values.len(),
        });
    }
    for value in values {
        validate_string(value, field, 1)?;
    }
    if values
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != values.len()
    {
        return Err(ValidationError::DuplicateItems { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ValidationError, clean_html_for_xml, validate_html, validate_item};
    use crate::item::ItemBody;
    use std::collections::BTreeMap;

    #[test]
    fn cleanup_matches_python_html_subset() {
        assert_eq!(clean_html_for_xml("simple&copy;"), "simple");
        assert_eq!(
            clean_html_for_xml("<script>let i=0;</script>"),
            "<script></script>"
        );
        assert_eq!(
            clean_html_for_xml("<th colspan=2>Header</th>"),
            "<th colspan=\"2\">Header</th>"
        );
        assert_eq!(
            clean_html_for_xml("<a href=\"https://x.com/page?q=123\">Link</a>"),
            "<a href=\"https://x.com/page\">Link</a>"
        );
        assert_eq!(
            clean_html_for_xml("smiles=\"C[C@H](N)C(=O)O\""),
            "smiles=\"\""
        );
    }

    #[test]
    fn html_parser_rejects_unclosed_tags_without_executing_script() {
        assert!(validate_html("<p>open").is_err());
        assert!(validate_html("<script>malicious()</script>").is_ok());
    }

    #[test]
    fn html_parser_forces_xml_attribute_and_entity_validation() {
        for malformed in [
            "<img src=x/>",
            "<p a=\"1\" a=\"2\">x</p>",
            "a & b",
            "a\u{1}b",
            "<!DOCTYPE root [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><p>&x;</p>",
        ] {
            assert!(
                validate_html(malformed).is_err(),
                "{malformed:?} should fail"
            );
        }
    }

    #[test]
    fn validation_errors_are_distinct() {
        let stem = "Valid stem";
        assert!(matches!(
            validate_item(
                " ",
                &ItemBody::Fib {
                    answers: vec!["a".into()]
                }
            ),
            Err(ValidationError::EmptyField { .. })
        ));
        assert!(matches!(
            validate_item(stem, &ItemBody::Fib { answers: vec![] }),
            Err(ValidationError::TooFewItems { .. })
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Fib {
                    answers: vec!["a".into(), "a".into()]
                }
            ),
            Err(ValidationError::DuplicateItems { .. })
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Mc {
                    choices: vec!["a".into(), "b".into()],
                    answer: "c".into()
                }
            ),
            Err(ValidationError::AnswerAbsentFromChoices)
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Ma {
                    choices: vec!["a".into(), "b".into(), "c".into()],
                    answers: vec!["a".into(), "b".into(), "c".into()],
                    min_answers_required: 1,
                    allow_all_correct: false
                }
            ),
            Err(ValidationError::AllCorrectNotAllowed)
        ));
        let mut map = BTreeMap::new();
        map.insert("blank".to_owned(), vec!["a".to_owned()]);
        assert!(matches!(
            validate_item(stem, &ItemBody::MultiFib { answers: map }),
            Err(ValidationError::MultiFibKeyAbsent { .. })
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Match {
                    prompts: vec!["a".into(), "b".into(), "c".into()],
                    choices: vec!["a".into(), "b".into()]
                }
            ),
            Err(ValidationError::PromptsExceedChoices { .. })
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Num {
                    answer: f64::NAN,
                    tolerance: 0.0,
                    tolerance_message: true
                }
            ),
            Err(ValidationError::NonFiniteAnswer)
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Num {
                    answer: 1.0,
                    tolerance: f64::INFINITY,
                    tolerance_message: true
                }
            ),
            Err(ValidationError::NonFiniteTolerance)
        ));
        assert!(matches!(
            validate_item(
                stem,
                &ItemBody::Num {
                    answer: 1.0,
                    tolerance: -0.1,
                    tolerance_message: true
                }
            ),
            Err(ValidationError::NegativeTolerance { .. })
        ));
        assert!(matches!(
            validate_item(
                "<p>open",
                &ItemBody::Fib {
                    answers: vec!["a".into()]
                }
            ),
            Err(ValidationError::MalformedHtml { .. })
        ));
    }
}
