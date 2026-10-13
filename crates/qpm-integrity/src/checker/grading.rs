//! QTI 1.2 and 2.1 answer linkage checks.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{Provenance, Violation};

use super::error;
use super::xml::{self, QTI12_NAMESPACE, QTI21_NAMESPACE, XmlNode};

const QTI21_CHOICES: &[&str] = &[
    "simpleChoice",
    "simpleAssociableChoice",
    "gap",
    "gapText",
    "gapImg",
    "inlineChoice",
    "hottext",
    "associableHotspot",
    "hotspotChoice",
];

pub(super) fn check_answer_linkage(
    entries: &BTreeMap<String, Vec<u8>>,
    violations: &mut Vec<Violation>,
) {
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let Ok(root) = xml::parse_xml(path, data) else {
            continue;
        };
        match (root.name.as_str(), root.namespace.as_deref()) {
            ("questestinterop", Some(QTI12_NAMESPACE)) => check_qti12(path, &root, violations),
            ("assessmentItem", Some(QTI21_NAMESPACE)) => check_qti21(path, &root, violations),
            _ => {}
        }
    }
}

fn check_qti12(path: &str, root: &XmlNode, violations: &mut Vec<Violation>) {
    let mut items = Vec::new();
    xml::namespaced_descendants(root, "item", QTI12_NAMESPACE, &mut items);
    for item in items {
        let item_id = item
            .attributes
            .get("ident")
            .map_or("<no-ident>", String::as_str);
        let mut responses = Vec::new();
        for response_name in ["response_lid", "response_str", "response_grp"] {
            xml::namespaced_descendants(item, response_name, QTI12_NAMESPACE, &mut responses);
        }
        let mut choices = BTreeMap::<String, BTreeSet<String>>::new();
        for response in responses {
            let Some(response_id) = response.attributes.get("ident") else {
                continue;
            };
            let mut render_choices = Vec::new();
            xml::namespaced_descendants(
                response,
                "render_choice",
                QTI12_NAMESPACE,
                &mut render_choices,
            );
            if render_choices.is_empty() {
                continue;
            }
            let mut labels = Vec::new();
            xml::namespaced_descendants(response, "response_label", QTI12_NAMESPACE, &mut labels);
            choices.insert(
                response_id.clone(),
                labels
                    .into_iter()
                    .filter_map(|label| label.attributes.get("ident").cloned())
                    .collect(),
            );
        }
        let mut values = Vec::new();
        xml::namespaced_descendants(item, "varequal", QTI12_NAMESPACE, &mut values);
        for value in values {
            let Some(response_id) = value.attributes.get("respident") else {
                continue;
            };
            let Some(declared) = choices.get(response_id) else {
                continue;
            };
            let answer = value.text.trim();
            if !declared.contains(answer) {
                violations.push(error(
                    "qti12-dangling-varequal",
                    path,
                    format!(
                        "item '{item_id}' scored varequal '{answer}' for '{response_id}' is undeclared"
                    ),
                    Provenance::FormatRequirement,
                ));
            }
        }
    }
}

fn check_qti21(path: &str, root: &XmlNode, violations: &mut Vec<Violation>) {
    let item_id = root
        .attributes
        .get("identifier")
        .map_or("<no-ident>", String::as_str);
    let mut interactions = Vec::new();
    xml::all_nodes(root, &mut interactions);
    let mut choices = BTreeMap::<String, BTreeSet<String>>::new();
    for interaction in interactions {
        if interaction.namespace.as_deref() != Some(QTI21_NAMESPACE)
            || !interaction.name.ends_with("Interaction")
        {
            continue;
        }
        let Some(response_id) = interaction.attributes.get("responseIdentifier") else {
            continue;
        };
        let mut descendants_of_interaction = Vec::new();
        xml::all_nodes(interaction, &mut descendants_of_interaction);
        choices.insert(
            response_id.clone(),
            descendants_of_interaction
                .into_iter()
                .filter(|node| {
                    node.namespace.as_deref() == Some(QTI21_NAMESPACE)
                        && QTI21_CHOICES.contains(&node.name.as_str())
                })
                .filter_map(|node| node.attributes.get("identifier").cloned())
                .collect(),
        );
    }
    let mut declarations = Vec::new();
    xml::namespaced_descendants(
        root,
        "responseDeclaration",
        QTI21_NAMESPACE,
        &mut declarations,
    );
    for declaration in declarations {
        let Some(response_id) = declaration.attributes.get("identifier") else {
            continue;
        };
        let Some(declared) = choices
            .get(response_id)
            .filter(|declared| !declared.is_empty())
        else {
            continue;
        };
        let mut values = Vec::new();
        xml::namespaced_descendants(declaration, "value", QTI21_NAMESPACE, &mut values);
        for value in values {
            for token in value.text.split_whitespace() {
                if !declared.contains(token) {
                    violations.push(error(
                        "qti21-dangling-correct-response",
                        path,
                        format!(
                            "item '{item_id}' correctResponse token '{token}' for '{response_id}' is undeclared"
                        ),
                        Provenance::FormatRequirement,
                    ));
                }
            }
        }
    }
}
