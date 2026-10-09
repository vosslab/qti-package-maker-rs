//! M13 differential Python-versus-Rust package comparison; the pinned Python source is oracle-only.

#[path = "parity_grading_program.rs"]
mod parity_grading_program;
#[path = "parity_inputs.rs"]
mod parity_inputs;
#[path = "parity_integrity.rs"]
mod parity_integrity;
#[path = "parity_process.rs"]
mod parity_process;
#[path = "parity_reader_receipts.rs"]
mod parity_reader_receipts;
#[path = "parity_receipts.rs"]
mod parity_receipts;
use parity_integrity::compare_zip_integrity;
#[path = "parity_report.rs"]
mod parity_report;
#[path = "parity_writer_receipts.rs"]
mod parity_writer_receipts;
use parity_inputs::{parse_arguments, resolve_inputs};
use parity_process::{
    compare_outputs, display_error, file_sha256, native_cli, pinned_python_root, python_command,
    repository_root, run_native_writers, run_python_writer,
};
use parity_receipts::{compare_media_receipts, compare_order_receipt};
use parity_writer_receipts::{comparable_engines, compare_writer_outcomes, parity_lane};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
