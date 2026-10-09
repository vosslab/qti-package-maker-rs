//! Display-only media collection for loose PLE Native JSON questions.

use std::collections::{BTreeMap, HashSet};

use qti_core::media::{
    AssetKind, AssetSource, MediaAsset, MediaWarning, apply_media_policy, assign_output_names,
    resolve_asset, rewrite_html_srcs, scan_html_for_assets,
};
use qti_core::{AssetCollectionAction, BankError, ItemCrc, NamedFile};
use scraper::{Html, Selector};

use super::export::{MappedQuestion, NativeExport, NativeQuestion, serialize};
use super::source::{Response, SourceDocument};
use crate::EngineError;

const ENGINE: &str = "ple_native_json";

pub(super) fn finish_export(
    assets: &dyn AssetSource,
    mapped: Vec<MappedQuestion>,
) -> Result<NativeExport, EngineError> {
    let mut assets_by_src = BTreeMap::new();
    let mut dependencies = Vec::with_capacity(mapped.len());
    for question in &mapped {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        for_display_field(&question.document, |html| {
            let found = scan_html_for_assets(html).map_err(|source| {
                collect_error(question.crc, None, AssetCollectionAction::ScanHtml, source)
            })?;
            for src in found {
                if seen.insert(src.clone()) {
                    sources.push(src);
                }
            }
            Ok(())
        })?;
        for src in &sources {
            if !assets_by_src.contains_key(src) {
                let asset = resolve_asset(src, assets).map_err(|source| {
                    collect_error(
                        question.crc,
                        Some(src.clone()),
                        AssetCollectionAction::ResolveAsset,
                        source,
                    )
                })?;
                assets_by_src.insert(src.clone(), asset);
            }
        }
        dependencies.push(sources);
    }

    let mut assets = assets_by_src.into_values().collect::<Vec<_>>();
    assign_output_names(&mut assets);
    let by_src = assets
        .iter()
        .map(|asset| (asset.src.as_str(), asset))
        .collect::<BTreeMap<_, _>>();
    let mut questions = Vec::with_capacity(mapped.len());
    let mut warnings = Vec::<MediaWarning>::new();

    for (mut question, sources) in mapped.into_iter().zip(dependencies) {
        let item_assets = sources
            .iter()
            .map(|src| *by_src.get(src.as_str()).expect("collected source exists"))
            .collect::<Vec<&MediaAsset>>();
        let policy_assets = item_assets
            .iter()
            .map(|asset| (*asset).clone())
            .collect::<Vec<_>>();
        let decision = apply_media_policy(
            crate::engine(ENGINE)
                .expect("registered PLE engine")
                .media_policy,
            &policy_assets,
            ENGINE,
            &question.crc.to_string(),
        )
        .map_err(|error| EngineError::InvalidFormat {
            engine: ENGINE,
            format: "media",
            message: error.to_string(),
        })?;
        warnings.extend(decision.warnings);

        let mut targets = BTreeMap::new();
        let mut files = Vec::new();
        for asset in item_assets {
            if asset.kind != AssetKind::Local {
                continue;
            }
            let name = asset
                .output_name
                .as_ref()
                .expect("local asset has assigned name");
            let target = format!("media/{name}");
            targets.insert(asset.src.as_str(), target.clone());
            let bytes = asset.shared_bytes().map_err(|source| {
                collect_error(
                    question.crc,
                    Some(asset.src.clone()),
                    AssetCollectionAction::ResolveAsset,
                    source,
                )
            })?;
            // ASVS 5.3.2: validate generated relative names before exposing output bytes.
            files.push(NamedFile::from_shared(target, bytes)?);
        }
        if !targets.is_empty() {
            for_display_field_mut(&mut question.document, |html| {
                *html = rewrite_html_srcs(html, |src| {
                    targets
                        .get(src.trim())
                        .cloned()
                        .unwrap_or_else(|| src.to_owned())
                })
                .map_err(|source| {
                    collect_error(question.crc, None, AssetCollectionAction::ScanHtml, source)
                })?;
                Ok(())
            })?;
        }
        verify_display_image_files(&question.document, &files, question.item_number)?;
        questions.push(NativeQuestion {
            item_number: question.item_number,
            crc: question.crc,
            source_json: serialize(question.item_number, &question.document)?,
            files,
        });
    }
    Ok(NativeExport {
        questions,
        warnings,
    })
}

fn verify_display_image_files(
    document: &SourceDocument,
    files: &[NamedFile],
    item_number: usize,
) -> Result<(), EngineError> {
    let images = Selector::parse("img[src]").expect("static image selector is valid");
    for_display_field(document, |html| {
        let fragment = Html::parse_fragment(html);
        for image in fragment.select(&images) {
            let Some(src) = image.value().attr("src") else {
                continue;
            };
            let src = src.trim();
            if src.is_empty() || qti_core::media::classify_src(src) != AssetKind::Local {
                continue;
            }
            let matches = files.iter().filter(|file| file.name() == src).count();
            if matches != 1 {
                return Err(EngineError::InvalidFormat {
                    engine: ENGINE,
                    format: "PLE Native JSON",
                    message: format!(
                        "item {item_number}: display image src {src:?} resolves to {matches} associated files after HTML entity decoding; expected exactly one"
                    ),
                });
            }
        }
        Ok(())
    })
}

fn collect_error(
    item_crc: ItemCrc,
    src: Option<String>,
    action: AssetCollectionAction,
    source: qti_core::media::MediaError,
) -> EngineError {
    BankError::CollectAsset {
        item_crc,
        src,
        action,
        source,
    }
    .into()
}

pub(super) fn for_display_field(
    document: &SourceDocument,
    mut visit: impl FnMut(&str) -> Result<(), EngineError>,
) -> Result<(), EngineError> {
    visit(&document.prompt)?;
    match &document.response {
        Response::SingleChoice { choices, .. } | Response::MultipleAnswer { choices, .. } => {
            for choice in choices {
                visit(&choice.text)?;
            }
        }
        Response::MultiFillIn { blanks } => {
            for blank in blanks {
                visit(&blank.label)?;
            }
        }
        Response::Matching {
            prompts, choices, ..
        } => {
            for prompt in prompts {
                visit(&prompt.text)?;
            }
            for choice in choices {
                visit(&choice.text)?;
            }
        }
        Response::Ordering { items, .. } => {
            for item in items {
                visit(&item.text)?;
            }
        }
        Response::FillIn { .. } | Response::Numeric { .. } => {}
    }
    Ok(())
}

fn for_display_field_mut(
    document: &mut SourceDocument,
    mut visit: impl FnMut(&mut String) -> Result<(), EngineError>,
) -> Result<(), EngineError> {
    visit(&mut document.prompt)?;
    match &mut document.response {
        Response::SingleChoice { choices, .. } | Response::MultipleAnswer { choices, .. } => {
            for choice in choices {
                visit(&mut choice.text)?;
            }
        }
        Response::MultiFillIn { blanks } => {
            for blank in blanks {
                visit(&mut blank.label)?;
            }
        }
        Response::Matching {
            prompts, choices, ..
        } => {
            for prompt in prompts {
                visit(&mut prompt.text)?;
            }
            for choice in choices {
                visit(&mut choice.text)?;
            }
        }
        Response::Ordering { items, .. } => {
            for item in items {
                visit(&mut item.text)?;
            }
        }
        Response::FillIn { .. } | Response::Numeric { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
