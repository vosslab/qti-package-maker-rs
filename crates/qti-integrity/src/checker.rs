//! Format-aware checks over an already safe in-memory package map.

use std::collections::{BTreeMap, BTreeSet};

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use regex::Regex;

use crate::types::{Provenance, Violation};

mod image_dimensions;

#[cfg(test)]
#[path = "checker_tests.rs"]
mod checker_tests;

const FILEBASE_PREFIX: &str = "$IMS-CC-FILEBASE$/";
const QTI12_NAMESPACE: &str = "http://www.imsglobal.org/xsd/ims_qtiasiv1p2";
const QTI21_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1";
const BB_NAMESPACE: &str = "http://www.blackboard.com/content-packaging/";
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

#[derive(Clone, Debug, Default)]
struct XmlNode {
    name: String,
    namespace: Option<String>,
    base: String,
    attributes: BTreeMap<String, String>,
    attribute_namespaces: BTreeMap<String, Option<String>>,
    text: String,
    children: Vec<XmlNode>,
}

#[derive(Clone, Debug, Default)]
struct Resource {
    identifier: String,
    href: Option<String>,
    files: BTreeSet<String>,
    dependencies: BTreeSet<String>,
}

/// Check entries previously read from a ZIP or package directory.
///
/// The map owns its byte vectors; this checker only borrows them and has no
/// dependency on a package producer. XML is parsed with `quick-xml`, whose
/// non-resolving parser prevents external entity expansion (ASVS 1.5.1).
pub fn check_entries(entries: &BTreeMap<String, Vec<u8>>) -> Vec<Violation> {
    let Some(manifest_bytes) = entries.get("imsmanifest.xml") else {
        return vec![error(
            "missing-manifest",
            "imsmanifest.xml",
            "imsmanifest.xml is missing from package",
            Provenance::FormatRequirement,
        )];
    };
    let manifest = match parse_xml("imsmanifest.xml", manifest_bytes) {
        Ok(root) => root,
        Err(violation) => return vec![violation],
    };

    let mut violations = Vec::new();
    check_relevant_xml(entries, &mut violations);
    let mut manifest_nodes = Vec::new();
    all_nodes(&manifest, &mut manifest_nodes);
    let blackboard_export = manifest_nodes.iter().any(|node| {
        node.attribute_namespaces.iter().any(|(name, namespace)| {
            local_name(name) == "file" && namespace.as_deref() == Some(BB_NAMESPACE)
        })
    });
    if blackboard_export {
        check_blackboard_export(entries, &manifest, &mut violations);
    } else {
        let resources = collect_resources(&manifest, &mut violations);
        check_manifest(entries, &resources, &mut violations);
        check_answer_linkage(entries, &mut violations);
        check_media_sources(entries, &resources, &mut violations);
    }
    image_dimensions::check_entries(entries, &mut violations);
    check_identifier_safety(entries, &manifest, &mut violations);
    check_score_outcomes(entries, &mut violations);
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

fn parse_xml(path: &str, data: &[u8]) -> Result<XmlNode, Violation> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::<XmlNode>::new();
    let mut document = None;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                if stack.is_empty() && document.is_some() {
                    return Err(xml_error(path, "XML contains multiple root elements"));
                }
                let node = xml_start_node(&event, &stack, path)?;
                stack.push(node);
            }
            Ok(Event::Empty(event)) => {
                let node = xml_start_node(&event, &stack, path)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    if document.replace(node).is_some() {
                        return Err(xml_error(path, "XML contains multiple root elements"));
                    }
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(event.as_ref());
                } else if !event.as_ref().trim().is_empty() {
                    return Err(xml_error(
                        path,
                        "XML contains text outside its root element",
                    ));
                }
            }
            Ok(Event::CData(event)) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(event.as_ref());
                } else if !event.as_ref().trim().is_empty() {
                    return Err(xml_error(
                        path,
                        "XML contains text outside its root element",
                    ));
                }
            }
            // ASVS 1.5.1: the oracle does not accept DTD-bearing package XML.
            Ok(Event::DocType(_)) => {
                return Err(xml_error(path, "DTD declarations are not allowed"));
            }
            Ok(Event::End(event)) => {
                let event_name = event.name();
                let raw_name = event_name.as_ref();
                let (prefix, end_name) = raw_name.rsplit_once(':').unwrap_or(("", raw_name));
                let no_attributes = BTreeMap::new();
                let end_namespace = namespace_for(prefix, &no_attributes, &stack);
                let Some(node) = stack.pop() else {
                    return Err(xml_error(path, "unexpected XML end tag"));
                };
                if node.name != end_name || node.namespace != end_namespace {
                    return Err(xml_error(path, "mismatched XML end tag"));
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    if document.replace(node).is_some() {
                        return Err(xml_error(path, "XML contains multiple root elements"));
                    }
                }
            }
            Ok(Event::Eof) => {
                return document.ok_or_else(|| xml_error(path, "XML has no root element"));
            }
            Ok(_) => {}
            Err(error) => return Err(xml_error(path, error)),
        }
        buffer.clear();
    }
}

fn xml_start_node(
    event: &quick_xml::events::BytesStart<'_>,
    stack: &[XmlNode],
    path: &str,
) -> Result<XmlNode, Violation> {
    let mut attributes = BTreeMap::new();
    let mut attribute_namespaces = BTreeMap::new();
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| xml_error(path, error))?;
        let key = attribute.key.as_ref().to_owned();
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| xml_error(path, error))?
            .into_owned();
        attributes.insert(key, value);
    }
    for key in attributes.keys() {
        let attribute_prefix = key.rsplit_once(':').map_or("", |(prefix, _)| prefix);
        attribute_namespaces.insert(
            key.clone(),
            namespace_for(attribute_prefix, &attributes, stack),
        );
    }
    let event_name = event.name();
    let raw_name = event_name.as_ref();
    let (prefix, name) = raw_name.rsplit_once(':').unwrap_or(("", raw_name));
    let namespace = namespace_for(prefix, &attributes, stack);
    let parent_base = stack.last().map_or("", |node| node.base.as_str());
    let base = attributes.get("xml:base").map_or_else(
        || parent_base.to_owned(),
        |value| resolve_package_uri(parent_base, value),
    );
    Ok(XmlNode {
        name: name.to_owned(),
        namespace,
        base,
        attributes,
        attribute_namespaces,
        text: String::new(),
        children: Vec::new(),
    })
}

fn namespace_for(
    prefix: &str,
    attributes: &BTreeMap<String, String>,
    stack: &[XmlNode],
) -> Option<String> {
    let declaration = if prefix.is_empty() {
        "xmlns".to_owned()
    } else {
        format!("xmlns:{prefix}")
    };
    attributes.get(&declaration).cloned().or_else(|| {
        stack
            .iter()
            .rev()
            .find_map(|node| node.attributes.get(&declaration).cloned())
    })
}

fn xml_error(path: &str, detail: impl std::fmt::Display) -> Violation {
    error(
        "invalid-xml",
        path,
        format!("could not parse XML safely: {detail}"),
        Provenance::SafeInputHandling,
    )
}

fn check_relevant_xml(entries: &BTreeMap<String, Vec<u8>>, violations: &mut Vec<Violation>) {
    for (path, data) in entries {
        if path != "imsmanifest.xml"
            && (path.ends_with(".xml") || path.ends_with(".dat"))
            && let Err(violation) = parse_xml(path, data)
        {
            violations.push(violation);
        }
    }
}

fn local_name(name: &str) -> String {
    name.rsplit(['}', ':']).next().unwrap_or(name).to_owned()
}

fn descendants<'a>(node: &'a XmlNode, name: &'a str, output: &mut Vec<&'a XmlNode>) {
    if node.name == name {
        output.push(node);
    }
    for child in &node.children {
        descendants(child, name, output);
    }
}

fn namespaced_descendants<'a>(
    node: &'a XmlNode,
    name: &'a str,
    namespace: &str,
    output: &mut Vec<&'a XmlNode>,
) {
    if node.name == name && node.namespace.as_deref() == Some(namespace) {
        output.push(node);
    }
    for child in &node.children {
        namespaced_descendants(child, name, namespace, output);
    }
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

fn all_nodes<'a>(node: &'a XmlNode, output: &mut Vec<&'a XmlNode>) {
    output.push(node);
    for child in &node.children {
        all_nodes(child, output);
    }
}

fn collect_resources(
    manifest: &XmlNode,
    violations: &mut Vec<Violation>,
) -> BTreeMap<String, Resource> {
    let mut resources = Vec::new();
    descendants(manifest, "resource", &mut resources);
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
            .map(|href| resolve_package_uri(&node.base, href));
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
                            .map(|href| resolve_package_uri(&file.base, href))
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

fn check_manifest(
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

fn check_answer_linkage(entries: &BTreeMap<String, Vec<u8>>, violations: &mut Vec<Violation>) {
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let Ok(root) = parse_xml(path, data) else {
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
    namespaced_descendants(root, "item", QTI12_NAMESPACE, &mut items);
    for item in items {
        let item_id = item
            .attributes
            .get("ident")
            .map_or("<no-ident>", String::as_str);
        let mut responses = Vec::new();
        for response_name in ["response_lid", "response_str", "response_grp"] {
            namespaced_descendants(item, response_name, QTI12_NAMESPACE, &mut responses);
        }
        let mut choices = BTreeMap::<String, BTreeSet<String>>::new();
        for response in responses {
            let Some(response_id) = response.attributes.get("ident") else {
                continue;
            };
            let mut render_choices = Vec::new();
            namespaced_descendants(
                response,
                "render_choice",
                QTI12_NAMESPACE,
                &mut render_choices,
            );
            if render_choices.is_empty() {
                continue;
            }
            let mut labels = Vec::new();
            namespaced_descendants(response, "response_label", QTI12_NAMESPACE, &mut labels);
            choices.insert(
                response_id.clone(),
                labels
                    .into_iter()
                    .filter_map(|label| label.attributes.get("ident").cloned())
                    .collect(),
            );
        }
        let mut values = Vec::new();
        namespaced_descendants(item, "varequal", QTI12_NAMESPACE, &mut values);
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
    all_nodes(root, &mut interactions);
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
        all_nodes(interaction, &mut descendants_of_interaction);
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
    namespaced_descendants(
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
        namespaced_descendants(declaration, "value", QTI21_NAMESPACE, &mut values);
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

fn check_media_sources(
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
        if let Ok(root) = parse_xml(path, data) {
            collect_image_sources(&root, &mut image_sources);
            let fallback_base = if root.base.is_empty() {
                package_directory(path)
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
        resolve_package_uri("", root_relative)
    } else {
        let base = if base.is_empty() {
            package_directory(item_path)
        } else {
            base.to_owned()
        };
        resolve_package_uri(&base, source)
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

fn resolve_package_uri(base: &str, reference: &str) -> String {
    let reference = reference.split(['?', '#']).next().unwrap_or_default();
    let decoded = percent_encoding::percent_decode_str(reference)
        .decode_utf8_lossy()
        .into_owned();
    if decoded.contains("://") || decoded.starts_with('/') || decoded.starts_with('\\') {
        return String::new();
    }
    let mut segments: Vec<&str> = if decoded.starts_with("./") || base.is_empty() {
        Vec::new()
    } else {
        base.split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    };
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return String::new();
                }
            }
            segment => segments.push(segment),
        }
    }
    segments.join("/")
}

fn package_directory(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(directory, _)| directory.to_owned())
}

fn html_unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn check_identifier_safety(
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
    descendants(manifest, "resource", &mut resources);
    descendants(manifest, "dependency", &mut dependencies);
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
        let Ok(root) = parse_xml(path, data) else {
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
            descendants(&root, "item", &mut items);
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

fn check_score_outcomes(entries: &BTreeMap<String, Vec<u8>>, violations: &mut Vec<Violation>) {
    for (path, data) in entries.iter().filter(|(path, _)| path.ends_with(".xml")) {
        let Ok(root) = parse_xml(path, data) else {
            continue;
        };
        if root.name != "assessmentItem" || root.namespace.as_deref() != Some(QTI21_NAMESPACE) {
            continue;
        }
        let mut declarations = Vec::new();
        namespaced_descendants(
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

fn check_blackboard_export(
    entries: &BTreeMap<String, Vec<u8>>,
    manifest: &XmlNode,
    violations: &mut Vec<Violation>,
) {
    let mut nodes = Vec::new();
    all_nodes(manifest, &mut nodes);
    for node in nodes {
        for (attribute, value) in &node.attributes {
            if local_name(attribute) == "file"
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
        let Ok(root) = parse_xml(path, data) else {
            continue;
        };
        let mut nodes = Vec::new();
        all_nodes(&root, &mut nodes);
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
