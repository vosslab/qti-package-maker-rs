use std::path::PathBuf;

use thiserror::Error;

/// Errors at the safe Rust boundary of the optional RDKit renderer.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoleculeError {
    #[error("RDKit canvas dimensions must be 1 through 4096 pixels; got {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
    #[error("RDKit highlight RGB components must be finite values in the closed range 0 through 1")]
    InvalidHighlightColour,
    #[error("Canvas {field} contains an interior NUL byte")]
    InteriorNul { field: &'static str },
    #[error(
        "RDKit canvas rendering is unavailable. Install the packaged qti RDKit shim and set QTI_RDKIT_SHIM; attempted: {attempts:?}"
    )]
    RdkitUnavailable { attempts: Vec<PathBuf> },
    #[error("RDKit shim ABI v1 is required, but the loaded shim reports v{found}")]
    ShimAbiMismatch { found: u32 },
    #[error("RDKit could not parse SMILES")]
    SmilesUnparseable,
    #[error("RDKit atom highlight index is outside the molecule")]
    AtomHighlightOutOfRange,
    #[error("RDKit bond highlight index is outside the molecule")]
    BondHighlightOutOfRange,
    #[error("RDKit peptide canvas contains no matching peptide bonds")]
    PeptideBondNoMatch,
    #[error("RDKit native renderer failed with status {status}: {message}")]
    NativeFailure { status: i32, message: String },
    #[error("RDKit native renderer returned an invalid PNG")]
    InvalidPng,
}
