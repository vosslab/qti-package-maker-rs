//! Current-Python ORDER and media receipt comparisons.

use super::parity_process::{display_error, python_command};
use super::parity_writer_receipts::WriterReceipt;
use super::{Divergence, ENGINES, OrderOutcome, OrderReceipt};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Compares current Python and Rust ORDER-only writer behavior.
pub(super) fn compare_order_receipt(
    repository: &Path,
    python_root: &Path,
    cli: &Path,
    temporary: &Path,
) -> Result<Vec<Divergence>, String> {
    let fixture = temporary
        .parent()
        .ok_or_else(|| "parity temporary directory has no parent".to_owned())?
        .join(format!("parity_fixtures_{}", std::process::id()))
        .join("bbq-parity-order-only-questions.txt");
    let python_dir = temporary.join("order_python");
    let rust_dir = temporary.join("order_rust");
    fs::create_dir_all(&python_dir).map_err(display_error)?;
    fs::create_dir_all(&rust_dir).map_err(display_error)?;
    let mut command = python_command(repository, python_root, "order-receipt")?;
    command.args([
        "--input",
        &fixture.to_string_lossy(),
        "--output",
        &python_dir.to_string_lossy(),
    ]);
    let output = command.output().map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "ORDER receipt oracle failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let python: OrderReceipt = serde_json::from_slice(&output.stdout).map_err(display_error)?;
    let mut rust = BTreeMap::new();
    for engine in ["canvas_qti_v1_2", "blackboard_export_zip"] {
        let prefix = if engine == "canvas_qti_v1_2" {
            "qti12"
        } else {
            "bez"
        };
        let destination = rust_dir.join(format!("{prefix}-parity-order-only.zip"));
        let output = Command::new(cli)
            .args(["-i", &fixture.to_string_lossy(), "-f", engine, "-q"])
            .current_dir(&rust_dir)
            .output()
            .map_err(display_error)?;
        rust.insert(
            engine,
            OrderOutcome {
                exit: output.status.code().unwrap_or(-1),
                file: destination.is_file(),
                diagnostic: format!(
                    "{} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            },
        );
    }
    let mut differences = Vec::new();
    for (engine, python) in [
        ("canvas_qti_v1_2", python.canvas_qti_v1_2),
        ("blackboard_export_zip", python.blackboard_export_zip),
    ] {
        let rust = rust.get(engine).expect("both fixed engines are recorded");
        if python.exit != rust.exit || python.file != rust.file {
            differences.push(Divergence {
                engine: engine.to_owned(),
                item: "ORDER-only".to_owned(),
                field: "writer outcome and phantom-file check".to_owned(),
                python: format!("exit={} file={}", python.exit, python.file),
                rust: format!("exit={} file={}", rust.exit, rust.file),
            });
        }
        if engine == "blackboard_export_zip"
            && (!python.diagnostic.contains("ORDER") || !rust.diagnostic.contains("ORDER"))
        {
            differences.push(Divergence {
                engine: engine.to_owned(),
                item: "ORDER-only".to_owned(),
                field: "unsupported-kind diagnostic".to_owned(),
                python: python.diagnostic,
                rust: rust.diagnostic.clone(),
            });
        }
    }
    Ok(differences)
}
/// Compares current Python and Rust media-writer outcomes for each fixture.
pub(super) fn compare_media_receipts(
    repository: &Path,
    python_root: &Path,
    cli: &Path,
    fixtures: &Path,
    temporary: &Path,
) -> Result<Vec<Divergence>, String> {
    let fixture_names = [
        "bbq-parity-media-spaces-questions.txt",
        "bbq-parity-media-nonpackageable-questions.txt",
    ];
    let mut differences = Vec::new();
    for name in fixture_names {
        let input = fixtures.join(name);
        let python_directory = temporary.join(format!("media_python_{}", name));
        let rust_directory = temporary.join(format!("media_rust_{}", name));
        fs::create_dir_all(&python_directory).map_err(display_error)?;
        fs::create_dir_all(&rust_directory).map_err(display_error)?;
        let mut command = python_command(repository, python_root, "write")?;
        command.args([
            "--input",
            &input.to_string_lossy(),
            "--output",
            &python_directory.to_string_lossy(),
            "--engines",
            &ENGINES.join(","),
        ]);
        let output = command.output().map_err(display_error)?;
        if !output.status.success() {
            return Err(format!(
                "media receipt oracle failed for {name}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let python: WriterReceipt =
            serde_json::from_slice(&output.stdout).map_err(display_error)?;
        for engine in ENGINES {
            let extension = output_extension(engine);
            let destination = rust_directory.join(format!("{engine}.{extension}"));
            let output = Command::new(cli)
                .args([
                    "-i",
                    &input.to_string_lossy(),
                    "-f",
                    engine,
                    "--allow-mixed",
                    "-o",
                    &destination.to_string_lossy(),
                ])
                .output()
                .map_err(display_error)?;
            let python_failed = python.errors.contains_key(*engine);
            let rust_failed = !output.status.success() || !destination.is_file();
            if python_failed != rust_failed {
                differences.push(Divergence {
                    engine: (*engine).to_owned(),
                    item: name.to_owned(),
                    field: "media writer outcome receipt".to_owned(),
                    python: if python_failed { "failure" } else { "output" }.to_owned(),
                    rust: if rust_failed { "failure" } else { "output" }.to_owned(),
                });
            }
        }
        if !python.outputs.is_empty() && python.outputs.len() + python.errors.len() != ENGINES.len()
        {
            return Err(format!("media receipt omitted an engine for {name}"));
        }
    }
    Ok(differences)
}

fn output_extension(engine: &str) -> &'static str {
    match engine {
        "canvas_qti_v1_2" | "blackboard_qti_v2_1" | "blackboard_export_zip" => "zip",
        "exam_yaml" => "yaml",
        "human_readable" | "html_selftest" => "html",
        _ => "txt",
    }
}
