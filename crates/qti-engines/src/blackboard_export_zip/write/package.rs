//! Blackboard pool ZIP assembly, media ownership, and package sidecars.

use std::collections::{BTreeMap, BTreeSet};

use crate::{EngineError, RenderHooks, WriteArtifact, WriteContext, WriteOutcome, render_bank};
use qti_core::media::{
    AssetKind, AssetSource, MediaAction, MediaWarning, apply_media_policy, rewrite_item_media,
};
use qti_core::{EntryMap, NamedFile, encode_zip};

use super::super::NAME;
use super::html::xml;
use super::item::render_item;

const TOKEN_PREFIX: &str = "@X@EmbeddedFile.requestUrlStub@X@bbcswebdav/xid-";

pub(in crate::blackboard_export_zip) fn write_package(
    bank: &qti_core::ItemBank,
    assets: &dyn AssetSource,
    context: &WriteContext,
) -> Result<WriteOutcome, EngineError> {
    let kinds = crate::engine(NAME)
        .expect("registered Blackboard export writer")
        .supported_kinds;
    let mut warnings = Vec::new();
    for item in bank
        .iter_ordered()
        .filter(|item| !kinds.contains(&item.kind()))
    {
        warnings.push(MediaWarning {
            engine_name: NAME.to_owned(),
            item_crc: item.crc().to_string(),
            src: "ORDER".to_owned(),
            resolved: "ORDER".to_owned(),
            action: MediaAction::KeptVerbatim,
            reason: "unsupported ORDER item was skipped".to_owned(),
        });
    }
    // Unsupported items cannot contribute package media or require source reads.
    let mut media_bank = qti_core::ItemBank::new(true);
    for item in bank
        .iter_ordered()
        .filter(|item| kinds.contains(&item.kind()))
    {
        media_bank.add_item(item.clone())?;
    }
    let collected = media_bank.collect_assets(assets)?;
    let assets = collected.assets();
    // ASVS 5.3.2: emit generated csfiles paths only for sources retained in
    // this package's rendered pool.  Unsupported items have no ASI object and
    // therefore cannot safely own a CSResourceLinks parentId.
    let rendered_local_sources = bank
        .iter_ordered()
        .filter(|item| kinds.contains(&item.kind()))
        .flat_map(|item| collected.dependencies_for(item.crc()).unwrap_or_default())
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| asset.src.clone())
        .collect::<BTreeSet<_>>();
    let mut xid_by_source = BTreeMap::new();
    let mut xid_by_content = BTreeMap::new();
    let mut embedded = Vec::new();
    for asset in assets
        .iter()
        .filter(|asset| rendered_local_sources.contains(&asset.src))
    {
        let output_name = asset.output_name.as_deref().unwrap_or(&asset.src);
        // Share bytes only when their packaged media interpretation also agrees.
        let content = (
            asset.mime_type.as_deref(),
            extension(output_name),
            asset.shared_bytes().map_err(media_error)?,
        );
        let xid = xid_by_content.entry(content).or_insert_with(|| {
            let xid = format!("{}_1", embedded.len() + 1);
            embedded.push((xid.clone(), asset));
            xid
        });
        xid_by_source.insert(asset.src.as_str(), xid.clone());
    }
    // Blackboard assigns every xid to the first item that refers to any of its
    // sources in bank order. The same ASI form is emitted in `skeleton`, so Learn can
    // resolve a CSResourceLinks parentId without relying on server metadata.
    let mut parent_by_xid = BTreeMap::new();
    for item in bank
        .iter_ordered()
        .filter(|item| kinds.contains(&item.kind()))
    {
        for asset in collected.dependencies_for(item.crc()).unwrap_or_default() {
            if let Some(xid) = xid_by_source.get(asset.src.as_str()) {
                parent_by_xid
                    .entry(xid.clone())
                    .or_insert_with(|| format!("_{}_1", item.crc()));
            }
        }
    }
    for item in bank
        .iter_ordered()
        .filter(|item| kinds.contains(&item.kind()))
    {
        let decision = apply_media_policy(
            qti_core::media::MediaPolicy::Package,
            collected.dependencies_for(item.crc()).unwrap_or_default(),
            NAME,
            &item.crc().to_string(),
        )
        .map_err(media_error)?;
        warnings.extend(decision.warnings);
    }
    let pre = |item: &qti_core::Item| {
        rewrite_item_media(item, |source| {
            xid_by_source
                .get(source)
                .map(|xid| format!("{TOKEN_PREFIX}{xid}"))
                .unwrap_or_else(|| source.to_owned())
        })
        .map_err(media_error)
    };
    let items = render_bank(
        bank,
        kinds,
        render_item,
        RenderHooks {
            pre_render: Some(&pre),
            post_render: None,
        },
    )?;
    let mut archive = EntryMap::new();
    archive.insert("imsmanifest.xml".to_owned(), manifest().into_bytes());
    archive.insert(
        "res00002.dat".to_owned(),
        pool_document(&items).into_bytes(),
    );
    archive.insert(
        "res00005.dat".to_owned(),
        resource_links(&embedded, &parent_by_xid)?.into_bytes(),
    );
    for (name, body) in fixed_sidecars() {
        archive.insert(name.to_owned(), body.into_bytes());
    }
    archive.insert(
        ".bb-package-info".to_owned(),
        b"PackageFormatVersion=6.0\n".to_vec(),
    );
    archive.insert(
        ".bb-log-info".to_owned(),
        b"Blackboard pool export\n".to_vec(),
    );
    for (xid, asset) in embedded {
        let output_name = asset.output_name.as_deref().unwrap_or(&asset.src);
        let extension = extension(output_name);
        let base = format!("csfiles/home_dir/__xid-{xid}.{extension}");
        let bytes = asset.read_bytes().map_err(media_error)?;
        archive.insert(base.clone(), bytes);
        archive.insert(
            format!("{base}.xml"),
            lom_sidecar(&xid, output_name).into_bytes(),
        );
    }
    let bytes = encode_zip(&archive, ["res00001/"])?;
    Ok(WriteOutcome {
        artifact: Some(WriteArtifact::File {
            primary: NamedFile::new(context.output_name().to_owned(), bytes)?,
            companions: Vec::new(),
        }),
        warnings,
    })
}

fn media_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format: "media",
        message: error.to_string(),
    }
}

fn extension(name: &str) -> &str {
    name.rsplit_once('.')
        .map_or("png", |(_, extension)| extension)
}

fn pool_document(items: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><questestinterop><assessment title=\"QTI package maker\"><section>{}</section></assessment></questestinterop>",
        items.join("")
    )
}

fn manifest() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest xmlns:bb=\"http://www.blackboard.com/content-packaging/\" identifier=\"main_manifest\"><organizations/><resources><resource bb:file=\"res00002.dat\" bb:title=\"QTI package maker\" identifier=\"res00002\" type=\"assessment/x-bb-qti-pool\" xml:base=\"res00002\"/><resource bb:file=\"res00005.dat\" bb:title=\"CSResourceLinks\" identifier=\"res00005\" type=\"course/x-bb-csresourcelinks\" xml:base=\"res00005\"/></resources></manifest>".to_owned()
}

fn fixed_sidecars() -> [(&'static str, String); 5] {
    [
        (
            "res00001.dat",
            "<?xml version=\"1.0\"?><COURSE/>".to_owned(),
        ),
        (
            "res00003.dat",
            "<?xml version=\"1.0\"?><ASSESSMENTCREATIONSETTINGS/>".to_owned(),
        ),
        (
            "res00004.dat",
            "<?xml version=\"1.0\"?><LEARNRUBRICS/>".to_owned(),
        ),
        (
            "res00006.dat",
            "<?xml version=\"1.0\"?><STDS_ALIGNMENTS/>".to_owned(),
        ),
        (
            "res00007.dat",
            "<?xml version=\"1.0\"?><COURSERUBRICASSOCIATIONS/>".to_owned(),
        ),
    ]
}

fn resource_links(
    embedded: &[(String, &qti_core::media::MediaAsset)],
    parent_by_xid: &BTreeMap<String, String>,
) -> Result<String, EngineError> {
    let mut links = String::new();
    for (xid, asset) in embedded {
        let parent = parent_by_xid
            .get(xid)
            .ok_or_else(|| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: format!("local asset '{}' has no owning item", asset.src),
            })?;
        links.push_str(&format!(
            "<cms_resource_link><parentId>{parent}</parentId><resourceId>{xid}</resourceId></cms_resource_link>"
        ));
    }
    Ok(format!(
        "<?xml version=\"1.0\"?><cms_resource_link_list>{links}</cms_resource_link_list>"
    ))
}

fn lom_sidecar(resource_id: &str, output_name: &str) -> String {
    const LOM_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsmd_rootv1p2p1";
    const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
    const LOM_SCHEMA_LOCATION: &str =
        "http://www.imsglobal.org/xsd/imsmd_rootv1p2p1 imsmd_rootv1p2p1.xsd";
    let output_name = output_name.rsplit('/').next().unwrap_or(output_name);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><lom xmlns=\"{LOM_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{LOM_SCHEMA_LOCATION}\"><relation><resource><identifier>{}#/courses/qti_package_maker/{}</identifier></resource></relation></lom>",
        xml(resource_id),
        xml(output_name)
    )
}
