use std::fs;
use std::path::PathBuf;

use qti_molecule::{CanvasSource, render_canvas_png};
use serde::Deserialize;

#[derive(Deserialize)]
struct CorpusSource {
    smiles: String,
    legend: Option<String>,
    explicit_methyl: bool,
    width: u32,
    height: u32,
    highlight_atoms: Vec<i32>,
    highlight_bonds: Vec<i32>,
    highlight_colour: Option<[f64; 3]>,
    highlight_peptide_bonds: bool,
}

impl From<CorpusSource> for CanvasSource {
    fn from(value: CorpusSource) -> Self {
        Self {
            smiles: value.smiles,
            legend: value.legend,
            explicit_methyl: value.explicit_methyl,
            width: value.width,
            height: value.height,
            highlight_atoms: value.highlight_atoms,
            highlight_bonds: value.highlight_bonds,
            highlight_colour: value.highlight_colour,
            highlight_peptide_bonds: value.highlight_peptide_bonds,
        }
    }
}

#[test]
#[ignore = "requires QTI_RDKIT_SHIM and QTI_CANVAS_CORPUS for native corpus proof"]
fn native_renderer_renders_every_harvested_canvas() {
    let directory = PathBuf::from(std::env::var_os("QTI_CANVAS_CORPUS").expect("corpus path"));
    let mut paths = fs::read_dir(directory)
        .expect("read corpus directory")
        .map(|entry| entry.expect("corpus entry").path())
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "corpus must contain at least one CanvasSource"
    );
    for path in &paths {
        let source: CanvasSource = serde_json::from_slice::<CorpusSource>(
            &fs::read(path).expect("read CanvasSource record"),
        )
        .expect("parse CanvasSource record")
        .into();
        let png = render_canvas_png(&source).expect("native renderer must accept corpus source");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(
            u32::from_be_bytes(png[16..20].try_into().unwrap()),
            source.width
        );
        assert_eq!(
            u32::from_be_bytes(png[20..24].try_into().unwrap()),
            source.height
        );
    }
    assert_eq!(
        paths.len(),
        58,
        "current M18 corpus size changed; refresh decision evidence"
    );
}
