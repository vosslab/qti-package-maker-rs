use std::collections::BTreeMap;

use thiserror::Error;

use super::{AssetKind, MediaAsset};

/// A writer's declared outcome for authored images.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaPolicy {
    Package,
    ReferenceWarn,
    PlaceholderWarn,
    Fail,
}

/// The operation recorded in a diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaAction {
    Packaged,
    KeptVerbatim,
    Substituted,
    Rejected,
}

/// A provenance-complete non-fatal media diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaWarning {
    pub engine_name: String,
    pub item_crc: String,
    pub src: String,
    pub resolved: String,
    pub action: MediaAction,
    pub reason: String,
}

/// The output of policy routing for one item's assets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaPolicyDecision {
    pub policy: MediaPolicy,
    pub warnings: Vec<MediaWarning>,
    pub placeholders: BTreeMap<String, String>,
}

/// A policy rejection with all item provenance required by the media contract.
#[derive(Debug, Error, Eq, PartialEq)]
#[error(
    "engine '{engine_name}' item {item_crc}: src '{src}' resolved to '{resolved}' was {action:?}: {reason}"
)]
pub struct MediaPolicyError {
    pub engine_name: String,
    pub item_crc: String,
    pub src: String,
    pub resolved: String,
    pub action: MediaAction,
    pub reason: String,
}

/// Applies one policy without performing package I/O.
pub fn apply_media_policy(
    policy: MediaPolicy,
    assets: &[MediaAsset],
    engine_name: &str,
    item_crc: &str,
) -> Result<MediaPolicyDecision, Box<MediaPolicyError>> {
    if policy == MediaPolicy::Fail && !assets.is_empty() {
        return Err(Box::new(rejection(
            engine_name,
            item_crc,
            &assets[0],
            "engine forbids images",
        )));
    }
    let mut warnings = Vec::new();
    let mut placeholders = BTreeMap::new();
    for asset in assets {
        match policy {
            MediaPolicy::Fail => {}
            MediaPolicy::ReferenceWarn => warnings.push(warning(
                engine_name,
                item_crc,
                asset,
                MediaAction::KeptVerbatim,
                "image reference kept verbatim; engine does not transport image files",
            )),
            MediaPolicy::PlaceholderWarn => {
                let text = placeholder_text(asset);
                placeholders.insert(asset.src.clone(), text);
                warnings.push(warning(
                    engine_name,
                    item_crc,
                    asset,
                    MediaAction::Substituted,
                    "image replaced with a text placeholder; format carries no image markup",
                ));
            }
            MediaPolicy::Package => match asset.kind {
                AssetKind::External => warnings.push(warning(
                    engine_name,
                    item_crc,
                    asset,
                    MediaAction::KeptVerbatim,
                    "external image URL kept verbatim; not bundled into the package",
                )),
                AssetKind::DataUri => {
                    return Err(Box::new(rejection(
                        engine_name,
                        item_crc,
                        asset,
                        "data URI image cannot be bundled as a file",
                    )));
                }
                AssetKind::Local if asset.mime_type.as_deref() == Some("image/svg+xml") => warnings
                    .push(warning(
                        engine_name,
                        item_crc,
                        asset,
                        MediaAction::Packaged,
                        "SVG image packaged, but LMS support is not guaranteed",
                    )),
                AssetKind::Local => {}
            },
        }
    }
    Ok(MediaPolicyDecision {
        policy,
        warnings,
        placeholders,
    })
}

/// Returns the human-readable placeholder used by text-only writers.
#[must_use]
pub fn placeholder_text(asset: &MediaAsset) -> String {
    // Frozen Python priority: a package-assigned name, then a data URI label, then source basename.
    let name = if let Some(output_name) = asset.output_name.as_deref() {
        output_name
    } else if asset.kind == AssetKind::DataUri {
        "embedded image"
    } else {
        asset
            .src
            .split('?')
            .next()
            .unwrap_or(&asset.src)
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(&asset.src)
    };
    format!("[image: {name}]")
}

fn resolved(asset: &MediaAsset) -> String {
    asset
        .file_path
        .as_ref()
        .map_or_else(|| asset.src.clone(), |path| path.display().to_string())
}

fn warning(
    engine_name: &str,
    item_crc: &str,
    asset: &MediaAsset,
    action: MediaAction,
    reason: &str,
) -> MediaWarning {
    MediaWarning {
        engine_name: engine_name.to_owned(),
        item_crc: item_crc.to_owned(),
        src: asset.src.clone(),
        resolved: resolved(asset),
        action,
        reason: reason.to_owned(),
    }
}

fn rejection(
    engine_name: &str,
    item_crc: &str,
    asset: &MediaAsset,
    reason: &str,
) -> MediaPolicyError {
    MediaPolicyError {
        engine_name: engine_name.to_owned(),
        item_crc: item_crc.to_owned(),
        src: asset.src.clone(),
        resolved: resolved(asset),
        action: MediaAction::Rejected,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaAction, MediaPolicy, apply_media_policy};
    use crate::media::{AssetKind, MediaAsset};

    fn local() -> MediaAsset {
        MediaAsset {
            src: "image.png".to_owned(),
            kind: AssetKind::Local,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: None,
            output_name: Some("image.png".to_owned()),
            content_hash: None,
        }
    }

    #[test]
    fn every_policy_has_a_distinct_explicit_outcome() {
        let asset = local();
        let package = apply_media_policy(
            MediaPolicy::Package,
            std::slice::from_ref(&asset),
            "canvas",
            "abcd",
        )
        .expect("local package policy");
        assert!(package.warnings.is_empty());

        let reference = apply_media_policy(
            MediaPolicy::ReferenceWarn,
            std::slice::from_ref(&asset),
            "text",
            "abcd",
        )
        .expect("reference policy");
        assert_eq!(reference.warnings[0].action, MediaAction::KeptVerbatim);

        let placeholder = apply_media_policy(
            MediaPolicy::PlaceholderWarn,
            std::slice::from_ref(&asset),
            "aiken",
            "abcd",
        )
        .expect("placeholder policy");
        assert_eq!(placeholder.placeholders["image.png"], "[image: image.png]");
        assert_eq!(placeholder.warnings[0].action, MediaAction::Substituted);

        let rejected = apply_media_policy(MediaPolicy::Fail, &[asset], "ultra", "abcd")
            .expect_err("fail policy rejects an image");
        assert_eq!(rejected.action, MediaAction::Rejected);
        assert_eq!(rejected.engine_name, "ultra");
        assert_eq!(rejected.item_crc, "abcd");
    }

    #[test]
    fn package_rejects_data_uri_and_keeps_remote_verbatim() {
        let remote = MediaAsset {
            src: "https://example.test/image.png".to_owned(),
            kind: AssetKind::External,
            mime_type: None,
            file_path: None,
            data_bytes: None,
            output_name: None,
            content_hash: None,
        };
        let decision = apply_media_policy(MediaPolicy::Package, &[remote], "canvas", "abcd")
            .expect("remote reference is kept");
        assert_eq!(decision.warnings[0].action, MediaAction::KeptVerbatim);
        let embedded = MediaAsset {
            src: "data:image/png;base64,AA==".to_owned(),
            kind: AssetKind::DataUri,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: Some(vec![0]),
            output_name: None,
            content_hash: None,
        };
        assert!(apply_media_policy(MediaPolicy::Package, &[embedded], "canvas", "abcd").is_err());
    }

    #[test]
    fn all_policies_route_local_remote_and_data_sources_with_provenance() {
        let local = local();
        let remote = MediaAsset {
            src: "https://example.test/image.png".to_owned(),
            kind: AssetKind::External,
            mime_type: None,
            file_path: None,
            data_bytes: None,
            output_name: None,
            content_hash: None,
        };
        let data = MediaAsset {
            src: "data:image/png;base64,AA==".to_owned(),
            kind: AssetKind::DataUri,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: Some(vec![0]),
            output_name: None,
            content_hash: None,
        };
        let assets = [local, remote, data];

        let package = apply_media_policy(MediaPolicy::Package, &assets, "canvas", "c0de")
            .expect_err("data URI blocks file packaging");
        assert_eq!(package.engine_name, "canvas");
        assert_eq!(package.item_crc, "c0de");
        assert_eq!(package.src, assets[2].src);
        assert_eq!(package.resolved, assets[2].src);
        assert_eq!(package.action, MediaAction::Rejected);

        let reference = apply_media_policy(MediaPolicy::ReferenceWarn, &assets, "text", "c0de")
            .expect("reference policy");
        assert_eq!(reference.warnings.len(), 3);
        assert!(reference.warnings.iter().all(|warning| {
            warning.engine_name == "text"
                && warning.item_crc == "c0de"
                && !warning.src.is_empty()
                && !warning.resolved.is_empty()
                && warning.action == MediaAction::KeptVerbatim
        }));

        let placeholders =
            apply_media_policy(MediaPolicy::PlaceholderWarn, &assets, "human", "c0de")
                .expect("placeholder policy");
        assert_eq!(placeholders.warnings.len(), 3);
        assert_eq!(placeholders.placeholders.len(), 3);

        assert!(apply_media_policy(MediaPolicy::Fail, &assets, "ultra", "c0de").is_err());
    }

    #[test]
    fn placeholders_follow_frozen_output_name_data_then_source_priority() {
        let local = MediaAsset {
            src: "figures/image.png".to_owned(),
            kind: AssetKind::Local,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: None,
            output_name: Some("image(2).png".to_owned()),
            content_hash: None,
        };
        let remote_with_output_name = MediaAsset {
            src: "https://example.test/media/remote.png?cache=4".to_owned(),
            kind: AssetKind::External,
            mime_type: None,
            file_path: None,
            data_bytes: None,
            output_name: Some("different-name.png".to_owned()),
            content_hash: None,
        };
        let embedded = MediaAsset {
            src: "data:image/png;base64,AA==".to_owned(),
            kind: AssetKind::DataUri,
            mime_type: Some("image/png".to_owned()),
            file_path: None,
            data_bytes: Some(vec![0]),
            output_name: None,
            content_hash: None,
        };
        let remote_without_output_name = MediaAsset {
            output_name: None,
            ..remote_with_output_name.clone()
        };
        assert_eq!(super::placeholder_text(&local), "[image: image(2).png]");
        assert_eq!(
            super::placeholder_text(&remote_with_output_name),
            "[image: different-name.png]"
        );
        assert_eq!(
            super::placeholder_text(&remote_without_output_name),
            "[image: remote.png]"
        );
        assert_eq!(
            super::placeholder_text(&embedded),
            "[image: embedded image]"
        );
    }
}
