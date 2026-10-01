//! Package integrity accounting for the differential parity harness.

use super::{Divergence, ZIP_ENGINES, display_error, file_sha256, parity_grading_program};
use qti_integrity::{Severity, check_package};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct SourceBinding {
    path: std::path::PathBuf,
    sha256: String,
}

#[derive(Deserialize)]
struct ReferenceRepair {
    response: String,
    answer: String,
    identifier: String,
}

#[derive(Deserialize)]
struct CanvasRepairReceipt {
    source: SourceBinding,
    raw_zip_sha256: String,
    items: usize,
    repairs: Vec<ReferenceRepair>,
}

fn verified_canvas_repair(
    input: &Path,
    directory: &Path,
    errors: &[String],
) -> Result<bool, String> {
    let path = directory.join("canvas_multifib_source_repair_receipt.json");
    if !path.is_file() {
        return Ok(false);
    }
    let receipt: CanvasRepairReceipt =
        serde_json::from_slice(&fs::read(path).map_err(display_error)?).map_err(display_error)?;
    let expected = "qti12-dangling-varequal:canvas_qti12_questions/canvas_qti12_questions.xml";
    Ok(receipt.source.path == input
        && receipt.source.sha256 == file_sha256(input)?
        && receipt.raw_zip_sha256 == file_sha256(&directory.join("canvas_qti_v1_2.zip"))?
        && receipt.items > 0
        && !receipt.repairs.is_empty()
        && receipt.repairs.len() == errors.len()
        && errors.iter().all(|error| error == expected)
        && receipt.repairs.iter().all(|repair| {
            !repair.response.is_empty()
                && !repair.answer.is_empty()
                && !repair.identifier.is_empty()
        }))
}

fn zip_integrity_errors(directory: &Path, engines: &[&str]) -> BTreeMap<&'static str, Vec<String>> {
    let mut results = BTreeMap::new();
    for engine in ZIP_ENGINES
        .iter()
        .copied()
        .filter(|engine| engines.contains(engine))
    {
        let path = directory.join(format!("{engine}.zip"));
        let errors = check_package(&path)
            .into_iter()
            .filter(|finding| finding.severity == Severity::Error)
            .map(|finding| format!("{}:{}", finding.code, finding.path))
            .collect::<Vec<_>>();
        results.insert(engine, errors);
    }
    results
}
/// Records a pinned Python Canvas MATCH package defect while requiring Rust's repair.
pub(super) fn compare_zip_integrity(
    input: &Path,
    python_directory: &Path,
    rust_directory: &Path,
    engines: &[&str],
) -> Result<Vec<Divergence>, String> {
    let python = zip_integrity_errors(python_directory, engines);
    let rust = zip_integrity_errors(rust_directory, engines);
    let canvas_multifib_fixture = parity_grading_program::is_canvas_multifib_repair_input(input);
    let expected_canvas_failure = [
        "qti12-dangling-varequal:canvas_qti12_questions/canvas_qti12_questions.xml",
        "qti12-dangling-varequal:canvas_qti12_questions/canvas_qti12_questions.xml",
    ];
    let mut differences = Vec::new();
    for engine in ZIP_ENGINES
        .iter()
        .copied()
        .filter(|engine| engines.contains(engine))
    {
        let python_errors = python.get(engine).expect("selected ZIP engine is present");
        let rust_errors = rust.get(engine).expect("selected ZIP engine is present");
        if engine == "canvas_qti_v1_2"
            && rust_errors.is_empty()
            && verified_canvas_repair(input, python_directory, python_errors)?
        {
            continue;
        } else if canvas_multifib_fixture && engine == "canvas_qti_v1_2" {
            if python_errors.as_slice() != expected_canvas_failure || !rust_errors.is_empty() {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "MULTI_FIB integrity repair".to_owned(),
                    field: "pinned Python known failure and Rust repair".to_owned(),
                    python: python_errors.join(", "),
                    rust: rust_errors.join(", "),
                });
            }
        } else if !python_errors.is_empty() || !rust_errors.is_empty() {
            differences.push(Divergence {
                engine: engine.to_owned(),
                item: "package".to_owned(),
                field: "qti-integrity".to_owned(),
                python: python_errors.join(", "),
                rust: rust_errors.join(", "),
            });
        }
    }
    Ok(differences)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_receipt_binds_source_archive_and_exact_error_count() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("qti-canvas-receipt-{unique}"));
        fs::create_dir(&directory).unwrap();
        let input = directory.join("source.txt");
        fs::write(&input, "authored input").unwrap();
        let archive = directory.join("canvas_qti_v1_2.zip");
        fs::write(&archive, "archive bytes").unwrap();
        let path = directory.join("canvas_multifib_source_repair_receipt.json");
        let receipt = serde_json::json!({
            "source": {"path": input, "sha256": file_sha256(&input).unwrap()},
            "raw_zip_sha256": file_sha256(&archive).unwrap(), "items": 1,
            "repairs": [{"response": "response_1", "answer": "37", "identifier": "choice_001"}]
        });
        fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        let errors = vec![
            "qti12-dangling-varequal:canvas_qti12_questions/canvas_qti12_questions.xml".to_owned(),
        ];
        assert!(verified_canvas_repair(&input, &directory, &errors).unwrap());
        assert!(!verified_canvas_repair(&input, &directory, &[]).unwrap());
        assert!(!verified_canvas_repair(&input, &directory, &["unexpected".to_owned()]).unwrap());
        fs::write(&archive, "changed archive").unwrap();
        assert!(!verified_canvas_repair(&input, &directory, &errors).unwrap());
        fs::write(&archive, "archive bytes").unwrap();
        fs::write(&input, "changed source").unwrap();
        assert!(!verified_canvas_repair(&input, &directory, &errors).unwrap());
        fs::remove_dir_all(directory).unwrap();
    }
}
