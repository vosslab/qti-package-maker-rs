//! Safe XML parsing, namespace-aware traversal, and package URI resolution.

use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

use crate::types::{Provenance, Violation};

use super::error;

pub(super) const QTI12_NAMESPACE: &str = "http://www.imsglobal.org/xsd/ims_qtiasiv1p2";
pub(super) const QTI21_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1";

#[derive(Clone, Debug, Default)]
pub(super) struct XmlNode {
    pub(super) name: String,
    pub(super) namespace: Option<String>,
    pub(super) base: String,
    pub(super) attributes: BTreeMap<String, String>,
    pub(super) attribute_namespaces: BTreeMap<String, Option<String>>,
    pub(super) text: String,
    pub(super) children: Vec<XmlNode>,
}

pub(super) fn parse_xml(path: &str, data: &[u8]) -> Result<XmlNode, Violation> {
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

pub(super) fn check_relevant_xml(
    entries: &BTreeMap<String, Vec<u8>>,
    violations: &mut Vec<Violation>,
) {
    for (path, data) in entries {
        if path != "imsmanifest.xml"
            && (path.ends_with(".xml") || path.ends_with(".dat"))
            && let Err(violation) = parse_xml(path, data)
        {
            violations.push(violation);
        }
    }
}

pub(super) fn local_name(name: &str) -> String {
    name.rsplit(['}', ':']).next().unwrap_or(name).to_owned()
}

pub(super) fn descendants<'a>(node: &'a XmlNode, name: &'a str, output: &mut Vec<&'a XmlNode>) {
    if node.name == name {
        output.push(node);
    }
    for child in &node.children {
        descendants(child, name, output);
    }
}

pub(super) fn namespaced_descendants<'a>(
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

pub(super) fn all_nodes<'a>(node: &'a XmlNode, output: &mut Vec<&'a XmlNode>) {
    output.push(node);
    for child in &node.children {
        all_nodes(child, output);
    }
}

pub(super) fn resolve_package_uri(base: &str, reference: &str) -> String {
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

pub(super) fn package_directory(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(directory, _)| directory.to_owned())
}
