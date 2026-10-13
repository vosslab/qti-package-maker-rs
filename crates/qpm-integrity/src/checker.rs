//! Format-aware checks over an already safe in-memory package map.

use std::collections::BTreeMap;

use crate::input;
use crate::types::{Provenance, Violation};

mod blackboard;
mod grading;
mod identifiers;
mod image_dimensions;
mod manifest;
mod media;
mod xml;

#[cfg(test)]
#[path = "checker_tests.rs"]
mod checker_tests;

/// Check package entries after validating their logical paths, count, and sizes.
///
/// The map owns its byte vectors; this checker only borrows them and has no
/// dependency on a package producer. XML is parsed with `quick-xml`, whose
/// non-resolving parser prevents external entity expansion (ASVS 1.5.1).
pub fn check_entries(entries: &BTreeMap<String, Vec<u8>>) -> Vec<Violation> {
    if let Err(violation) = input::validate_entries(entries) {
        return vec![violation];
    }
    let Some(manifest_bytes) = entries.get("imsmanifest.xml") else {
        return vec![error(
            "missing-manifest",
            "imsmanifest.xml",
            "imsmanifest.xml is missing from package",
            Provenance::FormatRequirement,
        )];
    };
    let manifest = match xml::parse_xml("imsmanifest.xml", manifest_bytes) {
        Ok(root) => root,
        Err(violation) => return vec![violation],
    };

    let mut violations = Vec::new();
    xml::check_relevant_xml(entries, &mut violations);
    let mut manifest_nodes = Vec::new();
    xml::all_nodes(&manifest, &mut manifest_nodes);
    let blackboard_export = manifest_nodes.iter().any(|node| {
        node.attribute_namespaces.iter().any(|(name, namespace)| {
            xml::local_name(name) == "file"
                && namespace.as_deref() == Some(blackboard::BB_NAMESPACE)
        })
    });
    if blackboard_export {
        blackboard::check_blackboard_export(entries, &manifest, &mut violations);
    } else {
        let resources = manifest::collect_resources(&manifest, &mut violations);
        manifest::check_manifest(entries, &resources, &mut violations);
        grading::check_answer_linkage(entries, &mut violations);
        media::check_media_sources(entries, &resources, &mut violations);
    }
    image_dimensions::check_entries(entries, &mut violations);
    identifiers::check_identifier_safety(entries, &manifest, &mut violations);
    identifiers::check_score_outcomes(entries, &mut violations);
    violations
}

fn error(
    code: &'static str,
    path: impl Into<String>,
    message: impl Into<String>,
    provenance: Provenance,
) -> Violation {
    Violation::error(code, provenance, path, message)
}
