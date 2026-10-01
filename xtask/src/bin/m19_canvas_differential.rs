//! Ignored development receipt for pinned Python static canvas-parser parity.

use qti_engines::html_to_image::parse_canvas_script;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs};

#[derive(Deserialize)]
struct Input {
    pin: String,
    oracle_snapshot_path: String,
    selectors_sha256: String,
    files: Vec<String>,
    manifest_sha256: String,
    file_sha256: std::collections::BTreeMap<String, String>,
    real: Vec<Case>,
    hostile: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: Option<String>,
    script: String,
    width: Option<String>,
    height: Option<String>,
    python: Outcome,
}
#[derive(Deserialize)]
struct Outcome {
    accepted: bool,
    source: Option<Value>,
}
#[derive(Serialize)]
struct Receipt {
    pin: String,
    oracle_snapshot_path: String,
    selectors_sha256: String,
    input_sha256: String,
    manifest_path: String,
    manifest_sha256: String,
    verified_file_sha256: std::collections::BTreeMap<String, String>,
    file_count: usize,
    real_count: usize,
    hostile_count: usize,
    disagreements: Vec<String>,
    pass: bool,
}

fn rust_outcome(case: &Case) -> Outcome {
    match parse_canvas_script(&case.script, case.width.as_deref(), case.height.as_deref()) {
        Ok(source) => Outcome {
            accepted: true,
            source: Some(
                json!({"smiles":source.smiles,"legend":source.legend,"explicit_methyl":source.explicit_methyl,"width":source.width,"height":source.height,"highlight_atoms":source.highlight_atoms,"highlight_bonds":source.highlight_bonds,"highlight_peptide_bonds":source.highlight_peptide_bonds,"highlight_colour":source.highlight_colour}),
            ),
        },
        Err(_) => Outcome {
            accepted: false,
            source: None,
        },
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verify_corpus(source: &Input, manifest_path: &str) -> Result<(), String> {
    let manifest_bytes = fs::read(manifest_path).map_err(|error| error.to_string())?;
    if sha256_hex(&manifest_bytes) != source.manifest_sha256 {
        return Err("manifest SHA-256 does not match the Python fixture".to_owned());
    }
    let manifest: Value =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    let files = manifest
        .get("bbq_files")
        .and_then(Value::as_array)
        .ok_or("manifest has no bbq_files array")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("manifest bbq_files value is not a string")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if files != source.files || files.len() != source.file_sha256.len() {
        return Err("manifest paths do not exactly match the Python fixture".to_owned());
    }
    for path in &files {
        let expected = source
            .file_sha256
            .get(path)
            .ok_or("fixture lacks a manifest file hash")?;
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        if sha256_hex(&bytes) != *expected {
            return Err(format!(
                "input SHA-256 differs from the Python fixture: {path}"
            ));
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .ok_or("usage: m19-canvas-differential INPUT_JSON MANIFEST_JSON RECEIPT_JSON")?;
    let manifest_path = args
        .next()
        .ok_or("usage: m19-canvas-differential INPUT_JSON MANIFEST_JSON RECEIPT_JSON")?;
    let output = args
        .next()
        .ok_or("usage: m19-canvas-differential INPUT_JSON MANIFEST_JSON RECEIPT_JSON")?;
    let input_bytes = fs::read(input).map_err(|e| e.to_string())?;
    let input_sha256 = sha256_hex(&input_bytes);
    let source: Input = serde_json::from_slice(&input_bytes).map_err(|e| e.to_string())?;
    verify_corpus(&source, &manifest_path)?;
    let mut disagreements = Vec::new();
    if source.real.is_empty() || source.real.iter().any(|case| !case.python.accepted) {
        return Err("real corpus must be nonempty and accepted by pinned Python".to_owned());
    }
    for case in &source.hostile {
        let accepted_control = matches!(
            case.name.as_deref(),
            Some("maximum_dimension") | Some("receiver_agnostic")
        );
        if case.python.accepted != accepted_control {
            return Err(format!(
                "hostile case {} has an unexpected pinned-Python outcome",
                case.name.as_deref().unwrap_or("unnamed")
            ));
        }
    }
    for (kind, cases) in [("real", &source.real), ("hostile", &source.hostile)] {
        for (index, case) in cases.iter().enumerate() {
            let rust = rust_outcome(case);
            if rust.accepted != case.python.accepted
                || (rust.accepted && rust.source != case.python.source)
            {
                disagreements.push(format!(
                    "{kind}[{index}] {}",
                    case.name.as_deref().unwrap_or("CanvasSource")
                ));
            }
        }
    }
    let file_count = source.file_sha256.len();
    let receipt = Receipt {
        pin: source.pin,
        oracle_snapshot_path: source.oracle_snapshot_path,
        selectors_sha256: source.selectors_sha256,
        input_sha256,
        manifest_sha256: source.manifest_sha256,
        manifest_path,
        verified_file_sha256: source.file_sha256,
        file_count,
        real_count: source.real.len(),
        hostile_count: source.hostile.len(),
        pass: disagreements.is_empty(),
        disagreements,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if receipt.pass {
        Ok(())
    } else {
        Err(format!(
            "{} canvas parser disagreement(s)",
            receipt.disagreements.len()
        ))
    }
}
