//! Time the neighboring Python package's HTML-to-image path on the harvested corpus.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use qti_engines::{AssetOverlay, ENGINES, ReadInput};
use qti_native::DirectoryAssets;
use qti_native::html_to_image::{
    ChromiumFragmentRenderer, ConversionMetrics, RenderCache, convert_bank_with_metrics,
};
use serde_json::{Value, json};

/// Run the temporary current-Python timing oracle and publish its factual Markdown report.
///
/// The Rust shipping crates never invoke Python.  This `xtask` command is a development-only
/// parity and performance oracle; it needs the sibling Python checkout selected by `source_me.sh`.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    let options = parse_options(&repository, arguments)?;
    if options.native {
        return run_native(&repository, &options.corpus);
    }
    let corpus = options.corpus;
    let python = crate::current_python::resolve(&repository, options.python_qti.as_deref())?;
    let manifest = corpus.join("manifest.json");
    if !manifest.is_file() {
        return Err(format!(
            "table corpus manifest is unavailable: {}; run cargo xtask table-corpus first",
            manifest.display()
        ));
    }
    let output = repository.join("output_tables").join("baseline.json");
    let script = repository.join("output_tables").join("table_bench.py");
    fs::create_dir_all(script.parent().ok_or("benchmark script has no parent")?)
        .map_err(display_error)?;
    fs::write(&script, include_str!("table_bench_python.py")).map_err(display_error)?;
    let command = format!(
        "source {} && python3 {} --corpus {} --report-json {}",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(&script),
        shell_quote(&corpus),
        shell_quote(&output),
    );
    let status = Command::new("bash")
        .args(["-lc", &command])
        .current_dir(&repository)
        .env("QTI_ORACLE_ROOT", &python.root)
        .status()
        .map_err(display_error)?;
    if !status.success() {
        return Err(format!("Python table baseline failed with status {status}"));
    }
    let value: Value = serde_json::from_slice(&fs::read(&output).map_err(display_error)?)
        .map_err(display_error)?;
    let report = render_report(&value)?;
    let report_path = repository
        .join("docs")
        .join("active_plans")
        .join("reports")
        .join("html_to_image_baseline.md");
    fs::write(&report_path, report).map_err(display_error)?;
    println!("Python table baseline: {}", report_path.display());
    Ok(())
}

struct Options {
    corpus: PathBuf,
    native: bool,
    python_qti: Option<PathBuf>,
}

fn parse_options(repository: &Path, arguments: &[String]) -> Result<Options, String> {
    let mut iterator = arguments.iter();
    let mut corpus = repository.join("output_tables").join("corpus");
    let mut native = false;
    let mut python_qti = None;
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--corpus" => corpus = PathBuf::from(iterator.next().ok_or("--corpus needs a path")?),
            "--native" => native = true,
            "--python-qti" => {
                python_qti = Some(PathBuf::from(
                    iterator.next().ok_or("--python-qti needs a path")?,
                ))
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: cargo xtask table-bench [--native] [--corpus PATH] [--python-qti PATH]"
                        .to_owned(),
                );
            }
            _ => return Err(format!("unknown table-bench argument: {argument}")),
        }
    }
    Ok(Options {
        corpus,
        native,
        python_qti,
    })
}

/// Time the real release CLI in per-input output directories.
///
/// This deliberately records only observable end-to-end totals and artifacts.  Native per-stage
/// instrumentation is added only when its owning conversion and raster APIs expose counters.
fn run_native(repository: &Path, corpus: &Path) -> Result<(), String> {
    let manifest = corpus.join("manifest.json");
    let manifest_value: Value =
        serde_json::from_slice(&fs::read(&manifest).map_err(display_error)?)
            .map_err(display_error)?;
    let inputs = manifest_value["bbq_files"]
        .as_array()
        .ok_or("corpus manifest has no bbq_files")?;
    if inputs.is_empty() {
        return Err("corpus manifest has no BBQ inputs".to_owned());
    }
    let input_provenance = inputs
        .iter()
        .map(|input| {
            let path = input
                .as_str()
                .ok_or("corpus bbq_files contains non-string")?;
            Ok(json!({"path": path, "sha256": sha256(Path::new(path))?}))
        })
        .collect::<Result<Vec<Value>, String>>()?;
    let manifest_sha = sha256(&manifest)?;
    let binary = repository
        .join("target")
        .join("release")
        .join("bbq-converter");
    if !binary.is_file() {
        return Err(format!(
            "release CLI is unavailable: {}; run CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo build --release -p qti-cli --bin bbq-converter first",
            binary.display()
        ));
    }
    let receipt_root = repository
        .join("output_tables")
        .join("native_table_bench")
        .join(format!("run-{}", std::process::id()));
    fs::create_dir_all(receipt_root.parent().expect("receipt parent")).map_err(display_error)?;
    fs::create_dir(&receipt_root).map_err(display_error)?;
    // Every invocation uses the same private executable even if Cargo rebuilds its target.
    let captured_binary = receipt_root.join("bbq-converter");
    fs::copy(&binary, &captured_binary).map_err(display_error)?;
    let binary = captured_binary;
    let binary_sha = sha256(&binary)?;
    let mut runs = serde_json::Map::new();
    for (name, flags, expected_outputs) in [
        ("blackboard_export", vec!["-B", "--html-to-image"], 1_u64),
        (
            "three_format",
            vec!["-1", "-2", "-B", "--html-to-image"],
            3_u64,
        ),
    ] {
        let work = receipt_root.join(name);
        fs::create_dir_all(&work).map_err(display_error)?;
        let started = Instant::now();
        let mut failures = Vec::new();
        let mut artifacts = 0_u64;
        let mut short_output_inputs = 0_u64;
        for (index, input) in inputs.iter().enumerate() {
            let input = input
                .as_str()
                .ok_or("corpus bbq_files contains non-string")?;
            let input = Path::new(input);
            if !input.is_file() {
                return Err(format!("corpus input is missing: {}", input.display()));
            }
            let output = work.join(format!("{index:04}"));
            fs::create_dir_all(&output).map_err(display_error)?;
            let command = Command::new(&binary)
                .arg("-i")
                .arg(input)
                .args(&flags)
                .arg("-q")
                .current_dir(&output)
                .output()
                .map_err(display_error)?;
            let files = fs::read_dir(&output).map_err(display_error)?.count() as u64;
            artifacts += files;
            if files != expected_outputs {
                short_output_inputs += 1;
            }
            if !command.status.success() {
                failures.push(json!({
                    "input": input,
                    "exit": command.status.code(),
                    "artifacts": files,
                    "stdout": String::from_utf8_lossy(&command.stdout),
                    "stderr": String::from_utf8_lossy(&command.stderr),
                }));
            }
        }
        let wall_seconds = started.elapsed().as_secs_f64();
        let (png_members, packages_with_png) = inspect_png_members(&work)?;
        runs.insert(
            name.to_owned(),
            json!({
                "flags": flags,
                "inputs": inputs.len(),
                "wall_seconds": wall_seconds,
                "png_members": png_members,
                "packages_with_png": packages_with_png,
                "artifacts": artifacts,
                "expected_artifacts_per_input": expected_outputs,
                "short_output_inputs": short_output_inputs,
                "failures": failures,
                "release_binary_stages": {
                    "load": null, "select": null, "canvas": null, "table": null,
                    "layout": null, "paint": null, "encode": null, "cache": null, "write": null,
                    "bookkeeping": null
                },
                "library_conversion_metrics": native_library_metrics(inputs)?,
            }),
        );
    }
    if sha256(&binary)? != binary_sha {
        return Err("captured benchmark executable changed during the run".to_owned());
    }
    if sha256(&manifest)? != manifest_sha {
        return Err("corpus manifest changed during the benchmark".to_owned());
    }
    for input in &input_provenance {
        let path = input["path"]
            .as_str()
            .ok_or("missing input provenance path")?;
        if sha256(Path::new(path))? != input["sha256"].as_str().unwrap_or_default() {
            return Err(format!("corpus input changed during the benchmark: {path}"));
        }
    }
    let receipt = json!({
        "command": "cargo xtask table-bench --native",
        "manifest_sha256": manifest_sha,
        "input_provenance": input_provenance,
        "corpus": corpus,
        "binary": binary,
        "binary_sha256": binary_sha,
        "runs": runs,
        "stage_status": "CLI wall time and separately attributed library conversion metrics"
    });
    fs::write(
        receipt_root.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).map_err(display_error)?,
    )
    .map_err(display_error)?;
    let receipt_path = repository
        .join("output_tables")
        .join("native_baseline.json");
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).map_err(display_error)?,
    )
    .map_err(display_error)?;
    let report_path =
        repository.join("docs/active_plans/reports/html_to_image_native_benchmark.md");
    fs::write(&report_path, native_report(&receipt)).map_err(display_error)?;
    println!("Native table baseline: {}", report_path.display());
    if receipt["runs"]
        .as_object()
        .expect("constructed runs")
        .values()
        .any(|run| {
            !run["failures"]
                .as_array()
                .expect("constructed failures")
                .is_empty()
                || !run["library_conversion_metrics"]["failures"]
                    .as_array()
                    .expect("constructed metric failures")
                    .is_empty()
        })
    {
        return Err(
            "native benchmark failed; retained receipt is not a performance baseline".to_owned(),
        );
    }
    Ok(())
}

fn native_library_metrics(inputs: &[Value]) -> Result<Value, String> {
    let entry = ENGINES
        .iter()
        .find(|entry| entry.name == "bbq_text_upload")
        .expect("static BBQ reader");
    let reader = entry.make_reader.expect("static BBQ reader factory")();
    let renderer = ChromiumFragmentRenderer::default();
    let mut metrics = ConversionMetrics::default();
    let mut load_seconds = 0.0;
    let mut conversion_wall_seconds = 0.0;
    let mut failures = Vec::new();
    for input in inputs {
        let input = Path::new(
            input
                .as_str()
                .ok_or("corpus bbq_files contains non-string")?,
        );
        let started = Instant::now();
        let load = || {
            let bytes = qti_native::read_source(input).map_err(display_error)?;
            let name = input
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("corpus input filename is not UTF-8")?;
            let outcome = reader
                .read_items(
                    ReadInput::File {
                        name,
                        bytes: &bytes,
                    },
                    false,
                )
                .map_err(display_error)?;
            let directory = DirectoryAssets::new(input.parent().unwrap_or(Path::new(".")))
                .map_err(display_error)?;
            Ok::<_, String>((outcome, directory))
        };
        let (outcome, directory) = match load() {
            Ok(loaded) => loaded,
            Err(error) => {
                failures.push(json!({"input": input, "stage": "load", "error": error.to_string()}));
                continue;
            }
        };
        let assets = AssetOverlay {
            memory: &outcome.assets,
            fallback: &directory,
        };
        load_seconds += started.elapsed().as_secs_f64();
        let started = Instant::now();
        let supported_kinds = ENGINES
            .iter()
            .filter(|entry| entry.native_rendering)
            .flat_map(|entry| entry.supported_kinds.iter().copied())
            .collect::<Vec<_>>();
        match convert_bank_with_metrics(
            &outcome.bank,
            &supported_kinds,
            &assets,
            &renderer,
            &RenderCache::new(),
        ) {
            Ok((_converted, _assets, input_metrics)) => metrics += input_metrics,
            Err(failure) => {
                metrics += *failure.metrics;
                failures.push(json!({
                    "input": input, "stage": "conversion", "error": failure.error.to_string(),
                    "metric_scope": "observed work before cancellation"
                }));
            }
        }
        conversion_wall_seconds += started.elapsed().as_secs_f64();
    }
    Ok(json!({
        "input_count": inputs.len(), "failures": failures,
        "load_wall_seconds": load_seconds,
        "conversion_wall_seconds": conversion_wall_seconds,
        "requested_fragments": metrics.requested_fragments,
        "cache": {"hits": metrics.cache_hits, "waits": metrics.cache_waits, "misses": metrics.cache_misses},
        "renderer": {"attempts": metrics.renderer_attempts, "successes": metrics.renderer_successes, "failures": metrics.renderer_failures},
        "cumulative_work_seconds": {
            "layout": null,
            "paint": null,
            "encode": null,
            "materialization": metrics.materialization.as_secs_f64(),
            "bookkeeping": metrics.conversion_bookkeeping.as_secs_f64()
        }
    }))
}

fn inspect_png_members(work: &Path) -> Result<(u64, u64), String> {
    let mut png_members = 0;
    let mut packages_with_png = 0;
    for directory in fs::read_dir(work).map_err(display_error)? {
        for entry in
            fs::read_dir(directory.map_err(display_error)?.path()).map_err(display_error)?
        {
            let path = entry.map_err(display_error)?.path();
            if path.extension().is_none_or(|extension| extension != "zip") {
                continue;
            }
            let file = fs::File::open(&path).map_err(display_error)?;
            let archive = zip::ZipArchive::new(file).map_err(display_error)?;
            let count = archive
                .file_names()
                .filter_map(Result::ok)
                .filter(|name| name.to_ascii_lowercase().ends_with(".png"))
                .count() as u64;
            png_members += count;
            packages_with_png += u64::from(count > 0);
        }
    }
    Ok((png_members, packages_with_png))
}

fn sha256(path: &Path) -> Result<String, String> {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err(format!("could not hash release CLI: {}", path.display()));
    }
    Ok(String::from_utf8(output.stdout)
        .map_err(display_error)?
        .split_whitespace()
        .next()
        .unwrap_or("unavailable")
        .to_owned())
}

fn native_report(receipt: &Value) -> String {
    let mut report = String::from(
        "# Native HTML-to-image benchmark\n\nThis receipt invokes the real release `bbq-converter` binary on each manifest input. It does not model or project Rust performance.\n\n",
    );
    report.push_str(&format!(
        "- Binary: `{}`\n- SHA-256: `{}`\n- Corpus: `{}`\n\n",
        string(receipt, "binary"),
        string(receipt, "binary_sha256"),
        string(receipt, "corpus")
    ));
    report.push_str("| Run | Inputs | Wall seconds | Artifacts | Short outputs | Failures |\n| --- | ---: | ---: | ---: | ---: | ---: |\n");
    for (name, run) in receipt["runs"].as_object().expect("constructed runs") {
        report.push_str(&format!(
            "| `{name}` | {} | {:.3} | {} | {} | {} |\n",
            run["inputs"],
            run["wall_seconds"].as_f64().unwrap_or_default(),
            run["artifacts"],
            run["short_output_inputs"],
            run["failures"].as_array().map_or(0, Vec::len)
        ));
    }
    report.push_str("\n## Capability receipt\n\n");
    for (name, run) in receipt["runs"].as_object().expect("constructed runs") {
        report.push_str(&format!(
            "- `{name}`: {} PNG members in {} packages. Inspection runs after the wall timer stops.\n",
            run["png_members"], run["packages_with_png"]
        ));
    }
    report.push('\n');
    report.push_str("Both modes request --html-to-image. The table above records actual invocation failures and output counts; detailed failures are retained in receipt.json. A short output can be expected when an input contains only an unsupported item type, such as ORDER for Canvas, but must be assessed alongside the recorded failures.\n");
    report.push_str("\n## Stage attribution\n\nThe release-binary lane records only end-to-end wall time and artifacts. The separate library pass below exposes the approved conversion and raster counters without altering CLI behavior.\n");
    report.push_str("\n## Library conversion metrics\n\nA separate library pass uses the same manifest inputs and one fresh run-scoped cache per input, matching the CLI conversion scope. `conversion_wall_seconds` is elapsed sequential wall time for that pass. Chromium does not expose separate layout, paint, or PNG encoding timings here; those fields are null. Materialization stages generated and retained asset bytes in memory; historical `materialization_write` timings measured the former filesystem stage. Materialization and bookkeeping are cumulative work under Rayon, not elapsed wall time.\n\n");
    report.push_str("| Run | Requests | Attempts | Cache hit/wait/miss | Conversion wall | Materialization work | Bookkeeping work |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (name, run) in receipt["runs"].as_object().expect("constructed runs") {
        let metrics = &run["library_conversion_metrics"];
        let work = &metrics["cumulative_work_seconds"];
        report.push_str(&format!(
            "| `{name}` | {} | {} | {}/{}/{} | {:.3} | {:.3} | {:.3} |\n",
            metrics["requested_fragments"],
            metrics["renderer"]["attempts"],
            metrics["cache"]["hits"],
            metrics["cache"]["waits"],
            metrics["cache"]["misses"],
            number(metrics, "conversion_wall_seconds"),
            number(work, "materialization"),
            number(work, "bookkeeping"),
        ));
    }
    report
}

fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask table-bench must run inside the Rust workspace".to_owned());
    }
    let path = String::from_utf8(output.stdout).map_err(display_error)?;
    Ok(PathBuf::from(path.trim()))
}

fn render_report(value: &Value) -> Result<String, String> {
    let runs = value["runs"]
        .as_object()
        .ok_or("baseline JSON has no runs")?;
    let mut report = String::from(
        "# Python baseline with development-only media-base repair\n\n\
Generated by `cargo xtask table-bench`. This development-only oracle applies the WP-T5 safe media-staging repair inside its temporary Python process: resolved existing local assets are copied under their original relative paths into the converted bank's new owned media root. It does not modify the Python checkout or the corpus. The Rust runtime has no Python dependency.\n\n",
    );
    report.push_str("## Environment\n\n");
    report.push_str(&format!(
        "- Machine: `{}` ({})\n",
        string(value, "machine"),
        string(value, "platform")
    ));
    report.push_str(&format!("- Python: `{}`\n", string(value, "python")));
    report.push_str(&format!(
        "- Certified oracle: `{}` from `{}`\n",
        string(value, "oracle_commit"),
        string(value, "oracle_root")
    ));
    report.push_str(&format!("- Corpus: `{}`\n\n", string(value, "corpus")));
    report.push_str("## Repaired measurements\n\n");
    report.push_str("| Run | Inputs | Wall seconds | Tables | Browser launches | Staged assets | Failures |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (name, run) in runs {
        let plain = &run["plain"];
        let stages = &run["instrumented"]["stages_seconds"];
        let failures = plain["failures"].as_array().map_or(0, Vec::len)
            + run["instrumented"]["failures"]
                .as_array()
                .map_or(0, Vec::len);
        report.push_str(&format!(
            "| `{name}` | {} | {:.3} | {:.0} | {:.0} | {:.0} | {failures} |\n",
            plain["input_count"],
            plain["wall_seconds"].as_f64().unwrap_or_default(),
            stages["table_count"].as_f64().unwrap_or_default(),
            number(stages, "browser_launch_count"),
            number(stages, "media_staging_asset_count"),
        ));
    }
    report.push_str("\n## Instrumented stage time\n\n");
    report.push_str("Times are cumulative across the run. `conversion_bookkeeping` is `convert_bank` time after the explicitly timed Playwright stages; it includes parsing, selection, cache work, item copying, and renderer teardown.\n\n");
    report.push_str("| Run | Browser launch | Page setup | Font CSS | Evaluate + fonts ready | Screenshot | Media staging | Conversion bookkeeping |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (name, run) in runs {
        let stages = &run["instrumented"]["stages_seconds"];
        let page_setup = number(stages, "browser_context")
            + number(stages, "page_creation")
            + number(stages, "static_page_setup");
        report.push_str(&format!(
            "| `{name}` | {:.3} | {page_setup:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |\n",
            number(stages, "browser_launch"),
            number(stages, "font_css"),
            number(stages, "table_evaluate_and_font_ready"),
            number(stages, "table_screenshot"),
            number(stages, "media_staging"),
            number(stages, "conversion_bookkeeping"),
        ));
    }
    report.push_str("\n## Unpatched capability-failure receipt\n\nThe unpatched Python conversion was attempted on this same 181-input corpus. It fails for `bbq-horse_coat_pattern_inference-questions.txt` after `convert_bank` creates a new `qti_media_*` root: the copied bank loses its source `media_base_dir`, so the existing local `mirrored/problems/inheritance-problems/horse_coat_patterns.png` cannot be resolved during package writing. The corpus asset exists and has SHA-256 `ed0b2089570d4d66622c3707608b60a975fdcbaaaeb936b2ceff56fcdacd499d`. That unpatched result is a capability-failure receipt, not a performance baseline.\n\n## Interpretation\n\nThe three-format run measures the current Python path and its run-scoped render-cache behavior; its stage totals reveal whether tables and browsers repeat for each selected engine on a particular corpus. The single `-B` run isolates one packaging conversion. This repaired baseline names the stages for the later Rust comparison; it sets no numeric performance threshold.\n");
    Ok(report)
}

fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or_default()
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("unavailable")
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn package_inspection_counts_png_members_and_media_bearing_packages() {
        let root = tempfile::tempdir().expect("receipt directory");
        let output = root.path().join("0000");
        fs::create_dir(&output).expect("output directory");
        for (name, members) in [
            (
                "media.zip",
                vec!["images/a.png", "images/B.PNG", "item.xml"],
            ),
            ("plain.zip", vec!["item.xml"]),
        ] {
            let file = fs::File::create(output.join(name)).expect("package file");
            let mut archive = zip::ZipWriter::new(file);
            for member in members {
                archive
                    .start_file(member, zip::write::SimpleFileOptions::default())
                    .expect("member");
            }
            archive.finish().expect("valid package");
        }
        assert_eq!(
            inspect_png_members(root.path()).expect("inspection"),
            (2, 1)
        );
    }

    #[test]
    fn report_names_the_measured_stages() {
        let value = json!({"machine":"arm64","platform":"test","python":"3.12","corpus":"/tmp/corpus","runs":{"blackboard_export":{"plain":{"input_count":1,"wall_seconds":1.0,"failures":[]},"instrumented":{"failures":[],"stages_seconds":{"table_count":2.0,"browser_launch_count":1.0,"browser_launch":0.1,"browser_context":0.1,"page_creation":0.1,"static_page_setup":0.1,"font_css":0.1,"table_evaluate_and_font_ready":0.2,"table_screenshot":0.3,"conversion_bookkeeping":0.4}}}}});
        let report = render_report(&value).expect("report renders");
        assert!(report.contains("Evaluate + fonts ready"));
        assert!(report.contains("Conversion bookkeeping"));
    }
}
