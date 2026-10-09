//! Typed, owned browser transport over the shared conversion and integrity engines.

mod adapter;
mod boundary;
mod diagnostics;
mod transport;

pub use adapter::{check_package_request, convert_request, format_inventory};
pub use boundary::{check_package, convert, formats};
pub use transport::{
    Artifact, CheckPackageResult, ConversionInput, ConvertRequest, ConvertResult, Diagnostic,
    DocumentOptions, FormatInfo, FormatInventory, IntegrityFinding, IntegrityReport, NamedBytes,
    PackageInput, Warning,
};
