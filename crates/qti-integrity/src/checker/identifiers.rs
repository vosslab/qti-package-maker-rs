//! Identifier safety and Blackboard SCORE outcome checks.

use std::collections::BTreeMap;

use crate::types::{Provenance, Violation};

use super::error;
use super::xml::{self, QTI12_NAMESPACE, QTI21_NAMESPACE, XmlNode};

pub(super) fn check_identifier_safety(
    entries: &BTreeMap<String, Vec<u8>>,
    manifest: &XmlNode,
    violations: &mut Vec<Violation>,
) {
    check_identifier(
        "imsmanifest.xml",
        "manifest identifier",
        manifest.attributes.get("identifier"),
        violations,
    );
    let mut resources = Vec::new();
    let mut dependencies = Vec::new();
    xml::descendants(manifest, "resource", &mut resources);
    xml::descendants(manifest, "dependency", &mut dependencies);
    for resource in resources {
        check_identifier(
            "imsmanifest.xml",
            "resource identifier",
            resource.attributes.get("identifier"),
            violations,
        );
    }
    for dependency in dependencies {
        check_identifier(
            "imsmanifest.xml",
            "dependency identifierref",
            dependency.attributes.get("identifierref"),
            violations,
        );
    }
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let Ok(root) = xml::parse_xml(path, data) else {
            continue;
        };
        if root.name == "assessmentItem" && root.namespace.as_deref() == Some(QTI21_NAMESPACE) {
            check_identifier(
                path,
                "assessmentItem identifier",
                root.attributes.get("identifier"),
                violations,
            );
        } else if root.name == "questestinterop"
            && root.namespace.as_deref() == Some(QTI12_NAMESPACE)
        {
            let mut items = Vec::new();
            xml::descendants(&root, "item", &mut items);
            for item in items {
                check_identifier(path, "item ident", item.attributes.get("ident"), violations);
            }
        }
    }
}

fn check_identifier(
    path: &str,
    label: &str,
    value: Option<&String>,
    violations: &mut Vec<Violation>,
) {
    let Some(value) = value else {
        return;
    };
    let safe = value
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if !safe {
        violations.push(error(
            "unsafe-identifier",
            path,
            format!("{label} '{value}' is not an id-safe token"),
            Provenance::BlackboardImportFailure,
        ));
    }
}

pub(super) fn check_score_outcomes(
    entries: &BTreeMap<String, Vec<u8>>,
    violations: &mut Vec<Violation>,
) {
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let Ok(root) = xml::parse_xml(path, data) else {
            continue;
        };
        if root.name != "assessmentItem" || root.namespace.as_deref() != Some(QTI21_NAMESPACE) {
            continue;
        }
        let mut declarations = Vec::new();
        xml::namespaced_descendants(
            &root,
            "outcomeDeclaration",
            QTI21_NAMESPACE,
            &mut declarations,
        );
        if !declarations.iter().any(|declaration| {
            declaration
                .attributes
                .get("identifier")
                .is_some_and(|id| id == "SCORE")
        }) {
            let item = root
                .attributes
                .get("identifier")
                .map_or("<no-ident>", String::as_str);
            violations.push(error(
                "missing-score-outcome",
                path,
                format!("assessmentItem '{item}' has no outcomeDeclaration identifier='SCORE'"),
                Provenance::BlackboardImportFailure,
            ));
        }
    }
}
