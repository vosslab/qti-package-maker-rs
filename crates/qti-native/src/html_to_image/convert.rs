//! Two-phase, source-preserving HTML table and canvas conversion.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};
use rayon::prelude::*;
use scraper::{Html, Selector};
use thiserror::Error;

use qti_core::media::{self, AssetKind, rewrite_html_srcs};
use qti_core::media::{AssetSource, MemoryAssets};
use qti_core::{BankError, FieldId, Item, ItemBank, ItemCrc, ItemKind, ValidationError};
use qti_molecule::CanvasSource;

use super::cache::{
    CacheOutcome, RenderCache, RenderFamily, RenderKey, RenderMetrics, RenderedPng,
};
use super::naming::{canvas_alt_text, generated_directory, generated_leaf_name, table_alt_text};
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
    /// Time spent assembling owned output asset bytes.
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

/// Converts selected tables and supported canvases into a bank plus owned memory assets.
///
/// Selection and rendering use the read-only caller source. No native output is written.
/// Unsupported kinds remain unchanged. Only images inside selected table fragments are read.
/// Returned assets contain generated PNGs; callers overlay them over the original input source.
pub fn convert_bank<R: FragmentRenderer>(
    bank: &ItemBank,
    supported_kinds: &[ItemKind],
    assets: &dyn AssetSource,
    renderer: &R,
    cache: &RenderCache,
) -> Result<(ItemBank, MemoryAssets), ConversionError> {
    convert_bank_with_metrics(bank, supported_kinds, assets, renderer, cache)
        .map(|(converted, assets, _)| (converted, assets))
        .map_err(|failure| *failure.error)
}

/// Converts a bank and returns cumulative conversion work measurements.
pub fn convert_bank_with_metrics<R: FragmentRenderer>(
    bank: &ItemBank,
    supported_kinds: &[ItemKind],
    assets: &dyn AssetSource,
    renderer: &R,
    cache: &RenderCache,
) -> Result<(ItemBank, MemoryAssets, ConversionMetrics), ConversionFailure> {
    let bookkeeping_started = Instant::now();
    let plans = bank
        .iter_ordered()
        .map(|item| ItemPlan::prepare(item, supported_kinds))
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
            MemoryAssets::new(),
            ConversionMetrics {
                conversion_bookkeeping: bookkeeping_started.elapsed(),
                ..ConversionMetrics::default()
            },
        ));
    }

    // Reserving names inspects display references without resolving unused payloads.
    let occupied_sources = plans
        .iter()
        .flat_map(|plan| &plan.fields)
        .map(|field| media::scan_html_for_assets(field.plan.prepared_html()))
        .collect::<Result<Vec<_>, _>>()?;
    let generated_directory =
        generated_directory(occupied_sources.iter().flatten().map(String::as_str));

    let preliminary_bookkeeping = bookkeeping_started.elapsed();
    let converted = plans
        .par_iter()
        .map(|plan| plan.render(assets, renderer, cache, &generated_directory))
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
    let mut output_assets = MemoryAssets::new();
    metrics.conversion_bookkeeping += setup_started.elapsed();
    let materialization_started = Instant::now();
    let mut generated = BTreeMap::new();
    for item in &converted {
        for (src, png) in &item.generated_pngs {
            generated.entry(src.clone()).or_insert_with(|| png.clone());
        }
    }
    for (src, png) in generated {
        output_assets.insert(src, png)?;
    }
    metrics.materialization += materialization_started.elapsed();
    let rebuild_started = Instant::now();
    let mut converted = converted.into_iter();
    let result = bank.with_rewritten_items(|item| {
        let converted = converted.next().ok_or(ConversionError::PlanMismatch)?;
        rebuild_item(item, &converted.fields)
    })?;
    metrics.conversion_bookkeeping += rebuild_started.elapsed();
    Ok((result, output_assets, metrics))
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
    fn prepare(item: &Item, supported_kinds: &[ItemKind]) -> Result<Self, HtmlToImageError> {
        let fields = if supported_kinds.contains(&item.kind()) {
            display_fields(item)
        } else {
            Vec::new()
        }
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
        assets: &dyn AssetSource,
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
                field,
                *self.item.crc(),
                assets,
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
            fields,
            generated_pngs,
            metrics,
        })
    }
}

struct ConvertedItem {
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
    planned: &PlannedField,
    crc: ItemCrc,
    assets: &dyn AssetSource,
    counters: &mut FamilyCounters,
    renderer: &R,
    cache: &RenderCache,
    generated_directory: &str,
) -> Result<ConvertedField, ConversionFailure> {
    let plan = &planned.plan;
    let field = &planned.field;
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
            let prepared = inline_table_images(html, assets)?;
            Ok((
                *id,
                render_table(renderer, cache, crc, field.clone(), *id, &prepared)?,
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

/// Fill-in answers are literal grading values; other answer fields also carry display HTML.
fn display_fields(item: &Item) -> Vec<(FieldId, &str)> {
    if matches!(item.kind(), ItemKind::Fib | ItemKind::MultiFib) {
        vec![(FieldId::Stem, item.common().question_text.as_str())]
    } else {
        media::item_html_fields(item)
    }
}

fn inline_table_images(html: &str, source: &dyn AssetSource) -> Result<String, ConversionError> {
    let raw_sources = media::scan_html_for_assets(html)?;
    let document = Html::parse_fragment(html);
    let images = Selector::parse("img[src]").expect("static image selector");
    let viewer_sources = document
        .select(&images)
        .filter_map(|image| image.value().attr("src"))
        .map(str::trim)
        .filter(|src| !src.is_empty())
        .collect::<Vec<_>>();
    if raw_sources.len() != viewer_sources.len() {
        return Err(ConversionError::PlanMismatch);
    }
    let mut replacements = HashMap::new();
    for (raw, decoded) in raw_sources.into_iter().zip(viewer_sources) {
        if media::classify_src(decoded) == AssetKind::Local && !replacements.contains_key(&raw) {
            // Browser attributes decode entities; bind the same bytes before replacing the URL.
            let asset = media::resolve_asset(decoded, source)?;
            let bytes = asset.read_bytes()?;
            let mime = asset.mime_type.ok_or(ConversionError::PlanMismatch)?;
            replacements.insert(
                raw,
                format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
            );
        }
    }
    Ok(rewrite_html_srcs(html, |src| {
        replacements
            .get(src.trim())
            .cloned()
            .unwrap_or_else(|| src.to_owned())
    })?)
}

fn rebuild_item(item: &Item, fields: &[String]) -> Result<Item, ConversionError> {
    if fields.is_empty() {
        return Ok(item.clone());
    }
    let grading_literals = matches!(item.kind(), ItemKind::Fib | ItemKind::MultiFib);
    let mut fields = fields.iter();
    let mut ordinal = 0;
    let rebuilt = item.with_rewritten_html_fields(|original| -> Result<_, ConversionError> {
        let is_display = ordinal == 0 || !grading_literals;
        ordinal += 1;
        if is_display {
            Ok(next_field(&mut fields)?.to_owned())
        } else {
            Ok(original.to_owned())
        }
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
