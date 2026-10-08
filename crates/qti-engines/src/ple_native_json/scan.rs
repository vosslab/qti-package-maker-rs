//! Checks represented display HTML and collects its external resource inventory.

use std::collections::HashMap;

use scraper::{Html, Selector};

use super::source::{ExternalResource, ExternalResourceKind, Response, SourceDocument};
use crate::EngineError;

const ENGINE: &str = "ple_native_json";
const FORMAT: &str = "PLE Native JSON";

/// Scan each represented display field once, in document order.
pub(super) fn scan_document(
    document: &SourceDocument,
    item_number: usize,
) -> Result<Vec<ExternalResource>, EngineError> {
    let mut resources = Vec::new();
    let mut seen_urls = HashMap::new();
    scan_field(
        &document.prompt,
        item_number,
        &mut seen_urls,
        &mut resources,
    )?;

    match &document.response {
        Response::SingleChoice { choices, .. } | Response::MultipleAnswer { choices, .. } => {
            for choice in choices {
                scan_field(&choice.text, item_number, &mut seen_urls, &mut resources)?;
            }
        }
        Response::MultiFillIn { blanks } => {
            for blank in blanks {
                scan_field(&blank.label, item_number, &mut seen_urls, &mut resources)?;
            }
        }
        Response::Matching {
            prompts, choices, ..
        } => {
            for prompt in prompts {
                scan_field(&prompt.text, item_number, &mut seen_urls, &mut resources)?;
            }
            for choice in choices {
                scan_field(&choice.text, item_number, &mut seen_urls, &mut resources)?;
            }
        }
        Response::Ordering { items, .. } => {
            for item in items {
                scan_field(&item.text, item_number, &mut seen_urls, &mut resources)?;
            }
        }
        Response::FillIn { .. } | Response::Numeric { .. } => {}
    }

    Ok(resources)
}

fn scan_field(
    html: &str,
    item_number: usize,
    seen_urls: &mut HashMap<String, ExternalResourceKind>,
    resources: &mut Vec<ExternalResource>,
) -> Result<(), EngineError> {
    let fragment = Html::parse_fragment(html);
    let elements = Selector::parse("*").expect("universal HTML selector is valid");
    for element in fragment.select(&elements) {
        let (attribute, kind) = match element.value().name() {
            // ASVS 1.3.5: reject scriptable display markup before export.
            "script" => {
                return Err(invalid(
                    item_number,
                    "display HTML contains a <script> element; use --html-to-image",
                ));
            }
            "img" => ("src", ExternalResourceKind::Image),
            "a" => ("href", ExternalResourceKind::Link),
            "link" => ("href", ExternalResourceKind::Stylesheet),
            _ => continue,
        };
        let Some(value) = element.value().attr(attribute) else {
            continue;
        };
        let url = value.trim();
        // ASVS 1.1.1 / 2.2.1: the HTML parser decodes attributes once; validate the
        // interpreted URL before placing it in PLE's HTTPS-only inventory.
        if kind == ExternalResourceKind::Image && url.to_ascii_lowercase().starts_with("data:") {
            // The existing Package media policy owns data-image rejection.
            continue;
        }
        if url.starts_with("//")
            || (has_scheme(url) && !url.to_ascii_lowercase().starts_with("https://"))
        {
            return Err(invalid(
                item_number,
                format!("{attribute} has a non-HTTPS URL that PLE cannot inventory: {url}"),
            ));
        }
        if url.to_ascii_lowercase().starts_with("https://") {
            if let Some(previous_kind) = seen_urls.get(url) {
                if *previous_kind != kind {
                    return Err(invalid(
                        item_number,
                        format!(
                            "external URL {url} is used as both {previous_kind:?} and {kind:?}; PLE requires one resource kind per URL"
                        ),
                    ));
                }
                continue;
            }
            seen_urls.insert(url.to_owned(), kind);
            resources.push(ExternalResource {
                url: url.to_owned(),
                kind,
            });
        }
    }
    Ok(())
}

fn has_scheme(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn invalid(item_number: usize, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: ENGINE,
        format: FORMAT,
        message: format!("item {item_number}: {}", message.into()),
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
