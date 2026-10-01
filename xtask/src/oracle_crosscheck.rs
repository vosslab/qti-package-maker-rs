//! Cross-check the Rust integrity oracle against the pinned Python implementation.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use qti_integrity::{Severity, check_package};
use serde::Deserialize;

const PINNED_PYTHON_HEAD: &str = "55e5f368777f7809fe2e91b5d070caf6df0cb581";

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct CanonicalViolation {
    code: String,
    path: String,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct OracleRecord {
    name: String,
    path: PathBuf,
    python: Vec<CanonicalViolation>,
}

#[derive(Debug, Deserialize)]
struct ProducerCoverage {
    engine: String,
    kinds: String,
    unsupported: String,
}

#[derive(Debug, Deserialize)]
struct OracleOutput {
    producer_coverage: Vec<ProducerCoverage>,
    records: Vec<OracleRecord>,
}

/// Cross-validate finished packages and independently constructed corruptions.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    let python_qti = parse_arguments(arguments, &repository)?;
    let python_snapshot = prepare_python_snapshot(&repository, &python_qti)?;
    let temporary = repository
        .join("tests")
        .join("_temp")
        .join(format!("oracle_crosscheck_{}", std::process::id()));
    fs::create_dir_all(&temporary).map_err(display_error)?;
    let real_packages = collect_real_packages(&repository, &python_snapshot)?;
    let output = run_python_oracle(&repository, &python_snapshot, &temporary, &real_packages)?;
    for coverage in &output.producer_coverage {
        let suffix = if coverage.unsupported.is_empty() {
            String::new()
        } else {
            format!("; declared unsupported: {}", coverage.unsupported)
        };
        println!(
            "producer coverage {}: {}{suffix}",
            coverage.engine, coverage.kinds
        );
    }

    let mut divergences = Vec::new();
    for record in &output.records {
        let mut python = record.python.clone();
        let mut rust = check_package(&record.path)
            .into_iter()
            .map(|violation| CanonicalViolation {
                code: violation.code.to_owned(),
                path: violation.path,
                severity: severity_name(violation.severity).to_owned(),
            })
            .collect::<Vec<_>>();
        python.sort();
        rust.sort();
        if python != rust {
            let extensions = rust
                .iter()
                .filter(|violation| !python.contains(*violation))
                .filter(|violation| is_rust_extension(&violation.code))
                .cloned()
                .collect::<Vec<_>>();
            divergences.push(format!(
                "{} ({})\n  Python: {:?}\n  Rust: {:?}\n  Rust extension findings (still divergences): {:?}",
                record.name,
                record.path.display(),
                python,
                rust,
                extensions,
            ));
        }
    }
    if divergences.is_empty() {
        println!(
            "oracle crosscheck: {} packages agree with Python at {PINNED_PYTHON_HEAD}",
            output.records.len()
        );
        Ok(())
    } else {
        Err(format!(
            "oracle crosscheck found {} Python/Rust divergences; no Rust-only finding was filtered:\n{}",
            divergences.len(),
            divergences.join("\n")
        ))
    }
}

fn parse_arguments(arguments: &[String], repository: &Path) -> Result<PathBuf, String> {
    let mut python_qti = repository.join("..").join("qti-package-maker");
    let mut iterator = arguments.iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--python-qti" => {
                python_qti = iterator
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| "--python-qti requires a path".to_owned())?;
            }
            "--help" | "-h" => {
                return Err("Usage: cargo xtask oracle-crosscheck [--python-qti PATH]".to_owned());
            }
            _ => return Err(format!("unknown oracle-crosscheck argument: {argument}")),
        }
    }
    if !python_qti.join("source_me.sh").is_file() {
        return Err(format!(
            "Python qti-package-maker checkout is unavailable: {}",
            python_qti.display()
        ));
    }
    Ok(python_qti)
}

fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask oracle-crosscheck must run inside the Rust workspace".to_owned());
    }
    String::from_utf8(output.stdout)
        .map_err(display_error)
        .map(|path| PathBuf::from(path.trim()))
}

fn prepare_python_snapshot(repository: &Path, source: &Path) -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args([
            "-C",
            &source.to_string_lossy(),
            "rev-parse",
            &format!("{PINNED_PYTHON_HEAD}^{{commit}}"),
        ])
        .output()
        .map_err(display_error)?;
    let head = String::from_utf8(output.stdout).map_err(display_error)?;
    if !output.status.success() || head.trim() != PINNED_PYTHON_HEAD {
        return Err(format!(
            "Python checkout cannot provide pinned oracle commit {PINNED_PYTHON_HEAD}; found {}",
            head.trim()
        ));
    }
    let snapshot = repository
        .join("output_tables")
        .join("oracle_snapshot")
        .join(PINNED_PYTHON_HEAD);
    let marker = snapshot.join("PINNED_ORACLE_PROVENANCE.txt");
    if snapshot.exists() {
        let recorded = fs::read_to_string(&marker).map_err(|_| {
            format!(
                "pinned oracle snapshot exists without provenance marker: {}; remove it manually after inspection",
                snapshot.display()
            )
        })?;
        if recorded.lines().next() == Some(PINNED_PYTHON_HEAD)
            && snapshot
                .join("qti_package_maker")
                .join("common")
                .join("package_integrity.py")
                .is_file()
        {
            return Ok(snapshot);
        }
        return Err(format!(
            "pinned oracle snapshot provenance does not match {PINNED_PYTHON_HEAD}: {}",
            marker.display()
        ));
    }
    let staging = snapshot.with_extension(format!("staging-{}", std::process::id()));
    fs::create_dir_all(&staging).map_err(display_error)?;
    let archive = Command::new("git")
        .args([
            "-C",
            &source.to_string_lossy(),
            "archive",
            "--format=tar",
            PINNED_PYTHON_HEAD,
        ])
        .output()
        .map_err(display_error)?;
    if !archive.status.success() {
        return Err(format!(
            "could not archive pinned Python oracle {PINNED_PYTHON_HEAD}: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        ));
    }
    let mut unpack = Command::new("tar")
        .args(["-x", "-C"])
        .arg(&staging)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(display_error)?;
    unpack
        .stdin
        .take()
        .ok_or_else(|| "tar archive extractor has no stdin".to_owned())?
        .write_all(&archive.stdout)
        .map_err(display_error)?;
    if !unpack.wait().map_err(display_error)?.success() {
        return Err(format!(
            "could not extract pinned Python oracle into {}",
            staging.display()
        ));
    }
    fs::write(
        staging.join("PINNED_ORACLE_PROVENANCE.txt"),
        format!("{PINNED_PYTHON_HEAD}\nsource={}\n", source.display()),
    )
    .map_err(display_error)?;
    fs::rename(&staging, &snapshot).map_err(display_error)?;
    Ok(snapshot)
}

fn collect_real_packages(repository: &Path, python_qti: &Path) -> Result<Vec<PathBuf>, String> {
    let mut packages = Vec::new();
    for directory in [
        repository.join("SAMPLES"),
        repository.join("tests").join("fixtures"),
        python_qti.join("SAMPLES"),
        python_qti.join("tests").join("fixtures"),
    ] {
        collect_zips(&directory, &mut packages)?;
    }
    packages.sort();
    packages.dedup();
    if packages.is_empty() {
        return Err("valid corpus has no ZIP packages under SAMPLES or tests/fixtures".to_owned());
    }
    Ok(packages)
}

fn collect_zips(directory: &Path, packages: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(display_error)? {
        let entry = entry.map_err(display_error)?;
        let path = entry.path();
        if entry.file_type().map_err(display_error)?.is_dir() {
            collect_zips(&path, packages)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            packages.push(path.canonicalize().map_err(display_error)?);
        }
    }
    Ok(())
}

fn run_python_oracle(
    repository: &Path,
    python_snapshot: &Path,
    temporary: &Path,
    real_packages: &[PathBuf],
) -> Result<OracleOutput, String> {
    let helper = repository
        .join("xtask")
        .join("support")
        .join("oracle_crosscheck.py");
    let command = format!(
        "source {} >/dev/null && python3 {} \"$@\"",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(&helper),
    );
    let mut invocation = Command::new("bash");
    invocation
        .args(["-c", &command, "oracle_crosscheck", "--output"])
        .arg(temporary);
    for package in real_packages {
        invocation.arg("--existing").arg(package);
    }
    let output = invocation
        .current_dir(repository)
        .env("QTI_ORACLE_ROOT", python_snapshot)
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "Python integrity oracle failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Python integrity oracle returned invalid JSON: {error}"))
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Advisory => "advisory",
    }
}

fn is_rust_extension(code: &str) -> bool {
    matches!(code, "unmanifested-item" | "broken-media-trace")
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\\"'\\\"'"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
