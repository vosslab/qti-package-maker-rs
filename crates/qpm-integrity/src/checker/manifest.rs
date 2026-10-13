//! Manifest resource collection and package link validation.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{Provenance, Violation};

use super::error;
use super::xml::{self, XmlNode};

#[derive(Clone, Debug, Default)]
pub(super) struct Resource {
    pub(super) identifier: String,
    pub(super) href: Option<String>,
    pub(super) files: BTreeSet<String>,
    pub(super) dependencies: BTreeSet<String>,
}

pub(super) fn collect_resources(
    manifest: &XmlNode,
    violations: &mut Vec<Violation>,
) -> BTreeMap<String, Resource> {
    let mut resources = Vec::new();
    xml::descendants(manifest, "resource", &mut resources);
    let mut collected = BTreeMap::new();
    for node in resources {
        let Some(identifier) = node.attributes.get("identifier").cloned() else {
            violations.push(error(
                "missing-resource-identifier",
                "imsmanifest.xml",
                "resource has no identifier",
                Provenance::FormatRequirement,
            ));
            continue;
        };
        if collected.contains_key(&identifier) {
            violations.push(error(
                "duplicate-resource-identifier",
                "imsmanifest.xml",
                format!("resource identifier '{identifier}' is declared more than once"),
                Provenance::FormatRequirement,
            ));
            continue;
        }
        let href = node
            .attributes
            .get("href")
            .map(|href| xml::resolve_package_uri(&node.base, href));
        let mut files = Vec::new();
        let mut dependencies = Vec::new();
        resource_descendants(node, "file", &mut files);
        resource_descendants(node, "dependency", &mut dependencies);
        collected.insert(
            identifier.clone(),
            Resource {
                identifier,
                href,
                files: files
                    .into_iter()
                    .filter_map(|file| {
                        file.attributes
                            .get("href")
                            .map(|href| xml::resolve_package_uri(&file.base, href))
                    })
                    .collect(),
                dependencies: dependencies
                    .into_iter()
                    .filter_map(|dependency| dependency.attributes.get("identifierref").cloned())
                    .collect(),
            },
        );
    }
    collected
}

fn resource_descendants<'a>(node: &'a XmlNode, name: &'a str, output: &mut Vec<&'a XmlNode>) {
    for child in &node.children {
        if child.name == "resource" {
            continue;
        }
        if child.name == name {
            output.push(child);
        }
        resource_descendants(child, name, output);
    }
}

pub(super) fn check_manifest(
    entries: &BTreeMap<String, Vec<u8>>,
    resources: &BTreeMap<String, Resource>,
    violations: &mut Vec<Violation>,
) {
    for resource in resources.values() {
        if let Some(href) = &resource.href
            && !entries.contains_key(href)
        {
            violations.push(error(
                "dangling-resource-href",
                "imsmanifest.xml",
                format!("resource href '{href}' is missing from package"),
                Provenance::FormatRequirement,
            ));
        }
        for file in &resource.files {
            if !entries.contains_key(file) {
                violations.push(error(
                    "dangling-file-href",
                    "imsmanifest.xml",
                    format!("file href '{file}' is missing from package"),
                    Provenance::FormatRequirement,
                ));
            }
        }
        for dependency in &resource.dependencies {
            if !resources.contains_key(dependency) {
                violations.push(error(
                    "dangling-dependency",
                    "imsmanifest.xml",
                    format!("dependency identifierref '{dependency}' resolves to no resource"),
                    Provenance::FormatRequirement,
                ));
            }
        }
    }
}
