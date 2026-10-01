use qti_molecule::{CanvasSource, MoleculeError, render_canvas_png};

fn source() -> CanvasSource {
    CanvasSource {
        smiles: "CC(=O)NCC(=O)O".to_owned(),
        legend: Some("gly-gly".to_owned()),
        explicit_methyl: true,
        width: 240,
        height: 256,
        highlight_atoms: vec![0],
        highlight_bonds: Vec::new(),
        highlight_colour: Some([0.0, 1.0, 0.0]),
        highlight_peptide_bonds: true,
    }
}

#[test]
#[ignore = "requires QTI_RDKIT_SHIM built for the current target"]
fn native_shim_renders_all_supported_canvas_options() {
    let png =
        render_canvas_png(&source()).expect("configured RDKit shim must render peptide canvas");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 240);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 256);
}

#[test]
#[ignore = "requires QTI_RDKIT_SHIM built for the current target"]
fn native_shim_preserves_precise_invalid_index_error() {
    let mut invalid = source();
    invalid.highlight_atoms = vec![-1];
    assert_eq!(
        render_canvas_png(&invalid),
        Err(MoleculeError::AtomHighlightOutOfRange)
    );
}

#[test]
#[ignore = "requires QTI_RDKIT_SHIM built for the current target"]
fn native_shim_preserves_all_remaining_precise_errors() {
    let mut unparseable = source();
    unparseable.smiles = "not a SMILES".to_owned();
    assert_eq!(
        render_canvas_png(&unparseable),
        Err(MoleculeError::SmilesUnparseable)
    );

    let mut invalid_bond = source();
    invalid_bond.highlight_bonds = vec![-1];
    assert_eq!(
        render_canvas_png(&invalid_bond),
        Err(MoleculeError::BondHighlightOutOfRange)
    );

    let mut no_peptide_match = source();
    no_peptide_match.smiles = "CCO".to_owned();
    no_peptide_match.highlight_peptide_bonds = true;
    assert_eq!(
        render_canvas_png(&no_peptide_match),
        Err(MoleculeError::PeptideBondNoMatch)
    );
}
