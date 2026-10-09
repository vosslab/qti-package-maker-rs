//! Typed, owned browser transport over the shared conversion and integrity engines.

mod adapter;
mod boundary;
mod diagnostics;
mod render_adapter;
mod render_transport;
mod transport;

pub use adapter::{check_package_request, convert_request, format_inventory};
pub use boundary::{check_package, convert, finish_convert, formats, plan_render_jobs};
pub use transport::{
    Artifact, CheckPackageResult, ConversionInput, ConvertRequest, ConvertResult, Diagnostic,
    DocumentOptions, FormatInfo, FormatInventory, IntegrityFinding, IntegrityReport, NamedBytes,
    PackageInput, Warning,
};

pub use render_adapter::{finish_convert_request, plan_render_jobs_request};
pub use render_transport::{
    CanvasSpec, DrawingDetails, PeptideQuery, RenderCompletion, RenderCompletions, RenderJob,
    RenderPlanResult,
};
