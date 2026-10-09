//! Two-phase, source-preserving HTML table and canvas conversion.

use std::error::Error;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use thiserror::Error;

use qti_core::media;
use qti_core::media::{AssetSource, MemoryAssets};
use qti_core::{BankError, FieldId, ItemBank, ItemCrc, ItemKind, ValidationError};
use qti_molecule::CanvasSource;

use super::cache::{
    CacheOutcome, RenderCache, RenderFamily, RenderKey, RenderMetrics, RenderedPng,
};
use super::{FragmentId, HtmlToImageError};
#[cfg(test)]
use qti_render::inline_table_images;
use qti_render::{
    RenderCompletion, RenderError, RenderJobKind, finish_bank, plan_bank, prepare_table,
};

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
                width: f64::from(source.width),
                height: f64::from(source.height),
                metrics: Default::default(),
            })
            .map_err(ChromiumRenderError::Canvas)
    }

    fn render_table(&self, html: &str) -> Result<RenderedPng, Self::Error> {
        self.table
            .render(html)
            .map(|(bytes, width, height)| RenderedPng {
                bytes,
                width,
                height,
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
    /// Portable planning or completion failed.
    #[error(transparent)]
    Portable(#[from] RenderError),
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
    let plan = plan_bank(bank, supported_kinds, assets).map_err(ConversionError::from)?;
    let mut metrics = ConversionMetrics {
        conversion_bookkeeping: bookkeeping_started.elapsed(),
        ..ConversionMetrics::default()
    };
    let mut completions = Vec::new();
    for family in [RenderJobKind::Canvas, RenderJobKind::Table] {
        let rendered = plan
            .jobs
            .par_iter()
            .filter(|job| job.kind == family)
            .map(|job| {
                let (png, counters) = match job.kind {
                    RenderJobKind::Canvas => render_canvas(
                        renderer,
                        cache,
                        job.item_crc,
                        job.field.clone(),
                        job.fragment_id,
                        job.canvas_spec
                            .as_ref()
                            .ok_or(ConversionError::PlanMismatch)?,
                    )?,
                    RenderJobKind::Table => {
                        let html =
                            prepare_table(job, &completions).map_err(ConversionError::from)?;
                        render_table(
                            renderer,
                            cache,
                            job.item_crc,
                            job.field.clone(),
                            job.fragment_id,
                            &html,
                        )?
                    }
                };
                Ok((
                    RenderCompletion {
                        id: job.id.clone(),
                        png: png.bytes,
                        width: png.width,
                        height: png.height,
                    },
                    counters,
                ))
            })
            .collect::<Result<Vec<_>, ConversionFailure>>()
            .map_err(|mut failure| {
                *failure.metrics += metrics.clone();
                failure
            })?;
        for (completion, counters) in rendered {
            completions.push(completion);
            metrics += counters;
        }
    }
    let reused = plan.requested_fragments().saturating_sub(plan.jobs.len());
    metrics.requested_fragments += reused;
    metrics.cache_hits += reused;
    let materialization_started = Instant::now();
    let (result, output_assets) =
        finish_bank(bank, &plan, &completions).map_err(ConversionError::from)?;
    metrics.materialization += materialization_started.elapsed();
    Ok((result, output_assets, metrics))
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

#[cfg(test)]
#[path = "convert/tests.rs"]
mod tests;
