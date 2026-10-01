//! Offline molecule-canvas rendering through an optional native RDKit shim.
//!
//! The workspace never links RDKit. Callers may create [`RdkitRenderer`] from
//! their packaged shim location, or use [`render_canvas_png`] with
//! `QTI_RDKIT_SHIM` set. Missing RDKit is an explicit, recoverable error.

mod error;
mod renderer;
mod source;

pub use error::MoleculeError;
pub use renderer::{RdkitRenderer, render_canvas_png};
pub use source::{CanvasSource, MAX_CANVAS_DIMENSION};
