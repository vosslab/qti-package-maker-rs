//! Blackboard pool discovery, XML recovery, and csfiles media extraction.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use qti_core::{EntryMap, ItemBank};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};

use crate::{EngineError, ReadInput, ReadLocation, ReadOutcome, ReadWarning};

use super::super::NAME;

const BB_FILE: &str = "file";
const POOL_TYPE: &str = "assessment/x-bb-qti-pool";
/// Parses ZIP bytes or already-decoded package entries supplied by the host.
pub(crate) fn read_package(
    input: ReadInput<'_>,
    allow_mixed: bool,
) -> Result<ReadOutcome, EngineError> {
    let files = match input {
        ReadInput::File { bytes, .. } => {
            Cow::Owned(qti_integrity::read_zip_entries(bytes).map_err(|error| {
                invalid(
                    "ZIP",
                    format!("{}: {} ({})", error.path, error.message, error.code),
                )
            })?)
        }
        ReadInput::Archive { entries, .. } => {
            qti_integrity::validate_entries(entries).map_err(|error| {
                invalid(
                    "archive entries",
                    format!("{}: {} ({})", error.path, error.message, error.code),
                )
            })?;
            Cow::Borrowed(entries)
        }
    };
    let files = normalize_root(files)?;
    let manifest = files
        .get("imsmanifest.xml")
        .ok_or_else(|| invalid("package", "missing imsmanifest.xml"))?;
    let manifest = parse_xml(manifest).map_err(|message| invalid("manifest XML", message))?;
    let pool_files = pool_files(&manifest)?;
    if pool_files.is_empty() {
        return Err(invalid(
            "package",
            "manifest has no Blackboard pool resource",
        ));
    }
    let media = super::media_csfiles::recover_media(&files, &manifest, &pool_files)?;
    let mut bank = ItemBank::new(allow_mixed);
    let mut warnings = Vec::new();
    for resource in pool_files {
        let bytes = files.get(&resource).ok_or_else(|| {
            invalid(
                "package",
                format!("manifest pool file '{resource}' is missing"),
            )
        })?;
        let pool = parse_xml(bytes)
            .map_err(|message| invalid("pool XML", format!("{resource}: {message}")))?;
        for (index, element) in descendants(&pool, "item").enumerate() {
            let location = ReadLocation::PoolItem {
                resource: resource.clone(),
                number: index + 1,
            };
            match super::types_core::parse_item(element, &media.source_names) {
                Ok(Some(item)) => match bank.add_item(item) {
                    Ok(qti_core::AddOutcome::Duplicate { crc }) => warnings.push(ReadWarning {
                        location,
                        message: format!("skipping duplicate item {crc}"),
                    }),
                    Ok(qti_core::AddOutcome::Added { .. }) => {}
                    Err(error) => warnings.push(ReadWarning {
                        location,
                        message: format!("skipping malformed item: {error}"),
                    }),
                },
                Ok(None) => warnings.push(ReadWarning {
                    location,
                    message: "skipping item with unknown bbmd_questiontype".to_owned(),
                }),
                Err(message) => warnings.push(ReadWarning {
                    location,
                    message: format!("skipping malformed item: {message}"),
                }),
            }
        }
    }
    Ok(ReadOutcome {
        bank,
        assets: media.assets,
        warnings,
    })
}

pub(super) fn invalid(format: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format,
        message: message.into(),
    }
}

fn normalize_root(files: Cow<'_, EntryMap>) -> Result<Cow<'_, EntryMap>, EngineError> {
    if files.contains_key("imsmanifest.xml") {
        return Ok(files);
    }
    let prefixes = files
        .keys()
        .filter_map(|name| name.strip_suffix("/imsmanifest.xml"))
        .collect::<BTreeSet<_>>();
    if prefixes.len() != 1 {
        return Err(invalid(
            "package",
            "missing or ambiguous imsmanifest.xml root",
        ));
    }
    let marker = format!("{}/", prefixes.into_iter().next().expect("one prefix"));
    let normalized = files
        .iter()
        .filter_map(|(name, bytes)| {
            name.strip_prefix(&marker)
                .map(|name| (name.to_owned(), bytes.clone()))
        })
        .collect();
    Ok(Cow::Owned(normalized))
}

#[derive(Debug, Clone)]
pub(super) struct Node {
    pub(super) name: String,
    pub(super) attributes: BTreeMap<String, String>,
    pub(super) bb_attributes: BTreeMap<String, String>,
    pub(super) xml_attributes: BTreeMap<String, String>,
    pub(super) text: String,
    pub(super) children: Vec<Node>,
}

pub(super) fn parse_xml(bytes: &[u8]) -> Result<Node, String> {
    let mut reader = NsReader::from_reader(Cursor::new(bytes));
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| error.to_string())?
        {
            Event::Start(event) => stack.push(node_from_event(&reader, &event)?),
            Event::Empty(event) => {
                append_node(&mut stack, &mut root, node_from_event(&reader, &event)?)?
            }
            Event::Text(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text
                        .push_str(&text.xml_content(XmlVersion::Implicit1_0));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push('&');
                    node.text.push_str(reference.as_ref());
                    node.text.push(';');
                }
            }
            Event::CData(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(text.as_ref());
                }
            }
            Event::End(_) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| "unexpected XML closing tag".to_owned())?;
                append_node(&mut stack, &mut root, node)?;
            }
            Event::DocType(_) => return Err("DTD declarations are not allowed".to_owned()),
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err("unclosed XML element".to_owned());
    }
    root.ok_or_else(|| "XML document has no root element".to_owned())
}

fn node_from_event(
    reader: &NsReader<Cursor<&[u8]>>,
    event: &quick_xml::events::BytesStart<'_>,
) -> Result<Node, String> {
    let (namespace, local_name) = reader.resolver().resolve_element(event.name());
    let name = match namespace {
        ResolveResult::Unbound | ResolveResult::Unknown(_) => local_name.as_ref().to_owned(),
        ResolveResult::Bound(_) => event.name().as_ref().to_owned(),
    };
    let mut attributes = BTreeMap::new();
    let mut bb_attributes = BTreeMap::new();
    let mut xml_attributes = BTreeMap::new();
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| error.to_string())?;
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| error.to_string())?
            .into_owned();
        let (namespace, local_name) = reader.resolver().resolve_attribute(attribute.key);
        match namespace {
            ResolveResult::Unbound => {
                attributes.insert(local_name.as_ref().to_owned(), value);
            }
            ResolveResult::Bound(namespace)
                if namespace.as_ref() == "http://www.blackboard.com/content-packaging/" =>
            {
                bb_attributes.insert(local_name.as_ref().to_owned(), value);
            }
            ResolveResult::Bound(namespace)
                if namespace.as_ref() == "http://www.w3.org/XML/1998/namespace" =>
            {
                xml_attributes.insert(local_name.as_ref().to_owned(), value);
            }
            ResolveResult::Unknown(_) | ResolveResult::Bound(_) => {}
        }
    }
    Ok(Node {
        name,
        attributes,
        bb_attributes,
        xml_attributes,
        text: String::new(),
        children: Vec::new(),
    })
}

fn append_node(stack: &mut [Node], root: &mut Option<Node>, node: Node) -> Result<(), String> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err("XML document has multiple roots".to_owned());
    }
    Ok(())
}

pub(super) fn descendants<'a>(node: &'a Node, name: &str) -> std::vec::IntoIter<&'a Node> {
    fn visit<'a>(node: &'a Node, name: &str, output: &mut Vec<&'a Node>) {
        for child in &node.children {
            if child.name == name {
                output.push(child);
            }
            visit(child, name, output);
        }
    }
    let mut output = Vec::new();
    visit(node, name, &mut output);
    output.into_iter()
}
pub(super) fn child<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    node.children.iter().find(|child| child.name == name)
}
pub(super) fn text(node: &Node) -> String {
    node.text.trim().to_owned()
}

fn pool_files(manifest: &Node) -> Result<Vec<String>, EngineError> {
    let mut pool = Vec::new();
    for resource in descendants(manifest, "resource") {
        if resource
            .attributes
            .get("type")
            .is_some_and(|typ| typ == POOL_TYPE)
        {
            let file = resource
                .bb_attributes
                .get(BB_FILE)
                .ok_or_else(|| invalid("manifest", "pool resource lacks bb:file"))?;
            checked_path(file).map_err(|message| invalid("manifest", message))?;
            pool.push(file.to_owned());
        }
    }
    Ok(pool)
}

pub(super) fn checked_path(path: &str) -> Result<(), String> {
    qti_core::validate_entry_name(path).map_err(|_| format!("unsafe package path '{path}'"))
}
