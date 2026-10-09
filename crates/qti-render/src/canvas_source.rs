use serde::{Deserialize, Serialize};

/// The maximum accepted canvas edge in pixels, matching the static script grammar.
pub const MAX_CANVAS_DIMENSION: u32 = 4096;

/// Static molecule-drawing options extracted from an allowed RDKit canvas script.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSource {
    /// SMILES passed to RDKit's molecule parser.
    pub smiles: String,
    /// Optional legend placed below the depiction.
    pub legend: Option<String>,
    /// Whether terminal methyl groups are drawn explicitly as `CH3`.
    pub explicit_methyl: bool,
    /// Requested PNG width in pixels.
    pub width: u32,
    /// Requested PNG height in pixels.
    pub height: u32,
    /// Atom indices requested for highlighting.
    pub highlight_atoms: Vec<i32>,
    /// Bond indices requested for highlighting.
    pub highlight_bonds: Vec<i32>,
    /// Optional RGB highlight color, with each component in the closed range 0..=1.
    pub highlight_colour: Option<[f64; 3]>,
    /// Whether bonds matching the fixed peptide SMARTS are highlighted.
    pub highlight_peptide_bonds: bool,
}

/// Source-owned query describing which matching atom pair forms a peptide bond.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeptideQuery {
    /// Chemical rule used by both the native shim and the browser RDKit adapter.
    pub smarts: &'static str,
    /// Query atom indices whose matched molecule atoms identify the highlighted bond.
    pub bond_atoms: [usize; 2],
}

impl CanvasSource {
    /// Returns RDKit drawing options without interpreting authored JavaScript in the host.
    #[must_use]
    pub fn drawing_details(&self) -> serde_json::Value {
        let mut details = serde_json::json!({
            "explicitMethyl": self.explicit_methyl,
            "atoms": self.highlight_atoms,
            "bonds": self.highlight_bonds,
        });
        if let Some(legend) = &self.legend {
            details["legend"] = serde_json::json!(legend);
        }
        if let Some(colour) = self.highlight_colour {
            details["highlightColour"] = serde_json::json!(colour);
        }
        details
    }

    /// Returns the fixed peptide rule for the host's generic RDKit query execution.
    #[must_use]
    pub fn peptide_query(&self) -> Option<PeptideQuery> {
        self.highlight_peptide_bonds.then_some(PeptideQuery {
            smarts: "CC(=O)NC",
            bond_atoms: [1, 3],
        })
    }
}
