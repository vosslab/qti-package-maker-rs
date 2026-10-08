//! Parsed command surfaces and their owned application flow.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use qti_core::{ItemBank, ItemKind};
use qti_engines::html_to_image::{ChromiumFragmentRenderer, RenderCache, convert_bank};
use qti_engines::{DocumentMetadata, ENGINES, EngineEntry, EngineOptions, MediaWarning, Writer};
use qti_integrity::{Severity, check_package};

use crate::CliError;

const BBQ_READER: &str = "bbq_text_upload";
const HTML_TO_IMAGE_ENGINES: &[&str] = &[
    "canvas_qti_v1_2",
    "blackboard_qti_v2_1",
    "blackboard_export_zip",
    "ple_native_json",
];

/// `bbq-converter`, matching the established Python entry point.
#[derive(Clone, Debug, Parser)]
#[command(
    name = "bbq-converter",
    about = "Convert BBQ questions to assessment formats"
)]
pub struct BbqConverterArgs {
    /// Source BBQ text file.
    #[arg(short = 'i', long, visible_alias = "input_file")]
    pub input: PathBuf,
    /// Explicit output path, permitted only with one selected format.
    #[arg(short = 'o', long, visible_alias = "output_file")]
    pub output: Option<PathBuf>,
    /// Retain at most this many input items.
    #[arg(short = 'n', long = "limit", visible_alias = "question_limit")]
    pub question_limit: Option<usize>,
    /// Suppress progress output.
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Print progress output. This is the legacy default.
    #[arg(short = 'v', long, conflicts_with = "quiet")]
    pub verbose: bool,
    /// Allow multiple assessment kinds while reading.
    #[arg(long = "allow-mixed")]
    pub allow_mixed: bool,
    /// Render HTML tables with Chromium and static RDKit canvases as PNGs.
    #[arg(long = "html-to-image")]
    pub html_to_image: bool,
    /// Select an engine by exact name or a unique registry prefix; repeatable.
    #[arg(short = 'f', long = "format")]
    pub formats: Vec<String>,
    /// Select every registered writer.
    #[arg(short = 'a', long = "all")]
    pub all: bool,
    /// Canvas QTI 1.2.
    #[arg(short = '1', long = "qti12", visible_alias = "canvas_qti_v1_2")]
    pub qti12: bool,
    /// Blackboard QTI 2.1.
    #[arg(short = '2', long = "qti21", visible_alias = "blackboard_qti_v2_1")]
    pub qti21: bool,
    /// Human-readable HTML.
    #[arg(short = 'r', long = "human", visible_alias = "human_readable")]
    pub human: bool,
    /// Blackboard Questions text upload.
    #[arg(short = 'b', long = "bbq", visible_alias = "bbq_text_upload")]
    pub bbq: bool,
    /// Standalone HTML self-test.
    #[arg(short = 's', long = "selftest", visible_alias = "html_selftest")]
    pub selftest: bool,
    /// Moodle Aiken text.
    #[arg(short = 'A', long = "aiken", visible_alias = "moodle_aiken")]
    pub aiken: bool,
    /// Blackboard Original pool export ZIP.
    #[arg(
        short = 'B',
        long = "bbexport",
        visible_alias = "blackboard_export_zip"
    )]
    pub bbexport: bool,
}

/// `qti-package-maker`, the native inspection command.
#[derive(Debug, Parser)]
#[command(
    name = "qti-package-maker",
    about = "Inspect QTI package-maker capabilities"
)]
pub struct PackageMakerArgs {
    #[command(subcommand)]
    pub command: PackageMakerCommand,
}

/// The supported package-maker inspection commands.
#[derive(Debug, Subcommand)]
pub enum PackageMakerCommand {
    /// List registered format engines and read/write capabilities.
    Engines,
    /// List the seven validated assessment item kinds.
    ItemTypes,
    /// Check a finished ZIP or extracted package for integrity violations.
    Check { path: PathBuf },
}

/// A top-level parser used by either process adapter.
#[derive(Debug, Parser)]
#[command(name = "qti-cli")]
pub enum Cli {
    /// BBQ conversion command.
    BbqConverter(BbqConverterArgs),
    /// Inspection command.
    PackageMaker(PackageMakerArgs),
}

/// Observable output from one complete BBQ conversion run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConverterReport {
    /// The source-derived content name used for package and document filenames.
    pub content_name: String,
    /// Writers selected by the invocation, including completed writers without an artifact.
    pub attempted_outputs: usize,
    /// Successful shared native HTML-to-image passes performed before writer fan-out.
    pub conversion_passes: u8,
    /// Legacy progress messages, present for the default and `-v` modes only.
    pub progress: Vec<String>,
    /// Completed output paths in requested engine order.
    pub outputs: Vec<PathBuf>,
    /// Recoverable media diagnostics in writer and item order.
    pub warnings: Vec<MediaWarning>,
}

/// Parses and runs `bbq-converter` from arbitrary testable argument values.
pub fn run_bbq_converter_from<I, T>(arguments: I) -> Result<ConverterReport, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args =
        BbqConverterArgs::try_parse_from(arguments).map_err(|error| CliError::Arguments {
            message: error.to_string(),
        })?;
    run_bbq_converter(args)
}

/// Reads a BBQ bank, performs one optional native conversion pass, and writes every format.
pub fn run_bbq_converter(args: BbqConverterArgs) -> Result<ConverterReport, CliError> {
    let selected = selected_engines(&args)?;
    validate_output_and_conversion(&args, &selected)?;
    let content_name = extract_content_name(&args.input)?;
    let options = engine_options_for_content_name(&content_name)?;
    let mut bank = read_bbq_bank(&args.input, args.allow_mixed)?;
    let loaded_items = bank.len();
    let verbose = !args.quiet;
    let mut progress = Vec::new();
    if verbose {
        progress.push(format!("Initialized Engine: {BBQ_READER} ({BBQ_READER})"));
        progress.push(format!(
            "Successfully loaded {loaded_items} new assessment items from {}.\nThe item bank now contains a total of {loaded_items} unique assessment items.",
            args.input.display()
        ));
    }
    if let Some(limit) = args.question_limit {
        bank.trim_to(limit);
    }

    let (converted, conversion_passes) =
        if args.html_to_image && selected.iter().any(|entry| is_html_to_image_engine(entry)) {
            let renderer = ChromiumFragmentRenderer::default();
            (
                Some(convert_bank(&bank, &renderer, &RenderCache::new())?),
                1,
            )
        } else {
            (None, 0)
        };
    if verbose && args.html_to_image {
        progress.push(format!(
            "Native HTML-to-image conversion passes: {conversion_passes}"
        ));
    }
    let attempted_outputs = selected.len();
    let mut outputs = Vec::with_capacity(selected.len());
    let mut warnings = Vec::new();
    for entry in selected {
        if verbose {
            progress.push(format!(
                "Initialized Engine: {} ({})",
                entry.name, entry.name
            ));
            progress.push(format!(
                "Saving package {}\n  with {} assessment items.",
                entry.name,
                bank.len()
            ));
        }
        let writer = writer_for(entry, options.clone())?;
        let output = args
            .output
            .as_deref()
            .map_or_else(|| output_name(entry.name, &content_name), Path::to_path_buf);
        // Conversion has already completed once. Writers always receive false so a direct-writer
        // implementation cannot duplicate conversion inside this fan-out loop.
        let output_bank = if args.html_to_image && is_html_to_image_engine(entry) {
            converted
                .as_ref()
                .expect("package engine implies converted bank")
        } else {
            &bank
        };
        let outcome = writer.save_package(output_bank, Some(&output))?;
        if let Some(path) = outcome.path {
            if verbose {
                progress.push(format!(
                    "Saved {} assessment items to {}",
                    bank.len(),
                    path.display()
                ));
            }
            outputs.push(path);
        }
        warnings.extend(outcome.warnings);
    }
    Ok(ConverterReport {
        content_name,
        attempted_outputs,
        conversion_passes,
        progress,
        outputs,
        warnings,
    })
}

/// Parses and runs `qti-package-maker` from arbitrary testable argument values.
pub fn run_package_maker_from<I, T>(arguments: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args =
        PackageMakerArgs::try_parse_from(arguments).map_err(|error| CliError::Arguments {
            message: error.to_string(),
        })?;
    run_package_maker(args)
}

/// Runs an inspection command and returns stable, printable output.
pub fn run_package_maker(args: PackageMakerArgs) -> Result<String, CliError> {
    match args.command {
        PackageMakerCommand::Engines => Ok(ENGINES
            .iter()
            .map(|entry| {
                format!(
                    "{}\tread={}\twrite={}\tmedia={:?}",
                    entry.name,
                    entry.can_read(),
                    entry.can_write(),
                    entry.media_policy
                )
            })
            .collect::<Vec<_>>()
            .join("\n")),
        PackageMakerCommand::ItemTypes => Ok(item_kind_names().join("\n")),
        PackageMakerCommand::Check { path } => {
            let violations = check_package(path);
            if violations.is_empty() {
                return Ok("OK".to_owned());
            }
            Ok(violations
                .iter()
                .map(|violation| {
                    format!(
                        "{:?}\t{}\t{}\t{}",
                        violation.severity, violation.code, violation.path, violation.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
    }
}

/// Returns whether integrity findings require a nonzero process exit.
#[must_use]
pub fn package_check_failed(path: &Path) -> bool {
    check_package(path)
        .iter()
        .any(|violation| violation.severity == Severity::Error)
}

fn selected_engines(args: &BbqConverterArgs) -> Result<Vec<&'static EngineEntry>, CliError> {
    if args.all {
        return Ok(ENGINES.iter().filter(|entry| entry.can_write()).collect());
    }
    let mut requested = args.formats.clone();
    for (selected, name) in [
        (args.qti12, "canvas_qti_v1_2"),
        (args.qti21, "blackboard_qti_v2_1"),
        (args.human, "human_readable"),
        (args.bbq, "bbq_text_upload"),
        (args.selftest, "html_selftest"),
        (args.aiken, "moodle_aiken"),
        (args.bbexport, "blackboard_export_zip"),
    ] {
        if selected {
            requested.push(name.to_owned());
        }
    }
    if requested.is_empty() {
        return Err(CliError::Arguments {
            message: "at least one output format must be specified with -f, -a, or a shortcut"
                .to_owned(),
        });
    }
    let mut selected = Vec::new();
    for name in requested {
        let entry = resolve_engine(&name)?;
        if !selected
            .iter()
            .any(|existing: &&EngineEntry| existing.name == entry.name)
        {
            selected.push(entry);
        }
    }
    Ok(selected)
}

fn resolve_engine(requested: &str) -> Result<&'static EngineEntry, CliError> {
    if let Some(entry) = ENGINES.iter().find(|entry| entry.name == requested) {
        return Ok(entry);
    }
    let matches = ENGINES
        .iter()
        .filter(|entry| entry.name.starts_with(requested))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => Err(CliError::UnknownEngine {
            requested: requested.to_owned(),
            candidates: engine_names().join(", "),
        }),
        _ => Err(CliError::AmbiguousEngine {
            requested: requested.to_owned(),
            candidates: matches
                .iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

fn validate_output_and_conversion(
    args: &BbqConverterArgs,
    selected: &[&EngineEntry],
) -> Result<(), CliError> {
    if args.output.is_some() && selected.len() != 1 {
        return Err(CliError::Arguments {
            message: "--output can only be used with one output format".to_owned(),
        });
    }
    if args.html_to_image
        && args.output.is_some()
        && !selected
            .first()
            .is_some_and(|entry| is_html_to_image_engine(entry))
    {
        return Err(CliError::Arguments {
            message: format!(
                "--html-to-image applies only to supported output formats ({}) when --output is supplied",
                HTML_TO_IMAGE_ENGINES.join(", ")
            ),
        });
    }
    Ok(())
}

fn is_html_to_image_engine(entry: &EngineEntry) -> bool {
    HTML_TO_IMAGE_ENGINES.contains(&entry.name)
}

fn read_bbq_bank(path: &Path, allow_mixed: bool) -> Result<ItemBank, CliError> {
    let entry = resolve_engine(BBQ_READER)?;
    let reader = entry
        .make_reader
        .ok_or_else(|| CliError::ReaderUnavailable {
            engine: entry.name.to_owned(),
        })?(EngineOptions::default());
    Ok(reader.read_items(path, allow_mixed)?.bank)
}

fn writer_for(entry: &EngineEntry, options: EngineOptions) -> Result<Box<dyn Writer>, CliError> {
    entry
        .make_writer
        .ok_or_else(|| CliError::WriterUnavailable {
            engine: entry.name.to_owned(),
        })
        .map(|factory| factory(options))
}

fn extract_content_name(input: &Path) -> Result<String, CliError> {
    let Some(file_name) = input.file_name().and_then(|name| name.to_str()) else {
        return Err(CliError::InputName {
            path: input.to_path_buf(),
        });
    };
    let Some(name) = file_name
        .strip_prefix("bbq-")
        .and_then(|name| name.strip_suffix("-questions.txt"))
    else {
        return Err(CliError::InputName {
            path: input.to_path_buf(),
        });
    };
    if name.is_empty() {
        return Err(CliError::InputName {
            path: input.to_path_buf(),
        });
    }
    Ok(name.to_owned())
}

fn output_name(engine: &str, content_name: &str) -> PathBuf {
    if engine == "ple_native_json" {
        return PathBuf::from(format!("ple-{content_name}"));
    }
    let (prefix, extension) = match engine {
        "canvas_qti_v1_2" => ("qti12", "zip"),
        "blackboard_qti_v2_1" => ("qti21", "zip"),
        "human_readable" => ("human", "html"),
        "bbq_text_upload" => ("bbq", "txt"),
        "html_selftest" => ("selftest", "html"),
        "moodle_aiken" => ("aiken", "txt"),
        "blackboard_export_zip" => ("bez", "zip"),
        "exam_yaml" => ("exam", "yaml"),
        "okla_chrst_bqgen" => ("okla", "txt"),
        "text2qti" => ("text2qti", "txt"),
        _ => unreachable!("every engine has an output naming contract"),
    };
    PathBuf::from(format!("{prefix}-{content_name}.{extension}"))
}

fn engine_names() -> Vec<&'static str> {
    ENGINES.iter().map(|entry| entry.name).collect()
}

fn item_kind_names() -> Vec<&'static str> {
    [
        ItemKind::Mc,
        ItemKind::Ma,
        ItemKind::Match,
        ItemKind::Num,
        ItemKind::Fib,
        ItemKind::MultiFib,
        ItemKind::Order,
    ]
    .into_iter()
    .map(|kind| match kind {
        ItemKind::Mc => "MC",
        ItemKind::Ma => "MA",
        ItemKind::Match => "MATCH",
        ItemKind::Num => "NUM",
        ItemKind::Fib => "FIB",
        ItemKind::MultiFib => "MULTI_FIB",
        ItemKind::Order => "ORDER",
    })
    .collect()
}

fn engine_options_for_content_name(content_name: &str) -> Result<EngineOptions, CliError> {
    let local_date = time::OffsetDateTime::now_local()
        .map_err(|error| CliError::LocalDate {
            message: error.to_string(),
        })?
        .date()
        .to_string();
    Ok(EngineOptions {
        html_to_image: false,
        document: DocumentMetadata {
            title: content_name.to_owned(),
            date: local_date,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        BbqConverterArgs, extract_content_name, output_name, run_bbq_converter,
        run_bbq_converter_from, run_package_maker_from,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn derives_legacy_content_and_every_output_name() {
        assert_eq!(
            extract_content_name(Path::new("nested/bbq-genetics-questions.txt"))
                .expect("valid name"),
            "genetics"
        );
        assert_eq!(
            extract_content_name(Path::new("bbq-parity-mc-only-questions.txt"))
                .expect("the frozen Python regular expression retains every core-name segment"),
            "parity-mc-only"
        );
        let names = [
            ("canvas_qti_v1_2", "qti12-genetics.zip"),
            ("blackboard_qti_v2_1", "qti21-genetics.zip"),
            ("human_readable", "human-genetics.html"),
            ("bbq_text_upload", "bbq-genetics.txt"),
            ("html_selftest", "selftest-genetics.html"),
            ("moodle_aiken", "aiken-genetics.txt"),
            ("blackboard_export_zip", "bez-genetics.zip"),
            ("exam_yaml", "exam-genetics.yaml"),
            ("okla_chrst_bqgen", "okla-genetics.txt"),
            ("text2qti", "text2qti-genetics.txt"),
            ("ple_native_json", "ple-genetics"),
        ];
        for (engine, expected) in names {
            assert_eq!(output_name(engine, "genetics"), PathBuf::from(expected));
        }
    }

    #[test]
    fn parser_rejects_multiple_formats_with_one_output() {
        let error = run_bbq_converter_from([
            "bbq-converter",
            "-i",
            "bbq-test-questions.txt",
            "-o",
            "out.zip",
            "-1",
            "-2",
        ])
        .expect_err("output conflicts with multiple formats");
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains("one output format"));
    }

    #[test]
    fn parser_reports_unknown_engine_before_opening_input() {
        let error = run_bbq_converter_from([
            "bbq-converter",
            "-i",
            "bbq-test-questions.txt",
            "-f",
            "not-a-format",
        ])
        .expect_err("unknown engine");
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains("unknown engine 'not-a-format'"));
    }

    #[test]
    fn inspection_commands_list_static_capabilities_and_item_kinds() {
        let engines =
            run_package_maker_from(["qti-package-maker", "engines"]).expect("engine list");
        assert!(engines.contains("blackboard_export_zip\tread=true\twrite=true"));
        let kinds =
            run_package_maker_from(["qti-package-maker", "item-types"]).expect("item kinds");
        assert_eq!(kinds.lines().count(), 7);
        assert!(kinds.contains("MULTI_FIB"));
    }

    #[test]
    fn reads_and_writes_a_blackboard_package_at_an_explicit_output_path() {
        let directory = tempfile::tempdir().expect("temporary output directory");
        let input = directory.path().join("bbq-cli-contract-questions.txt");
        let output = directory.path().join("pool.zip");
        std::fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("input fixture");
        let report = run_bbq_converter(BbqConverterArgs {
            input,
            output: Some(output.clone()),
            question_limit: None,
            quiet: true,
            verbose: false,
            allow_mixed: false,
            html_to_image: false,
            formats: vec!["blackboard_export_zip".to_owned()],
            all: false,
            qti12: false,
            qti21: false,
            human: false,
            bbq: false,
            selftest: false,
            aiken: false,
            bbexport: false,
        })
        .expect("Blackboard writer accepts a multiple-choice bank");
        assert_eq!(report.outputs, [output]);
        assert!(report.warnings.is_empty());
        assert!(qti_integrity::check_package(&report.outputs[0]).is_empty());
    }

    #[test]
    fn exam_yaml_receives_the_python_package_name_as_document_metadata() {
        let directory = tempfile::tempdir().expect("temporary output directory");
        let input = directory.path().join("bbq-genetics-questions.txt");
        let output = directory.path().join("exam.yaml");
        std::fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("input fixture");
        run_bbq_converter(BbqConverterArgs {
            input,
            output: Some(output.clone()),
            question_limit: None,
            quiet: true,
            verbose: false,
            allow_mixed: false,
            html_to_image: false,
            formats: vec!["exam_yaml".to_owned()],
            all: false,
            qti12: false,
            qti21: false,
            human: false,
            bbq: false,
            selftest: false,
            aiken: false,
            bbexport: false,
        })
        .expect("exam YAML output");
        let yaml = std::fs::read_to_string(output).expect("exam YAML text");
        assert!(yaml.contains("title: genetics"));
        assert!(yaml.contains("heading: genetics"));
    }

    #[test]
    fn order_only_canvas_request_reports_no_saved_output() {
        let directory = tempfile::tempdir().expect("temporary output directory");
        let input = directory.path().join("bbq-order-questions.txt");
        let output = directory.path().join("qti12-order.zip");
        std::fs::write(&input, "ORD\tPut these in order\tfirst\tsecond\n").expect("input fixture");
        let report = run_bbq_converter(BbqConverterArgs {
            input,
            output: Some(output.clone()),
            question_limit: None,
            quiet: true,
            verbose: false,
            allow_mixed: false,
            html_to_image: false,
            formats: Vec::new(),
            all: false,
            qti12: true,
            qti21: false,
            human: false,
            bbq: false,
            selftest: false,
            aiken: false,
            bbexport: false,
        })
        .expect("ORDER is a valid input even though Canvas skips it");
        assert!(report.outputs.is_empty());
        assert!(!output.exists());
    }
}
