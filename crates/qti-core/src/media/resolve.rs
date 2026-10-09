use std::collections::HashSet;
use std::sync::Arc;

use base64::Engine;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use percent_encoding::percent_decode_str;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::AssetSource;
use crate::{FieldId, Item, ItemBody, MediaRef};

/// The origin of an image reference in authored item HTML.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    /// A local image reference served by the caller's asset source.
    Local,
    /// An `http`, `https`, or protocol-relative URL.  This module never fetches it.
    External,
    /// An inline `data:` URI.
    DataUri,
}

/// A derived image record, keyed by the exact in-content `src` spelling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaAsset {
    pub src: String,
    pub kind: AssetKind,
    pub mime_type: Option<String>,
    /// Immutable payload shared by all dependencies for this exact source.
    pub data_bytes: Option<Arc<[u8]>>,
    pub output_name: Option<String>,
    pub content_hash: Option<[u8; 32]>,
}

impl MediaAsset {
    /// Copies a resolved payload owned by this asset.
    pub fn read_bytes(&self) -> Result<Vec<u8>, MediaError> {
        self.shared_bytes().map(|bytes| bytes.to_vec())
    }

    /// Shares the resolved immutable payload without copying image bytes.
    pub fn shared_bytes(&self) -> Result<Arc<[u8]>, MediaError> {
        self.data_bytes
            .clone()
            .ok_or_else(|| MediaError::NoPayload {
                src: self.src.clone(),
            })
    }
}

/// An asset together with the fingerprint location that referenced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemMediaAsset {
    pub asset: MediaAsset,
    pub field: FieldId,
    pub ordinal: usize,
}

/// Failures at the HTML, URI, or caller-owned asset boundary.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error(
        "unsupported image type '{extension}' for src '{src}'; supported: png, jpg, jpeg, gif, svg"
    )]
    UnsupportedMime { src: String, extension: String },
    #[error("invalid asset name '{src}': {reason}")]
    InvalidName { src: String, reason: String },
    #[error("image asset not found for src '{src}'")]
    MissingAsset { src: String },
    #[error("conflicting image payloads for exact source '{src}'")]
    ConflictingAsset { src: String },
    #[error("could not read image asset '{src}': {message}")]
    AssetRead { src: String, message: String },
    #[error("media asset '{src}' has no resolvable payload")]
    NoPayload { src: String },
    #[error("invalid data URI for src '{src}': {reason}")]
    InvalidDataUri { src: String, reason: String },
    #[error("could not inspect image HTML: {0}")]
    Html(String),
}

/// Classifies a source without touching the network or filesystem.
///
/// ASVS 12.2.1 / 1.3.6: this boundary never follows author-controlled URLs.
#[must_use]
pub fn classify_src(src: &str) -> AssetKind {
    let lower = src.trim().to_ascii_lowercase();
    if lower.starts_with("data:") {
        AssetKind::DataUri
    } else if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
    {
        AssetKind::External
    } else {
        AssetKind::Local
    }
}

/// Returns the supported MIME type based on a filename-like source.
pub fn guess_mime_type(name: &str) -> Result<&'static str, MediaError> {
    let path = name.split(['?', '#']).next().unwrap_or(name);
    let basename = source_basename(path);
    let extension = basename
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or_else(String::new, |(_, value)| {
            format!(".{}", value.to_ascii_lowercase())
        });
    match extension.as_str() {
        ".png" => Ok("image/png"),
        ".jpg" | ".jpeg" => Ok("image/jpeg"),
        ".gif" => Ok("image/gif"),
        ".svg" => Ok("image/svg+xml"),
        _ => Err(MediaError::UnsupportedMime {
            src: name.to_owned(),
            extension,
        }),
    }
}

/// Finds `<img src>` values in document order, ignoring script text and pseudo-attributes.
///
/// ASVS 1.1.2 / 1.2.1: source HTML is left in its original context; no decoded or rewritten
/// representation is stored here.
pub fn scan_html_for_assets(html: &str) -> Result<Vec<String>, MediaError> {
    if !html.to_ascii_lowercase().contains("<img") {
        return Ok(Vec::new());
    }
    let mut sources = Vec::new();
    rewrite_str(
        html,
        RewriteStrSettings::new().append_element_content_handler(element!("img[src]", |element| {
            if let Some(src) = element.get_attribute("src") {
                let src = src.trim();
                if !src.is_empty() {
                    sources.push(src.to_owned());
                }
            }
            Ok(())
        })),
    )
    .map_err(|error| MediaError::Html(error.to_string()))?;
    Ok(sources)
}

/// Describes one source without reading local bytes; inline data is decoded in memory.
pub fn describe_asset(src: &str) -> Result<MediaAsset, MediaError> {
    let kind = classify_src(src);
    let (mime_type, output_name) = if kind == AssetKind::Local {
        (
            Some(guess_mime_type(src)?.to_owned()),
            Some(source_basename(src).to_owned()),
        )
    } else {
        (None, None)
    };
    let (mime_type, data_bytes) = if kind == AssetKind::DataUri {
        let (mime_type, bytes) = parse_data_uri(src)?;
        (Some(mime_type), Some(bytes.into()))
    } else {
        (mime_type, None)
    };
    Ok(MediaAsset {
        src: src.to_owned(),
        kind,
        mime_type,
        data_bytes,
        output_name,
        content_hash: None,
    })
}

/// Inspects an item's distinct sources in field and document order without reading local payloads.
/// Reference and placeholder writers can apply media policy without requiring unused files.
pub fn inspect_item_assets(item: &Item) -> Result<Vec<MediaAsset>, MediaError> {
    let mut seen = HashSet::new();
    let mut assets = Vec::new();
    for (_, html) in item_html_fields(item) {
        for src in scan_html_for_assets(html)? {
            if seen.insert(src.clone()) {
                assets.push(describe_asset(&src)?);
            }
        }
    }
    Ok(assets)
}

/// Resolves a source into owned bytes; external URLs are classified and never fetched.
/// The source provider owns local authorization and exact-key resolution.
pub fn resolve_asset(src: &str, source: &dyn AssetSource) -> Result<MediaAsset, MediaError> {
    let mut asset = describe_asset(src)?;
    match asset.kind {
        AssetKind::External => {}
        AssetKind::DataUri => {}
        AssetKind::Local => {
            asset.data_bytes = Some(source.read(src)?.into_owned().into());
        }
    }
    Ok(asset)
}

pub(super) fn source_basename(src: &str) -> &str {
    src.split(['?', '#'])
        .next()
        .unwrap_or(src)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("image")
}

/// Hashes an asset's bytes with SHA-256 and records no identity change.
pub fn compute_content_hash(asset: &MediaAsset) -> Result<[u8; 32], MediaError> {
    Ok(Sha256::digest(asset.shared_bytes()?).into())
}

/// Resolves every hashable image in an item and returns fingerprint references in field/document order.
///
/// `ordinal` counts every `<img src>` in a field. External URLs produce no reference and are never
/// fetched; data URIs are hashable payloads and do produce a reference. This lets a local source
/// and a self-test engine's equivalent data URI compare by content and placement.
pub fn resolve_item_media_refs(
    item: &Item,
    source: &dyn AssetSource,
) -> Result<Vec<MediaRef>, MediaError> {
    let mut references = Vec::new();
    for (field, html) in item_html_fields(item) {
        for (ordinal, src) in scan_html_for_assets(html)?.into_iter().enumerate() {
            let asset = resolve_asset(&src, source)?;
            if asset.kind != AssetKind::External {
                references.push(MediaRef {
                    content_hash: compute_content_hash(&asset)?,
                    field: field.clone(),
                    ordinal,
                });
            }
        }
    }
    Ok(references)
}

/// Enumerates all HTML-bearing item fields using the same identifiers as fingerprinting.
pub fn item_html_fields(item: &Item) -> Vec<(FieldId, &str)> {
    let mut fields = vec![(FieldId::Stem, item.common().question_text.as_str())];
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            fields.extend(
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Choice(i), value.as_str())),
            );
            fields.push((FieldId::Answer(0), answer));
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            fields.extend(
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Choice(i), value.as_str())),
            );
            fields.extend(
                answers
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Answer(i), value.as_str())),
            );
        }
        ItemBody::Match { prompts, choices } => {
            fields.extend(
                prompts
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Prompt(i), value.as_str())),
            );
            fields.extend(
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Choice(i), value.as_str())),
            );
        }
        ItemBody::Fib { answers } | ItemBody::Order { answers } => {
            fields.extend(
                answers
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (FieldId::Answer(i), value.as_str())),
            );
        }
        ItemBody::MultiFib { answers } => {
            for (key, values) in answers {
                fields.extend(values.iter().enumerate().map(|(i, value)| {
                    (
                        FieldId::MultiFibAnswer {
                            key: key.clone(),
                            answer: i,
                        },
                        value.as_str(),
                    )
                }));
            }
        }
        ItemBody::Num { .. } => {}
    }
    fields
}

fn parse_data_uri(src: &str) -> Result<(String, Vec<u8>), MediaError> {
    let source = src.trim();
    let Some(prefix) = source.get(..5) else {
        return Err(MediaError::InvalidDataUri {
            src: src.to_owned(),
            reason: "missing data: scheme".to_owned(),
        });
    };
    if !prefix.eq_ignore_ascii_case("data:") {
        return Err(MediaError::InvalidDataUri {
            src: src.to_owned(),
            reason: "missing data: scheme".to_owned(),
        });
    }
    let body = &source[5..];
    let Some((metadata, payload)) = body.split_once(',') else {
        return Err(MediaError::InvalidDataUri {
            src: src.to_owned(),
            reason: "missing payload separator".to_owned(),
        });
    };
    let base64_encoded = metadata
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"));
    let mime_type = metadata
        .split(';')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let bytes = if base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|error| MediaError::InvalidDataUri {
                src: src.to_owned(),
                reason: error.to_string(),
            })?
    } else {
        percent_decode_str(payload)
            .decode_utf8()
            .map_err(|error| MediaError::InvalidDataUri {
                src: src.to_owned(),
                reason: error.to_string(),
            })?
            .into_owned()
            .into_bytes()
    };
    Ok((mime_type, bytes))
}
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::MemoryAssets;

    fn sources() -> MemoryAssets {
        let mut source = MemoryAssets::new();
        for name in ["a/image.png", "b/image.png", "c/image.png", "image.png"] {
            source.insert(name, vec![0]).expect("image source");
        }
        source
    }

    #[test]
    fn classifies_external_sources_without_reading_provider() {
        struct Unreadable;
        impl AssetSource for Unreadable {
            fn read(&self, _: &str) -> Result<Cow<'_, [u8]>, MediaError> {
                panic!("external references must not be fetched")
            }
        }
        for src in ["HTTPS://example.test/image.png", "//example.test/image.png"] {
            assert_eq!(classify_src(src), AssetKind::External);
            assert_eq!(
                resolve_asset(src, &Unreadable)
                    .expect("external description")
                    .kind,
                AssetKind::External
            );
        }
        let inline =
            resolve_asset("DATA:image/png;base64,AA==", &Unreadable).expect("inline image");
        assert_eq!(inline.read_bytes().expect("payload"), [0]);
    }

    #[test]
    fn names_colliding_sources_and_hashes_owned_bytes() {
        let source = sources();
        let mut assets = ["a/image.png", "b/image.png", "a/image.png", "c/image.png"]
            .map(|src| resolve_asset(src, &source).expect("local source"));
        super::super::assign_output_names(&mut assets);
        assert_eq!(
            assets
                .iter()
                .map(|asset| asset.output_name.as_deref())
                .collect::<Vec<_>>(),
            [
                Some("image.png"),
                Some("image(1).png"),
                Some("image.png"),
                Some("image(2).png")
            ]
        );
        assert_eq!(
            compute_content_hash(&assets[0]).expect("hash"),
            compute_content_hash(&assets[1]).expect("same content")
        );
    }

    #[test]
    fn unsupported_extension_keeps_source_provenance() {
        assert!(matches!(resolve_asset("bad.webp", &MemoryAssets::new()),
            Err(MediaError::UnsupportedMime { src, extension }) if src == "bad.webp" && extension == ".webp"));
    }

    #[test]
    fn local_and_inline_bytes_share_identity_and_remote_refs_keep_ordinals() {
        let source = sources();
        let local = Item::new("<img src='image.png'/>".to_owned(), valid_mc()).expect("local item");
        let embedded = Item::new(
            "<img src='data:image/png;base64,AA=='/>".to_owned(),
            valid_mc(),
        )
        .expect("inline item");
        assert_eq!(
            resolve_item_media_refs(&local, &source).expect("local refs"),
            resolve_item_media_refs(&embedded, &source).expect("inline refs")
        );
        let mixed = Item::new(
            "<img src='https://example.test/a.png'/><img src='image.png'/>".to_owned(),
            valid_mc(),
        )
        .expect("mixed item");
        let refs = resolve_item_media_refs(&mixed, &source).expect("media refs");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ordinal, 1);
    }

    #[test]
    fn scans_in_document_order_ignoring_script_text() {
        let html = "<script>const x='<img src=\"not.png\">';</script><IMG src='one.png'><img data-src='not-two.png' src=\"two.png\">";
        assert_eq!(
            scan_html_for_assets(html).expect("scan"),
            ["one.png", "two.png"]
        );
    }

    fn valid_mc() -> ItemBody {
        ItemBody::Mc {
            choices: vec!["one".to_owned(), "two".to_owned()],
            answer: "one".to_owned(),
        }
    }
}
