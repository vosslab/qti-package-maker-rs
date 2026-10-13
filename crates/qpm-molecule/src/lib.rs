//! Offline molecule-canvas rendering through an optional native RDKit shim.
//!
//! The workspace never links RDKit. Callers may create [`RdkitRenderer`] from
//! an explicit packaged shim location or [`RdkitRenderer::load_default`]. Missing RDKit is an
//! explicit, recoverable error.

mod error;
mod renderer;

pub use error::MoleculeError;
pub use qpm_render::{CanvasSource, MAX_CANVAS_DIMENSION};
pub use renderer::{RdkitRenderer, default_shim_paths, render_canvas_png};
