//! Static selection and validation for opt-in HTML-to-image conversion.
//!
//! This module recognizes the constrained RDKit drawing scripts emitted by the
//! Python generators. It never evaluates an item script.

mod cache;
mod canvas_script;
mod chromium;
mod convert;
mod naming;
mod selectors;

pub use cache::{CacheOutcome, RenderCache, RenderMetrics, RenderedPng};
pub use convert::{
    ChromiumFragmentRenderer, ChromiumRenderError, ConversionError, ConversionFailure,
    ConversionMetrics, FragmentRenderer, convert_bank, convert_bank_with_metrics,
};

pub use canvas_script::{CanvasScriptError, parse_canvas_script};
pub use selectors::{
    CanvasTarget, FieldConversionPlan, Fragment, FragmentId, FragmentKind, FragmentReplacement,
    HtmlToImageError, PreparedFragment, apply_replacements, find_canvas_targets,
    find_table_fragments, remove_rdkit_loader_scripts,
};
