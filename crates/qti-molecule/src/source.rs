/// The maximum accepted canvas edge in pixels, matching the static script grammar.
pub const MAX_CANVAS_DIMENSION: u32 = 4096;

/// Static molecule-drawing options extracted from an allowed RDKit canvas script.
#[derive(Clone, Debug, PartialEq)]
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
