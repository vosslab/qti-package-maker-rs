//! Owned render transport; original bank bindings remain inside the portable core.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{Diagnostic, Warning};

/// Static molecule drawing options recovered by the shared parser.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSpec {
    pub drawing_details: DrawingDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peptide_query: Option<PeptideQuery>,
    pub smiles: String,
    pub legend: Option<String>,
    pub explicit_methyl: bool,
    pub width: u32,
    pub height: u32,
    pub highlight_atoms: Vec<i32>,
    pub highlight_bonds: Vec<i32>,
    pub highlight_colour: Option<[f64; 3]>,
    pub highlight_peptide_bonds: bool,
}

impl From<qti_render::CanvasSource> for CanvasSpec {
    fn from(source: qti_render::CanvasSource) -> Self {
        let drawing_details = serde_json::from_value(source.drawing_details())
            .expect("portable drawing details match the typed host contract");
        let peptide_query = source.peptide_query().map(|query| PeptideQuery {
            smarts: query.smarts.into(),
            bond_atoms: query.bond_atoms,
        });
        Self {
            drawing_details,
            peptide_query,
            smiles: source.smiles,
            legend: source.legend,
            explicit_methyl: source.explicit_methyl,
            width: source.width,
            height: source.height,
            highlight_atoms: source.highlight_atoms,
            highlight_bonds: source.highlight_bonds,
            highlight_colour: source.highlight_colour,
            highlight_peptide_bonds: source.highlight_peptide_bonds,
        }
    }
}

/// Canonical RDKit drawing options serialized as a JavaScript object.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingDetails {
    pub explicit_methyl: bool,
    pub atoms: Vec<i32>,
    pub bonds: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_colour: Option<[f64; 3]>,
}

/// Source-owned chemical query metadata for generic RDKit execution.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PeptideQuery {
    pub smarts: String,
    pub bond_atoms: [usize; 2],
}

/// The host-visible job omits private bindings to original question fields.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct RenderJob {
    pub id: String,
    #[tsify(type = "'table' | 'canvas'")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas_spec: Option<CanvasSpec>,
    pub content_hash: String,
    pub dependencies: Vec<String>,
}

/// A host-owned PNG with finite positive logical CSS dimensions.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(deny_unknown_fields)]
pub struct RenderCompletion {
    pub id: String,
    #[serde(with = "serde_bytes")]
    #[tsify(type = "Uint8Array")]
    pub png: Vec<u8>,
    pub width: f64,
    pub height: f64,
}

/// Expected planning failures use the same diagnostics as conversion.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RenderPlanResult {
    Success {
        jobs: Vec<RenderJob>,
        wrapper: String,
        item_count: usize,
        warnings: Vec<Warning>,
    },
    Error {
        error: Diagnostic,
        warnings: Vec<Warning>,
    },
}

/// Transparent completion array for the generated wasm-bindgen ABI.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct RenderCompletions(pub Vec<RenderCompletion>);
