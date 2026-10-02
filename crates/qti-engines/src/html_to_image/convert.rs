//! Two-phase, source-preserving HTML table and canvas conversion.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use thiserror::Error;

use qti_core::media::{self, AssetKind, MediaAsset, rewrite_html_srcs};
use qti_core::{BankError, FieldId, Item, ItemBank, ItemCrc, MediaBaseDir, ValidationError};
use qti_molecule::CanvasSource;

use super::cache::{
    CacheOutcome, RenderCache, RenderFamily, RenderKey, RenderMetrics, RenderedPng,
};
use super::naming::{
    INPUT_DIRECTORY, canvas_alt_text, generated_directory, generated_leaf_name, reserved_directory,
    table_alt_text,
};
use super::{FieldConversionPlan, FragmentId, FragmentReplacement, HtmlToImageError};

/// Renders statically selected fragments without executing authored JavaScript.
pub trait FragmentRenderer: Sync {
    /// Renderer-specific failure returned for a canvas or table input.
    type Error: Error + Send + Sync + 'static;

    /// Returns a stable description of every renderer setting that affects PNG output or acceptance.
    ///
    /// A shared [`RenderCache`] incorporates this value into every key. Implementations must
    /// change it whenever their rendering configuration changes.
    fn cache_discriminator(&self) -> String;

    /// Renders one parsed RDKit canvas source to PNG bytes.
    fn render_canvas(&self, source: &CanvasSource) -> Result<RenderedPng, Self::Error>;

    /// Renders one prepared outer-table HTML fragment to PNG bytes.
    fn render_table(&self, html: &str) -> Result<RenderedPng, Self::Error>;
}

/// Renders tables with Chromium and statically parsed molecular canvases with RDKit.
#[derive(Clone, Debug, Default)]
pub struct ChromiumFragmentRenderer {
    table: std::sync::Arc<super::chromium::ChromiumRenderer>,
}

impl FragmentRenderer for ChromiumFragmentRenderer {
    type Error = ChromiumRenderError;

    fn cache_discriminator(&self) -> String {
        let browser = std::env::var_os("QTI_CHROMIUM")
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "auto".to_owned());
        let shim = std::env::var_os("QTI_RDKIT_SHIM")
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unconfigured".to_owned());
        format!("chromium-v2;browser={browser};scale=2;rdkit-shim={shim}")
    }

    fn render_canvas(&self, source: &CanvasSource) -> Result<RenderedPng, Self::Error> {
        qti_molecule::render_canvas_png(source)
            .map(|bytes| RenderedPng {
                bytes,
                metrics: Default::default(),
            })
            .map_err(ChromiumRenderError::Canvas)
    }

    fn render_table(&self, html: &str) -> Result<RenderedPng, Self::Error> {
        self.table
            .render(html)
            .map(|bytes| RenderedPng {
                bytes,
                metrics: Default::default(),
            })
            .map_err(|error| ChromiumRenderError::Table(error.to_string()))
    }
}

/// A fragment rendering failure with its originating renderer identified.
#[derive(Debug, Error)]
pub enum ChromiumRenderError {
    /// RDKit could not rasterize the parsed canvas source.
    #[error(transparent)]
    Canvas(#[from] qti_molecule::MoleculeError),
    /// Chromium could not render or capture the table.
    #[error("Chromium table rendering failed: {0}")]
    Table(String),
}

/// Cumulative conversion work measurements collected without shared timing state.
///
/// Durations are summed across independent Rayon work items and represent work, not elapsed wall
/// time. Benchmark callers measure end-to-end wall time outside this API.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConversionMetrics {
    /// Selected canvas and table fragments, including per-item reused field content.
    pub requested_fragments: usize,
    /// Fragment requests served by a completed render cache or per-item field reuse.
    pub cache_hits: usize,
    /// Fragment lookups that waited for a concurrent matching render.
    pub cache_waits: usize,
    /// Fragment lookups rendered by this work item.
    pub cache_misses: usize,
    /// Renderer calls actually invoked after cache lookup.
    pub renderer_attempts: usize,
    /// Successful renderer calls.
    pub renderer_successes: usize,
    /// Failed renderer calls.
    pub renderer_failures: usize,
    /// Cumulative local renderer stages for successful cache-miss renders.
    pub render: RenderMetrics,
    /// Time spent writing the derived owned media root.
    pub materialization: Duration,
    /// Non-overlapping serial selection, setup, and reconstruction orchestration time.
    pub conversion_bookkeeping: Duration,
}

/// A conversion error together with metrics observed before the failing work was cancelled.
///
/// Under Rayon, sibling work may already have completed when a failure is returned. These metrics
/// therefore describe observed work before cancellation, rather than a complete conversion run.
#[derive(Debug, Error)]
#[error("{error}")]
pub struct ConversionFailure {
    /// The original typed conversion error.
    #[source]
    pub error: Box<ConversionError>,
    /// Counts and completed timing spans observed through the failure.
    pub metrics: Box<ConversionMetrics>,
}

impl From<ConversionError> for ConversionFailure {
    fn from(error: ConversionError) -> Self {
        Self {
            error: Box::new(error),
            metrics: Box::new(ConversionMetrics::default()),
        }
    }
}

impl From<HtmlToImageError> for ConversionFailure {
    fn from(error: HtmlToImageError) -> Self {
        ConversionError::from(error).into()
    }
}

impl From<BankError> for ConversionFailure {
    fn from(error: BankError) -> Self {
        ConversionError::from(error).into()
    }
}

impl From<media::MediaError> for ConversionFailure {
    fn from(error: media::MediaError) -> Self {
        ConversionError::from(error).into()
    }
}

impl From<ValidationError> for ConversionFailure {
    fn from(error: ValidationError) -> Self {
        ConversionError::from(error).into()
    }
}

impl std::ops::AddAssign for ConversionMetrics {
    fn add_assign(&mut self, other: Self) {
        self.requested_fragments += other.requested_fragments;
        self.cache_hits += other.cache_hits;
        self.cache_waits += other.cache_waits;
        self.cache_misses += other.cache_misses;
        self.renderer_attempts += other.renderer_attempts;
        self.renderer_successes += other.renderer_successes;
        self.renderer_failures += other.renderer_failures;
        self.render += other.render;
        self.materialization += other.materialization;
        self.conversion_bookkeeping += other.conversion_bookkeeping;
    }
}

/// Failures while converting an item bank without mutating its source.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Static fragment selection or positional replacement failed.
    #[error(transparent)]
    Selection(#[from] HtmlToImageError),
    /// The source bank's media could not be resolved before conversion materialization.
    #[error(transparent)]
    Bank(#[from] BankError),
    /// An image source rewrite or media operation failed.
    #[error(transparent)]
    Media(#[from] media::MediaError),
    /// Reconstructing a changed item did not satisfy its validation contract.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// A validated item body and its planned HTML fields did not agree.
    #[error("field conversion plan did not match item shape")]
    PlanMismatch,
    /// A renderer rejected one prepared fragment.
    #[error(
        "{family} renderer failed for item {item_crc}, field {field:?}, fragment {fragment_id:?}: {source}"
    )]
    Render {
        item_crc: ItemCrc,
        field: FieldId,
        fragment_id: FragmentId,
        /// The renderer family that rejected its input.
        family: &'static str,
        snippet: String,
        /// The renderer's original typed failure.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}

/// Converts selected tables and supported canvases into a new, bank-owned media workspace.
///
/// All selection, source resolution, and rendering finish before a temporary output root exists.
/// With no selected fragment this returns `bank.clone()`. Otherwise existing local sources are
/// copied into a new temporary root, so the source bank's external directory remains untouched.
pub fn convert_bank<R: FragmentRenderer>(
    bank: &ItemBank,
    renderer: &R,
    cache: &RenderCache,
) -> Result<ItemBank, ConversionError> {
    convert_bank_with_metrics(bank, renderer, cache)
        .map(|(converted, _)| converted)
        .map_err(|failure| *failure.error)
}

/// Converts a bank and returns cumulative conversion work measurements.
pub fn convert_bank_with_metrics<R: FragmentRenderer>(
    bank: &ItemBank,
    renderer: &R,
    cache: &RenderCache,
) -> Result<(ItemBank, ConversionMetrics), ConversionFailure> {
    let bookkeeping_started = Instant::now();
    let plans = bank
        .iter_ordered()
        .map(ItemPlan::prepare)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ConversionFailure {
            error: Box::new(error.into()),
            metrics: Box::new(ConversionMetrics {
                conversion_bookkeeping: bookkeeping_started.elapsed(),
                ..ConversionMetrics::default()
            }),
        })?;
    if plans.iter().all(ItemPlan::is_empty) {
        return Ok((
            bank.clone(),
            ConversionMetrics {
                conversion_bookkeeping: bookkeeping_started.elapsed(),
                ..ConversionMetrics::default()
            },
        ));
    }

    // This resolves every source before rendering or creating the derived media root.
    let source_assets = collect_source_assets(bank)?;
    let occupied_sources = source_assets.iter().map(|asset| asset.src.as_str());
    let generated_directory = generated_directory(occupied_sources.clone());
    let input_directory = reserved_directory(INPUT_DIRECTORY, occupied_sources);
    let snapshotted_assets = snapshot_local_assets(&source_assets, &input_directory)?;

    let preliminary_bookkeeping = bookkeeping_started.elapsed();
    let converted = plans
        .par_iter()
        .map(|plan| plan.render(renderer, cache, &generated_directory))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|mut failure: ConversionFailure| {
            failure.metrics.conversion_bookkeeping += preliminary_bookkeeping;
            failure
        })?;

    let mut metrics = converted
        .iter()
        .fold(ConversionMetrics::default(), |mut total, item| {
            total += item.metrics.clone();
            total
        });
    metrics.conversion_bookkeeping += preliminary_bookkeeping;

    let setup_started = Instant::now();
    let temporary_root = MediaBaseDir::temporary()?;
    let mut result = ItemBank::with_media_base_dir(bank.allow_mixed(), temporary_root);
    metrics.conversion_bookkeeping += setup_started.elapsed();
    let materialization_started = Instant::now();
    let absolute_aliases = materialize_source_assets(&mut result, &snapshotted_assets)?;
    let mut generated = BTreeMap::new();
    for item in &converted {
        for (src, png) in &item.generated_pngs {
            generated.entry(src.clone()).or_insert_with(|| png.clone());
        }
    }
    for (src, png) in generated {
        result.add_image(&src, &png)?;
    }
    metrics.materialization += materialization_started.elapsed();
    let rebuild_started = Instant::now();
    for item in converted {
        let rebuilt = rebuild_item(&item.item, &item.fields, &absolute_aliases)?;
        result.add_item(rebuilt)?;
    }
    metrics.conversion_bookkeeping += rebuild_started.elapsed();
    Ok((result, metrics))
}

#[derive(Clone)]
struct ItemPlan {
    item: Item,
    fields: Vec<PlannedField>,
}

#[derive(Clone)]
struct PlannedField {
    field: FieldId,
    plan: FieldConversionPlan,
}

impl ItemPlan {
    fn prepare(item: &Item) -> Result<Self, HtmlToImageError> {
        let fields = media::item_html_fields(item)
            .into_iter()
            .map(|(field, html)| {
                FieldConversionPlan::prepare(html).map(|plan| PlannedField { field, plan })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            item: item.clone(),
            fields,
        })
    }

    fn is_empty(&self) -> bool {
        self.fields
            .iter()
            .all(|field| field.plan.fragments().is_empty())
    }

    fn render<R: FragmentRenderer>(
        &self,
        renderer: &R,
        cache: &RenderCache,
        generated_directory: &str,
    ) -> Result<ConvertedItem, ConversionFailure> {
        let mut counters = FamilyCounters::default();
        let mut field_cache: HashMap<String, String> = HashMap::new();
        let mut fields = Vec::with_capacity(self.fields.len());
        let mut generated_pngs = Vec::new();
        let mut metrics = ConversionMetrics::default();
        for field in &self.fields {
            let source = field.plan.prepared_html().to_owned();
            if let Some(cached) = field_cache.get(&source) {
                metrics.requested_fragments += field.plan.fragments().len();
                metrics.cache_hits += field.plan.fragments().len();
                fields.push(cached.clone());
                continue;
            }
            let converted = render_field(
                &field.plan,
                *self.item.crc(),
                &field.field,
                &mut counters,
                renderer,
                cache,
                generated_directory,
            )
            .map_err(|mut failure| {
                *failure.metrics += metrics.clone();
                failure
            })?;
            generated_pngs.extend(converted.pngs.iter().cloned());
            metrics += converted.metrics;
            field_cache.insert(source, converted.html.clone());
            fields.push(converted.html);
        }
        Ok(ConvertedItem {
            item: self.item.clone(),
            fields,
            generated_pngs,
            metrics,
        })
    }
}

struct ConvertedItem {
    item: Item,
    fields: Vec<String>,
    generated_pngs: Vec<(String, Vec<u8>)>,
    metrics: ConversionMetrics,
}

struct ConvertedField {
    html: String,
    pngs: Vec<(String, Vec<u8>)>,
    metrics: ConversionMetrics,
}

#[derive(Default)]
struct FamilyCounters {
    canvas: usize,
    table: usize,
}

fn render_field<R: FragmentRenderer>(
    plan: &FieldConversionPlan,
    crc: ItemCrc,
    field: &FieldId,
    counters: &mut FamilyCounters,
    renderer: &R,
    cache: &RenderCache,
    generated_directory: &str,
) -> Result<ConvertedField, ConversionFailure> {
    let mut metrics = ConversionMetrics::default();
    let canvases = plan
        .fragments()
        .iter()
        .filter_map(|fragment| {
            fragment
                .canvas_source()
                .map(|source| (fragment.id(), source))
        })
        .collect::<Vec<_>>();
    let rendered_canvases = canvases
        .par_iter()
        .map(|(id, source)| {
            Ok((
                *id,
                render_canvas(renderer, cache, crc, field.clone(), *id, source)?,
            ))
        })
        .collect::<Result<Vec<_>, ConversionFailure>>()?;
    let mut pngs = Vec::new();
    let mut inline_replacements = Vec::new();
    let mut standalone_rewrites = HashMap::new();
    for (id, source, (png, fragment_metrics)) in rendered_canvases
        .into_iter()
        .zip(canvases.iter().map(|(_, source)| *source))
        .map(|((id, png), source)| (id, source, png))
    {
        metrics += fragment_metrics;
        counters.canvas += 1;
        let src = format!(
            "{generated_directory}/{}",
            generated_leaf_name(crc, "canvas", counters.canvas)
        );
        let data_uri = format!("data:image/png;base64,{}", STANDARD.encode(&png.bytes));
        inline_replacements.push(FragmentReplacement {
            id,
            html: image_html(&data_uri, &canvas_alt_text(source)),
        });
        standalone_rewrites.insert(data_uri, src.clone());
        pngs.push((src, png.bytes));
    }
    let table_plan = plan.after_canvas_replacements(&inline_replacements)?;
    let tables = table_plan
        .fragments()
        .iter()
        .filter_map(|fragment| fragment.table_html().map(|html| (fragment.id(), html)))
        .collect::<Vec<_>>();
    let rendered_tables = tables
        .par_iter()
        .map(|(id, html)| {
            Ok((
                *id,
                render_table(renderer, cache, crc, field.clone(), *id, html)?,
            ))
        })
        .collect::<Result<Vec<_>, ConversionFailure>>()
        .map_err(|mut failure| {
            *failure.metrics += metrics.clone();
            failure
        })?;
    let mut table_replacements = Vec::new();
    for ((id, html), (_, (png, fragment_metrics))) in tables.into_iter().zip(rendered_tables) {
        metrics += fragment_metrics;
        counters.table += 1;
        let src = format!(
            "{generated_directory}/{}",
            generated_leaf_name(crc, "table", counters.table)
        );
        table_replacements.push(FragmentReplacement {
            id,
            html: image_html(&src, &table_alt_text(html)),
        });
        pngs.push((src, png.bytes));
    }
    let html = table_plan.apply_replacements(&table_replacements)?;
    let html = rewrite_html_srcs(&html, |src| {
        standalone_rewrites
            .get(src)
            .cloned()
            .unwrap_or_else(|| src.to_owned())
    })?;
    let referenced = media::scan_html_for_assets(&html)?
        .into_iter()
        .collect::<HashSet<_>>();
    pngs.retain(|(src, _)| referenced.contains(src));
    Ok(ConvertedField {
        html,
        pngs,
        metrics,
    })
}

fn render_canvas<R: FragmentRenderer>(
    renderer: &R,
    cache: &RenderCache,
    item_crc: ItemCrc,
    field: FieldId,
    fragment_id: FragmentId,
    source: &CanvasSource,
) -> Result<(RenderedPng, ConversionMetrics), ConversionFailure> {
    let input = format!("{source:?}");
    render_cached(
        RenderFamily::Canvas,
        input.as_bytes(),
        cache,
        RenderContext {
            item_crc,
            field,
            fragment_id,
            snippet: &input,
            renderer_discriminator: renderer.cache_discriminator(),
        },
        || renderer.render_canvas(source),
    )
}

fn render_table<R: FragmentRenderer>(
    renderer: &R,
    cache: &RenderCache,
    item_crc: ItemCrc,
    field: FieldId,
    fragment_id: FragmentId,
    html: &str,
) -> Result<(RenderedPng, ConversionMetrics), ConversionFailure> {
    render_cached(
        RenderFamily::Table,
        html.as_bytes(),
        cache,
        RenderContext {
            item_crc,
            field,
            fragment_id,
            snippet: html,
            renderer_discriminator: renderer.cache_discriminator(),
        },
        || renderer.render_table(html),
    )
}

struct RenderContext<'a> {
    item_crc: ItemCrc,
    field: FieldId,
    fragment_id: FragmentId,
    snippet: &'a str,
    renderer_discriminator: String,
}

fn render_cached<E: Error + Send + Sync + 'static>(
    family: RenderFamily,
    input: &[u8],
    cache: &RenderCache,
    context: RenderContext<'_>,
    render: impl FnOnce() -> Result<RenderedPng, E>,
) -> Result<(RenderedPng, ConversionMetrics), ConversionFailure> {
    let key = RenderKey::new(family, &context.renderer_discriminator, input);
    let (outcome, rendered) = cache.get_or_render(key, render);
    let mut metrics = ConversionMetrics {
        requested_fragments: 1,
        ..ConversionMetrics::default()
    };
    match outcome {
        CacheOutcome::Hit => metrics.cache_hits = 1,
        CacheOutcome::Wait => metrics.cache_waits = 1,
        CacheOutcome::Miss => {
            metrics.cache_misses = 1;
            metrics.renderer_attempts = 1;
        }
    }
    match rendered {
        Ok(png) => {
            if outcome == CacheOutcome::Miss {
                metrics.renderer_successes = 1;
                metrics.render = png.metrics;
            }
            Ok((png, metrics))
        }
        Err(error) => {
            if outcome == CacheOutcome::Miss {
                metrics.renderer_failures = 1;
            }
            Err(ConversionFailure {
                error: Box::new(ConversionError::Render {
                    family: match family {
                        RenderFamily::Canvas => "canvas",
                        RenderFamily::Table => "table",
                    },
                    item_crc: context.item_crc,
                    field: context.field,
                    fragment_id: context.fragment_id,
                    snippet: context.snippet.chars().take(240).collect(),
                    source: Box::new(error),
                }),
                metrics: Box::new(metrics),
            })
        }
    }
}

struct SnapshottedAsset {
    src: String,
    target: String,
    bytes: Vec<u8>,
}

/// Resolves source media while accepting deliberate absolute input paths for confined aliasing.
///
/// The core package resolver rejects absolute paths outside a bank base, correctly for ordinary
/// package writes. Conversion has a stricter lifecycle: it reads an explicitly named absolute
/// file before an owned output root exists, then rewrites it below `__qti_input`.
fn collect_source_assets(bank: &ItemBank) -> Result<Vec<MediaAsset>, ConversionError> {
    let mut assets = BTreeMap::new();
    for item in bank.iter_ordered() {
        for (_, html) in media::item_html_fields(item) {
            for src in media::scan_html_for_assets(html)? {
                if assets.contains_key(&src) {
                    continue;
                }
                let asset = if Path::new(&src).is_absolute() {
                    absolute_source_asset(&src)?
                } else {
                    media::resolve_asset(&src, bank.media_base_dir())?
                };
                assets.insert(src, asset);
            }
        }
    }
    Ok(assets.into_values().collect())
}

fn absolute_source_asset(src: &str) -> Result<MediaAsset, ConversionError> {
    let path = PathBuf::from(src);
    let mime_type = media::guess_mime_type(src)?.to_owned();
    if !path.is_file() {
        return Err(media::MediaError::MissingFile {
            src: src.to_owned(),
            path,
        }
        .into());
    }
    let file_path = path
        .canonicalize()
        .map_err(|source| media::MediaError::ReadFile {
            src: src.to_owned(),
            path: path.clone(),
            source,
        })?;
    Ok(MediaAsset {
        src: src.to_owned(),
        kind: AssetKind::Local,
        mime_type: Some(mime_type),
        file_path: Some(file_path),
        data_bytes: None,
        output_name: None,
        content_hash: None,
    })
}

fn snapshot_local_assets(
    assets: &[MediaAsset],
    input_directory: &str,
) -> Result<Vec<SnapshottedAsset>, ConversionError> {
    assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| {
            let bytes = asset.read_bytes()?;
            let target = if Path::new(&asset.src).is_absolute() {
                let extension = Path::new(&asset.src)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("png");
                let digest = Sha256::digest(&bytes);
                let hex = digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                format!("{input_directory}/{hex}.{extension}")
            } else {
                asset.src.clone()
            };
            Ok(SnapshottedAsset {
                src: asset.src.clone(),
                target,
                bytes,
            })
        })
        .collect()
}

fn materialize_source_assets(
    result: &mut ItemBank,
    assets: &[SnapshottedAsset],
) -> Result<HashMap<String, String>, ConversionError> {
    let mut absolute_aliases = HashMap::new();
    for asset in assets {
        if Path::new(&asset.src).is_absolute() {
            absolute_aliases.insert(asset.src.clone(), asset.target.clone());
        }
        result.add_image(&asset.target, &asset.bytes)?;
    }
    Ok(absolute_aliases)
}

fn rebuild_item(
    item: &Item,
    fields: &[String],
    absolute_aliases: &HashMap<String, String>,
) -> Result<Item, ConversionError> {
    let mut fields = fields.iter();
    let rebuilt = item.with_rewritten_html_fields(|_| {
        rewrite_absolute_sources(next_field(&mut fields)?, absolute_aliases)
    })?;
    if fields.next().is_some() {
        return Err(ConversionError::PlanMismatch);
    }
    Ok(rebuilt)
}

fn next_field<'a>(fields: &mut std::slice::Iter<'a, String>) -> Result<&'a str, ConversionError> {
    fields
        .next()
        .map(String::as_str)
        .ok_or(ConversionError::PlanMismatch)
}

fn rewrite_absolute_sources(
    html: &str,
    aliases: &HashMap<String, String>,
) -> Result<String, ConversionError> {
    rewrite_html_srcs(html, |src| {
        aliases.get(src).cloned().unwrap_or_else(|| src.to_owned())
    })
    .map_err(ConversionError::from)
}

fn image_html(src: &str, alt: &str) -> String {
    format!(
        "<img src=\"{}\" alt=\"{}\" />",
        attribute_escape(src),
        attribute_escape(alt)
    )
}

fn attribute_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "convert/tests.rs"]
mod tests;
