use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::MediaAsset;

/// Assigns collision-safe names deterministically by source spelling.
pub fn assign_output_names(assets: &mut [MediaAsset]) {
    let mut representatives = BTreeMap::new();
    for asset in &*assets {
        if asset.file_path.is_some() {
            representatives.entry(asset.src.clone()).or_insert(asset);
        }
    }
    let mut names = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (src, asset) in representatives {
        let authored_path = asset.src.split(['?', '#']).next().unwrap_or(&asset.src);
        let natural = Path::new(authored_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("image");
        let path = Path::new(natural);
        let stem = path
            .file_stem()
            .and_then(|part| part.to_str())
            .unwrap_or("image");
        let extension = path
            .extension()
            .and_then(|part| part.to_str())
            .map_or(String::new(), |part| format!(".{part}"));
        let mut candidate = natural.to_owned();
        let mut suffix = 1;
        while !used.insert(candidate.clone()) {
            candidate = format!("{stem}({suffix}){extension}");
            suffix += 1;
        }
        names.insert(src, candidate);
    }
    for asset in assets {
        if let Some(name) = names.get(&asset.src) {
            asset.output_name = Some(name.clone());
        }
    }
}
