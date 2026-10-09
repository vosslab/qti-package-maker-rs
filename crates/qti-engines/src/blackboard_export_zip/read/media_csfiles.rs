//! Blackboard csfiles and hotspot-media recovery.

use std::collections::{BTreeMap, BTreeSet};

use qti_core::MemoryAssets;

use crate::EngineError;

use super::discovery::{Node, child, descendants, invalid, parse_xml, text};

const BB_FILE: &str = "file";
const TOKEN_PREFIX: &str = "@X@EmbeddedFile.requestUrlStub@X@bbcswebdav/xid-";

pub(super) struct RecoveredMedia {
    pub(super) assets: MemoryAssets,
    pub(super) source_names: BTreeMap<String, String>,
}

pub(super) fn recover_media(
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Node,
    pools: &[String],
) -> Result<RecoveredMedia, EngineError> {
    // The identity is either a csfiles token or a pool-relative hotspot URI.
    // Keeping both in one table makes collision handling deterministic across
    // Blackboard's two unrelated media transport mechanisms.
    let mut token_names = BTreeMap::new();
    let link_ids = cs_link_ids(files, manifest)?;
    for pool in pools {
        let data = files
            .get(pool)
            .ok_or_else(|| invalid("package", format!("manifest pool file '{pool}' is missing")))?;
        let input = String::from_utf8_lossy(data);
        for xid in xid_tokens(&input) {
            if !link_ids.contains(&xid) {
                return Err(invalid(
                    "media",
                    format!("csfiles token xid-{xid} has no CSResourceLinks resourceId"),
                ));
            }
            let (path, sidecar) = find_csfile(files, &xid)?;
            let desired = lom_name(sidecar).unwrap_or_else(|| {
                path.rsplit('/')
                    .next()
                    .unwrap_or("image.png")
                    .replace(&format!("__xid-{xid}."), "")
            });
            token_names.insert(xid, (desired, path));
        }
        token_names.extend(super::media_hotspot::hotspot_media(
            files, manifest, pool, data,
        )?);
    }
    if token_names.is_empty() {
        return Ok(RecoveredMedia {
            assets: MemoryAssets::new(),
            source_names: BTreeMap::new(),
        });
    }
    let mut assets = MemoryAssets::new();
    let mut used = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (xid, (desired, path)) in token_names {
        let name = unique_name(&desired, &mut used);
        let bytes = files.get(&path).expect("csfile exists");
        assets
            .insert(name.clone(), bytes.clone())
            .map_err(qti_core::BankError::from)?;
        if !xid.starts_with("hotspot:") {
            names.insert(format!("{TOKEN_PREFIX}{xid}"), name);
        }
    }
    Ok(RecoveredMedia {
        assets,
        source_names: names,
    })
}

fn cs_link_ids(
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Node,
) -> Result<BTreeSet<String>, EngineError> {
    let mut ids = BTreeSet::new();
    for resource in descendants(manifest, "resource") {
        if resource
            .attributes
            .get("type")
            .is_some_and(|typ| typ == "course/x-bb-csresourcelinks")
        {
            let file = resource
                .bb_attributes
                .get(BB_FILE)
                .ok_or_else(|| invalid("manifest", "CSResourceLinks resource lacks bb:file"))?;
            let bytes = files.get(file).ok_or_else(|| {
                invalid(
                    "package",
                    format!("CSResourceLinks file '{file}' is missing"),
                )
            })?;
            let root =
                parse_xml(bytes).map_err(|message| invalid("CSResourceLinks XML", message))?;
            for link in descendants(&root, "cms_resource_link") {
                if let Some(resource_id) = child(link, "resourceId") {
                    ids.insert(text(resource_id));
                }
            }
        }
    }
    Ok(ids)
}

fn xid_tokens(data: &str) -> BTreeSet<String> {
    data.match_indices(TOKEN_PREFIX)
        .filter_map(|(offset, _)| {
            let rest = &data[offset + TOKEN_PREFIX.len()..];
            let value = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            (!value.is_empty()).then_some(value)
        })
        .collect()
}

fn find_csfile<'a>(
    files: &'a BTreeMap<String, Vec<u8>>,
    xid: &str,
) -> Result<(String, &'a [u8]), EngineError> {
    let prefix = format!("csfiles/home_dir/__xid-{xid}.");
    let path = files
        .keys()
        .find(|path| path.starts_with(&prefix) && !path.ends_with(".xml"))
        .ok_or_else(|| invalid("media", format!("csfiles binary for xid-{xid} is missing")))?
        .to_owned();
    let sidecar = files
        .get(&format!("{path}.xml"))
        .ok_or_else(|| invalid("media", format!("LOM sidecar for xid-{xid} is missing")))?;
    Ok((path, sidecar.as_slice()))
}

fn lom_name(sidecar: &[u8]) -> Option<String> {
    let root = parse_xml(sidecar).ok()?;
    let identifier = descendants(&root, "identifier").next()?;
    let value = text(identifier);
    value
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn unique_name(desired: &str, used: &mut BTreeSet<String>) -> String {
    let desired = desired
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("image.png");
    if used.insert(desired.to_owned()) {
        return desired.to_owned();
    }
    let (stem, ext) = match desired.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (desired, String::new()),
    };
    for suffix in 2.. {
        let candidate = format!("{stem}_{suffix}{ext}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded collision suffix space")
}
