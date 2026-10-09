//! Blackboard QTI 2.1 package assembly and media ownership.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qti_core::media::{
    AssetKind, AssetSource, MediaPolicy, apply_media_policy, packageable_assets, rewrite_item_media,
};
use qti_core::{
    EntryMap, ItemKind, ItemResource, ManifestConfig, NamedFile, QtiVersion, encode_zip,
    generate_manifest,
};

use super::fragment::xml_attr;
use super::item_xml::{QTI_NAMESPACE, QTI_SCHEMA, XSI_NAMESPACE, render_item};
use crate::{
    EngineError, RenderHooks, WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "blackboard_qti_v2_1";
const ITEM_DIRECTORY: &str = "qti21_items";

/// Creates the fixed-registry Blackboard QTI 2.1 writer.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(BlackboardQti21Writer)
}

pub(super) struct BlackboardQti21Writer;

impl Writer for BlackboardQti21Writer {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> MediaPolicy {
        crate::engine(NAME)
            .expect("registered Blackboard QTI writer")
            .media_policy
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(NAME)
            .expect("registered Blackboard QTI writer")
            .supported_kinds
    }

    fn write_package(
        &self,
        bank: &qti_core::ItemBank,
        assets: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        let collected = bank.collect_assets(assets)?;
        let all_assets = collected.assets().to_vec();
        for asset in &all_assets {
            if asset.kind == AssetKind::Local && asset.output_name.is_none() {
                return Err(invalid(
                    "media",
                    format!("asset '{}' has no package name", asset.src),
                ));
            }
        }
        let names = all_assets
            .iter()
            .filter_map(|asset| {
                asset
                    .output_name
                    .as_deref()
                    .map(|name| (asset.src.as_str(), name))
            })
            .collect::<BTreeMap<_, _>>();
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &qti_core::Item| {
            let dependencies = collected.dependencies_for(item.crc()).unwrap_or_default();
            let decision = apply_media_policy(
                MediaPolicy::Package,
                dependencies,
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| invalid("media", error.to_string()))?;
            // `render_bank` visits renderable items in bank order, preserving the same ordering
            // for externally useful media diagnostics.
            warnings.borrow_mut().extend(decision.warnings);
            rewrite_item_media(item, |src| {
                names
                    .get(src)
                    .map_or_else(|| src.to_owned(), |name| format!("../{name}"))
            })
            .map_err(|error| invalid("HTML", error.to_string()))
        };
        let rendered = render_bank(
            bank,
            self.supported_kinds(),
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;

        let packaged_assets = all_assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Local)
            .cloned()
            .collect::<Vec<_>>();
        let packageable = packageable_assets(&packaged_assets)
            .map_err(|error| invalid("media", error.to_string()))?;
        let mut map = EntryMap::new();
        let mut item_resources = Vec::with_capacity(rendered.len());
        for (position, item) in rendered.iter().enumerate() {
            let file_name = format!("item_{:05}.xml", position + 1);
            let path = format!("{ITEM_DIRECTORY}/{file_name}");
            map.insert(path.clone(), item.xml.as_bytes().to_vec());
            let dependencies = collected
                .dependencies_for(&item.crc)
                .unwrap_or_default()
                .iter()
                .filter_map(|dependency| {
                    packaged_assets
                        .iter()
                        .position(|asset| asset.src == dependency.src)
                })
                .collect();
            item_resources.push(ItemResource {
                href: path,
                asset_indexes: dependencies,
            });
        }
        if item_resources.is_empty() {
            return Err(invalid("QTI 2.1", "bank contains no renderable items"));
        }
        map.insert(
            format!("{ITEM_DIRECTORY}/assessment_meta.xml"),
            assessment_meta("QTI 2.1", rendered.len()).into_bytes(),
        );
        for asset in packageable {
            let name = asset
                .output_name
                .clone()
                .expect("packageable assets have names");
            let bytes = asset
                .read_bytes()
                .map_err(|error| invalid("media", error.to_string()))?;
            map.insert(name, bytes);
        }
        let manifest = generate_manifest(&ManifestConfig {
            package_name: "QTI 2.1".to_owned(),
            version: QtiVersion::V2p1,
            items: item_resources,
            assets: &packaged_assets,
            metadata_date: context.document.date.clone(),
        })?;
        map.insert("imsmanifest.xml".to_owned(), manifest);
        let bytes = encode_zip(&map, std::iter::empty::<&str>())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary: NamedFile::new(context.output_name().to_owned(), bytes)?,
                companions: Vec::new(),
            }),
            warnings: warnings.into_inner(),
        })
    }
}

fn assessment_meta(title: &str, count: usize) -> String {
    let references = (1..=count)
        .map(|number| {
            format!(
                "<assessmentItemRef identifier=\"item_{number:05}\" href=\"item_{number:05}.xml\"/>"
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><assessmentTest xmlns=\"{QTI_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{QTI_SCHEMA}\" identifier=\"assessment_meta\" title=\"{}\"><testPart identifier=\"test_part\" navigationMode=\"nonlinear\" submissionMode=\"simultaneous\"><assessmentSection identifier=\"section_part\" visible=\"false\" title=\"Question Pool\">{references}</assessmentSection></testPart></assessmentTest>",
        xml_attr(title)
    )
}

fn invalid(format: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format,
        message: message.into(),
    }
}
