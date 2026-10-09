//! Current Python migration oracle and current native CLI invocation.

use super::parity_writer_receipts::WriterReceipt;
use super::{Comparison, ENGINES, ZIP_ENGINES};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn run_python_writer(
    repository: &Path,
    python_root: &Path,
    input: &Path,
    output: &Path,
    html_to_image: bool,
    engines: &[&str],
) -> Result<WriterReceipt, String> {
    let mut command = python_command(repository, python_root, "write")?;
    command.args([
        "--input",
        &input.to_string_lossy(),
        "--output",
        &output.to_string_lossy(),
    ]);
    if html_to_image {
        command.args(["--html-to-image", "--zip-only"]);
    }
    command.args(["--engines", &engines.join(",")]);
    let result = command.output().map_err(display_error)?;
    if !result.status.success() {
        return Err(format!(
            "current Python writer failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    serde_json::from_slice(&result.stdout).map_err(|error| {
        format!(
            "current Python writer returned invalid JSON: {error}; output: {}",
            String::from_utf8_lossy(&result.stdout)
        )
    })
}

pub(super) fn run_native_writers(
    cli: &Path,
    input: &Path,
    output: &Path,
    html_to_image: bool,
    engines: &[&str],
) -> Result<WriterReceipt, String> {
    let mut receipt = WriterReceipt::default();
    for engine in ENGINES {
        if !engines.contains(engine) {
            continue;
        }
        if html_to_image && !ZIP_ENGINES.contains(engine) {
            continue;
        }
        let extension = if matches!(
            *engine,
            "canvas_qti_v1_2" | "blackboard_qti_v2_1" | "blackboard_export_zip"
        ) {
            "zip"
        } else if *engine == "exam_yaml" {
            "yaml"
        } else if matches!(*engine, "human_readable" | "html_selftest") {
            "html"
        } else {
            "txt"
        };
        let destination = output.join(format!("{engine}.{extension}"));
        let mut command = Command::new(cli);
        command
            .args([
                "-i",
                &input.to_string_lossy(),
                "-f",
                engine,
                "--allow-mixed",
                "-o",
                &destination.to_string_lossy(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if html_to_image
            && matches!(
                *engine,
                "canvas_qti_v1_2" | "blackboard_qti_v2_1" | "blackboard_export_zip"
            )
        {
            command.arg("--html-to-image");
        }
        let result = command.output().map_err(display_error)?;
        if result.status.success() && destination.is_file() {
            receipt.outputs.insert(
                (*engine).to_owned(),
                destination.to_string_lossy().into_owned(),
            );
        } else if result.status.success() {
            receipt.no_output.insert(
                (*engine).to_owned(),
                "writer completed successfully without an output artifact".to_owned(),
            );
        } else {
            let detail = format!(
                "exit {}; stdout: {}; stderr: {}",
                result.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&result.stdout).trim(),
                String::from_utf8_lossy(&result.stderr).trim(),
            );
            receipt.errors.insert((*engine).to_owned(), detail);
        }
    }
    Ok(receipt)
}

pub(super) fn compare_outputs(
    repository: &Path,
    python_root: &Path,
    python_output: &Path,
    rust_output: &Path,
    html_to_image: bool,
    engines: &[&str],
) -> Result<Comparison, String> {
    let mut command = python_command(repository, python_root, "compare")?;
    command.args([
        "--python-output",
        &python_output.to_string_lossy(),
        "--rust-output",
        &rust_output.to_string_lossy(),
    ]);
    if html_to_image {
        command.args(["--html-to-image", "--zip-only"]);
    }
    command.args(["--engines", &engines.join(",")]);
    let output = command.output().map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "parity comparator failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "parity comparator returned invalid JSON: {error}; output: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

pub(super) fn python_command(
    repository: &Path,
    python_root: &Path,
    mode: &str,
) -> Result<Command, String> {
    let script = repository.join("xtask/support/parity_oracle.py");
    if !script.is_file() {
        return Err(format!(
            "parity support script is unavailable: {}",
            script.display()
        ));
    }
    let mut command = Command::new("python3");
    command
        .arg(script)
        .arg(mode)
        .args(["--oracle-root", &python_root.to_string_lossy()])
        .env("PYTHONPATH", repository)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(command)
}

pub(super) fn native_cli(repository: &Path) -> Result<PathBuf, String> {
    if std::env::var_os("QTI_PARITY_CLI").is_some() {
        return Err(
            "QTI_PARITY_CLI is not accepted by cargo xtask parity; it must build and exercise the current workspace qti-cli binary".to_owned(),
        );
    }
    let mut build = Command::new("cargo");
    build
        .current_dir(repository)
        .args(["build", "-p", "qti-cli", "--bin", "bbq-converter"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_command(build, "native CLI build")?;
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| absolute_path(&path, repository))
        .unwrap_or_else(|| repository.join("target"));
    let path = target.join("debug/bbq-converter");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "native CLI build did not produce expected binary: {}",
            path.display()
        ))
    }
}

pub(super) fn file_sha256(path: &Path) -> Result<String, String> {
    let content = fs::read(path).map_err(display_error)?;
    Ok(Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask parity must run inside the Rust workspace".to_owned());
    }
    String::from_utf8(output.stdout)
        .map_err(display_error)
        .map(|path| PathBuf::from(path.trim()))
}

pub(super) fn absolute_path(path: &Path, repository: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    }
}

pub(super) fn run_command(mut command: Command, label: &str) -> Result<(), String> {
    let output = command.output().map_err(display_error)?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{label} failed with {}:\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim(),
    ))
}

pub(super) fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
