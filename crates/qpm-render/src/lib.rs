//! Portable, identity-preserving render planning and finalization.
//!
//! Hosts render the returned canvas jobs first, prepare dependent table jobs with
//! [`prepare_table`], then pass all completed PNGs to [`finish_bank`].

mod canvas_script;
mod canvas_source;
mod naming;
mod render_plan;
#[cfg(test)]
mod render_plan_tests;
mod selectors;
mod wrapper;

pub use canvas_script::{CanvasScriptError, parse_canvas_script};
pub use canvas_source::{CanvasSource, MAX_CANVAS_DIMENSION, PeptideQuery};
pub use render_plan::{
    BankRenderPlan, RenderCompletion, RenderError, RenderJob, RenderJobKind, finish_bank,
    inline_table_images, plan_bank, prepare_table,
};
pub use selectors::{
    CanvasTarget, FieldConversionPlan, Fragment, FragmentId, FragmentKind, FragmentReplacement,
    HtmlToImageError, PreparedFragment, apply_replacements, find_canvas_targets,
    find_table_fragments, remove_rdkit_loader_scripts,
};
pub use wrapper::static_document;
