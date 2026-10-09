use std::collections::{BTreeMap, BTreeSet};

use super::resolve::source_basename;
use super::{AssetKind, MediaAsset};

/// Assigns collision-safe names deterministically by source spelling.
pub fn assign_output_names(assets: &mut [MediaAsset]) {
    let mut representatives = BTreeMap::new();
    for asset in &*assets {
        if asset.kind == AssetKind::Local {
            representatives.entry(asset.src.clone()).or_insert(asset);
        }
    }
    let mut names = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (src, asset) in representatives {
        let natural = source_basename(&asset.src);
        let (stem, extension) = natural
            .rsplit_once('.')
            .filter(|(stem, _)| !stem.is_empty())
            .map_or((natural, String::new()), |(stem, extension)| {
                (stem, format!(".{extension}"))
            });
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
