//! Blackboard MULTI_FIB and MATCH item-body recovery.

use std::collections::BTreeMap;

use qti_core::ItemBody;

use super::discovery::{Node, child, descendants, text};
use super::types_core::smart_text;

pub(super) fn multi_fib_body(element: &Node) -> Result<ItemBody, String> {
    let correct = descendants(element, "respcondition")
        .find(|condition| {
            condition
                .attributes
                .get("title")
                .is_some_and(|title| title == "correct")
        })
        .ok_or_else(|| "MULTI_FIB item has no correct branch".to_owned())?;
    let mut answers = BTreeMap::new();
    for group in descendants(correct, "or") {
        for value in descendants(group, "varequal") {
            if let Some(key) = value.attributes.get("respident") {
                answers
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(text(value));
            }
        }
    }
    (!answers.is_empty())
        .then_some(ItemBody::MultiFib { answers })
        .ok_or_else(|| "MULTI_FIB item has no answer groups".to_owned())
}

pub(super) fn match_body(
    element: &Node,
    names: &BTreeMap<String, String>,
) -> Result<ItemBody, String> {
    let presentation = child(element, "presentation")
        .ok_or_else(|| "MATCH item has no presentation".to_owned())?;
    let right = descendants(presentation, "flow")
        .filter(|flow| {
            flow.attributes
                .get("class")
                .is_some_and(|class| class == "RIGHT_MATCH_BLOCK")
        })
        .flat_map(|flow| flow.children.iter().filter(|child| child.name == "flow"))
        .map(|flow| smart_text(flow, names))
        .collect::<Result<Vec<_>, _>>()?;
    let correct = descendants(element, "respcondition")
        .filter(|condition| {
            !condition
                .attributes
                .get("title")
                .is_some_and(|title| title == "incorrect")
        })
        .flat_map(|condition| descendants(condition, "varequal"))
        .filter_map(|value| {
            value
                .attributes
                .get("respident")
                .map(|id| (id.clone(), text(value)))
        })
        .collect::<BTreeMap<_, _>>();
    let mut prompts = Vec::new();
    let mut choices = Vec::new();
    for flow in descendants(presentation, "flow").filter(|flow| {
        flow.attributes
            .get("class")
            .is_some_and(|class| class == "Block")
            && child(flow, "response_lid").is_some()
    }) {
        let response = child(flow, "response_lid").expect("filtered response lid");
        let prompt = flow
            .children
            .iter()
            .filter(|child| child.name == "flow")
            .find(|child| {
                child
                    .attributes
                    .get("class")
                    .is_some_and(|class| class == "FORMATTED_TEXT_BLOCK")
            })
            .ok_or_else(|| "MATCH prompt has no text".to_owned())?;
        let labels = descendants(response, "response_label").collect::<Vec<_>>();
        let answer = correct
            .get(
                response
                    .attributes
                    .get("ident")
                    .ok_or_else(|| "MATCH prompt lacks ident".to_owned())?,
            )
            .ok_or_else(|| "MATCH prompt has no scoring answer".to_owned())?;
        let index = labels
            .iter()
            .position(|label| label.attributes.get("ident") == Some(answer))
            .ok_or_else(|| "MATCH answer does not name a label".to_owned())?;
        prompts.push(smart_text(prompt, names)?);
        choices.push(
            right
                .get(index)
                .ok_or_else(|| "MATCH answer index exceeds choices".to_owned())?
                .clone(),
        );
    }
    if prompts.is_empty() {
        return Err("MATCH has no prompts".to_owned());
    }
    Ok(ItemBody::Match { prompts, choices })
}
