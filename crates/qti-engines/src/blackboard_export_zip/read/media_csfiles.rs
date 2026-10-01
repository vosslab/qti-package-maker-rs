//! Blackboard csfiles and hotspot-media recovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qti_core::{ItemBank, MediaBaseDir};

use crate::EngineError;

use super::discovery::{Node, child, descendants, invalid, parse_xml, text};

const BB_FILE: &str = "file";
const TOKEN_PREFIX: &str = "@X@EmbeddedFile.requestUrlStub@X@bbcswebdav/xid-";

pub(super) struct RecoveredMedia {
    pub(super) base: Option<MediaBaseDir>,
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
        let data = files.get(pool).expect("pool checked by caller");
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
            base: None,
            source_names: BTreeMap::new(),
        });
    }
    let base = MediaBaseDir::temporary().map_err(EngineError::from)?;
    let mut used = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (xid, (desired, path)) in token_names {
        let name = unique_name(&desired, &mut used);
        let bytes = files.get(&path).expect("csfile exists");
        let mut holder = ItemBank::with_media_base_dir(true, base.clone());
        holder.add_image(&name, bytes).map_err(EngineError::from)?;
        if !xid.starts_with("hotspot:") {
            names.insert(format!("{TOKEN_PREFIX}{xid}"), name);
        }
    }
    Ok(RecoveredMedia {
        base: Some(base),
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
    let desired = Path::new(desired)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("image.png");
    if used.insert(desired.to_owned()) {
        return desired.to_owned();
    }
    let path = Path::new(desired);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    for suffix in 2.. {
        let candidate = format!("{stem}_{suffix}{ext}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded collision suffix space")
}
