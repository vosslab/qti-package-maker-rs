//! Parity input selection, fixture discovery, and corpus projection.

use super::parity_process::{absolute_path, display_error, file_sha256, run_command};
use super::{CorpusProjection, ENGINES, Options, ParityInput, ZIP_ENGINES};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn parse_arguments(arguments: &[String], repository: &Path) -> Result<Options, String> {
    let mut options = Options {
        inputs: Vec::new(),
        corpus: None,
        max_inputs: None,
        html_to_image: false,
        fixtures: false,
        python_qti: None,
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
            "--python-qti" => {
                options.python_qti = Some(PathBuf::from(
                    iterator
                        .next()
                        .ok_or_else(|| "--python-qti requires a path".to_owned())?,
                ))
            }
            "--help" | "-h" => {
                return Err("Usage: cargo xtask parity [--fixtures | --input BBQ... | --corpus DIR] [--python-qti PATH] [--max-inputs N] [--html-to-image]\n\n--fixtures creates deterministic ignored M13 inputs. Without an input source, the harvested corpus at output_tables/corpus/generated is used. Generated packages are ignored under tests/_temp/.".to_owned());
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

pub(super) fn resolve_inputs(
    options: &Options,
    repository: &Path,
) -> Result<Vec<ParityInput>, String> {
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
