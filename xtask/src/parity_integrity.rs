//! Package integrity accounting for the differential parity harness.

use super::{Divergence, ZIP_ENGINES};
use qti_integrity::Severity;
use qti_native::check_package_path;
use std::collections::BTreeMap;
use std::path::Path;

fn zip_integrity_errors(directory: &Path, engines: &[&str]) -> BTreeMap<&'static str, Vec<String>> {
    let mut results = BTreeMap::new();
    for engine in ZIP_ENGINES
        .iter()
        .copied()
        .filter(|engine| engines.contains(engine))
    {
        let path = directory.join(format!("{engine}.zip"));
        let errors = check_package_path(&path)
            .into_iter()
            .filter(|finding| finding.severity == Severity::Error)
            .map(|finding| format!("{}:{}", finding.code, finding.path))
            .collect::<Vec<_>>();
        results.insert(engine, errors);
    }
    results
}
/// Compare produced ZIP integrity findings without source-specific repairs.
pub(super) fn compare_zip_integrity(
    _input: &Path,
    python_directory: &Path,
    rust_directory: &Path,
    engines: &[&str],
) -> Result<Vec<Divergence>, String> {
    let python = zip_integrity_errors(python_directory, engines);
    let rust = zip_integrity_errors(rust_directory, engines);
    let mut differences = Vec::new();
    for engine in ZIP_ENGINES
        .iter()
        .copied()
        .filter(|engine| engines.contains(engine))
    {
        let python_errors = python.get(engine).expect("selected ZIP engine is present");
        let rust_errors = rust.get(engine).expect("selected ZIP engine is present");
        if !python_errors.is_empty() || !rust_errors.is_empty() {
            differences.push(Divergence {
                engine: engine.to_owned(),
                item: "package".to_owned(),
                field: "qti-integrity".to_owned(),
                python: python_errors.join(", "),
                rust: rust_errors.join(", "),
            });
        }
    }
    Ok(differences)
}
