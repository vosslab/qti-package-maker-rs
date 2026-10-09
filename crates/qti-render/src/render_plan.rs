//! Plans source-owned presentation work and applies host-rendered PNGs.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use lol_html::{Settings, element, rewrite_str};
use qti_core::media::{self, AssetKind, AssetSource, MemoryAssets, rewrite_html_srcs};
use qti_core::{BankError, FieldId, Item, ItemBank, ItemCrc, ItemKind, ValidationError};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::naming::{canvas_alt_text, generated_directory, generated_leaf_name, table_alt_text};
use crate::{CanvasSource, FieldConversionPlan, FragmentId, FragmentReplacement, HtmlToImageError};

/// Renderer family selected by the source grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderJobKind {
    /// A static molecule depiction.
    Canvas,
    /// An outermost HTML table.
    Table,
}

/// One deterministic host rendering request.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderJob {
    /// Unique identity within the original bank, independent of rendered bytes.
    pub id: String,
    /// The host renderer to invoke.
    pub kind: RenderJobKind,
    /// Prepared table HTML, with explicit nested-canvas image references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// Full source-owned molecule options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas_spec: Option<CanvasSource>,
    /// SHA-256 of the complete deterministic renderer input, for host reuse.
    pub content_hash: String,
    /// Canvas jobs whose PNGs are required before rendering this table.
    pub dependencies: Vec<String>,
    /// Original item identity for error context.
    pub item_crc: ItemCrc,
    /// Original display field for error context.
    pub field: FieldId,
    /// Original selected node for error context.
    pub fragment_id: FragmentId,
}

/// Host-rendered image and its logical CSS dimensions, independent of pixel density.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderCompletion {
    /// Corresponding planned job identity.
    pub id: String,
    /// Complete PNG bytes.
    pub png: Vec<u8>,
    /// Positive finite CSS width.
    pub width: f64,
    /// Positive finite CSS height.
    pub height: f64,
}

/// A bank plan retains the original assessment and field selection privately.
#[derive(Clone, Debug)]
pub struct BankRenderPlan {
    /// Unique canvas jobs followed by table jobs; dependencies name exact canvas jobs.
    pub jobs: Vec<RenderJob>,
    items: Vec<PlannedItem>,
    directory: String,
    requested_fragments: usize,
}

impl BankRenderPlan {
    /// Fragment requests including repeated presentation fields served by reuse.
    pub fn requested_fragments(&self) -> usize {
        self.requested_fragments
    }
}

#[derive(Clone, Debug)]
struct PlannedItem {
    original: Item,
    fields: Vec<PlannedField>,
}
#[derive(Clone, Debug)]
struct PlannedTable {
    fragment_id: FragmentId,
    job_id: String,
    leaf: String,
}

#[derive(Clone, Debug)]
struct PlannedField {
    table_plan: FieldConversionPlan,
    tables: Vec<PlannedTable>,
    canvases: Vec<(String, String)>,
}

/// A portable planning or completion failure.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Selection failed without executing authored scripts.
    #[error(transparent)]
    Selection(#[from] HtmlToImageError),
    /// An authored asset could not be resolved.
    #[error(transparent)]
    Media(#[from] media::MediaError),
    /// A bank identity check failed.
    #[error(transparent)]
    Bank(#[from] BankError),
    /// A rewritten item failed validation.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// The supplied original bank or job shape does not match this plan.
    #[error("render plan does not match the original bank or requested job")]
    PlanMismatch,
    /// A required completion is absent.
    #[error("missing render completion for job {0}")]
    MissingCompletion(String),
    /// A completion does not belong to this plan.
    #[error("unknown render completion for job {0}")]
    UnknownCompletion(String),
    /// A job was completed more than once.
    #[error("duplicate render completion for job {0}")]
    DuplicateCompletion(String),
    /// Host output is not a PNG or has invalid CSS dimensions.
    #[error("invalid PNG or logical CSS dimensions for job {0}")]
    InvalidCompletion(String),
    /// Serialization of a validated source unexpectedly failed.
    #[error("could not serialize canvas source: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Selects supported presentation fields and resolves only images in selected tables.
pub fn plan_bank(
    bank: &ItemBank,
    supported_kinds: &[ItemKind],
    assets: &dyn AssetSource,
) -> Result<BankRenderPlan, RenderError> {
    let occupied = bank
        .iter_ordered()
        .flat_map(display_fields)
        .map(|(_, html)| media::scan_html_for_assets(html))
        .collect::<Result<Vec<_>, _>>()?;
    let directory = generated_directory(occupied.iter().flatten().map(String::as_str));
    let mut jobs = Vec::new();
    let mut items = Vec::new();
    let mut requested_fragments = 0;
    for (item_index, item) in bank.iter_ordered().enumerate() {
        let mut fields = Vec::new();
        let mut reused = HashMap::<String, PlannedField>::new();
        let mut canvas_number = 0;
        let mut table_number = 0;
        if supported_kinds.contains(&item.kind()) {
            for (field_index, (field, html)) in display_fields(item).into_iter().enumerate() {
                let selection = FieldConversionPlan::prepare(html)?;
                requested_fragments += selection.fragments().len();
                if let Some(previous) = reused.get(selection.prepared_html()) {
                    fields.push(previous.clone());
                    continue;
                }
                let mut replacements = Vec::new();
                let mut canvases = Vec::new();
                for fragment in selection.fragments() {
                    if let Some(source) = fragment.canvas_source() {
                        canvas_number += 1;
                        let id = format!(
                            "{}-item{item_index}-field{field_index}-canvas{canvas_number}",
                            item.crc()
                        );
                        let path = generated_leaf_name(*item.crc(), "canvas", canvas_number);
                        replacements.push(FragmentReplacement {
                            id: fragment.id(),
                            html: image_html(
                                &placeholder(&id),
                                &canvas_alt_text(source),
                                f64::from(source.width),
                                f64::from(source.height),
                            ),
                        });
                        canvases.push((id.clone(), path));
                        jobs.push(RenderJob {
                            id,
                            kind: RenderJobKind::Canvas,
                            html: None,
                            canvas_spec: Some(source.clone()),
                            content_hash: digest(&serde_json::to_vec(source)?),
                            dependencies: Vec::new(),
                            item_crc: *item.crc(),
                            field: field.clone(),
                            fragment_id: fragment.id(),
                        });
                    }
                }
                let table_plan = selection.after_canvas_replacements(&replacements)?;
                let mut tables = Vec::new();
                for fragment in table_plan.fragments() {
                    if let Some(html) = fragment.table_html() {
                        table_number += 1;
                        let id = format!(
                            "{}-item{item_index}-field{field_index}-table{table_number}",
                            item.crc()
                        );
                        let html = inline_table_images(html, assets)?;
                        let refs = media::scan_html_for_assets(&html)?;
                        let dependencies = canvases
                            .iter()
                            .filter(|(id, _)| refs.contains(&placeholder(id)))
                            .map(|(id, _)| id.clone())
                            .collect::<Vec<_>>();
                        // Canonicalize dependency references by source content, preserving reuse
                        // across item identities without conflating different molecule depictions.
                        let dependency_hashes = dependencies
                            .iter()
                            .map(|dependency| {
                                let job = jobs
                                    .iter()
                                    .find(|job| &job.id == dependency)
                                    .ok_or(RenderError::PlanMismatch)?;
                                Ok((placeholder(dependency), placeholder(&job.content_hash)))
                            })
                            .collect::<Result<HashMap<_, _>, RenderError>>()?;
                        let canonical = rewrite_html_srcs(&html, |src| {
                            dependency_hashes
                                .get(src)
                                .cloned()
                                .unwrap_or_else(|| src.to_owned())
                        })?;
                        jobs.push(RenderJob {
                            id: id.clone(),
                            kind: RenderJobKind::Table,
                            content_hash: digest(canonical.as_bytes()),
                            html: Some(html),
                            canvas_spec: None,
                            dependencies,
                            item_crc: *item.crc(),
                            field: field.clone(),
                            fragment_id: fragment.id(),
                        });
                        tables.push(PlannedTable {
                            fragment_id: fragment.id(),
                            job_id: id,
                            leaf: generated_leaf_name(*item.crc(), "table", table_number),
                        });
                    }
                }
                let planned = PlannedField {
                    table_plan,
                    tables,
                    canvases,
                };
                reused.insert(selection.prepared_html().to_owned(), planned.clone());
                fields.push(planned);
            }
        }
        items.push(PlannedItem {
            original: item.clone(),
            fields,
        });
    }
    jobs.sort_by_key(|job| match job.kind {
        RenderJobKind::Canvas => 0,
        RenderJobKind::Table => 1,
    });
    Ok(BankRenderPlan {
        jobs,
        items,
        directory,
        requested_fragments,
    })
}

/// Substitutes exact nested-canvas references with completed data PNGs and CSS dimensions.
pub fn prepare_table(job: &RenderJob, renders: &[RenderCompletion]) -> Result<String, RenderError> {
    if job.kind != RenderJobKind::Table {
        return Err(RenderError::PlanMismatch);
    }
    let mut replacements = HashMap::new();
    for id in &job.dependencies {
        let mut matching = renders.iter().filter(|render| &render.id == id);
        let render = matching
            .next()
            .ok_or_else(|| RenderError::MissingCompletion(id.clone()))?;
        if matching.next().is_some() {
            return Err(RenderError::DuplicateCompletion(id.clone()));
        }
        validate_completion(render)?;
        replacements.insert(
            placeholder(id),
            (
                format!("data:image/png;base64,{}", STANDARD.encode(&render.png)),
                render.width,
                render.height,
            ),
        );
    }
    substitute_images(
        job.html.as_deref().ok_or(RenderError::PlanMismatch)?,
        &replacements,
    )
}

/// Rewrites the original bank without recomputing CRCs, item order, or assessment metadata.
pub fn finish_bank(
    bank: &ItemBank,
    plan: &BankRenderPlan,
    renders: &[RenderCompletion],
) -> Result<(ItemBank, MemoryAssets), RenderError> {
    if bank.len() != plan.items.len()
        || bank.iter_ordered().zip(&plan.items).any(|(item, planned)| {
            item != &planned.original || item.common() != planned.original.common()
        })
    {
        return Err(RenderError::PlanMismatch);
    }
    let known = plan
        .jobs
        .iter()
        .map(|job| job.id.as_str())
        .collect::<HashSet<_>>();
    let mut completed = HashMap::new();
    for render in renders {
        if !known.contains(render.id.as_str()) {
            return Err(RenderError::UnknownCompletion(render.id.clone()));
        }
        validate_completion(render)?;
        if completed.insert(render.id.as_str(), render).is_some() {
            return Err(RenderError::DuplicateCompletion(render.id.clone()));
        }
    }
    for job in &plan.jobs {
        if !completed.contains_key(job.id.as_str()) {
            return Err(RenderError::MissingCompletion(job.id.clone()));
        }
    }
    if plan.jobs.is_empty() {
        return Ok((bank.clone(), MemoryAssets::new()));
    }
    let mut assets = MemoryAssets::new();
    let mut planned_items = plan.items.iter();
    let result = bank.with_rewritten_items(|item| -> Result<Item, RenderError> {
        let planned = planned_items.next().ok_or(RenderError::PlanMismatch)?;
        let mut fields = Vec::new();
        for field in &planned.fields {
            let mut replacements = Vec::new();
            let mut pngs = Vec::new();
            for table in &field.tables {
                let id = &table.job_id;
                let render = completed
                    .get(id.as_str())
                    .ok_or_else(|| RenderError::MissingCompletion(id.clone()))?;
                let job = plan
                    .jobs
                    .iter()
                    .find(|job| &job.id == id)
                    .ok_or(RenderError::PlanMismatch)?;
                let src = format!("{}/{}", plan.directory, table.leaf);
                replacements.push(FragmentReplacement {
                    id: table.fragment_id,
                    html: image_html(
                        &src,
                        &table_alt_text(job.html.as_deref().ok_or(RenderError::PlanMismatch)?),
                        render.width,
                        render.height,
                    ),
                });
                pngs.push((src, render.png.clone()));
            }
            let html = field.table_plan.apply_replacements(&replacements)?;
            let mut canvas_replacements = HashMap::new();
            for (id, leaf) in &field.canvases {
                let render = completed
                    .get(id.as_str())
                    .ok_or_else(|| RenderError::MissingCompletion(id.clone()))?;
                let src = format!("{}/{leaf}", plan.directory);
                canvas_replacements
                    .insert(placeholder(id), (src.clone(), render.width, render.height));
                pngs.push((src, render.png.clone()));
            }
            let html = substitute_images(&html, &canvas_replacements)?;
            let referenced = media::scan_html_for_assets(&html)?
                .into_iter()
                .collect::<HashSet<_>>();
            for (src, png) in pngs {
                if referenced.contains(&src) {
                    assets.insert(src, png)?;
                }
            }
            fields.push(html);
        }
        rebuild_item(item, &fields)
    })?;
    Ok((result, assets))
}

fn validate_completion(render: &RenderCompletion) -> Result<(), RenderError> {
    if !render.png.starts_with(b"\x89PNG\r\n\x1a\n")
        || !render.width.is_finite()
        || !render.height.is_finite()
        || render.width <= 0.0
        || render.height <= 0.0
    {
        return Err(RenderError::InvalidCompletion(render.id.clone()));
    }
    validate_png(&render.png).map_err(|_| RenderError::InvalidCompletion(render.id.clone()))
}

fn validate_png(bytes: &[u8]) -> Result<(), png::DecodingError> {
    // ASVS 15.2.2: use the decoder's established allocation bound and discard decoded rows.
    let limits = png::Limits::default();
    let mut decoder = png::Decoder::new_with_limits(Cursor::new(bytes), limits);
    decoder.ignore_checksums(false);
    let mut reader = decoder.read_info()?;
    while reader.next_row()?.is_some() {}
    reader.finish()
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn placeholder(id: &str) -> String {
    format!("qti-render:{id}")
}
fn display_fields(item: &Item) -> Vec<(FieldId, &str)> {
    if matches!(item.kind(), ItemKind::Fib | ItemKind::MultiFib) {
        vec![(FieldId::Stem, item.common().question_text.as_str())]
    } else {
        media::item_html_fields(item)
    }
}

/// Inlines local assets selected inside a table using browser-decoded attribute keys.
pub fn inline_table_images(html: &str, source: &dyn AssetSource) -> Result<String, RenderError> {
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
        return Err(RenderError::PlanMismatch);
    }
    let mut replacements = HashMap::new();
    for (raw, decoded) in raw_sources.into_iter().zip(viewer_sources) {
        if !decoded.starts_with("qti-render:")
            && media::classify_src(decoded) == AssetKind::Local
            && !replacements.contains_key(&raw)
        {
            let asset = media::resolve_asset(decoded, source)?;
            let bytes = asset.read_bytes()?;
            let mime = asset.mime_type.ok_or(RenderError::PlanMismatch)?;
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

fn substitute_images(
    html: &str,
    replacements: &HashMap<String, (String, f64, f64)>,
) -> Result<String, RenderError> {
    rewrite_str(
        html,
        Settings::new().append_element_content_handler(element!("img[src]", |element| {
            if let Some((src, width, height)) = element
                .get_attribute("src")
                .as_ref()
                .and_then(|src| replacements.get(src))
            {
                element.set_attribute("src", src)?;
                element.set_attribute("width", &width.to_string())?;
                element.set_attribute("height", &height.to_string())?;
            }
            Ok(())
        })),
    )
    .map_err(|error| HtmlToImageError::Rewrite(error.to_string()).into())
}
fn rebuild_item(item: &Item, fields: &[String]) -> Result<Item, RenderError> {
    if fields.is_empty() {
        return Ok(item.clone());
    }
    let grading_literals = matches!(item.kind(), ItemKind::Fib | ItemKind::MultiFib);
    let mut fields = fields.iter();
    let mut ordinal = 0;
    let rebuilt = item.with_rewritten_html_fields(|original| -> Result<_, RenderError> {
        let is_display = ordinal == 0 || !grading_literals;
        ordinal += 1;
        if is_display {
            Ok(fields.next().ok_or(RenderError::PlanMismatch)?.clone())
        } else {
            Ok(original.to_owned())
        }
    })?;
    if fields.next().is_some() {
        return Err(RenderError::PlanMismatch);
    }
    Ok(rebuilt)
}
fn image_html(src: &str, alt: &str, width: f64, height: f64) -> String {
    format!(
        "<img src=\"{}\" alt=\"{}\" width=\"{width}\" height=\"{height}\" style=\"max-width: 100%; height: auto;\" />",
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
