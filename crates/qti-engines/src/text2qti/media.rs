//! Markdown image conversion and local asset copying for text2qti.

use std::collections::BTreeMap;

use lol_html::{RewriteStrSettings, element, rewrite_str};
use qti_core::ItemBank;
use qti_core::media::{AssetKind, MediaPolicy, MediaWarning, apply_media_policy};
use regex::Regex;

use super::NAME;
use crate::EngineError;

pub(super) struct MarkdownMedia {
    pub(super) targets: BTreeMap<qti_core::ItemCrc, BTreeMap<String, String>>,
    pub(super) warnings: BTreeMap<qti_core::ItemCrc, Vec<MediaWarning>>,
}

pub(super) fn markdown_media(
    collected: &qti_core::CollectedAssets,
    bank: &ItemBank,
) -> Result<MarkdownMedia, EngineError> {
    let mut targets = BTreeMap::new();
    let mut warnings = BTreeMap::new();
    for item in bank.iter_ordered() {
        let dependencies = collected.dependencies_for(item.crc()).unwrap_or_default();
        let decision = apply_media_policy(
            MediaPolicy::ReferenceWarn,
            dependencies,
            NAME,
            &item.crc().to_string(),
        )
        .map_err(|error| EngineError::InvalidFormat {
            engine: NAME,
            format: "media",
            message: error.to_string(),
        })?;
        let item_targets = dependencies
            .iter()
            .map(|asset| {
                let target =
                    if asset.kind == AssetKind::Local {
                        let name = asset.output_name.as_deref().ok_or_else(|| {
                            EngineError::InvalidFormat {
                                engine: NAME,
                                format: "media",
                                message: format!("asset '{}' has no output name", asset.src),
                            }
                        })?;
                        format!("media/{name}")
                    } else {
                        asset.src.clone()
                    };
                Ok((asset.src.clone(), target))
            })
            .collect::<Result<BTreeMap<_, _>, EngineError>>()?;
        targets.insert(*item.crc(), item_targets);
        warnings.insert(*item.crc(), decision.warnings);
    }
    Ok(MarkdownMedia { targets, warnings })
}

pub(super) fn markdown_images(
    text: &str,
    targets: &BTreeMap<String, String>,
) -> Result<String, EngineError> {
    rewrite_str(
        text,
        RewriteStrSettings::new().append_element_content_handler(element!("img[src]", |element| {
            let source = element
                .get_attribute("src")
                .expect("img[src] selector guarantees a source attribute");
            let target = targets.get(&source).unwrap_or(&source);
            let alt = element.get_attribute("alt").unwrap_or_default();
            // The replacement is text2qti syntax, not author-supplied markup. The reader
            // context-encodes its two fields before recreating HTML (ASVS 1.2.1).
            element.replace(
                &markdown_image(&alt, target),
                lol_html::html_content::ContentType::Html,
            );
            Ok(())
        })),
    )
    .map_err(|error| EngineError::InvalidFormat {
        engine: NAME,
        format: "HTML",
        message: error.to_string(),
    })
}

fn markdown_image(alt: &str, target: &str) -> String {
    format!("![{alt}]({target})")
}

pub(super) fn local_copies(
    collected: &qti_core::CollectedAssets,
) -> Result<Vec<(String, Vec<u8>)>, EngineError> {
    collected
        .assets()
        .iter()
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| {
            let name = asset
                .output_name
                .clone()
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "media",
                    message: format!("asset '{}' has no output name", asset.src),
                })?;
            let bytes = asset
                .read_bytes()
                .map_err(|error| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "media",
                    message: error.to_string(),
                })?;
            Ok((name, bytes))
        })
        .collect()
}

pub(super) fn restore_markdown_images(text: &str) -> String {
    // The grammar intentionally accepts the form emitted by this writer. It does not interpret
    // arbitrary Markdown, execute links, or fetch remote resources (ASVS 1.3.5, 1.3.6).
    let pattern = Regex::new(r"!\[([^\]]*)\]\(([^)]*)\)").expect("constant Markdown image regex");
    pattern
        .replace_all(text, |captures: &regex::Captures<'_>| {
            format!(
                "<img src=\"{}\" alt=\"{}\"/>",
                html_attribute(&unescape_markdown(&captures[2])),
                html_attribute(&unescape_markdown(&captures[1]))
            )
        })
        .into_owned()
}

fn unescape_markdown(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            result.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
