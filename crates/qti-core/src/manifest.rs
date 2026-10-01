//! IMS Content Packaging manifests for Blackboard-compatible QTI exports.
//!
//! `ManifestConfig` owns the package description while borrowing the media records collected by
//! the caller.  Item resources refer to those media records by index, which makes a shared asset
//! produce one `webcontent` resource and one dependency from each item that uses it.

use std::collections::BTreeSet;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesText, Event};
use thiserror::Error;

use crate::media::MediaAsset;

const IMSCP_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imscp_v1p1";
const IMSMD_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsmd_v1p2";
const LOM_NAMESPACE: &str = "http://ltsc.ieee.org/xsd/LOM";
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
const SCHEMA_LOCATION: &str = "http://www.imsglobal.org/xsd/imscp_v1p1 http://www.imsglobal.org/xsd/imscp_v1p1.xsd http://www.imsglobal.org/xsd/imsmd_v1p2 http://www.imsglobal.org/xsd/imsmd_v1p2.xsd http://ltsc.ieee.org/xsd/LOM http://ieee-sa.imeetcentral.com/ltsc/";

/// QTI manifest variants emitted by the Python exporter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QtiVersion {
    /// QTI 1.2 item package.
    V1p2,
    /// QTI 2.1 item package.
    V2p1,
}

/// A QTI item file and the packaged media it references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemResource {
    /// Package-relative XML path for the assessment item.
    pub href: String,
    /// Indexes into [`ManifestConfig::assets`] in the item's dependency order.
    pub asset_indexes: Vec<usize>,
}

impl ItemResource {
    /// Creates an item resource with no packaged-media dependencies.
    #[must_use]
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            asset_indexes: Vec::new(),
        }
    }
}

/// Inputs needed to construct one `imsmanifest.xml` document.
#[derive(Clone, Debug)]
pub struct ManifestConfig<'a> {
    /// Display name written to IEEE LOM metadata.
    pub package_name: String,
    /// QTI version that determines item and assessment-meta resource types.
    pub version: QtiVersion,
    /// Assessment item resources. QTI 1.2 requires exactly one.
    pub items: Vec<ItemResource>,
    /// Media copied into the package root; each needs an assigned output name.
    pub assets: &'a [MediaAsset],
    /// Optional ISO-8601 date for repeatable builds; absent uses today's UTC date.
    pub metadata_date: Option<String>,
}

/// Manifest construction failures before any package bytes are returned.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The export did not include an assessment item.
    #[error("cannot generate manifest: no assessment files provided")]
    NoItems,
    /// QTI 1.2 is a single-assessment package in the Python contract.
    #[error("QTI version 1 only supports one assessment file")]
    Qti12MultipleItems,
    /// QTI 1.2 places `name.xml` inside a matching `name/` directory.
    #[error("QTI version 1 requires file name '{href}' to match its directory")]
    Qti12PathMismatch { href: String },
    /// An item named a media asset that is absent from the supplied asset list.
    #[error("item '{href}' references unknown media asset index {asset_index}")]
    UnknownAssetIndex { href: String, asset_index: usize },
    /// A webcontent declaration needs the same collision-safe name used by the package writer.
    #[error("media asset '{src}' has no assigned package output name")]
    MissingAssetOutputName { src: String },
    /// A resource or media name could escape or be interpreted outside the package root.
    #[error("invalid package-relative {field} path '{path}': {reason}")]
    InvalidPackagePath {
        field: &'static str,
        path: String,
        reason: &'static str,
    },
    /// XML writing failed.
    #[error("could not write IMS manifest XML: {0}")]
    Xml(#[from] io::Error),
}

/// Generates a complete, UTF-8 `imsmanifest.xml` document.
///
/// The result owns its XML bytes. The caller retains ownership of `config` and its media slice;
/// this function only reads the media's collision-safe `output_name` values.
pub fn generate_manifest(config: &ManifestConfig<'_>) -> Result<Vec<u8>, ManifestError> {
    validate_config(config)?;
    let asset_ids = asset_identifiers(config.assets)?;
    let item_ids = item_identifiers(&config.items, &asset_ids);
    let metadata_date = config
        .metadata_date
        .clone()
        .unwrap_or_else(current_utc_date);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer
        .create_element("manifest")
        .with_attributes([
            ("xmlns", IMSCP_NAMESPACE),
            ("xmlns:lom", LOM_NAMESPACE),
            ("xmlns:imsmd", IMSMD_NAMESPACE),
            ("xmlns:xsi", XSI_NAMESPACE),
            ("identifier", "main_manifest"),
            ("xsi:schemaLocation", SCHEMA_LOCATION),
        ])
        .write_inner_content(|writer| {
            write_metadata(writer, &config.package_name, config.version, &metadata_date)?;
            writer.create_element("organizations").write_empty()?;
            write_resources(writer, config, &item_ids, &asset_ids)?;
            Ok(())
        })?;
    Ok(writer.into_inner())
}

fn validate_config(config: &ManifestConfig<'_>) -> Result<(), ManifestError> {
    if config.items.is_empty() {
        return Err(ManifestError::NoItems);
    }
    for item in &config.items {
        validate_package_file_path("item resource", &item.href)?;
    }
    for asset in config.assets {
        let output_name =
            asset
                .output_name
                .as_deref()
                .ok_or_else(|| ManifestError::MissingAssetOutputName {
                    src: asset.src.clone(),
                })?;
        validate_package_file_path("media output", output_name)?;
    }
    if config.version == QtiVersion::V1p2 {
        if config.items.len() != 1 {
            return Err(ManifestError::Qti12MultipleItems);
        }
        let href = &config.items[0].href;
        let path = std::path::Path::new(href);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let directory = path
            .parent()
            .and_then(|value| value.file_name())
            .and_then(|value| value.to_str());
        if directory != Some(stem) {
            return Err(ManifestError::Qti12PathMismatch { href: href.clone() });
        }
    }
    for item in &config.items {
        for &asset_index in &item.asset_indexes {
            if asset_index >= config.assets.len() {
                return Err(ManifestError::UnknownAssetIndex {
                    href: item.href.clone(),
                    asset_index,
                });
            }
        }
    }
    Ok(())
}

fn validate_package_file_path(field: &'static str, path: &str) -> Result<(), ManifestError> {
    // This mirrors the ZIP writer's root-relative POSIX subset at the manifest boundary.  The
    // helper remains local because the ZIP module intentionally keeps its archive validator
    // private; both boundaries must reject zip-slip and platform-specific absolute spellings.
    let invalid = |reason| ManifestError::InvalidPackagePath {
        field,
        path: path.to_owned(),
        reason,
    };
    if path.is_empty() {
        return Err(invalid("path is empty"));
    }
    if path.starts_with('/') {
        return Err(invalid("path is absolute"));
    }
    if path.ends_with('/') {
        return Err(invalid("path names a directory"));
    }
    if path.contains('\\') {
        return Err(invalid("backslashes are not valid package separators"));
    }
    if path.chars().any(char::is_control) {
        return Err(invalid("path contains a control character"));
    }
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(invalid("path contains an empty or traversal component"));
        }
        if component.contains(':') {
            return Err(invalid("path contains a drive-prefix separator"));
        }
    }
    Ok(())
}

fn write_metadata(
    writer: &mut Writer<Vec<u8>>,
    package_name: &str,
    version: QtiVersion,
    metadata_date: &str,
) -> io::Result<()> {
    let (schema, schema_version) = match version {
        QtiVersion::V1p2 => ("IMS Content", "1.1.3"),
        QtiVersion::V2p1 => ("QTIv2.1", "2.0"),
    };
    writer
        .create_element("metadata")
        .write_inner_content(|writer| {
            write_text_element(writer, "schema", schema)?;
            write_text_element(writer, "schemaversion", schema_version)?;
            writer
                .create_element("imsmd:lom")
                .write_inner_content(|writer| {
                    writer
                        .create_element("imsmd:general")
                        .write_inner_content(|writer| {
                            writer
                                .create_element("imsmd:title")
                                .write_inner_content(|writer| {
                                    write_text_element(writer, "imsmd:string", package_name)
                                })?;
                            Ok(())
                        })?;
                    writer
                        .create_element("imsmd:lifeCycle")
                        .write_inner_content(|writer| {
                            writer
                                .create_element("imsmd:contribute")
                                .write_inner_content(|writer| {
                                    writer
                                        .create_element("imsmd:date")
                                        .write_inner_content(|writer| {
                                            write_text_element(
                                                writer,
                                                "imsmd:dateTime",
                                                metadata_date,
                                            )
                                        })?;
                                    Ok(())
                                })?;
                            Ok(())
                        })?;
                    writer
                        .create_element("imsmd:rights")
                        .write_inner_content(|writer| {
                            writer
                                .create_element("imsmd:description")
                                .write_inner_content(|writer| {
                                    write_text_element(
                                        writer,
                                        "imsmd:string",
                                        "CC Attribution - http://creativecommons.org/licenses/by/4.0",
                                    )
                                })?;
                            Ok(())
                        })?;
                    Ok(())
                })?;
            Ok(())
        })?;
    Ok(())
}

fn write_resources(
    writer: &mut Writer<Vec<u8>>,
    config: &ManifestConfig<'_>,
    item_ids: &[String],
    asset_ids: &[String],
) -> io::Result<()> {
    let (meta_type, item_type) = match config.version {
        QtiVersion::V1p2 => (
            "associatedcontent/imscc_xmlv1p1/learning-application-resource",
            "imsqti_xmlv1p2",
        ),
        QtiVersion::V2p1 => ("imsqti_test_xmlv2p1", "imsqti_item_xmlv2p1"),
    };
    let item_directory = std::path::Path::new(&config.items[0].href)
        .parent()
        .and_then(|path| path.to_str())
        .unwrap_or_default();
    let meta_href = if item_directory.is_empty() {
        "assessment_meta.xml".to_owned()
    } else {
        format!("{item_directory}/assessment_meta.xml")
    };
    writer
        .create_element("resources")
        .write_inner_content(|writer| {
            for (asset, asset_id) in config.assets.iter().zip(asset_ids) {
                let output_name = asset
                    .output_name
                    .as_deref()
                    .expect("validated before XML writing");
                writer
                    .create_element("resource")
                    .with_attributes([
                        ("href", output_name),
                        ("identifier", asset_id),
                        ("type", "webcontent"),
                    ])
                    .write_inner_content(|writer| write_file(writer, output_name))?;
            }
            for (item, item_id) in config.items.iter().zip(item_ids) {
                writer
                    .create_element("resource")
                    .with_attributes([
                        ("href", item.href.as_str()),
                        ("identifier", item_id),
                        ("type", item_type),
                    ])
                    .write_inner_content(|writer| {
                        write_file(writer, &item.href)?;
                        write_dependency(writer, "assessment_meta")?;
                        for &asset_index in &item.asset_indexes {
                            write_dependency(writer, &asset_ids[asset_index])?;
                        }
                        Ok(())
                    })?;
            }
            writer
                .create_element("resource")
                .with_attributes([
                    ("href", meta_href.as_str()),
                    ("identifier", "assessment_meta"),
                    ("type", meta_type),
                ])
                .write_inner_content(|writer| {
                    for item_id in item_ids {
                        write_dependency(writer, item_id)?;
                    }
                    write_file(writer, &meta_href)
                })?;
            Ok(())
        })?;
    Ok(())
}

fn write_file(writer: &mut Writer<Vec<u8>>, href: &str) -> io::Result<()> {
    writer
        .create_element("file")
        .with_attribute(("href", href))
        .write_empty()?;
    Ok(())
}

fn write_dependency(writer: &mut Writer<Vec<u8>>, identifier: &str) -> io::Result<()> {
    writer
        .create_element("dependency")
        .with_attribute(("identifierref", identifier))
        .write_empty()?;
    Ok(())
}

fn write_text_element(writer: &mut Writer<Vec<u8>>, name: &str, text: &str) -> io::Result<()> {
    writer
        .create_element(name)
        .write_text_content(BytesText::new(text))?;
    Ok(())
}

fn item_identifiers(items: &[ItemResource], asset_ids: &[String]) -> Vec<String> {
    let mut used = BTreeSet::from(["assessment_meta".to_owned()]);
    used.extend(asset_ids.iter().cloned());
    items
        .iter()
        .map(|item| {
            let stem = std::path::Path::new(&item.href)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("item");
            unique_xml_identifier(stem, &mut used)
        })
        .collect()
}

fn asset_identifiers(assets: &[MediaAsset]) -> Result<Vec<String>, ManifestError> {
    let mut ids = Vec::with_capacity(assets.len());
    for (offset, asset) in assets.iter().enumerate() {
        if asset.output_name.is_none() {
            return Err(ManifestError::MissingAssetOutputName {
                src: asset.src.clone(),
            });
        }
        ids.push(format!("ccres{:05}", offset + 1));
    }
    Ok(ids)
}

fn unique_xml_identifier(raw: &str, used: &mut BTreeSet<String>) -> String {
    let mut identifier = xml_name_safe(raw);
    let root = identifier.clone();
    let mut suffix = 2;
    while !used.insert(identifier.clone()) {
        identifier = format!("{root}_{suffix}");
        suffix += 1;
    }
    identifier
}

fn xml_name_safe(raw: &str) -> String {
    let mut identifier: String = raw
        .chars()
        .map(|character| match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' => character,
            _ => '_',
        })
        .collect();
    if identifier.is_empty() {
        identifier.push_str("item");
    }
    if !matches!(identifier.as_bytes()[0], b'A'..=b'Z' | b'a'..=b'z' | b'_') {
        identifier.insert(0, '_');
    }
    identifier
}

fn current_utc_date() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_date_from_unix_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_date_from_unix_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use quick_xml::events::Event;
    use quick_xml::{Reader, XmlVersion};

    use super::*;
    use crate::media::AssetKind;

    fn asset(name: &str) -> MediaAsset {
        MediaAsset {
            src: format!("images/{name}"),
            kind: AssetKind::Local,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: None,
            output_name: Some(name.to_owned()),
            content_hash: None,
        }
    }

    #[test]
    fn qti21_manifest_has_python_metadata_resources_and_resolved_references() {
        let assets = [asset("shared & image.png"), asset("second.png")];
        let manifest = generate_manifest(&ManifestConfig {
            package_name: "Cell & gene <bank>".to_owned(),
            version: QtiVersion::V2p1,
            items: vec![
                ItemResource {
                    href: "qti21_items/item 1.xml".to_owned(),
                    asset_indexes: vec![0],
                },
                ItemResource {
                    href: "qti21_items/item-2.xml".to_owned(),
                    asset_indexes: vec![0, 1],
                },
            ],
            assets: &assets,
            metadata_date: Some("2026-09-30".to_owned()),
        })
        .expect("manifest should generate");
        let xml = std::str::from_utf8(&manifest).expect("manifest is UTF-8");
        assert!(xml.contains("xmlns=\"http://www.imsglobal.org/xsd/imscp_v1p1\""));
        assert!(xml.contains("QTIv2.1"));
        assert!(xml.contains("Cell &amp; gene &lt;bank&gt;"));
        assert!(xml.contains("type=\"webcontent\""));
        assert!(xml.contains("identifier=\"item_1\""));
        assert!(xml.contains("href=\"shared &amp; image.png\""));
        assert_manifest_references_resolve(&manifest);
    }

    #[test]
    fn qti12_preserves_blackboard_resource_types_and_path_contract() {
        let manifest = generate_manifest(&ManifestConfig {
            package_name: "QTI 1.2".to_owned(),
            version: QtiVersion::V1p2,
            items: vec![ItemResource::new("qti12_items/qti12_items.xml")],
            assets: &[],
            metadata_date: Some("2026-09-30".to_owned()),
        })
        .expect("manifest should generate");
        let xml = std::str::from_utf8(&manifest).expect("manifest is UTF-8");
        assert!(xml.contains("IMS Content"));
        assert!(xml.contains("associatedcontent/imscc_xmlv1p1/learning-application-resource"));
        assert!(xml.contains("imsqti_xmlv1p2"));
        assert_manifest_references_resolve(&manifest);
    }

    #[test]
    fn rejects_invalid_python_contracts_and_unknown_media() {
        let empty = ManifestConfig {
            package_name: "empty".to_owned(),
            version: QtiVersion::V2p1,
            items: vec![],
            assets: &[],
            metadata_date: None,
        };
        assert!(matches!(
            generate_manifest(&empty),
            Err(ManifestError::NoItems)
        ));
        let bad_path = ManifestConfig {
            package_name: "bad".to_owned(),
            version: QtiVersion::V1p2,
            items: vec![ItemResource::new("folder/other.xml")],
            assets: &[],
            metadata_date: None,
        };
        assert!(matches!(
            generate_manifest(&bad_path),
            Err(ManifestError::Qti12PathMismatch { .. })
        ));
        let unknown_asset = ManifestConfig {
            package_name: "bad".to_owned(),
            version: QtiVersion::V2p1,
            items: vec![ItemResource {
                href: "items/item.xml".to_owned(),
                asset_indexes: vec![0],
            }],
            assets: &[],
            metadata_date: None,
        };
        assert!(matches!(
            generate_manifest(&unknown_asset),
            Err(ManifestError::UnknownAssetIndex { .. })
        ));
    }

    #[test]
    fn rejects_unsafe_package_paths_before_writing_xml() {
        for href in [
            "../item.xml",
            "/item.xml",
            "items\\item.xml",
            "items/./item.xml",
        ] {
            let config = ManifestConfig {
                package_name: "unsafe item".to_owned(),
                version: QtiVersion::V2p1,
                items: vec![ItemResource::new(href)],
                assets: &[],
                metadata_date: None,
            };
            assert!(matches!(
                generate_manifest(&config),
                Err(ManifestError::InvalidPackagePath {
                    field: "item resource",
                    ..
                })
            ));
        }

        for output_name in ["../image.png", "images\\image.png", "C:/image.png"] {
            let assets = [asset(output_name)];
            let config = ManifestConfig {
                package_name: "unsafe media".to_owned(),
                version: QtiVersion::V2p1,
                items: vec![ItemResource::new("items/item.xml")],
                assets: &assets,
                metadata_date: None,
            };
            assert!(matches!(
                generate_manifest(&config),
                Err(ManifestError::InvalidPackagePath {
                    field: "media output",
                    ..
                })
            ));
        }
    }

    fn assert_manifest_references_resolve(manifest: &[u8]) {
        let mut reader = Reader::from_reader(std::io::Cursor::new(manifest));
        reader.config_mut().trim_text(true);
        let mut buffer = Vec::new();
        let mut resource_ids = BTreeSet::new();
        let mut resource_hrefs = HashMap::new();
        let mut resource_count = 0;
        let mut file_hrefs = BTreeSet::new();
        let mut dependencies = Vec::new();
        loop {
            match reader
                .read_event_into(&mut buffer)
                .expect("manifest should parse")
            {
                Event::Start(event) | Event::Empty(event)
                    if event.name().as_ref() == "resource" =>
                {
                    resource_count += 1;
                    let attributes = event
                        .attributes()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("resource attributes");
                    let mut identifier = None;
                    let mut href = None;
                    for attribute in attributes {
                        match attribute.key.as_ref() {
                            "identifier" => {
                                identifier = Some(
                                    attribute
                                        .normalized_value(XmlVersion::Explicit1_0)
                                        .expect("identifier escape")
                                        .into_owned(),
                                )
                            }
                            "href" => {
                                href = Some(
                                    attribute
                                        .normalized_value(XmlVersion::Explicit1_0)
                                        .expect("href escape")
                                        .into_owned(),
                                )
                            }
                            _ => {}
                        }
                    }
                    resource_hrefs.insert(
                        identifier.clone().expect("resource ID"),
                        href.expect("resource href"),
                    );
                    resource_ids.insert(identifier.expect("resource ID"));
                }
                Event::Empty(event) if event.name().as_ref() == "file" => {
                    for attribute in event.attributes() {
                        let attribute = attribute.expect("file attribute");
                        if attribute.key.as_ref() == "href" {
                            file_hrefs.insert(
                                attribute
                                    .normalized_value(XmlVersion::Explicit1_0)
                                    .expect("file href escape")
                                    .into_owned(),
                            );
                        }
                    }
                }
                Event::Empty(event) if event.name().as_ref() == "dependency" => {
                    for attribute in event.attributes() {
                        let attribute = attribute.expect("dependency attribute");
                        if attribute.key.as_ref() == "identifierref" {
                            dependencies.push(
                                attribute
                                    .normalized_value(XmlVersion::Explicit1_0)
                                    .expect("dependency escape")
                                    .into_owned(),
                            );
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buffer.clear();
        }
        assert_eq!(
            resource_hrefs.values().cloned().collect::<BTreeSet<_>>(),
            file_hrefs
        );
        assert_eq!(
            resource_ids.len(),
            resource_count,
            "resource IDs are unique"
        );
        assert!(
            dependencies
                .iter()
                .all(|identifier| resource_ids.contains(identifier))
        );
    }
}
