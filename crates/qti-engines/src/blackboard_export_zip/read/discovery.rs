//! Blackboard pool discovery, XML recovery, and csfiles media extraction.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use qti_core::ItemBank;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};
use zip::ZipArchive;

use crate::{EngineError, ReadLocation, ReadOutcome, ReadWarning};

use super::super::NAME;

const BB_FILE: &str = "file";
const POOL_TYPE: &str = "assessment/x-bb-qti-pool";
// Match the integrity checker bounds so reading a package never becomes a
// less-safe alternate path around the release verifier.
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;

/// Parses an exported package from a ZIP or an already-unpacked directory.
pub(crate) fn read_package(input: &Path, allow_mixed: bool) -> Result<ReadOutcome, EngineError> {
    let files = read_input(input)?;
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
    let mut bank = match media.base {
        Some(base) => ItemBank::with_media_base_dir(allow_mixed, base),
        None => ItemBank::new(allow_mixed),
    };
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
    Ok(ReadOutcome { bank, warnings })
}

pub(super) fn invalid(format: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format,
        message: message.into(),
    }
}

fn read_input(input: &Path) -> Result<BTreeMap<String, Vec<u8>>, EngineError> {
    if input.is_dir() {
        return read_directory(input);
    }
    let file = fs::File::open(input).map_err(|source| EngineError::Io {
        engine: NAME,
        path: input.to_owned(),
        source,
    })?;
    let mut archive = ZipArchive::new(file).map_err(|error| invalid("ZIP", error.to_string()))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(invalid("ZIP", "archive exceeds the entry-count limit"));
    }
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| invalid("ZIP", error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        if entry.size() > MAX_ENTRY_BYTES || total.saturating_add(entry.size()) > MAX_PACKAGE_BYTES
        {
            return Err(invalid(
                "ZIP",
                "archive exceeds the uncompressed-size limit",
            ));
        }
        total += entry.size();
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| invalid("ZIP", format!("unsafe archive entry '{}'", entry.name())))?;
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if name.is_empty() || files.contains_key(&name) {
            return Err(invalid(
                "ZIP",
                format!("duplicate or empty archive entry '{name}'"),
            ));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| invalid("ZIP", error.to_string()))?;
        files.insert(name, bytes);
    }
    normalize_root(files)
}

fn read_directory(input: &Path) -> Result<BTreeMap<String, Vec<u8>>, EngineError> {
    let root = find_manifest_root(input)?;
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    collect_files(&root, &root, &mut files, &mut total)?;
    Ok(files)
}

fn find_manifest_root(input: &Path) -> Result<PathBuf, EngineError> {
    if input.join("imsmanifest.xml").is_file() {
        return Ok(input.to_owned());
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(input).map_err(|source| EngineError::Io {
        engine: NAME,
        path: input.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| EngineError::Io {
            engine: NAME,
            path: input.to_owned(),
            source,
        })?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|source| EngineError::Io {
                engine: NAME,
                path: path.clone(),
                source,
            })?
            .is_dir()
            && path.join("imsmanifest.xml").is_file()
        {
            candidates.push(path);
        }
    }
    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(invalid("package", "missing imsmanifest.xml")),
        _ => Err(invalid("package", "multiple nested manifest roots")),
    }
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    total: &mut u64,
) -> Result<(), EngineError> {
    for entry in fs::read_dir(current).map_err(|source| EngineError::Io {
        engine: NAME,
        path: current.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| EngineError::Io {
            engine: NAME,
            path: current.to_owned(),
            source,
        })?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| EngineError::Io {
            engine: NAME,
            path: path.clone(),
            source,
        })?;
        if kind.is_symlink() {
            return Err(invalid(
                "package directory",
                format!("symbolic link is not permitted: {}", path.display()),
            ));
        }
        if kind.is_dir() {
            collect_files(root, &path, files, total)?;
            continue;
        }
        if !kind.is_file() {
            return Err(invalid(
                "package directory",
                format!("non-regular file: {}", path.display()),
            ));
        }
        let relative = path
            .strip_prefix(root)
            .expect("walk stays below root")
            .to_string_lossy()
            .replace('\\', "/");
        if files.len() >= MAX_ARCHIVE_ENTRIES {
            return Err(invalid(
                "package directory",
                "directory exceeds the entry-count limit",
            ));
        }
        let metadata = entry.metadata().map_err(|source| EngineError::Io {
            engine: NAME,
            path: path.clone(),
            source,
        })?;
        if metadata.len() > MAX_ENTRY_BYTES
            || total.saturating_add(metadata.len()) > MAX_PACKAGE_BYTES
        {
            return Err(invalid(
                "package directory",
                "directory exceeds the uncompressed-size limit",
            ));
        }
        let bytes = fs::read(&path).map_err(|source| EngineError::Io {
            engine: NAME,
            path: path.clone(),
            source,
        })?;
        let size = u64::try_from(bytes.len()).expect("usize fits in u64");
        if size > MAX_ENTRY_BYTES || total.saturating_add(size) > MAX_PACKAGE_BYTES {
            return Err(invalid(
                "package directory",
                "directory exceeds the uncompressed-size limit",
            ));
        }
        *total += size;
        if files.insert(relative.clone(), bytes).is_some() {
            return Err(invalid(
                "package directory",
                format!("duplicate package path '{relative}'"),
            ));
        }
    }
    Ok(())
}

fn normalize_root(
    mut files: BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, EngineError> {
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
    let prefix = prefixes.into_iter().next().expect("one prefix");
    let marker = format!("{prefix}/");
    let mut normalized = BTreeMap::new();
    for (name, bytes) in std::mem::take(&mut files) {
        if let Some(name) = name.strip_prefix(&marker) {
            normalized.insert(name.to_owned(), bytes);
        }
    }
    Ok(normalized)
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
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(format!("unsafe package path '{path}'"));
    }
    Ok(())
}
