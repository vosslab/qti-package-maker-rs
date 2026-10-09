//! Cross-language CRC identity sweep over real Blackboard text-upload question corpora.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use qti_core::{Item, ItemBody, get_crc16_from_string};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OracleOutput {
    records: Vec<OracleRecord>,
    errors: Vec<OracleError>,
    synthetic_records: usize,
}

#[derive(Debug, Deserialize)]
struct OracleError {
    path: String,
    line: usize,
    error: String,
}

#[derive(Debug, Deserialize)]
struct OracleRecord {
    path: String,
    line: usize,
    input: RawItem,
    python: PythonItem,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    question: String,
    body: ItemBody,
}

#[derive(Debug, Deserialize)]
struct PythonItem {
    item_type: String,
    item_crc16: String,
    question_crc16: String,
    secondary_crc16: String,
    fields: serde_json::Value,
}

/// Compare Rust item construction with the current Python implementation over available corpora.
///
/// The command accepts optional `--biology-problems PATH` and `--python-qti PATH` overrides.
/// Corpus discovery uses existing BBQ files only: it never regenerates questions or mutates either
/// neighboring checkout. Identity errors stop the sweep; field differences are reported after
/// checking every identity so a display-normalization difference cannot conceal CRC regressions.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    let options = Options::parse(arguments, &repository)?;
    let python = crate::current_python::resolve(&repository, Some(&options.python_qti))?;
    let files = collect_corpus_files(&options.biology_problems, &repository)?;
    if files.is_empty() {
        return Err(format!(
            "crc corpus is empty; expected BBQ files below {} or {}/output_tables/corpus",
            options.biology_problems.display(),
            repository.display()
        ));
    }
    let oracle = run_python_oracle(&repository, &python.root, &files)?;
    if let Some(error) = oracle.errors.first() {
        return Err(format!(
            "Python corpus parse error at {}:{}: {} ({} parsed records before failure)",
            error.path,
            error.line,
            error.error,
            oracle.records.len()
        ));
    }
    let expected_kinds = ["MC", "MA", "MATCH", "NUM", "FIB", "MULTI_FIB", "ORDER"];
    let observed_kinds = oracle
        .records
        .iter()
        .map(|record| record.python.item_type.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(missing) = expected_kinds
        .iter()
        .find(|kind| !observed_kinds.contains(**kind))
    {
        return Err(format!("CRC corpus lacked required item shape: {missing}"));
    }
    let mut first_field_difference = None;
    let mut field_differences = 0;
    for record in &oracle.records {
        if let Some(difference) = compare_record(record)? {
            field_differences += 1;
            first_field_difference.get_or_insert(difference);
        }
    }
    if let Some(difference) = first_field_difference {
        return Err(format!(
            "all {} item identities agree across {} files and {} synthetic cases; \
             {field_differences} normalized-field differences remain; first: {difference}",
            oracle.records.len(),
            files.len(),
            oracle.synthetic_records,
        ));
    }
    println!(
        "crc corpus: {} files, {} Python items, {} synthetic shape cases, all identities agree",
        files.len(),
        oracle.records.len(),
        oracle.synthetic_records
    );
    Ok(())
}

struct Options {
    biology_problems: PathBuf,
    python_qti: PathBuf,
}

impl Options {
    fn parse(arguments: &[String], repository: &Path) -> Result<Self, String> {
        let mut biology_problems = repository.join("..").join("biology-problems");
        let mut python_qti = repository.join("..").join("qti-package-maker");
        let mut iterator = arguments.iter();
        while let Some(argument) = iterator.next() {
            match argument.as_str() {
                "--biology-problems" => biology_problems = next_path(&mut iterator, argument)?,
                "--python-qti" => python_qti = next_path(&mut iterator, argument)?,
                "--help" | "-h" => {
                    return Err("Usage: cargo xtask crc-corpus [--biology-problems PATH] [--python-qti PATH]".to_owned());
                }
                _ => return Err(format!("unknown crc-corpus argument: {argument}")),
            }
        }
        if !python_qti.join("source_me.sh").is_file() {
            return Err(format!(
                "Python qti-package-maker checkout is unavailable: {}",
                python_qti.display()
            ));
        }
        Ok(Self {
            biology_problems,
            python_qti,
        })
    }
}

fn next_path<'a>(
    iterator: &mut impl Iterator<Item = &'a String>,
    flag: &str,
) -> Result<PathBuf, String> {
    iterator
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask crc-corpus must run inside the Rust workspace".to_owned());
    }
    String::from_utf8(output.stdout)
        .map_err(display_error)
        .map(|path| PathBuf::from(path.trim()))
}

fn collect_corpus_files(biology: &Path, repository: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = BTreeSet::new();
    if biology.is_dir() {
        collect_bbq_files(biology, &mut files)?;
    }
    let corpus = repository.join("output_tables").join("corpus");
    let generated = corpus.join("generated");
    if generated.is_dir() {
        collect_bbq_files(&generated, &mut files)?;
    }
    collect_manifest_files(&corpus.join("manifest.json"), &mut files)?;
    Ok(files.into_iter().collect())
}

fn collect_bbq_files(directory: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(display_error)? {
        let entry = entry.map_err(display_error)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(display_error)?;
        if file_type.is_dir() {
            collect_bbq_files(&path, files)?;
        } else if file_type.is_file()
            && path.extension().is_some_and(|extension| extension == "txt")
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("bbq-"))
        {
            files.insert(path.canonicalize().map_err(display_error)?);
        }
    }
    Ok(())
}

fn collect_manifest_files(manifest: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    if !manifest.is_file() {
        return Ok(());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(manifest).map_err(display_error)?)
            .map_err(display_error)?;
    let paths = value["bbq_files"].as_array().ok_or_else(|| {
        format!(
            "corpus manifest has no bbq_files array: {}",
            manifest.display()
        )
    })?;
    for path in paths.iter().filter_map(serde_json::Value::as_str) {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(format!(
                "corpus manifest names a missing BBQ file: {}",
                path.display()
            ));
        }
        files.insert(path.canonicalize().map_err(display_error)?);
    }
    Ok(())
}

fn run_python_oracle(
    repository: &Path,
    python_snapshot: &Path,
    files: &[PathBuf],
) -> Result<OracleOutput, String> {
    let helper = repository
        .join("xtask")
        .join("support")
        .join("crc_oracle.py");
    let command = format!(
        "source {} >/dev/null && PYTHONPATH={}:$PYTHONPATH python3 {} \"$@\"",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(python_snapshot),
        shell_quote(&helper),
    );
    let output = Command::new("bash")
        .args(["-c", &command, "crc_oracle"])
        .args(files)
        .current_dir(repository)
        .env("QTI_ORACLE_ROOT", python_snapshot)
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "Python CRC oracle failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Python CRC oracle returned invalid JSON: {error}"))
}

fn compare_record(record: &OracleRecord) -> Result<Option<String>, String> {
    let item =
        Item::new(record.input.question.clone(), record.input.body.clone()).map_err(|error| {
            disagreement(
                record,
                "Rust Item::new",
                &error.to_string(),
                "Python accepted item",
            )
        })?;
    let rust_crc = item.crc().to_string();
    if rust_crc != record.python.item_crc16 {
        return Err(disagreement(
            record,
            "item CRC",
            &rust_crc,
            &record.python.item_crc16,
        ));
    }
    let rust_question_crc =
        get_crc16_from_string(&item.common().question_text).map_err(|error| {
            disagreement(
                record,
                "Rust question CRC",
                &error.to_string(),
                &record.python.question_crc16,
            )
        })?;
    if rust_question_crc != record.python.question_crc16 {
        return Err(disagreement(
            record,
            "question CRC",
            &rust_question_crc,
            &record.python.question_crc16,
        ));
    }
    let rust_secondary_crc = format!("{:04x}", item.crc().secondary_crc());
    if rust_secondary_crc != record.python.secondary_crc16 {
        return Err(disagreement(
            record,
            "secondary CRC",
            &rust_secondary_crc,
            &record.python.secondary_crc16,
        ));
    }
    let rust_fields = rust_fields(&item);
    if rust_fields != record.python.fields {
        return Ok(Some(disagreement(
            record,
            "normalized item fields",
            &rust_fields.to_string(),
            &record.python.fields.to_string(),
        )));
    }
    Ok(None)
}

fn rust_fields(item: &Item) -> serde_json::Value {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "question_text".to_owned(),
        serde_json::Value::String(item.common().question_text.clone()),
    );
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            fields.insert("choices_list".to_owned(), json_strings(choices));
            fields.insert(
                "answer_text".to_owned(),
                serde_json::Value::String(answer.clone()),
            );
        }
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => {
            fields.insert("choices_list".to_owned(), json_strings(choices));
            fields.insert("answers_list".to_owned(), json_strings(answers));
            fields.insert(
                "min_answers_required".to_owned(),
                (*min_answers_required).into(),
            );
            fields.insert("allow_all_correct".to_owned(), (*allow_all_correct).into());
        }
        ItemBody::Match { prompts, choices } => {
            fields.insert("prompts_list".to_owned(), json_strings(prompts));
            fields.insert("choices_list".to_owned(), json_strings(choices));
        }
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            fields.insert("answer_float".to_owned(), serde_json::json!(answer));
            fields.insert("tolerance_float".to_owned(), serde_json::json!(tolerance));
            fields.insert("tolerance_message".to_owned(), (*tolerance_message).into());
        }
        ItemBody::Fib { answers } => {
            fields.insert("answers_list".to_owned(), json_strings(answers));
        }
        ItemBody::MultiFib { answers } => {
            fields.insert("answer_map".to_owned(), serde_json::json!(answers));
        }
        ItemBody::Order { answers } => {
            fields.insert("ordered_answers_list".to_owned(), json_strings(answers));
        }
    }
    serde_json::Value::Object(fields)
}

fn json_strings(values: &[String]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect(),
    )
}

fn disagreement(record: &OracleRecord, field: &str, rust: &str, python: &str) -> String {
    format!(
        "CRC parity disagreement at {}:{} ({}) for {field}: Rust={rust:?}; Python={python:?}",
        record.path, record.line, record.python.item_type
    )
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

    #[test]
    fn rust_fields_preserve_normalized_prefixed_choice_values() {
        let item = Item::new(
            "Question stem?".to_owned(),
            ItemBody::Mc {
                choices: vec!["A. one".to_owned(), "B. two".to_owned()],
                answer: "A. one".to_owned(),
            },
        )
        .expect("valid inline item");
        assert_eq!(
            rust_fields(&item)["choices_list"],
            serde_json::json!(["one", "two"])
        );
        assert_eq!(item.crc().to_string().split('_').nth(1), Some("574a"));
    }

    #[test]
    fn missing_manifest_is_optional_but_malformed_manifest_is_an_error() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut files = BTreeSet::new();
        collect_manifest_files(&directory.path().join("missing.json"), &mut files)
            .expect("missing manifest is optional");
        fs::write(directory.path().join("manifest.json"), "{}").expect("write inline manifest");
        assert!(
            collect_manifest_files(&directory.path().join("manifest.json"), &mut files).is_err()
        );
    }
}
