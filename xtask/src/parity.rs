//! M13 differential Python-versus-Rust package comparison; the pinned Python source is oracle-only.

#[path = "parity_grading_program.rs"]
mod parity_grading_program;
#[path = "parity_integrity.rs"]
mod parity_integrity;
#[path = "parity_reader_receipts.rs"]
mod parity_reader_receipts;
use parity_integrity::compare_zip_integrity;
#[path = "parity_report.rs"]
mod parity_report;
#[path = "parity_writer_receipts.rs"]
mod parity_writer_receipts;
use parity_writer_receipts::{
    WriterReceipt, comparable_engines, compare_writer_outcomes, parity_lane,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PINNED_PYTHON_HEAD: &str = "55e5f368777f7809fe2e91b5d070caf6df0cb581";
const ENGINES: &[&str] = &[
    "bbq_text_upload",
    "text2qti",
    "okla_chrst_bqgen",
    "blackboard_export_zip",
    "canvas_qti_v1_2",
    "blackboard_qti_v2_1",
    "exam_yaml",
    "moodle_aiken",
    "human_readable",
    "html_selftest",
];
const ZIP_ENGINES: &[&str] = &[
    "blackboard_export_zip",
    "canvas_qti_v1_2",
    "blackboard_qti_v2_1",
];
#[derive(Debug)]
struct Options {
    inputs: Vec<PathBuf>,
    corpus: Option<PathBuf>,
    max_inputs: Option<usize>,
    html_to_image: bool,
    fixtures: bool,
}
#[derive(Debug)]
struct ParityInput {
    path: PathBuf,
    engines: Vec<&'static str>,
}
#[derive(Debug, Deserialize)]
struct Comparison {
    divergences: Vec<Divergence>,
}
#[derive(Debug, Deserialize, Serialize)]
struct Divergence {
    engine: String,
    item: String,
    field: String,
    python: String,
    rust: String,
}
#[derive(Debug, Deserialize)]
struct OrderReceipt {
    canvas_qti_v1_2: OrderOutcome,
    blackboard_export_zip: OrderOutcome,
}
#[derive(Debug, Deserialize)]
struct OrderOutcome {
    exit: i32,
    file: bool,
    diagnostic: String,
}
#[derive(Debug, Deserialize)]
struct CorpusProjection {
    projections: Vec<CorpusProjectionEntry>,
}
#[derive(Debug, Deserialize)]
struct CorpusProjectionEntry {
    engine: String,
    path: PathBuf,
}

/// Runs the M13 differential harness.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    println!("{}", parity_grading_program::verify(&repository)?);
    let options = parse_arguments(arguments, &repository)?;
    let inputs = resolve_inputs(&options, &repository)?;
    let python_root = pinned_python_root(&repository)?;
    let cli = native_cli(&repository)?;
    println!(
        "parity native CLI: {} sha256={}",
        cli.display(),
        file_sha256(&cli)?
    );
    let temporary = repository
        .join("tests")
        .join("_temp")
        .join(format!("parity_{}", std::process::id()));
    fs::create_dir_all(&temporary).map_err(display_error)?;
    let report_context = parity_report::RunContext::capture(
        &inputs,
        &cli,
        &repository,
        options.fixtures,
        options.html_to_image,
    )?;
    let (mut all_divergences, mut ranges) = (Vec::new(), Vec::new());
    for (index, input) in inputs.iter().enumerate() {
        let start = all_divergences.len();
        let run = temporary.join(format!("input_{index:04}"));
        let python_output = run.join("python");
        let rust_output = run.join("rust");
        fs::create_dir_all(&python_output).map_err(display_error)?;
        fs::create_dir_all(&rust_output).map_err(display_error)?;
        let python_receipt = run_python_writer(
            &repository,
            &python_root,
            &input.path,
            &python_output,
            options.html_to_image,
            &input.engines,
        )?;
        let rust_receipt = run_native_writers(
            &cli,
            &input.path,
            &rust_output,
            options.html_to_image,
            &input.engines,
        )?;
        let comparable_engines = comparable_engines(&input.engines, &python_receipt, &rust_receipt);
        all_divergences.extend(compare_writer_outcomes(
            &input.path,
            &input.engines,
            &python_receipt,
            &rust_receipt,
        ));
        if !comparable_engines.is_empty() {
            let comparison = compare_outputs(
                &repository,
                &python_root,
                &python_output,
                &rust_output,
                options.html_to_image,
                &comparable_engines,
                parity_grading_program::is_canvas_multifib_repair_input(&input.path),
            )?;
            all_divergences.extend(comparison.divergences);
        }
        all_divergences.extend(compare_zip_integrity(
            &input.path,
            &python_output,
            &rust_output,
            &comparable_engines,
        )?);
        ranges.push((index, start, all_divergences.len()));
    }
    if options.fixtures {
        let fixture_directory = inputs
            .first()
            .and_then(|input| input.path.parent())
            .ok_or_else(|| "fixture inputs have no parent directory".to_owned())?;
        all_divergences.extend(compare_order_receipt(
            &repository,
            &python_root,
            &cli,
            &temporary,
        )?);
        all_divergences.extend(compare_media_receipts(
            &repository,
            &python_root,
            &cli,
            fixture_directory,
            &temporary,
        )?);
        all_divergences.extend(parity_reader_receipts::native_reader_roundtrips(
            &repository,
            &python_root,
            fixture_directory,
            &temporary,
        )?);
    }
    parity_report::persist(&temporary, &report_context, &all_divergences, &ranges)?;
    if all_divergences.is_empty() {
        if options.html_to_image {
            println!(
                "parity html-to-image: {} input bank(s); 3 frozen-CLI packaging ZIP formats agree structurally with pinned Python {PINNED_PYTHON_HEAD}",
                inputs.len()
            );
        } else {
            println!(
                "parity: {} input bank(s); 7 frozen-CLI formats and 3 fixed registered-only adapter formats agree with pinned Python {PINNED_PYTHON_HEAD}",
                inputs.len()
            );
        }
        return Ok(());
    }
    let details = all_divergences
        .iter()
        .map(|difference| {
            format!(
                "lane={} engine={} item={} field={}\n  Python: {}\n  Rust: {}",
                parity_lane(&difference.engine),
                difference.engine,
                difference.item,
                difference.field,
                difference.python,
                difference.rust
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "parity found {} semantic divergence(s); no comparison was weakened:\n{details}",
        all_divergences.len()
    ))
}
/// Checks the frozen ORDER-only behavior without pretending that all writers support ORDER.
fn compare_order_receipt(
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
fn parse_arguments(arguments: &[String], repository: &Path) -> Result<Options, String> {
    let mut options = Options {
        inputs: Vec::new(),
        corpus: None,
        max_inputs: None,
        html_to_image: false,
        fixtures: false,
    };
    let mut iterator = arguments.iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--input" => options.inputs.push(PathBuf::from(
                iterator
                    .next()
                    .ok_or_else(|| "--input requires a path".to_owned())?,
            )),
            "--corpus" => {
                options.corpus = Some(PathBuf::from(
                    iterator
                        .next()
                        .ok_or_else(|| "--corpus requires a directory".to_owned())?,
                ));
            }
            "--max-inputs" => {
                let value = iterator
                    .next()
                    .ok_or_else(|| "--max-inputs requires a positive integer".to_owned())?;
                options.max_inputs = Some(value.parse().map_err(|_| {
                    format!("--max-inputs must be a positive integer, got '{value}'")
                })?);
            }
            "--html-to-image" => options.html_to_image = true,
            "--fixtures" => options.fixtures = true,
            "--help" | "-h" => {
                return Err("Usage: cargo xtask parity [--fixtures | --input BBQ... | --corpus DIR] [--max-inputs N] [--html-to-image]\n\n--fixtures creates deterministic ignored M13 inputs. Without an input source, the harvested corpus at output_tables/corpus/generated is used. Generated packages are ignored under tests/_temp/.".to_owned());
            }
            _ => return Err(format!("unknown parity argument: {argument}")),
        }
    }
    if options.fixtures && (!options.inputs.is_empty() || options.corpus.is_some()) {
        return Err("--fixtures cannot be combined with --input or --corpus".to_owned());
    }
    if options.inputs.is_empty() && options.corpus.is_none() && !options.fixtures {
        options.corpus = Some(repository.join("output_tables/corpus/generated"));
    }
    Ok(options)
}

fn resolve_inputs(options: &Options, repository: &Path) -> Result<Vec<ParityInput>, String> {
    let mut inputs = options.inputs.clone();
    if options.fixtures {
        let fixture_dir = repository
            .join("tests")
            .join("_temp")
            .join(format!("parity_fixtures_{}", std::process::id()));
        let script = repository.join("xtask/support/parity_fixtures.py");
        let mut command = Command::new("python3");
        command
            .arg(script)
            .args(["--output", &fixture_dir.to_string_lossy()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        run_command(command, "parity fixture builder")?;
        if options.html_to_image {
            return Ok(vec![ParityInput {
                path: fixture_dir.join("bbq-parity-html-table-questions.txt"),
                engines: ZIP_ENGINES.to_vec(),
            }]);
        }
        return fixture_inputs(&fixture_dir);
    }
    if let Some(corpus) = &options.corpus {
        let corpus = absolute_path(corpus, repository);
        collect_bbq_files(&corpus, &mut inputs)?;
    }
    inputs = inputs
        .into_iter()
        .map(|input| absolute_path(&input, repository))
        .collect();
    inputs.sort();
    inputs.dedup();
    if let Some(limit) = options.max_inputs {
        if limit == 0 {
            return Err("--max-inputs must be greater than zero".to_owned());
        }
        inputs.truncate(limit);
    }
    if inputs.is_empty() {
        return Err("parity corpus contains no bbq-*-questions.txt input files".to_owned());
    }
    if let Some(input) = inputs.iter().find(|input| !input.is_file()) {
        return Err(format!("parity input is unavailable: {}", input.display()));
    }
    let projections = project_corpus_inputs(&inputs, repository)?;
    Ok(projections
        .into_iter()
        .filter(|input| {
            !options.html_to_image
                || input
                    .engines
                    .iter()
                    .all(|engine| ZIP_ENGINES.contains(engine))
        })
        .collect())
}
/// Creates one order-preserving supported-kind bank for every source bank and writer.
fn project_corpus_inputs(
    inputs: &[PathBuf],
    repository: &Path,
) -> Result<Vec<ParityInput>, String> {
    let directory = repository
        .join("tests")
        .join("_temp")
        .join(format!("parity_corpus_projection_{}", std::process::id()));
    fs::create_dir_all(&directory).map_err(display_error)?;
    let script = repository.join("xtask/support/parity_projection.py");
    let mut projected = Vec::new();
    for input in inputs {
        let identity = &file_sha256(input)?[..16];
        let mut command = Command::new("python3");
        command
            .arg(&script)
            .args([
                "--input",
                &input.to_string_lossy(),
                "--output",
                &directory.to_string_lossy(),
                "--identity",
                identity,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = command.output().map_err(display_error)?;
        if !output.status.success() {
            return Err(format!(
                "M6 corpus projection failed for {}: {}",
                input.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let receipt: CorpusProjection =
            serde_json::from_slice(&output.stdout).map_err(|error| {
                format!(
                    "M6 corpus projection returned invalid JSON for {}: {error}",
                    input.display()
                )
            })?;
        for entry in receipt.projections {
            let engine = ENGINES
                .iter()
                .copied()
                .find(|engine| *engine == entry.engine)
                .ok_or_else(|| {
                    format!(
                        "M6 corpus projection emitted unknown engine {}",
                        entry.engine
                    )
                })?;
            projected.push(ParityInput {
                path: entry.path,
                engines: vec![engine],
            });
        }
    }
    if projected.is_empty() {
        return Err("M6 corpus projection produced no supported writer banks".to_owned());
    }
    Ok(projected)
}

fn fixture_inputs(directory: &Path) -> Result<Vec<ParityInput>, String> {
    let prefix = "bbq-parity-projection-";
    let suffix = "-questions.txt";
    let mut inputs = Vec::new();
    for entry in fs::read_dir(directory).map_err(display_error)? {
        let path = entry.map_err(display_error)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(projection) = name
            .strip_prefix(prefix)
            .and_then(|name| name.strip_suffix(suffix))
        else {
            continue;
        };
        let Some((engine, _kind)) = projection.split_once("--") else {
            return Err(format!("invalid parity projection name: {name}"));
        };
        let Some(engine) = ENGINES.iter().copied().find(|known| *known == engine) else {
            return Err(format!("unknown parity projection engine: {engine}"));
        };
        inputs.push(ParityInput {
            path,
            engines: vec![engine],
        });
    }
    for name in [
        "bbq-parity-media-local-questions.txt",
        "bbq-parity-media-repeated-questions.txt",
        "bbq-parity-media-noimage-questions.txt",
    ] {
        let path = directory.join(name);
        if !path.is_file() {
            return Err(format!(
                "required parity media fixture is unavailable: {}",
                path.display()
            ));
        }
        inputs.push(ParityInput {
            path,
            engines: ENGINES.to_vec(),
        });
    }
    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    if inputs.is_empty() {
        return Err(format!(
            "parity projection directory contains no {prefix}*{suffix} files: {}",
            directory.display()
        ));
    }
    Ok(inputs)
}

/// Compares the two retained frozen-media failure receipts without treating either as a test input
/// that writers can silently skip.  The space spelling is a frozen self-test source defect; data
/// URIs are deliberately rejected by the three package writers on both sides.
fn compare_media_receipts(
    repository: &Path,
    python_root: &Path,
    cli: &Path,
    fixtures: &Path,
    temporary: &Path,
) -> Result<Vec<Divergence>, String> {
    let known_failures: [(&str, &[&str]); 2] = [
        ("bbq-parity-media-spaces-questions.txt", &["html_selftest"]),
        (
            "bbq-parity-media-nonpackageable-questions.txt",
            &[
                "blackboard_export_zip",
                "canvas_qti_v1_2",
                "blackboard_qti_v2_1",
            ],
        ),
    ];
    let mut differences = Vec::new();
    for (name, expected_failures) in known_failures {
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
            if name == "bbq-parity-media-spaces-questions.txt" && *engine == "html_selftest" {
                if !python_failed || rust_failed {
                    differences.push(Divergence {
                        engine: (*engine).to_owned(),
                        item: name.to_owned(),
                        field: "frozen space-path defect receipt and Rust repair".to_owned(),
                        python: if python_failed {
                            "frozen FileNotFoundError"
                        } else {
                            "writer unexpectedly succeeded"
                        }
                        .to_owned(),
                        rust: if rust_failed {
                            "repair failed to produce a self-test"
                        } else {
                            "self-test output present"
                        }
                        .to_owned(),
                    });
                } else {
                    let rendered = fs::read_to_string(&destination).map_err(display_error)?;
                    if !rendered.contains("data:image/png;base64,") {
                        differences.push(Divergence {
                            engine: (*engine).to_owned(),
                            item: name.to_owned(),
                            field: "space-path repair embedded-media proof".to_owned(),
                            python: "frozen source failure".to_owned(),
                            rust: "self-test lacks embedded PNG data URI".to_owned(),
                        });
                    }
                }
                continue;
            }
            if python_failed != rust_failed {
                differences.push(Divergence {
                    engine: (*engine).to_owned(),
                    item: name.to_owned(),
                    field: "media writer outcome receipt".to_owned(),
                    python: if python_failed {
                        "expected failure"
                    } else {
                        "output"
                    }
                    .to_owned(),
                    rust: if rust_failed { "failure" } else { "output" }.to_owned(),
                });
            }
        }
        for engine in expected_failures {
            if !python.errors.contains_key(*engine) {
                differences.push(Divergence {
                    engine: (*engine).to_owned(),
                    item: name.to_owned(),
                    field: "retained frozen source-failure receipt".to_owned(),
                    python: "writer unexpectedly succeeded".to_owned(),
                    rust: "expected frozen failure receipt".to_owned(),
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

fn collect_bbq_files(directory: &Path, inputs: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot read parity corpus {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(display_error)?;
        let path = entry.path();
        if path.is_dir() {
            collect_bbq_files(&path, inputs)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("bbq-") && name.ends_with("-questions.txt"))
        {
            inputs.push(path);
        }
    }
    Ok(())
}

fn run_python_writer(
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
            "pinned Python writer failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    serde_json::from_slice(&result.stdout).map_err(|error| {
        format!(
            "pinned Python writer returned invalid JSON: {error}; output: {}",
            String::from_utf8_lossy(&result.stdout)
        )
    })
}

fn run_native_writers(
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

fn compare_outputs(
    repository: &Path,
    python_root: &Path,
    python_output: &Path,
    rust_output: &Path,
    html_to_image: bool,
    engines: &[&str],
    canvas_multifib_repair: bool,
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
    if canvas_multifib_repair {
        command.arg("--canvas-multifib-repair");
    }
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

fn python_command(repository: &Path, python_root: &Path, mode: &str) -> Result<Command, String> {
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(command)
}

fn pinned_python_root(repository: &Path) -> Result<PathBuf, String> {
    let path = repository
        .join("output_tables/oracle_snapshot")
        .join(PINNED_PYTHON_HEAD);
    if path.join("qti_package_maker").is_dir() {
        Ok(path)
    } else {
        Err(format!(
            "pinned Python oracle is unavailable: {}; run cargo xtask oracle-crosscheck first",
            path.display()
        ))
    }
}

fn native_cli(repository: &Path) -> Result<PathBuf, String> {
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

fn file_sha256(path: &Path) -> Result<String, String> {
    let content = fs::read(path).map_err(display_error)?;
    Ok(Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn repository_root() -> Result<PathBuf, String> {
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

fn absolute_path(path: &Path, repository: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    }
}

fn run_command(mut command: Command, label: &str) -> Result<(), String> {
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

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
