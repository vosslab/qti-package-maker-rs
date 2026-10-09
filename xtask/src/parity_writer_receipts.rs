//! Writer outcome comparison for the M13 differential parity harness.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::Divergence;

pub(super) fn parity_lane(engine: &str) -> &'static str {
    match engine {
        "text2qti" | "okla_chrst_bqgen" | "exam_yaml" => "registered-only adapter",
        _ => "current Python CLI",
    }
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct WriterReceipt {
    pub(super) outputs: BTreeMap<String, String>,
    pub(super) errors: BTreeMap<String, String>,
    pub(super) no_output: BTreeMap<String, String>,
}

pub(super) fn comparable_engines<'a>(
    engines: &'a [&'a str],
    python: &WriterReceipt,
    rust: &WriterReceipt,
) -> Vec<&'a str> {
    engines
        .iter()
        .copied()
        .filter(|engine| python.outputs.contains_key(*engine) && rust.outputs.contains_key(*engine))
        .collect()
}

pub(super) fn compare_writer_outcomes(
    input: &Path,
    engines: &[&str],
    python: &WriterReceipt,
    rust: &WriterReceipt,
) -> Vec<Divergence> {
    let mut divergences = Vec::new();
    for engine in engines {
        let python_outcome = outcome(python, engine);
        let rust_outcome = outcome(rust, engine);
        match (python_outcome, rust_outcome) {
            (Ok(python_outcome), Ok(rust_outcome))
                if python_outcome.0 == rust_outcome.0
                    && (python_outcome.0 != "failure" || python_outcome.1 == rust_outcome.1) => {}
            (Ok(python_outcome), Ok(rust_outcome)) => divergences.push(Divergence {
                engine: (*engine).to_owned(),
                item: input.display().to_string(),
                field: "writer outcome".to_owned(),
                python: format!("{}: {}", python_outcome.0, python_outcome.1),
                rust: format!("{}: {}", rust_outcome.0, rust_outcome.1),
            }),
            (Err(python), Err(rust)) => divergences.push(Divergence {
                engine: (*engine).to_owned(),
                item: input.display().to_string(),
                field: "invalid writer receipt".to_owned(),
                python,
                rust,
            }),
            (Err(python), Ok(rust)) => divergences.push(Divergence {
                engine: (*engine).to_owned(),
                item: input.display().to_string(),
                field: "invalid writer receipt".to_owned(),
                python,
                rust: format!("{}: {}", rust.0, rust.1),
            }),
            (Ok(python), Err(rust)) => divergences.push(Divergence {
                engine: (*engine).to_owned(),
                item: input.display().to_string(),
                field: "invalid writer receipt".to_owned(),
                python: format!("{}: {}", python.0, python.1),
                rust,
            }),
        }
    }
    divergences
}

fn outcome<'a>(
    receipt: &'a WriterReceipt,
    engine: &str,
) -> Result<(&'static str, &'a str), String> {
    let outcomes = [
        receipt
            .outputs
            .get(engine)
            .map(|detail| ("output", detail.as_str())),
        receipt
            .no_output
            .get(engine)
            .map(|detail| ("no_output", detail.as_str())),
        receipt
            .errors
            .get(engine)
            .map(|detail| ("failure", detail.as_str())),
    ];
    let mut present = outcomes.into_iter().flatten();
    let first = present
        .next()
        .ok_or_else(|| "missing writer outcome".to_owned())?;
    if present.next().is_some() {
        return Err("multiple writer outcome categories".to_owned());
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_conflicting_outcomes_never_certify_parity() {
        let mut receipt = WriterReceipt::default();
        let input = Path::new("source.txt");
        assert_eq!(
            compare_writer_outcomes(input, &["human_readable"], &receipt, &receipt).len(),
            1
        );
        receipt
            .outputs
            .insert("human_readable".to_owned(), "output.html".to_owned());
        receipt
            .no_output
            .insert("human_readable".to_owned(), "no artifact".to_owned());
        assert_eq!(
            compare_writer_outcomes(input, &["human_readable"], &receipt, &receipt).len(),
            1
        );
    }

    #[test]
    fn successful_no_output_is_distinct_from_a_created_artifact() {
        let mut absent = WriterReceipt::default();
        absent
            .no_output
            .insert("human_readable".to_owned(), "no artifact".to_owned());
        let mut created = WriterReceipt::default();
        created
            .outputs
            .insert("human_readable".to_owned(), "output.html".to_owned());
        let input = Path::new("source.txt");
        assert!(compare_writer_outcomes(input, &["human_readable"], &absent, &absent).is_empty());
        assert_eq!(
            compare_writer_outcomes(input, &["human_readable"], &absent, &created).len(),
            1
        );
    }
}
