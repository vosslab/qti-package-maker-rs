use super::{AssetKind, MediaAsset, MediaError};

/// Returns local assets a packaging writer may copy after policy approval.
/// The caller owns output I/O; this shared layer never creates a package or a network request.
pub fn packageable_assets(assets: &[MediaAsset]) -> Result<Vec<&MediaAsset>, MediaError> {
    assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| {
            if asset.output_name.is_none() {
                return Err(MediaError::Html(format!(
                    "local image '{}' has no assigned output name",
                    asset.src
                )));
            }
            Ok(asset)
        })
        .collect()
}
