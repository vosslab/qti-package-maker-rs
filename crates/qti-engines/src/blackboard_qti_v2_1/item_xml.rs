//! Blackboard QTI 2.1 assessment-item XML for supported question kinds.

use qti_core::{ItemBody, ItemRenderView};

use super::NAME;
use super::fragment::{fragment, plain_text, xml, xml_attr};
use crate::EngineError;

pub(super) const QTI_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1";
pub(super) const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
pub(super) const QTI_SCHEMA: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1 http://www.imsglobal.org/xsd/qti/qtiv2p1/imsqti_v2p1.xsd";

pub(super) struct RenderedItem {
    pub(super) xml: String,
    pub(super) crc: qti_core::ItemCrc,
}

pub(super) fn render_item(item: &ItemRenderView) -> Result<Option<RenderedItem>, EngineError> {
    let xml = match item.body() {
        ItemBody::Mc { choices, answer } => {
            choice_item(item, choices, std::slice::from_ref(answer), 1)?
        }
        ItemBody::Ma {
            choices, answers, ..
        } => choice_item(item, choices, answers, answers.len())?,
        ItemBody::Match { prompts, choices } => match_item(item, prompts, choices),
        ItemBody::Num {
            answer, tolerance, ..
        } => numeric_item(item, *answer, *tolerance),
        ItemBody::Fib { answers } => fib_item(item, answers),
        ItemBody::MultiFib { answers } => multi_fib_item(item, answers),
        ItemBody::Order { answers } => order_item(item, answers),
    };
    Ok(Some(RenderedItem {
        xml,
        crc: *item.crc(),
    }))
}

fn choice_item(
    item: &ItemRenderView,
    choices: &[String],
    answers: &[String],
    max: usize,
) -> Result<String, EngineError> {
    let identifiers = answers
        .iter()
        .map(|answer| {
            choices
                .iter()
                .position(|choice| choice == answer)
                .map(|index| format!("answer_{}", index + 1))
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "QTI 2.1",
                    message: "correct answer is absent from choices".to_owned(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let declaration = response_declaration(
        "identifier",
        if identifiers.len() == 1 {
            "single"
        } else {
            "multiple"
        },
        "RESPONSE",
        &identifiers,
        "",
    );
    let choices = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            format!(
                "<simpleChoice fixed=\"true\" identifier=\"answer_{}\"><p>{}</p></simpleChoice>",
                index + 1,
                fragment(choice)
            )
        })
        .collect::<String>();
    let body = format!(
        "<itemBody><div>{}</div><choiceInteraction maxChoices=\"{max}\" responseIdentifier=\"RESPONSE\" shuffle=\"true\">{choices}</choiceInteraction></itemBody>",
        fragment(&item.common().question_text)
    );
    Ok(assessment_item(
        item,
        &declaration,
        &body,
        &standard_processing(),
    ))
}

fn match_item(item: &ItemRenderView, prompts: &[String], choices: &[String]) -> String {
    let pairs = prompts
        .iter()
        .enumerate()
        .map(|(index, _)| format!("prompt_{:03} choice_{:03}", index + 1, index + 1))
        .collect::<Vec<_>>();
    let values = pairs
        .iter()
        .map(|pair| format!("<value>{}</value>", xml(pair)))
        .collect::<String>();
    let map_entries = pairs
        .iter()
        .map(|pair| {
            format!(
                "<mapEntry mapKey=\"{}\" mappedValue=\"1\"/>",
                xml_attr(pair)
            )
        })
        .collect::<String>();
    let declaration = format!(
        "<responseDeclaration baseType=\"directedPair\" cardinality=\"multiple\" identifier=\"RESPONSE\"><correctResponse>{values}</correctResponse><mapping defaultValue=\"0\">{map_entries}</mapping></responseDeclaration>"
    );
    let prompt_nodes = prompts.iter().enumerate().map(|(index, prompt)| format!("<simpleAssociableChoice identifier=\"prompt_{:03}\" fixed=\"true\" matchMax=\"1\" matchMin=\"0\"><p>{}</p></simpleAssociableChoice>", index + 1, fragment(prompt))).collect::<String>();
    let choice_nodes = choices.iter().enumerate().map(|(index, choice)| format!("<simpleAssociableChoice identifier=\"choice_{:03}\" fixed=\"true\" matchMax=\"{}\" matchMin=\"0\"><p>{}</p></simpleAssociableChoice>", index + 1, prompts.len(), fragment(choice))).collect::<String>();
    let stem = fragment(&item.common().question_text);
    let body = format!(
        "<itemBody><div>{stem}</div><matchInteraction responseIdentifier=\"RESPONSE\" shuffle=\"true\" maxAssociations=\"{}\"><prompt>{}</prompt><simpleMatchSet>{prompt_nodes}</simpleMatchSet><simpleMatchSet>{choice_nodes}</simpleMatchSet></matchInteraction></itemBody>",
        prompts.len(),
        plain_text(&item.common().question_text)
    );
    assessment_item(
        item,
        &declaration,
        &body,
        "<responseProcessing template=\"http://www.imsglobal.org/question/qti_v2p1/rptemplates/map_response\"/>",
    )
}

fn numeric_item(item: &ItemRenderView, answer: f64, tolerance: f64) -> String {
    let answer = number(answer);
    let tolerance = number(tolerance);
    let declaration = response_declaration("float", "single", "RESPONSE", &[answer], "");
    let body = entry_body(item, "RESPONSE");
    let processing = format!(
        "<responseProcessing><responseCondition><responseIf><equal toleranceMode=\"absolute\" tolerance=\"{tolerance} {tolerance}\" includeLowerBound=\"true\" includeUpperBound=\"true\"><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></equal><setOutcomeValue identifier=\"SCORE\"><baseValue baseType=\"float\">100</baseValue></setOutcomeValue></responseIf><responseElse><setOutcomeValue identifier=\"SCORE\"><baseValue baseType=\"float\">0</baseValue></setOutcomeValue></responseElse></responseCondition></responseProcessing>"
    );
    assessment_item(item, &declaration, &body, &processing)
}

fn fib_item(item: &ItemRenderView, answers: &[String]) -> String {
    let values = answers
        .iter()
        .map(|answer| format!("<value>{}</value>", xml(answer)))
        .collect::<String>();
    let mappings = answers
        .iter()
        .map(|answer| {
            format!(
                "<mapEntry mapKey=\"{}\" caseSensitive=\"false\" mappedValue=\"100.0\"/>",
                xml_attr(answer)
            )
        })
        .collect::<String>();
    let declaration = format!(
        "<responseDeclaration baseType=\"string\" cardinality=\"single\" identifier=\"RESPONSE\"><correctResponse>{values}</correctResponse><mapping>{mappings}</mapping></responseDeclaration>"
    );
    assessment_item(
        item,
        &declaration,
        &entry_body(item, "RESPONSE"),
        &standard_processing(),
    )
}

pub(super) fn multi_fib_item(
    item: &ItemRenderView,
    answers: &std::collections::BTreeMap<String, Vec<String>>,
) -> String {
    let declarations = answers
        .iter()
        .map(|(key, values)| response_declaration("string", "single", key, &values[..1], ""))
        .collect::<String>();
    let mut stem = item.common().question_text.clone();
    let mut processing = String::from("<responseProcessing>");
    let score = 100.0 / answers.len() as f64;
    for key in answers.keys() {
        stem = stem.replace(
            &format!("[{key}]"),
            &format!(
                "<textEntryInteraction responseIdentifier=\"{}\"/>",
                xml_attr(key)
            ),
        );
        let predicate = if answers[key].len() == 1 {
            format!(
                "<match><variable identifier=\"{}\"/><correct identifier=\"{}\"/></match>",
                xml_attr(key),
                xml_attr(key)
            )
        } else {
            let alternatives = answers[key].iter().map(|answer| {
                format!("<match><variable identifier=\"{}\"/><baseValue baseType=\"string\">{}</baseValue></match>", xml_attr(key), xml(answer))
            }).collect::<String>();
            format!("<or>{alternatives}</or>")
        };
        processing.push_str(&format!("<responseCondition><responseIf>{predicate}<setOutcomeValue identifier=\"SCORE\"><sum><variable identifier=\"SCORE\"/><baseValue baseType=\"float\">{score:.2}</baseValue></sum></setOutcomeValue></responseIf></responseCondition>"));
    }
    processing.push_str("</responseProcessing>");
    let body = format!("<itemBody><div>{}</div></itemBody>", fragment(&stem));
    assessment_item_with_score_default(item, &declarations, &body, &processing, Some("0"))
}

fn order_item(item: &ItemRenderView, answers: &[String]) -> String {
    let identifiers = (1..=answers.len())
        .map(|index| format!("choice_{index:03}"))
        .collect::<Vec<_>>();
    let declaration = response_declaration("identifier", "ordered", "RESPONSE", &identifiers, "");
    let nodes = answers
        .iter()
        .enumerate()
        .map(|(index, answer)| {
            format!(
                "<simpleChoice identifier=\"choice_{:03}\"><p>{}</p></simpleChoice>",
                index + 1,
                fragment(answer)
            )
        })
        .collect::<String>();
    let body = format!(
        "<itemBody><div>{}</div><orderInteraction responseIdentifier=\"RESPONSE\" shuffle=\"true\"><prompt>{}</prompt>{nodes}</orderInteraction></itemBody>",
        fragment(&item.common().question_text),
        plain_text(&item.common().question_text)
    );
    assessment_item(
        item,
        &declaration,
        &body,
        "<responseProcessing template=\"http://www.imsglobal.org/question/qti_v2p1/rptemplates/match_correct\"/>",
    )
}

fn assessment_item(
    item: &ItemRenderView,
    declaration: &str,
    body: &str,
    processing: &str,
) -> String {
    assessment_item_with_score_default(item, declaration, body, processing, None)
}

fn assessment_item_with_score_default(
    item: &ItemRenderView,
    declaration: &str,
    body: &str,
    processing: &str,
    score_default: Option<&str>,
) -> String {
    let identifier = format!("QUE_{}_{:04x}", item.crc(), item.common().item_number);
    let score_outcome = score_default.map_or_else(
        || {
            "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"/>"
                .to_owned()
        },
        |value| {
            format!(
                "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"><defaultValue><value>{}</value></defaultValue></outcomeDeclaration>",
                xml(value)
            )
        },
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><assessmentItem xmlns=\"{QTI_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{QTI_SCHEMA}\" title=\"{identifier}\" adaptive=\"false\" timeDependent=\"false\" identifier=\"{identifier}\">{declaration}{score_outcome}{body}{processing}</assessmentItem>"
    )
}

fn response_declaration(
    base_type: &str,
    cardinality: &str,
    identifier: &str,
    values: &[String],
    suffix: &str,
) -> String {
    let values = values
        .iter()
        .map(|value| format!("<value>{}</value>", xml(value)))
        .collect::<String>();
    format!(
        "<responseDeclaration baseType=\"{base_type}\" cardinality=\"{cardinality}\" identifier=\"{}\"><correctResponse>{values}</correctResponse>{suffix}</responseDeclaration>",
        xml_attr(identifier)
    )
}

fn entry_body(item: &ItemRenderView, identifier: &str) -> String {
    format!(
        "<itemBody><div>{}</div><p><textEntryInteraction responseIdentifier=\"{}\"/></p></itemBody>",
        fragment(&item.common().question_text),
        xml_attr(identifier)
    )
}

fn standard_processing() -> String {
    "<responseProcessing><responseCondition><responseIf><match><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></match></responseIf></responseCondition></responseProcessing>".to_owned()
}

fn number(value: f64) -> String {
    value.to_string()
}
