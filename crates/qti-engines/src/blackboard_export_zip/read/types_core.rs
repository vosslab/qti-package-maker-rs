//! Blackboard pool item conversion into qti-core item bodies.

use std::collections::BTreeMap;

use qti_core::{Item, ItemBody};
use regex::Regex;

use super::discovery::{Node, child, descendants, text};

const MA_MINIMUM: &str = "bbmd_qti_package_maker_ma_min_answers_required";
const MA_ALLOW_ALL: &str = "bbmd_qti_package_maker_ma_allow_all_correct";
const NUM_TOLERANCE: &str = "bbmd_qti_package_maker_num_tolerance";
const NUM_TOLERANCE_MESSAGE: &str = "bbmd_qti_package_maker_num_tolerance_message";

pub(super) fn parse_item(
    element: &Node,
    names: &BTreeMap<String, String>,
) -> Result<Option<Item>, String> {
    let question_type = descendants(element, "bbmd_questiontype")
        .next()
        .map(text)
        .unwrap_or_default();
    if !matches!(
        question_type.as_str(),
        "Multiple Choice"
            | "Multiple Answer"
            | "Fill in the Blank"
            | "Fill in the Blank Plus"
            | "Numeric"
            | "Matching"
            | "True/False"
    ) {
        return Ok(None);
    }
    let question = question_html(element, names)?;
    let body = match question_type.as_str() {
        "Multiple Choice" | "Multiple Answer" | "True/False" => choice_body(element, names)?,
        "Fill in the Blank" => fib_body(element)?,
        "Fill in the Blank Plus" => super::types_extra::multi_fib_body(element)?,
        "Numeric" => numeric_body(element)?,
        "Matching" => super::types_extra::match_body(element, names)?,
        _ => unreachable!(),
    };
    Item::new(question, body)
        .map_err(|error| error.to_string())
        .map(Some)
}

fn question_html(element: &Node, names: &BTreeMap<String, String>) -> Result<String, String> {
    let flow = descendants(element, "flow")
        .find(|flow| {
            flow.attributes
                .get("class")
                .is_some_and(|class| class == "QUESTION_BLOCK")
        })
        .ok_or_else(|| "no QUESTION_BLOCK flow".to_owned())?;
    smart_text(flow, names)
}

pub(super) fn smart_text(
    element: &Node,
    names: &BTreeMap<String, String>,
) -> Result<String, String> {
    let source = descendants(element, "mat_formattedtext")
        .next()
        .map(|node| node.text.clone())
        .unwrap_or_default();
    if source.is_empty() {
        return Err("no mat_formattedtext element".to_owned());
    }
    // Blackboard's SMART_TEXT payload is entity-escaped inside the outer XML.
    // Some exports retain that layer after XML text decoding, so decode it here
    // before looking for image tokens or repairing HTML void elements.
    let mut rewritten = quick_xml::escape::unescape(&source)
        .map_err(|error| error.to_string())?
        .into_owned();
    for (token, name) in names {
        rewritten = rewritten.replace(token, name);
    }
    Ok(repair_void_html(&rewritten))
}

/// Blackboard stores HTML fragments, including ordinary HTML void tags, in a
/// QTI text node.  Core items deliberately validate XML-compatible HTML, so
/// make the two void tags used by exported pool content explicit before item
/// construction.  Other markup is retained verbatim for the core validator.
fn repair_void_html(html: &str) -> String {
    let void = Regex::new(r"(?i)<(br|img)([^>]*)>").expect("static void-tag regex is valid");
    void.replace_all(html, |captures: &regex::Captures<'_>| {
        let suffix = captures.get(2).map_or("", |value| value.as_str());
        if suffix.trim_end().ends_with('/') {
            captures[0].to_owned()
        } else {
            format!("<{}{} />", &captures[1], suffix)
        }
    })
    .into_owned()
}

fn choice_body(element: &Node, names: &BTreeMap<String, String>) -> Result<ItemBody, String> {
    let response = descendants(element, "response_lid")
        .next()
        .ok_or_else(|| "choice item has no response_lid".to_owned())?;
    let labels = descendants(response, "response_label").collect::<Vec<_>>();
    let choices = labels
        .iter()
        .map(|label| smart_text(label, names))
        .collect::<Result<Vec<_>, _>>()?;
    let correct = correct_varequals(element);
    let answers = correct
        .iter()
        .filter_map(|ident| {
            labels
                .iter()
                .position(|label| label.attributes.get("ident") == Some(ident))
                .map(|index| choices[index].clone())
        })
        .collect::<Vec<_>>();
    if answers.is_empty() {
        return Err("correct choice ids do not name a response label".to_owned());
    }
    if response
        .attributes
        .get("rcardinality")
        .is_some_and(|value| value == "Multiple")
        || answers.len() > 1
    {
        let min_answers_required = metadata_i64(element, MA_MINIMUM)?.unwrap_or(1);
        let allow_all_correct = metadata_bool(element, MA_ALLOW_ALL)?.unwrap_or(true);
        Ok(ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        })
    } else {
        Ok(ItemBody::Mc {
            choices,
            answer: answers[0].clone(),
        })
    }
}

fn correct_varequals(element: &Node) -> Vec<String> {
    fn collect_positive_varequals(node: &Node, values: &mut Vec<String>) {
        for child in &node.children {
            // Blackboard MA marks every non-answer choice by wrapping its
            // varequal in <not>.  Descending into it would turn distractors
            // into accepted answers on a writer-reader round trip.
            if child.name == "not" {
                continue;
            }
            if child.name == "varequal" {
                let value = text(child);
                if !value.is_empty() {
                    values.push(value);
                }
            }
            collect_positive_varequals(child, values);
        }
    }

    let mut values = Vec::new();
    for condition in descendants(element, "respcondition").filter(|condition| {
        condition
            .attributes
            .get("title")
            .is_some_and(|title| title == "correct")
    }) {
        collect_positive_varequals(condition, &mut values);
    }
    values
}

fn fib_body(element: &Node) -> Result<ItemBody, String> {
    let answers = descendants(element, "respcondition")
        .filter(|condition| {
            !condition
                .attributes
                .get("title")
                .is_some_and(|title| title == "incorrect")
        })
        .flat_map(|condition| descendants(condition, "varequal"))
        .filter(|value| {
            value
                .attributes
                .get("respident")
                .is_some_and(|id| id == "response")
        })
        .map(text)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    (!answers.is_empty())
        .then_some(ItemBody::Fib { answers })
        .ok_or_else(|| "FIB item has no accepted answers".to_owned())
}

fn numeric_body(element: &Node) -> Result<ItemBody, String> {
    let condition = descendants(element, "respcondition")
        .find(|condition| {
            descendants(condition, "vargte").next().is_some()
                || descendants(condition, "varequal").next().is_some()
        })
        .ok_or_else(|| "NUM item has no correct condition".to_owned())?;
    let answer = descendants(condition, "varequal")
        .next()
        .map(text)
        .ok_or_else(|| "NUM item has no answer".to_owned())?
        .parse::<f64>()
        .map_err(|_| "NUM answer is not a float".to_owned())?;
    let lower = descendants(condition, "vargte")
        .next()
        .map(text)
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| "NUM lower bound is not a float".to_owned())
        })
        .transpose()?;
    let upper = descendants(condition, "varlte")
        .next()
        .map(text)
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| "NUM upper bound is not a float".to_owned())
        })
        .transpose()?;
    let tolerance = metadata_float(element, NUM_TOLERANCE)?.unwrap_or_else(|| {
        lower
            .zip(upper)
            .map_or(0.0, |(lower, upper)| (upper - lower) / 2.0)
    });
    Ok(ItemBody::Num {
        answer,
        tolerance,
        tolerance_message: metadata_bool(element, NUM_TOLERANCE_MESSAGE)?.unwrap_or(true),
    })
}

fn metadata_value(element: &Node, name: &str) -> Option<String> {
    child(element, "itemmetadata")
        .and_then(|metadata| child(metadata, name))
        .map(text)
}

fn metadata_i64(element: &Node, name: &str) -> Result<Option<i64>, String> {
    metadata_value(element, name)
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| format!("{name} metadata is not an integer"))
        })
        .transpose()
}

fn metadata_float(element: &Node, name: &str) -> Result<Option<f64>, String> {
    metadata_value(element, name)
        .map(|value| {
            value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{name} metadata is not a finite float"))
        })
        .transpose()
}

fn metadata_bool(element: &Node, name: &str) -> Result<Option<bool>, String> {
    metadata_value(element, name)
        .map(|value| match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("{name} metadata is not a boolean")),
        })
        .transpose()
}
