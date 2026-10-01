//! Ignored structured receipts for parity divergences.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use super::{Divergence, PINNED_PYTHON_HEAD, ParityInput, display_error, file_sha256};

#[derive(Serialize)]
pub(super) struct RunContext {
    pinned_python_commit: &'static str,
    native_cli_path: String,
    native_cli_initial_sha256: String,
    oracle_path: String,
    oracle_initial_sha256: String,
    oracle_support_directory: String,
    oracle_support_initial_sha256: BTreeMap<String, String>,
    fixtures: bool,
    html_to_image: bool,
    ordered_inputs: Vec<InputReceipt>,
}

#[derive(Serialize)]
struct InputReceipt {
    path: String,
    sha256: String,
    engines: Vec<String>,
}

impl RunContext {
    pub(super) fn capture(
        inputs: &[ParityInput],
        cli: &Path,
        repository: &Path,
        fixtures: bool,
        html_to_image: bool,
    ) -> Result<Self, String> {
        let oracle = repository.join("xtask/support/parity_oracle.py");
        let support = repository.join("xtask/support");
        Ok(Self {
            pinned_python_commit: PINNED_PYTHON_HEAD,
            native_cli_path: cli.display().to_string(),
            native_cli_initial_sha256: file_sha256(cli)?,
            oracle_path: oracle.display().to_string(),
            oracle_initial_sha256: file_sha256(&oracle)?,
            oracle_support_directory: support.display().to_string(),
            oracle_support_initial_sha256: support_hashes(&support)?,
            fixtures,
            html_to_image,
            ordered_inputs: inputs
                .iter()
                .map(|input| {
                    Ok(InputReceipt {
                        path: input.path.display().to_string(),
                        sha256: file_sha256(&input.path)?,
                        engines: input.engines.iter().map(ToString::to_string).collect(),
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    }
}

fn support_hashes(directory: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut hashes = BTreeMap::new();
    for entry in std::fs::read_dir(directory).map_err(display_error)? {
        let path = entry.map_err(display_error)?.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "py") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("non-UTF-8 oracle support filename: {}", path.display()))?;
            hashes.insert(name.to_owned(), file_sha256(&path)?);
        }
    }
    Ok(hashes)
}

pub(super) fn persist(
    temporary: &Path,
    context: &RunContext,
    divergences: &[Divergence],
    ranges: &[(usize, usize, usize)],
) -> Result<(), String> {
    let mut grouped = BTreeMap::<String, usize>::new();
    for divergence in divergences {
        *grouped
            .entry(format!("{} :: {}", divergence.engine, divergence.field))
            .or_default() += 1;
    }
    let support_final = support_hashes(Path::new(&context.oracle_support_directory))?;
    let report = json!({
        "contract": "m13_differential_parity_divergence_report",
        "run_context": context,
        "native_cli_final_sha256": file_sha256(Path::new(&context.native_cli_path))?,
        "oracle_final_sha256": file_sha256(Path::new(&context.oracle_path))?,
        "oracle_support_final_sha256": support_final,
        "divergence_count": divergences.len(),
        "grouped_counts": grouped,
        "divergences": attributed_divergences(context, divergences, ranges)?,
    });
    let path = temporary.join("divergences.json");
    let text = serde_json::to_string_pretty(&report).map_err(display_error)?;
    std::fs::write(&path, text).map_err(display_error)?;
    if report["native_cli_final_sha256"].as_str()
        != Some(context.native_cli_initial_sha256.as_str())
        || report["oracle_final_sha256"].as_str() != Some(context.oracle_initial_sha256.as_str())
        || support_final != context.oracle_support_initial_sha256
    {
        return Err(format!(
            "parity sources changed during run; inspect {}",
            path.display()
        ));
    }
    println!(
        "parity divergence receipt: {} ({} records)",
        path.display(),
        divergences.len()
    );
    for (group, count) in report["grouped_counts"]
        .as_object()
        .ok_or_else(|| "parity divergence report lost grouped counts".to_owned())?
    {
        println!("parity divergence group: {count} :: {group}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::support_hashes;

    #[test]
    fn support_snapshot_detects_changed_added_and_removed_helpers() {
        let directory = std::env::temp_dir().join(format!(
            "qti-parity-support-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let helper = directory.join("parity_qti21.py");
        std::fs::write(&helper, "original").unwrap();
        let initial = support_hashes(&directory).unwrap();
        std::fs::write(directory.join("unrelated.txt"), "ignored").unwrap();
        assert_eq!(support_hashes(&directory).unwrap(), initial);
        std::fs::write(&helper, "changed").unwrap();
        assert_ne!(support_hashes(&directory).unwrap(), initial);
        std::fs::write(&helper, "original").unwrap();
        std::fs::write(directory.join("parity_bb.py"), "new helper").unwrap();
        assert_ne!(support_hashes(&directory).unwrap(), initial);
        std::fs::remove_file(directory.join("parity_bb.py")).unwrap();
        std::fs::remove_file(&helper).unwrap();
        assert_ne!(support_hashes(&directory).unwrap(), initial);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

/// Bind each per-bank finding to the input and retained artifact directory.
fn attributed_divergences(
    context: &RunContext,
    divergences: &[Divergence],
    ranges: &[(usize, usize, usize)],
) -> Result<Vec<serde_json::Value>, String> {
    divergences
        .iter()
        .enumerate()
        .map(|(position, divergence)| {
            let mut value = serde_json::to_value(divergence).map_err(display_error)?;
            if let Some(&(index, _, _)) = ranges
                .iter()
                .find(|&&(_, start, end)| position >= start && position < end)
            {
                let source = context
                    .ordered_inputs
                    .get(index)
                    .ok_or_else(|| "invalid parity input attribution".to_owned())?;
                value["input_index"] = json!(index);
                value["input_path"] = json!(source.path);
                value["input_sha256"] = json!(source.sha256);
                value["artifact_directory"] = json!(format!("input_{index:04}"));
            }
            Ok(value)
        })
        .collect()
}

#[cfg(test)]
mod attribution_tests {
    use super::*;

    #[test]
    fn binds_bank_findings_without_misattributing_global_checks() {
        let context = RunContext {
            pinned_python_commit: PINNED_PYTHON_HEAD,
            native_cli_path: String::new(),
            native_cli_initial_sha256: String::new(),
            oracle_path: String::new(),
            oracle_initial_sha256: String::new(),
            oracle_support_directory: String::new(),
            oracle_support_initial_sha256: BTreeMap::new(),
            fixtures: true,
            html_to_image: false,
            ordered_inputs: vec![InputReceipt {
                path: "bank.txt".into(),
                sha256: "source-hash".into(),
                engines: vec!["engine".into()],
            }],
        };
        let findings = (0..2)
            .map(|_| Divergence {
                engine: "engine".into(),
                item: "all".into(),
                field: "content".into(),
                python: "expected".into(),
                rust: "actual".into(),
            })
            .collect::<Vec<_>>();
        let rows = attributed_divergences(&context, &findings, &[(0, 0, 1)]).unwrap();
        assert_eq!(rows[0]["input_path"], "bank.txt");
        assert_eq!(rows[0]["input_sha256"], "source-hash");
        assert_eq!(rows[0]["artifact_directory"], "input_0000");
        assert!(rows[1].get("input_index").is_none());
        assert!(attributed_divergences(&context, &findings, &[(1, 0, 1)]).is_err());
    }
}
