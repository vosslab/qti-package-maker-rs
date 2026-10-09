//! Package-ready asset collection from caller-owned media sources.

use std::collections::{BTreeMap, HashSet};

use indexmap::IndexMap;

use super::{BankError, ItemBank};
use crate::crc::ItemCrc;
use crate::media::{self, AssetSource, MediaAsset};

/// Derived package-ready media for an [`ItemBank`].
///
/// Assets are keyed by the exact authored `src` spelling and are freshly rebuilt on every
/// [`ItemBank::collect_assets`] call. No asset registry is retained by the bank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedAssets {
    assets: Vec<MediaAsset>,
    item_dependencies: IndexMap<ItemCrc, Vec<MediaAsset>>,
}

impl CollectedAssets {
    /// Returns each distinct authored image source in deterministic source order.
    #[must_use]
    pub fn assets(&self) -> &[MediaAsset] {
        &self.assets
    }

    /// Returns the distinct assets referenced by one item in document order.
    #[must_use]
    pub fn dependencies_for(&self, crc: &ItemCrc) -> Option<&[MediaAsset]> {
        self.item_dependencies.get(crc).map(Vec::as_slice)
    }

    /// Iterates item dependencies in bank insertion order.
    pub fn iter_item_dependencies(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ItemCrc, &[MediaAsset])> {
        self.item_dependencies
            .iter()
            .map(|(crc, assets)| (crc, assets.as_slice()))
    }
}

/// The conversion step that failed while collecting one item's media.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetCollectionAction {
    /// Scanning an authored HTML field for image sources.
    ScanHtml,
    /// Resolving an authored image source into a media asset.
    ResolveAsset,
}

impl std::fmt::Display for AssetCollectionAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanHtml => formatter.write_str("scan HTML"),
            Self::ResolveAsset => formatter.write_str("resolve asset"),
        }
    }
}

impl ItemBank {
    /// Resolves all image references into a fresh package-ready asset view.
    ///
    /// Resolution is read-only with respect to item content. Each exact `src` is resolved once,
    /// dependencies retain document order per item, and collision-safe output names are assigned
    /// across the full bank.
    pub fn collect_assets(&self, source: &dyn AssetSource) -> Result<CollectedAssets, BankError> {
        self.collect_with(|src| media::resolve_asset(src, source))
    }

    /// Inspects source metadata and assigns names without reading local payloads.
    pub fn inspect_assets(&self) -> Result<CollectedAssets, BankError> {
        self.collect_with(media::describe_asset)
    }

    fn collect_with(
        &self,
        resolve: impl Fn(&str) -> Result<MediaAsset, media::MediaError>,
    ) -> Result<CollectedAssets, BankError> {
        let mut asset_by_src = BTreeMap::new();
        let mut item_dependencies = IndexMap::new();

        for (crc, item) in &self.items {
            let mut dependencies = Vec::new();
            let mut seen_sources = HashSet::new();
            for (_, html) in media::item_html_fields(item) {
                let sources = media::scan_html_for_assets(html).map_err(|source| {
                    BankError::CollectAsset {
                        item_crc: *crc,
                        src: None,
                        action: AssetCollectionAction::ScanHtml,
                        source,
                    }
                })?;
                for src in sources {
                    if !asset_by_src.contains_key(&src) {
                        let asset = resolve(&src).map_err(|source| BankError::CollectAsset {
                            item_crc: *crc,
                            src: Some(src.clone()),
                            action: AssetCollectionAction::ResolveAsset,
                            source,
                        })?;
                        asset_by_src.insert(src.clone(), asset);
                    }
                    let asset = asset_by_src
                        .get(&src)
                        .expect("asset was inserted when its source was first encountered");
                    if seen_sources.insert(src) {
                        dependencies.push(asset.clone());
                    }
                }
            }
            if !dependencies.is_empty() {
                item_dependencies.insert(*crc, dependencies);
            }
        }

        let mut assets = asset_by_src.into_values().collect::<Vec<_>>();
        media::assign_output_names(&mut assets);
        let output_names = assets
            .iter()
            .filter_map(|asset| {
                asset
                    .output_name
                    .as_ref()
                    .map(|name| (asset.src.clone(), name.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        for dependencies in item_dependencies.values_mut() {
            for asset in dependencies {
                asset.output_name = output_names.get(&asset.src).cloned();
            }
        }

        Ok(CollectedAssets {
            assets,
            item_dependencies,
        })
    }
}
