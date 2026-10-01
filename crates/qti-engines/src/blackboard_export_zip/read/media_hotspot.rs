//! Blackboard hotspot media recovery from pool-relative `matapplication` URIs.

use std::collections::BTreeMap;

use crate::EngineError;

use super::discovery::{Node, checked_path, descendants, invalid, parse_xml};

pub(super) fn hotspot_media(
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Node,
    pool: &str,
    data: &[u8],
) -> Result<BTreeMap<String, (String, String)>, EngineError> {
    let pool_root = pool_resource_base(manifest, pool)?;
    let document =
        parse_xml(data).map_err(|message| invalid("pool XML", format!("{pool}: {message}")))?;
    let mut recovered = BTreeMap::new();
    for application in descendants(&document, "matapplication") {
        let uri = application
            .attributes
            .get("uri")
            .ok_or_else(|| invalid("media", format!("matapplication in '{pool}' lacks uri")))?;
        checked_path(uri).map_err(|message| invalid("media", message))?;
        let path = format!("{pool_root}/{uri}");
        if !files.contains_key(&path) {
            return Err(invalid(
                "media",
                format!("matapplication URI '{uri}' resolves to missing '{path}'"),
            ));
        }
        let label = application
            .attributes
            .get("label")
            .cloned()
            .unwrap_or_else(|| uri.rsplit('/').next().unwrap_or("image.png").to_owned());
        recovered.insert(format!("hotspot:{pool}:{uri}"), (label, path));
    }
    Ok(recovered)
}

fn pool_resource_base(manifest: &Node, pool_file: &str) -> Result<String, EngineError> {
    for resource in descendants(manifest, "resource") {
        if resource
            .attributes
            .get("type")
            .is_some_and(|typ| typ == "assessment/x-bb-qti-pool")
            && resource
                .bb_attributes
                .get("file")
                .is_some_and(|file| file == pool_file)
        {
            let base = resource
                .xml_attributes
                .get("base")
                .cloned()
                .unwrap_or_else(|| pool_file.trim_end_matches(".dat").to_owned());
            checked_path(&base).map_err(|message| invalid("manifest", message))?;
            return Ok(base);
        }
    }
    Err(invalid(
        "manifest",
        format!("pool resource '{pool_file}' disappeared during media recovery"),
    ))
}
