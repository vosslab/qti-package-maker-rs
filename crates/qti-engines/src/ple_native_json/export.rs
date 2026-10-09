//! In-memory export of validated QPM items as PLE Native JSON.

use qti_core::media::AssetSource;
use qti_core::{ItemBank, ItemCrc, ItemRenderView, NamedFile};

use super::{mapping::map_item, media::finish_export, scan::scan_document, source::SourceDocument};
use crate::{EngineError, MediaWarning, RenderHooks, render_bank};

const ENGINE: &str = "ple_native_json";
const FORMAT: &str = "PLE Native JSON";
/// A serialized question and the files needed to interpret it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeQuestion {
    /// Bank-assigned, one-based item number.
    pub item_number: usize,
    /// Stable identity of the authored source item.
    pub crc: ItemCrc,
    /// Compact `pleQuestionJson` document.
    pub source_json: String,
    /// Associated files for this question.
    pub files: Vec<NamedFile>,
}

/// Completed in-memory conversion, in bank order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeExport {
    /// Every rendered question in bank order.
    pub questions: Vec<NativeQuestion>,
    /// Recoverable media diagnostics.
    pub warnings: Vec<MediaWarning>,
}

/// Export the seven supported item kinds to compact PLE Native JSON.
///
/// The export scans represented display HTML, resolves local media, and records external URLs.
///
/// # Errors
///
/// Returns [`EngineError`] when QPM cannot represent a mapped item, its display HTML or media
/// cannot be scanned, resolved, or serialized, or its external-resource inventory is invalid.
pub fn export_bank(bank: &ItemBank, assets: &dyn AssetSource) -> Result<NativeExport, EngineError> {
    let kinds = crate::engine(ENGINE)
        .expect("registered PLE engine")
        .supported_kinds;
    let mapped = render_bank(bank, kinds, render_item, RenderHooks::default())?;
    finish_export(assets, mapped)
}

pub(super) struct MappedQuestion {
    pub(super) item_number: usize,
    pub(super) crc: ItemCrc,
    pub(super) document: SourceDocument,
}

fn render_item(item: &ItemRenderView) -> Result<Option<MappedQuestion>, EngineError> {
    let mut document = map_item(item)?;
    document.external_resources = scan_document(&document, item.common().item_number)?;
    Ok(Some(MappedQuestion {
        item_number: item.common().item_number,
        crc: *item.crc(),
        document,
    }))
}

pub(super) fn serialize(
    item_number: usize,
    document: &SourceDocument,
) -> Result<String, EngineError> {
    serde_json::to_string(document).map_err(|error| EngineError::InvalidFormat {
        engine: ENGINE,
        format: FORMAT,
        message: format!("item {item_number}: {error}"),
    })
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
