//! Item image references and manifest dependency traces.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;

use crate::types::{Provenance, Violation};

use super::error;
use super::manifest::Resource;
use super::xml::{self, XmlNode};

const FILEBASE_PREFIX: &str = "$IMS-CC-FILEBASE$/";

pub(super) fn check_media_sources(
    entries: &BTreeMap<String, Vec<u8>>,
    resources: &BTreeMap<String, Resource>,
    violations: &mut Vec<Violation>,
) {
    let image_pattern = Regex::new(r#"(?i)<img[^>]*\ssrc\s*=\s*['\"]([^'\"]+)['\"]"#)
        .expect("literal image regex is valid");
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let source_text = String::from_utf8_lossy(data);
        let text = html_unescape(&source_text);
        let mut image_sources = BTreeSet::new();
        if let Ok(root) = xml::parse_xml(path, data) {
            collect_image_sources(&root, &mut image_sources);
            let fallback_base = if root.base.is_empty() {
                xml::package_directory(path)
            } else {
                root.base
            };
            if source_text.to_ascii_lowercase().contains("&lt;img") {
                for source in image_pattern
                    .captures_iter(&text)
                    .map(|captures| captures[1].to_owned())
                {
                    image_sources.insert((source, fallback_base.clone()));
                }
            }
        }
        for (source, base) in image_sources {
            check_image_source(path, &source, &base, entries, resources, violations);
        }
    }
}

fn html_unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn collect_image_sources(node: &XmlNode, output: &mut BTreeSet<(String, String)>) {
    if node.name.eq_ignore_ascii_case("img")
        && let Some(source) = node.attributes.get("src")
    {
        output.insert((source.clone(), node.base.clone()));
    }
    for child in &node.children {
        collect_image_sources(child, output);
    }
}

fn check_image_source(
    item_path: &str,
    source: &str,
    base: &str,
    entries: &BTreeMap<String, Vec<u8>>,
    resources: &BTreeMap<String, Resource>,
    violations: &mut Vec<Violation>,
) {
    if source.contains("://") || source.starts_with("data:") || source.starts_with("mailto:") {
        return;
    }
    if source.starts_with('$') && !source.starts_with(FILEBASE_PREFIX) {
        return;
    }
    let target = if let Some(root_relative) = source.strip_prefix(FILEBASE_PREFIX) {
        xml::resolve_package_uri("", root_relative)
    } else {
        let base = if base.is_empty() {
            xml::package_directory(item_path)
        } else {
            base.to_owned()
        };
        xml::resolve_package_uri(&base, source)
    };
    if target.is_empty() || !entries.contains_key(&target) {
        violations.push(error(
            "dangling-image-source",
            item_path,
            format!("img src '{source}' resolves to missing entry '{target}'"),
            Provenance::FormatRequirement,
        ));
        return;
    }
    check_media_trace(item_path, &target, resources, violations);
}

fn check_media_trace(
    item_path: &str,
    image_path: &str,
    resources: &BTreeMap<String, Resource>,
    violations: &mut Vec<Violation>,
) {
    let Some(item_resource) = resources.values().find(|resource| {
        resource.href.as_deref() == Some(item_path) || resource.files.contains(item_path)
    }) else {
        violations.push(error(
            "unmanifested-item",
            item_path,
            "item containing an image has no manifest resource",
            Provenance::FormatRequirement,
        ));
        return;
    };
    if item_resource.files.contains(image_path) {
        return;
    }
    let mut pending: Vec<&str> = item_resource
        .dependencies
        .iter()
        .map(String::as_str)
        .collect();
    let mut visited = BTreeSet::new();
    let mut traced = false;
    while let Some(dependency) = pending.pop() {
        if !visited.insert(dependency) {
            continue;
        }
        let Some(resource) = resources.get(dependency) else {
            continue;
        };
        if resource.files.contains(image_path) {
            traced = true;
            break;
        }
        pending.extend(resource.dependencies.iter().map(String::as_str));
    }
    if !traced {
        violations.push(error(
            "broken-media-trace",
            item_path,
            format!(
                "image '{image_path}' is not reached from resource '{}' through a manifest dependency",
                item_resource.identifier
            ),
            Provenance::BlackboardImportFailure,
        ));
    }
}
