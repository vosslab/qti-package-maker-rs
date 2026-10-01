use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_uchar};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use crate::{CanvasSource, MAX_CANVAS_DIMENSION, MoleculeError};

type RenderFn = unsafe extern "C" fn(
    *const c_char,
    usize,
    *const c_char,
    usize,
    u32,
    u32,
    bool,
    *const c_int,
    usize,
    *const c_int,
    usize,
    *const f64,
    bool,
    *mut *mut c_uchar,
    *mut usize,
    *mut *mut c_char,
) -> c_int;
type FreeBytesFn = unsafe extern "C" fn(*mut c_uchar);
type FreeErrorFn = unsafe extern "C" fn(*mut c_char);
type AbiVersionFn = unsafe extern "C" fn() -> u32;

const REQUIRED_SHIM_ABI: u32 = 1;

/// A loaded native shim whose function pointers remain valid for its lifetime.
pub struct RdkitRenderer {
    // The library must outlive all copied C function pointers. It is intentionally
    // retained for every call; no pointer crosses the safe API (ASVS 1.4.1, 1.4.3).
    _library: Library,
    render: RenderFn,
    free_bytes: FreeBytesFn,
    free_error: FreeErrorFn,
}

impl RdkitRenderer {
    /// Load the release-provided native shim at an explicit trusted application path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, MoleculeError> {
        let path = path.as_ref();
        let library =
            unsafe { Library::new(path) }.map_err(|_| MoleculeError::RdkitUnavailable {
                attempts: vec![path.to_path_buf()],
            })?;
        // The ABI is restricted to POD values and owned buffers. The shim source
        // and this signature are versioned together in `native/qti_rdkit_shim.h`.
        let abi_version =
            load_symbol::<AbiVersionFn>(&library, b"qti_rdkit_shim_abi_version\0", path)?;
        let found_abi = unsafe { abi_version() };
        if found_abi != REQUIRED_SHIM_ABI {
            return Err(MoleculeError::ShimAbiMismatch { found: found_abi });
        }
        let render = load_symbol::<RenderFn>(&library, b"qti_rdkit_render_png\0", path)?;
        let free_bytes = load_symbol::<FreeBytesFn>(&library, b"qti_rdkit_free_bytes\0", path)?;
        let free_error = load_symbol::<FreeErrorFn>(&library, b"qti_rdkit_free_error\0", path)?;
        Ok(Self {
            _library: library,
            render,
            free_bytes,
            free_error,
        })
    }

    /// Load the shim specified by the application environment.
    pub fn load_from_env() -> Result<Self, MoleculeError> {
        match env::var_os("QTI_RDKIT_SHIM") {
            Some(path) => Self::load(PathBuf::from(path)),
            None => Err(MoleculeError::RdkitUnavailable {
                attempts: Vec::new(),
            }),
        }
    }

    /// Render one statically validated canvas source to its declared PNG dimensions.
    pub fn render_canvas_png(&self, source: &CanvasSource) -> Result<Vec<u8>, MoleculeError> {
        validate_source(source)?;
        let smiles = CString::new(source.smiles.as_str())
            .map_err(|_| MoleculeError::InteriorNul { field: "SMILES" })?;
        let legend_text = source.legend.as_deref().unwrap_or_default();
        let legend = CString::new(legend_text)
            .map_err(|_| MoleculeError::InteriorNul { field: "legend" })?;
        let mut png = std::ptr::null_mut();
        let mut png_length = 0_usize;
        let mut native_error = std::ptr::null_mut();
        let status = unsafe {
            (self.render)(
                smiles.as_ptr(),
                smiles.as_bytes().len(),
                legend.as_ptr(),
                legend.as_bytes().len(),
                source.width,
                source.height,
                source.explicit_methyl,
                source.highlight_atoms.as_ptr(),
                source.highlight_atoms.len(),
                source.highlight_bonds.as_ptr(),
                source.highlight_bonds.len(),
                source
                    .highlight_colour
                    .as_ref()
                    .map_or(std::ptr::null(), |colour| colour.as_ptr()),
                source.highlight_peptide_bonds,
                &mut png,
                &mut png_length,
                &mut native_error,
            )
        };
        if status != 0 {
            let message = take_error(native_error, self.free_error)
                .unwrap_or_else(|| format!("native shim returned status {status}"));
            return Err(status_error(status, message));
        }
        if !native_error.is_null() {
            unsafe { (self.free_error)(native_error) };
        }
        if png.is_null() {
            return Err(MoleculeError::InvalidPng);
        }
        // The shim allocates this exact byte range and requires its paired free
        // function. Rust copies it before release, avoiding foreign ownership.
        let copied = unsafe { std::slice::from_raw_parts(png, png_length).to_vec() };
        unsafe { (self.free_bytes)(png) };
        if png_dimensions(&copied) != Some((source.width, source.height)) {
            return Err(MoleculeError::InvalidPng);
        }
        Ok(copied)
    }
}

/// Render through the optional shim configured by `QTI_RDKIT_SHIM`.
pub fn render_canvas_png(source: &CanvasSource) -> Result<Vec<u8>, MoleculeError> {
    RdkitRenderer::load_from_env()?.render_canvas_png(source)
}

fn load_symbol<T: Copy>(library: &Library, name: &[u8], path: &Path) -> Result<T, MoleculeError> {
    let symbol: Symbol<T> =
        unsafe { library.get(name) }.map_err(|_| MoleculeError::RdkitUnavailable {
            attempts: vec![path.to_path_buf()],
        })?;
    Ok(*symbol)
}

fn validate_source(source: &CanvasSource) -> Result<(), MoleculeError> {
    if source.width == 0
        || source.height == 0
        || source.width > MAX_CANVAS_DIMENSION
        || source.height > MAX_CANVAS_DIMENSION
    {
        return Err(MoleculeError::InvalidDimensions {
            width: source.width,
            height: source.height,
        });
    }
    if source.highlight_colour.is_some_and(|colour| {
        colour
            .iter()
            .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
    }) {
        return Err(MoleculeError::InvalidHighlightColour);
    }
    Ok(())
}

fn take_error(pointer: *mut c_char, free_error: FreeErrorFn) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let message = unsafe { CStr::from_ptr(pointer).to_string_lossy().into_owned() };
    unsafe { free_error(pointer) };
    Some(message)
}

fn status_error(status: i32, message: String) -> MoleculeError {
    match status {
        3 => MoleculeError::SmilesUnparseable,
        4 => MoleculeError::AtomHighlightOutOfRange,
        5 => MoleculeError::BondHighlightOutOfRange,
        6 => MoleculeError::PeptideBondNoMatch,
        _ => MoleculeError::NativeFailure { status, message },
    }
}

fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    let header = png.get(..24)?;
    if &header[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some((
        u32::from_be_bytes(header[16..20].try_into().ok()?),
        u32::from_be_bytes(header[20..24].try_into().ok()?),
    ))
}

#[cfg(test)]
mod tests {
    use super::{RdkitRenderer, png_dimensions, status_error, validate_source};
    use crate::{CanvasSource, MAX_CANVAS_DIMENSION, MoleculeError};

    fn source(width: u32, height: u32) -> CanvasSource {
        CanvasSource {
            smiles: "CCO".to_owned(),
            legend: None,
            explicit_methyl: false,
            width,
            height,
            highlight_atoms: Vec::new(),
            highlight_bonds: Vec::new(),
            highlight_colour: None,
            highlight_peptide_bonds: false,
        }
    }

    #[test]
    fn source_dimensions_match_the_static_parser_limit() {
        assert!(validate_source(&source(MAX_CANVAS_DIMENSION, MAX_CANVAS_DIMENSION)).is_ok());
        assert_eq!(
            validate_source(&source(MAX_CANVAS_DIMENSION + 1, 1)),
            Err(MoleculeError::InvalidDimensions {
                width: MAX_CANVAS_DIMENSION + 1,
                height: 1,
            })
        );
    }

    #[test]
    fn highlight_colour_is_validated_at_the_public_renderer_boundary() {
        for colour in [
            [f64::NAN, 0.0, 0.0],
            [f64::INFINITY, 0.0, 0.0],
            [f64::NEG_INFINITY, 0.0, 0.0],
            [-0.000_001, 0.0, 0.0],
            [0.0, 1.000_001, 0.0],
        ] {
            let mut invalid = source(1, 1);
            invalid.highlight_colour = Some(colour);
            assert_eq!(
                validate_source(&invalid),
                Err(MoleculeError::InvalidHighlightColour)
            );
        }

        let mut valid = source(1, 1);
        valid.highlight_colour = Some([0.0, 0.5, 1.0]);
        assert!(validate_source(&valid).is_ok());
    }

    #[test]
    fn native_statuses_map_to_precise_public_errors() {
        assert_eq!(
            status_error(3, "parse".to_owned()),
            MoleculeError::SmilesUnparseable
        );
        assert_eq!(
            status_error(4, "atom".to_owned()),
            MoleculeError::AtomHighlightOutOfRange
        );
        assert_eq!(
            status_error(5, "bond".to_owned()),
            MoleculeError::BondHighlightOutOfRange
        );
        assert_eq!(
            status_error(6, "peptide".to_owned()),
            MoleculeError::PeptideBondNoMatch
        );
    }

    #[test]
    fn png_header_requires_a_complete_ihdr() {
        assert_eq!(png_dimensions(b"not a PNG"), None);
    }

    #[test]
    fn missing_optional_shim_has_an_actionable_error() {
        let missing = std::env::temp_dir().join("qti-molecule-test-no-rdkit-shim.dylib");
        assert!(matches!(
            RdkitRenderer::load(&missing),
            Err(MoleculeError::RdkitUnavailable { attempts }) if attempts == vec![missing]
        ));
    }
}
