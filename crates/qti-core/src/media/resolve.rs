use std::fs;
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use percent_encoding::percent_decode_str;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{FieldId, Item, ItemBody, MediaRef};

/// The origin of an image reference in authored item HTML.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    /// A filesystem-backed image reference.
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
    pub file_path: Option<PathBuf>,
    pub data_bytes: Option<Vec<u8>>,
    pub output_name: Option<String>,
    pub content_hash: Option<[u8; 32]>,
}

impl MediaAsset {
    /// Reads a payload from owned bytes or the validated local path.
    pub fn read_bytes(&self) -> Result<Vec<u8>, MediaError> {
        if let Some(bytes) = &self.data_bytes {
            return Ok(bytes.clone());
        }
        let Some(path) = &self.file_path else {
            return Err(MediaError::NoPayload {
                src: self.src.clone(),
            });
        };
        fs::read(path).map_err(|source| MediaError::ReadFile {
            src: self.src.clone(),
            path: path.clone(),
            source,
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

/// Failures at the HTML, URI, or filesystem boundary.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error(
        "unsupported image type '{extension}' for src '{src}'; supported: png, jpg, jpeg, gif, svg"
    )]
    UnsupportedMime { src: String, extension: String },
    #[error("cannot resolve local image '{src}' without a base directory")]
    MissingBaseDir { src: String },
    #[error("local image path '{src}' escapes base directory '{base_dir}'")]
    Traversal { src: String, base_dir: PathBuf },
    #[error("image file not found for src '{src}': {path}")]
    MissingFile { src: String, path: PathBuf },
    #[error("could not read image for src '{src}' at {path}: {source}")]
    ReadFile {
        src: String,
        path: PathBuf,
        source: std::io::Error,
    },
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
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map_or_else(String::new, |value| {
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

/// Resolves a local source below `base_dir`.
///
/// Sources are lexical-normalized and must remain below the supplied base.  This rejects traversal
/// before file I/O (ASVS 2.2.1), including an absolute spelling outside the authorized base.
pub fn resolve_local_path(base_dir: &Path, src: &str) -> Result<PathBuf, MediaError> {
    let source = Path::new(src);
    let base = normalize_path(base_dir);
    let joined = if source.is_absolute() {
        source.to_path_buf()
    } else {
        base.join(source)
    };
    let resolved = normalize_path(&joined);
    if resolved != base && !resolved.starts_with(&base) {
        return Err(MediaError::Traversal {
            src: src.to_owned(),
            base_dir: base,
        });
    }
    Ok(resolved)
}

/// Resolves a source into a derived asset.  External URLs are classified only and never fetched.
pub fn resolve_asset(src: &str, base_dir: Option<&Path>) -> Result<MediaAsset, MediaError> {
    let kind = classify_src(src);
    match kind {
        AssetKind::External => Ok(MediaAsset {
            src: src.to_owned(),
            kind,
            mime_type: None,
            file_path: None,
            data_bytes: None,
            output_name: None,
            content_hash: None,
        }),
        AssetKind::DataUri => {
            let (mime_type, data_bytes) = parse_data_uri(src)?;
            Ok(MediaAsset {
                src: src.to_owned(),
                kind,
                mime_type: Some(mime_type),
                file_path: None,
                data_bytes: Some(data_bytes),
                output_name: None,
                content_hash: None,
            })
        }
        AssetKind::Local => {
            let base_dir = base_dir.ok_or_else(|| MediaError::MissingBaseDir {
                src: src.to_owned(),
            })?;
            let lexical_path = resolve_local_path(base_dir, src)?;
            let mime_type = guess_mime_type(src)?.to_owned();
            if !lexical_path.is_file() {
                return Err(MediaError::MissingFile {
                    src: src.to_owned(),
                    path: lexical_path,
                });
            }
            // ASVS 2.2.1: resolve symlinks after existence is known, so a syntactically safe
            // `images/link.png` cannot escape the authorized media directory through a link.
            let canonical_base =
                base_dir
                    .canonicalize()
                    .map_err(|source| MediaError::ReadFile {
                        src: src.to_owned(),
                        path: base_dir.to_path_buf(),
                        source,
                    })?;
            let file_path = lexical_path
                .canonicalize()
                .map_err(|source| MediaError::ReadFile {
                    src: src.to_owned(),
                    path: lexical_path.clone(),
                    source,
                })?;
            if file_path != canonical_base && !file_path.starts_with(&canonical_base) {
                return Err(MediaError::Traversal {
                    src: src.to_owned(),
                    base_dir: canonical_base,
                });
            }
            let output_name = Path::new(src.split(['?', '#']).next().unwrap_or(src))
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            Ok(MediaAsset {
                src: src.to_owned(),
                kind,
                mime_type: Some(mime_type),
                file_path: Some(file_path),
                data_bytes: None,
                output_name,
                content_hash: None,
            })
        }
    }
}

/// Hashes an asset's bytes with SHA-256 and records no identity change.
pub fn compute_content_hash(asset: &MediaAsset) -> Result<[u8; 32], MediaError> {
    Ok(Sha256::digest(asset.read_bytes()?).into())
}

/// Resolves every hashable image in an item and returns fingerprint references in field/document order.
///
/// `ordinal` counts every `<img src>` in a field. External URLs produce no reference and are never
/// fetched; data URIs are hashable payloads and do produce a reference. This lets a local source
/// and a self-test engine's equivalent data URI compare by content and placement.
pub fn resolve_item_media_refs(item: &Item, base_dir: &Path) -> Result<Vec<MediaRef>, MediaError> {
    let mut references = Vec::new();
    for (field, html) in item_html_fields(item) {
        for (ordinal, src) in scan_html_for_assets(html)?.into_iter().enumerate() {
            let asset = resolve_asset(&src, Some(base_dir))?;
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

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        AssetKind, MediaError, classify_src, compute_content_hash, resolve_asset,
        resolve_item_media_refs, resolve_local_path, scan_html_for_assets,
    };
    use crate::{Item, ItemBody};

    // Keep the naming test's import local to this module while preserving the public API's
    // ownership in `media::naming`.
    fn assign_output_names_for_test(assets: &mut [super::MediaAsset]) {
        super::super::assign_output_names(assets);
    }

    #[test]
    fn source_classification_never_implies_a_network_request() {
        assert_eq!(
            classify_src("HTTPS://example.test/image.png"),
            AssetKind::External
        );
        assert_eq!(
            classify_src("//example.test/image.png"),
            AssetKind::External
        );
        assert_eq!(
            classify_src("data:image/png;base64,AA=="),
            AssetKind::DataUri
        );
        assert_eq!(classify_src("images/a.png"), AssetKind::Local);
    }

    #[test]
    fn resolves_hashes_and_names_colliding_local_sources_by_src() {
        let temp = tempfile::tempdir().expect("temporary base");
        fs::create_dir_all(temp.path().join("a")).expect("directory a");
        fs::create_dir_all(temp.path().join("b")).expect("directory b");
        fs::create_dir_all(temp.path().join("c")).expect("directory c");
        fs::write(temp.path().join("a/image.png"), b"first").expect("first file");
        fs::write(temp.path().join("b/image.png"), b"first").expect("second file");
        fs::write(temp.path().join("c/image.png"), b"first").expect("third file");
        let mut assets = vec![
            resolve_asset("a/image.png", Some(temp.path())).expect("first asset"),
            resolve_asset("b/image.png", Some(temp.path())).expect("second asset"),
            resolve_asset("a/image.png", Some(temp.path())).expect("duplicate source"),
            resolve_asset("c/image.png", Some(temp.path())).expect("third source"),
        ];
        assign_output_names_for_test(&mut assets);
        assert_eq!(assets[0].output_name.as_deref(), Some("image.png"));
        assert_eq!(assets[1].output_name.as_deref(), Some("image(1).png"));
        assert_eq!(assets[2].output_name, assets[0].output_name);
        assert_eq!(assets[3].output_name.as_deref(), Some("image(2).png"));
        assert_eq!(
            compute_content_hash(&assets[0]).expect("first hash"),
            compute_content_hash(&assets[1]).expect("second hash")
        );
        assert_eq!(
            compute_content_hash(&assets[0]).expect("first hash"),
            compute_content_hash(&assets[3]).expect("third hash")
        );
    }

    #[test]
    fn path_boundary_rejects_relative_and_absolute_escapes() {
        let temp = tempfile::tempdir().expect("temporary base");
        assert!(matches!(
            resolve_local_path(temp.path(), "../outside.png"),
            Err(MediaError::Traversal { .. })
        ));
        assert!(matches!(
            resolve_local_path(temp.path(), "/private/tmp/outside.png"),
            Err(MediaError::Traversal { .. })
        ));
    }

    #[test]
    fn accepts_an_authorized_absolute_source_and_names_its_authored_basename() {
        let temp = tempfile::tempdir().expect("temporary base");
        let path = temp.path().join("authored.png");
        fs::write(&path, [0_u8]).expect("image file");
        let asset = resolve_asset(&path.display().to_string(), Some(temp.path()))
            .expect("absolute source inside base");
        assert_eq!(asset.output_name.as_deref(), Some("authored.png"));
    }

    #[test]
    fn unsupported_extension_names_the_source_and_extension() {
        let temp = tempfile::tempdir().expect("temporary base");
        assert!(matches!(
            resolve_asset("bad.webp", Some(temp.path())),
            Err(MediaError::UnsupportedMime { src, extension }) if src == "bad.webp" && extension == ".webp"
        ));
    }

    #[test]
    fn parses_uppercase_data_scheme_like_python() {
        let asset = resolve_asset("DATA:image/png;base64,AA==", None).expect("uppercase data uri");
        assert_eq!(asset.kind, AssetKind::DataUri);
        assert_eq!(asset.read_bytes().expect("payload"), [0]);
    }

    #[test]
    fn path_boundary_rejects_symlink_escapes_during_resolution() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().expect("temporary base");
            let outside = tempfile::NamedTempFile::new().expect("outside file");
            symlink(outside.path(), temp.path().join("escaped.png")).expect("symlink");
            assert!(matches!(
                resolve_asset("escaped.png", Some(temp.path())),
                Err(MediaError::Traversal { .. })
            ));
        }
    }

    #[test]
    fn local_and_data_image_with_same_bytes_produce_same_media_ref() {
        let temp = tempfile::tempdir().expect("temporary base");
        fs::write(temp.path().join("image.png"), [0_u8]).expect("image file");
        let local = Item::new("<img src='image.png'/>".to_owned(), valid_mc()).expect("local item");
        let embedded = Item::new(
            "<img src='data:image/png;base64,AA=='/>".to_owned(),
            valid_mc(),
        )
        .expect("embedded item");
        assert_eq!(
            resolve_item_media_refs(&local, temp.path()).expect("local refs"),
            resolve_item_media_refs(&embedded, temp.path()).expect("embedded refs")
        );
    }

    #[test]
    fn remote_before_local_counts_in_placement_ordinal() {
        let temp = tempfile::tempdir().expect("temporary base");
        fs::write(temp.path().join("image.png"), [0_u8]).expect("image file");
        let item = Item::new(
            "<img src='https://example.test/a.png'/><img src='image.png'/>".to_owned(),
            valid_mc(),
        )
        .expect("item");
        let refs = resolve_item_media_refs(&item, temp.path()).expect("media refs");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ordinal, 1);
    }

    #[test]
    fn scan_is_document_order_and_ignores_script_text() {
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
