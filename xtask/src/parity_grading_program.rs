//! Embedded proof receipt for the bounded QTI grading-program certificate.

use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

#[derive(Deserialize)]
struct Receipt {
    status: String,
    responses: usize,
    labels_per_response: usize,
    elapsed_ms: f64,
}

/// Runs the oracle's 5-by-5 mutation proof before each M13 parity comparison.
pub(super) fn verify(repository: &Path) -> Result<String, String> {
    let script = repository.join("xtask/support/parity_oracle.py");
    let output = Command::new("python3")
        .arg(script)
        .arg("selftest")
        .env("PYTHONPATH", repository)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("grading-program selftest could not start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "grading-program selftest failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let receipt: Receipt = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "grading-program selftest returned invalid JSON: {error}; output: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        )
    })?;
    if receipt.status != "passed" || receipt.responses != 5 || receipt.labels_per_response != 5 {
        return Err(
            "grading-program selftest did not prove the required 5-by-5 receipt".to_owned(),
        );
    }
    Ok(format!(
        "parity grading-program proof: 5 responses x 5 labels in {} ms",
        receipt.elapsed_ms
    ))
}
