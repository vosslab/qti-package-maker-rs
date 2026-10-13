//! Blackboard export link and sidecar integrity checks.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;

use crate::types::{Provenance, Violation};

use super::error;
use super::xml::{self, XmlNode};

pub(super) const BB_NAMESPACE: &str = "http://www.blackboard.com/content-packaging/";

pub(super) fn check_blackboard_export(
    entries: &BTreeMap<String, Vec<u8>>,
    manifest: &XmlNode,
    violations: &mut Vec<Violation>,
) {
    let mut nodes = Vec::new();
    xml::all_nodes(manifest, &mut nodes);
    for node in nodes {
        for (attribute, value) in &node.attributes {
            if xml::local_name(attribute) == "file"
                && node
                    .attribute_namespaces
                    .get(attribute)
                    .and_then(Option::as_deref)
                    == Some(BB_NAMESPACE)
                && !entries.contains_key(value)
            {
                violations.push(error(
                    "dangling-bb-file",
                    "imsmanifest.xml",
                    format!("bb:file '{value}' is missing from package"),
                    Provenance::FormatRequirement,
                ));
            }
        }
    }

    let mut asi_object_ids = BTreeSet::new();
    let mut resource_ids = BTreeSet::new();
    let mut parent_ids = Vec::new();
    let mut dat_text = String::new();
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".dat")) {
        dat_text.push_str(&String::from_utf8_lossy(data));
        let Ok(root) = xml::parse_xml(path, data) else {
            continue;
        };
        let mut nodes = Vec::new();
        xml::all_nodes(&root, &mut nodes);
        for node in nodes {
            if node.name == "bbmd_asi_object_id" && !node.text.trim().is_empty() {
                asi_object_ids.insert(node.text.trim().to_owned());
            }
            if node.name == "cms_resource_link" {
                if let Some(value) = child_text(node, "resourceId") {
                    resource_ids.insert(value.to_owned());
                }
                if let Some(value) = child_text(node, "parentId") {
                    parent_ids.push(value.to_owned());
                }
            }
        }
    }
    let xid_pattern =
        Regex::new(r"bbcswebdav/xid-([0-9A-Za-z_]+)").expect("literal xid regex is valid");
    for xid in xid_pattern
        .captures_iter(&dat_text)
        .map(|capture| capture[1].to_owned())
        .collect::<BTreeSet<_>>()
    {
        let prefix = format!("__xid-{xid}.");
        if !entries.keys().any(|path| {
            path.rsplit('/')
                .next()
                .is_some_and(|name| name.starts_with(&prefix))
        }) {
            violations.push(error(
                "orphaned-xid-token",
                "blackboard-export",
                format!("xid token 'xid-{xid}' has no csfiles binary"),
                Provenance::FormatRequirement,
            ));
        }
        if !resource_ids.contains(&xid) {
            violations.push(error(
                "orphaned-xid-resource-link",
                "blackboard-export",
                format!("xid token 'xid-{xid}' has no CSResourceLinks resourceId"),
                Provenance::FormatRequirement,
            ));
        }
    }
    for parent_id in parent_ids {
        if !asi_object_ids.contains(&parent_id) {
            violations.push(error(
                "orphaned-cs-parent",
                "blackboard-export",
                format!("CSResourceLinks parentId '{parent_id}' has no pool object"),
                Provenance::FormatRequirement,
            ));
        }
    }
    for path in entries.keys().filter(|path| {
        path.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("__xid-"))
            && !path.ends_with(".xml")
    }) {
        let sidecar = format!("{path}.xml");
        if !entries.contains_key(&sidecar) {
            violations.push(error(
                "missing-lom-sidecar",
                path,
                format!("csfiles binary is missing LOM sidecar '{sidecar}'"),
                Provenance::FormatRequirement,
            ));
        }
    }
}

fn child_text<'a>(node: &'a XmlNode, name: &str) -> Option<&'a str> {
    node.children
        .iter()
        .find(|child| child.name == name)
        .map(|child| child.text.trim())
        .filter(|text| !text.is_empty())
}
