//! Item validation, including safe XML well-formedness checks for HTML-bearing fields.

use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

use crate::crc::CrcError;
use crate::item::ItemBody;
use crate::strings::python_whitespace;

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
            // Python accepts negative minima; the list must still contain an answer.
            let minimum = usize::try_from((*min_answers_required).max(1)).unwrap_or(usize::MAX);
            validate_list(answers, "answers", minimum)?;
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
                    // Python validates these as answer text, rather than HTML-bearing fields.
                    validate_text(value, "MULTI_FIB answer values", 1)?;
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
    static SCRIPT_OPEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<script[^>]*>").expect("static script-open regex is valid"));
    static SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<script\b[^>]*>.*?</script>").expect("static script regex is valid")
    });
    static NUMERIC_ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"((?:colspan|rowspan|width|height|size)[\s\x1c-\x1f]*=[\s\x1c-\x1f]*)(\d+)")
            .expect("static numeric-attribute regex is valid")
    });
    static ENTITY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"&[#a-zA-Z0-9]+;").expect("static entity regex is valid"));
    static HREF_QUERY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(href=['"])(https?://[^'"]+?)\?.*?(['"])"#)
            .expect("static href regex is valid")
    });
    static SMILES: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"smiles="[^"]*?""#).expect("static smiles regex is valid"));

    // Preserve Python's order, case sensitivity, and single-line script matching.
    let html = SCRIPT_OPEN.replace_all(html, "<script>");
    let html = SCRIPT.replace_all(&html, "<script></script>");
    let html = NUMERIC_ATTRIBUTE.replace_all(&html, |captures: &regex::Captures<'_>| {
        let matched = captures.get(0).expect("whole numeric attribute capture");
        // Python's (?!["\w]) leaves already quoted and unit-suffixed numbers untouched.
        // Inspect the following character without consuming a later attribute's boundary.
        if html[..matched.start()]
            .chars()
            .next_back()
            .is_some_and(|c| c == '_' || c.is_alphanumeric())
            || html[matched.end()..]
                .chars()
                .next()
                .is_some_and(|c| c == '"' || c == '_' || c.is_alphanumeric())
        {
            matched.as_str().to_owned()
        } else {
            format!("{}\"{}\"", &captures[1], &captures[2])
        }
    });
    let html = ENTITY.replace_all(&html, "");
    let html = HREF_QUERY.replace_all(&html, "$1$2$3");
    SMILES
        .replace_all(&html, "smiles=\"\"")
        .trim_matches(python_whitespace)
        .to_owned()
}

/// Checks the cleaned fragment with Python/lxml's XML document boundary.
pub fn validate_html(html: &str) -> Result<(), String> {
    crate::xml_validation::validate_fragment(&clean_html_for_xml(html))
}

fn validate_string(
    value: &str,
    field: &'static str,
    minimum: usize,
) -> Result<(), ValidationError> {
    validate_text(value, field, minimum)?;
    validate_html(value).map_err(|reason| ValidationError::MalformedHtml { field, reason })
}

fn validate_text(value: &str, field: &'static str, minimum: usize) -> Result<(), ValidationError> {
    let found = value.trim_matches(python_whitespace).chars().count();
    if found < minimum {
        return Err(ValidationError::EmptyField {
            field,
            minimum,
            found,
        });
    }
    Ok(())
}

fn validate_list(
    values: &[String],
    field: &'static str,
    minimum: usize,
) -> Result<(), ValidationError> {
    if values.is_empty() || values.len() < minimum {
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
            "<p a=\"1\"b=\"2\">x</p>",
            "<1bad>Text</1bad>",
            "<x:p/>",
            "<p xmlns:x=\"urn:x\" xmlns:y=\"urn:x\" x:a=\"1\" y:a=\"2\"/>",
            "<p xml:id=\"bad id\"/>",
            "<p xml:id=\"same\"/><p xml:id=\"same\"/>",
            "<!-- bad -- comment -->",
            "x ]]> y",
            "<?xml version=\"1.0\"?>",
            "a & b",
            "a\u{1}b",
            "<!DOCTYPE root [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><p>&x;</p>",
        ] {
            assert!(
                validate_html(malformed).is_err(),
                "{malformed:?} should fail"
            );
        }
        for valid in [
            "<p xmlns:x=\"urn:x\"><x:span xml:id=\"label\">Text</x:span></p>",
            "<![CDATA[x < y & z]]>",
            "Text\u{7f}",
        ] {
            assert!(validate_html(valid).is_ok(), "{valid:?} should pass");
        }
    }

    #[test]
    fn html_cleanup_preserves_python_case_and_line_boundaries() {
        for valid in [
            "<td colspan=2 rowspan=3>Cell</td>",
            "<script>if (a < b) run();</script>",
            "<p smiles=\"C<C&N\"/>",
        ] {
            assert!(validate_html(valid).is_ok(), "{valid:?} should pass");
        }
        for invalid in [
            "<td COLSPAN=2>Cell</td>",
            "<script>\nif (a < b) run();\n</script>",
            "<SCRIPT>if (a < b) run();</SCRIPT>",
            "<p SMILES=\"C<C&N\"/>",
        ] {
            assert!(validate_html(invalid).is_err(), "{invalid:?} should fail");
        }
    }

    #[test]
    fn multifib_answers_are_nonempty_text_while_fib_answers_are_unique_html() {
        let stem = "Complete [blank].";
        let multi = |values| ItemBody::MultiFib {
            answers: BTreeMap::from([("blank".into(), values)]),
        };
        let values = vec!["x < 5".into(), "A & B".into(), "A & B".into()];
        assert!(validate_item(stem, &multi(values.clone())).is_ok());
        assert!(validate_item(stem, &ItemBody::Fib { answers: values }).is_err());
        for values in [vec![], vec!["".into()], vec!["\u{1c} \t".into()]] {
            assert!(validate_item(stem, &multi(values)).is_err());
        }
    }

    #[test]
    fn ma_minimum_preserves_python_nonempty_answer_rule() {
        for minimum in [-2, 0, 1, 2] {
            let mut body = ItemBody::Ma {
                choices: vec!["one".into(), "two".into(), "three".into()],
                answers: vec![],
                min_answers_required: minimum,
                allow_all_correct: true,
            };
            assert!(validate_item("A question", &body).is_err());
            if let ItemBody::Ma { answers, .. } = &mut body {
                answers.push("one".into());
            }
            assert_eq!(validate_item("A question", &body).is_ok(), minimum <= 1);
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
