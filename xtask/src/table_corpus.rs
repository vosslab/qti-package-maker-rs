//! Harvest real table and RDKit canvas fragments from biology-problems generators.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FIXED_SEED: &str = "20260930";
const QUESTION_COUNT: &str = "1";

/// Regenerate `output_tables/corpus` from the neighboring biology-problems checkout.
///
/// The command accepts one optional `--biology-problems PATH` override.  It deliberately runs
/// generators, instead of searching source literals, because generated item fields are the
/// renderer's input and expose random branches, helper-produced markup, and answer-map values.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    let biology = biology_root(&repository, arguments)?;
    let biology_revision = git_revision(&biology)?;
    let output = repository.join("output_tables").join("corpus");
    recreate_directory(&output)?;
    write_seed_hook(&output)?;

    let generators = discover_generators(&biology.join("problems"))?;
    let generated_dir = output.join("generated");
    fs::create_dir_all(&generated_dir).map_err(display_error)?;
    let mut reports = Vec::new();
    let mut bbq_files = Vec::new();
    for generator in generators {
        let report = run_generator(&repository, &biology, &output, &generated_dir, &generator)?;
        bbq_files.extend(report.bbq_files.iter().cloned());
        reports.push(report);
    }

    let bbq_provenance = bbq_provenance(&reports);
    let extracted = extract_records(&repository, &biology, &bbq_files, &bbq_provenance, &output)?;
    let (fragments, provenance) = write_unique_fragments(&output, extracted)?;
    let census = census_fragments(&fragments)?;
    write_reports(
        &output,
        &biology,
        &biology_revision,
        &reports,
        &fragments,
        &provenance,
        &census,
    )?;
    println!(
        "table corpus: {} generators, {} BBQ files, {} unique tables, {} unique canvases: {}",
        reports.len(),
        bbq_files.len(),
        fragments
            .iter()
            .filter(|fragment| fragment.family == "table")
            .count(),
        fragments
            .iter()
            .filter(|fragment| fragment.family == "canvas")
            .count(),
        output.display()
    );
    Ok(())
}

#[derive(Debug)]
struct GeneratorReport {
    generator: PathBuf,
    source_sha256: String,
    arguments: Vec<String>,
    status: String,
    stderr: String,
    bbq_files: Vec<PathBuf>,
    assets: Vec<CorpusAsset>,
}

#[derive(Debug)]
struct CorpusAsset {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct Fragment {
    family: String,
    hash: String,
    content: String,
}

type CorpusFragments = Vec<Fragment>;
type Provenance = BTreeMap<String, Vec<Value>>;

fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask table-corpus must run inside the Rust workspace".to_owned());
    }
    String::from_utf8(output.stdout)
        .map_err(display_error)
        .map(|path| PathBuf::from(path.trim()))
}

fn biology_root(repository: &Path, arguments: &[String]) -> Result<PathBuf, String> {
    let mut supplied = None;
    let mut iterator = arguments.iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--biology-problems" => supplied = iterator.next().map(PathBuf::from),
            "--help" | "-h" => {
                return Err("Usage: cargo xtask table-corpus [--biology-problems PATH]".to_owned());
            }
            _ => return Err(format!("unknown table-corpus argument: {argument}")),
        }
    }
    let path = supplied.unwrap_or_else(|| repository.join("..").join("biology-problems"));
    if path.join("source_me.sh").is_file() && path.join("problems").is_dir() {
        Ok(path)
    } else {
        Err(format!(
            "biology-problems checkout is unavailable: {}",
            path.display()
        ))
    }
}

fn recreate_directory(directory: &Path) -> Result<(), String> {
    if directory.exists() {
        fs::remove_dir_all(directory).map_err(display_error)?;
    }
    fs::create_dir_all(directory).map_err(display_error)
}

fn write_seed_hook(output: &Path) -> Result<(), String> {
    fs::write(
        output.join("sitecustomize.py"),
        format!(
            "import random\nrandom.seed({FIXED_SEED})\n\ntry:\n    import numpy\nexcept ImportError:\n    pass\nelse:\n    numpy.random.seed({FIXED_SEED})\n"
        ),
    )
    .map_err(display_error)
}

fn discover_generators(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    discover_generators_at(directory, &mut result)?;
    result.sort();
    Ok(result)
}

fn discover_generators_at(directory: &Path, result: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(display_error)? {
        let path = entry.map_err(display_error)?.path();
        if path.is_dir() {
            discover_generators_at(&path, result)?;
        } else if is_question_generator(&path)? {
            result.push(path);
        }
    }
    Ok(())
}

fn is_question_generator(path: &Path) -> Result<bool, String> {
    if !path.extension().is_some_and(|extension| extension == "py")
        || path.file_name().is_some_and(|name| name == "__init__.py")
    {
        return Ok(false);
    }
    let source = fs::read_to_string(path).map_err(display_error)?;
    Ok(source.contains("import bptools") || source.contains("from bptools import"))
}

fn run_generator(
    repository: &Path,
    biology: &Path,
    work: &Path,
    generated: &Path,
    generator: &Path,
) -> Result<GeneratorReport, String> {
    let relative = generator.strip_prefix(biology).map_err(display_error)?;
    let source_sha256 = sha256_bytes(&fs::read(generator).map_err(display_error)?);
    let safe_name = relative.to_string_lossy().replace(['/', '\\', '.'], "_");
    let sandbox = work.join("runs").join(&safe_name);
    fs::create_dir_all(&sandbox).map_err(display_error)?;
    write_seed_hook(&sandbox)?;
    let runnable = mirror_generator_assets(generator, relative, &sandbox)?;
    let arguments = generator_arguments(relative)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let argument_text = arguments
        .iter()
        .map(|argument| shell_quote_text(argument))
        .collect::<Vec<_>>()
        .join(" ");
    let command = format!(
        "source {} >/dev/null && python3 {} -d {} {}",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(&runnable),
        QUESTION_COUNT,
        argument_text,
    );
    let output = Command::new("bash")
        .args(["-lc", &command])
        .current_dir(&sandbox)
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONPATH", sandbox.as_os_str())
        .env(
            "QTI_CORPUS_IMPORT_PATH",
            corpus_import_path(&sandbox, biology, generator.parent()),
        )
        .output()
        .map_err(display_error)?;
    let mut bbq_files = find_bbq_files(&sandbox)?;
    let destination = generated.join(&safe_name);
    fs::create_dir_all(&destination).map_err(display_error)?;
    let assets = copy_image_assets(&sandbox, &destination)?;
    for source in &bbq_files {
        let filename = source
            .file_name()
            .ok_or_else(|| "BBQ output had no filename".to_owned())?;
        fs::copy(source, destination.join(filename)).map_err(display_error)?;
    }
    bbq_files = find_bbq_files(&destination)?;
    let status = if output.status.success() {
        if bbq_files.is_empty() {
            "no_bbq_output"
        } else {
            "ok"
        }
    } else {
        "failed"
    }
    .to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Ok(GeneratorReport {
        generator: relative.to_path_buf(),
        source_sha256,
        arguments,
        status,
        stderr,
        bbq_files,
        assets,
    })
}

fn copy_image_assets(source: &Path, destination: &Path) -> Result<Vec<CorpusAsset>, String> {
    let mut assets = Vec::new();
    copy_image_assets_at(source, source, destination, &mut assets)?;
    Ok(assets)
}

fn copy_image_assets_at(
    root: &Path,
    source: &Path,
    destination: &Path,
    assets: &mut Vec<CorpusAsset>,
) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(display_error)? {
        let path = entry.map_err(display_error)?.path();
        if path.is_dir() {
            copy_image_assets_at(root, &path, destination, assets)?;
            continue;
        }
        if !is_image_asset(&path) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(display_error)?;
        let copied = destination.join(relative);
        let parent = copied
            .parent()
            .ok_or_else(|| "image asset had no parent directory".to_owned())?;
        fs::create_dir_all(parent).map_err(display_error)?;
        let bytes = fs::read(&path).map_err(display_error)?;
        fs::write(&copied, &bytes).map_err(display_error)?;
        assets.push(CorpusAsset {
            path: copied,
            sha256: sha256_bytes(&bytes),
        });
    }
    Ok(())
}

fn is_image_asset(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg")
    )
}

fn generator_arguments(relative: &Path) -> &'static [&'static str] {
    match relative.to_string_lossy().as_ref() {
        "problems/biochemistry-problems/carbs/classify_Haworth.py"
        | "problems/biochemistry-problems/carbs/convert_Fischer_to_Haworth.py"
        | "problems/biochemistry-problems/carbs/convert_Haworth_to_Fischer.py" => &["-p"],
        "problems/molecular_biology-problems/rna_transcribe.py" => &["-m", "-D"],
        _ => &[],
    }
}

fn mirror_generator_assets(
    generator: &Path,
    relative: &Path,
    sandbox: &Path,
) -> Result<PathBuf, String> {
    if relative != Path::new("problems/inheritance-problems/horse_coat_pattern_inference.py") {
        return Ok(generator.to_path_buf());
    }
    let mirrored = sandbox.join("mirrored").join(relative);
    let parent = mirrored
        .parent()
        .ok_or_else(|| "horse generator had no parent directory".to_owned())?;
    fs::create_dir_all(parent).map_err(display_error)?;
    fs::copy(generator, &mirrored).map_err(display_error)?;
    let image = generator.with_file_name("horse_coat_patterns.png");
    fs::copy(&image, parent.join("horse_coat_patterns.png")).map_err(display_error)?;
    Ok(mirrored)
}

fn find_bbq_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    find_bbq_files_at(directory, &mut result)?;
    result.sort();
    Ok(result)
}

fn find_bbq_files_at(directory: &Path, result: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(display_error)? {
        let path = entry.map_err(display_error)?.path();
        if path.is_dir() {
            find_bbq_files_at(&path, result)?;
        } else if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("bbq-"))
            && path.extension().is_some_and(|extension| extension == "txt")
        {
            result.push(path);
        }
    }
    Ok(())
}

fn extract_records(
    repository: &Path,
    biology: &Path,
    files: &[PathBuf],
    bbq_provenance: &BTreeMap<PathBuf, Value>,
    work: &Path,
) -> Result<Vec<Value>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let helper = repository
        .join("xtask")
        .join("support")
        .join("corpus_harvest.py");
    let command = format!(
        "source {} >/dev/null && python3 {} {}",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(&helper),
        files
            .iter()
            .map(|path| shell_quote(path))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let output = Command::new("bash")
        .args(["-lc", &command])
        .current_dir(work)
        .env(
            "QTI_CORPUS_IMPORT_PATH",
            corpus_import_path(work, biology, None),
        )
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "corpus extractor failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let parsed: Value = serde_json::from_slice(&output.stdout).map_err(display_error)?;
    let mut records = Vec::new();
    for file in parsed["files"]
        .as_array()
        .ok_or_else(|| "extractor did not return files".to_owned())?
    {
        let path = PathBuf::from(
            file["path"]
                .as_str()
                .ok_or_else(|| "extractor record missing path".to_owned())?,
        );
        let provenance = bbq_provenance.get(&path).ok_or_else(|| {
            format!(
                "extractor returned an unrecognized BBQ file: {}",
                path.display()
            )
        })?;
        for record in file["records"].as_array().cloned().unwrap_or_default() {
            let mut record = record;
            record["bbq_file"] = Value::String(path.display().to_string());
            record["generator"] = provenance["generator"].clone();
            record["generator_source_sha256"] = provenance["source_sha256"].clone();
            records.push(record);
        }
    }
    Ok(records)
}

fn write_unique_fragments(
    output: &Path,
    records: Vec<Value>,
) -> Result<(CorpusFragments, Provenance), String> {
    let mut fragments = BTreeMap::new();
    let mut provenance: Provenance = BTreeMap::new();
    for mut record in records {
        let family = record["family"]
            .as_str()
            .ok_or_else(|| "record missing family".to_owned())?
            .to_owned();
        let content = if family == "table" {
            record["fragment"]
                .as_str()
                .ok_or_else(|| "table record missing fragment".to_owned())?
                .to_owned()
        } else {
            serde_json::to_string_pretty(&record["fragment"]).map_err(display_error)?
        };
        let hash = sha256(&content);
        record["sha256"] = Value::String(hash.clone());
        let key = format!("{family}:{hash}");
        fragments.entry(key.clone()).or_insert_with(|| Fragment {
            family: family.clone(),
            hash: hash.clone(),
            content: content.clone(),
        });
        provenance.entry(key).or_default().push(record);
    }
    let fragments = fragments.into_values().collect::<Vec<_>>();
    for fragment in &fragments {
        let extension = if fragment.family == "table" {
            "html"
        } else {
            "json"
        };
        let directory = output.join(if fragment.family == "table" {
            "tables"
        } else {
            "canvases"
        });
        fs::create_dir_all(&directory).map_err(display_error)?;
        fs::write(
            directory.join(format!("{}.{}", fragment.hash, extension)),
            &fragment.content,
        )
        .map_err(display_error)?;
    }
    Ok((fragments, provenance))
}

fn sha256(content: &str) -> String {
    sha256_bytes(content.as_bytes())
}

fn sha256_bytes(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn census_fragments(fragments: &[Fragment]) -> Result<Value, String> {
    let mut tags = Counter::default();
    let mut attributes = Counter::default();
    let mut properties = Counter::default();
    let mut units = Counter::default();
    let mut colors = Counter::default();
    for fragment in fragments
        .iter()
        .filter(|fragment| fragment.family == "table")
    {
        census_html(
            &fragment.content,
            &mut tags,
            &mut attributes,
            &mut properties,
            &mut units,
            &mut colors,
        )?;
    }
    Ok(json!({
        "tags": tags.values,
        "attributes": attributes.values,
        "css_properties": properties.values,
        "units": units.values,
        "color_forms": colors.values,
    }))
}

#[derive(Default)]
struct Counter {
    values: BTreeMap<String, Value>,
}

impl Counter {
    fn add(&mut self, name: &str, example: &str) {
        let entry = self
            .values
            .entry(name.to_owned())
            .or_insert_with(|| json!({"count": 0, "example": example}));
        entry["count"] = json!(entry["count"].as_u64().unwrap_or(0) + 1);
    }
}

fn census_html(
    html: &str,
    tags: &mut Counter,
    attributes: &mut Counter,
    properties: &mut Counter,
    units: &mut Counter,
    colors: &mut Counter,
) -> Result<(), String> {
    let tag_pattern =
        regex::Regex::new(r#"(?is)<([a-z][a-z0-9:-]*)\b([^>]*)>"#).map_err(display_error)?;
    let attribute_pattern =
        regex::Regex::new(r#"(?is)([a-z_:][-a-z0-9_:.]*)(?:\s*=\s*(\"[^\"]*\"|'[^']*'|[^\s>]+))?"#)
            .map_err(display_error)?;
    let unit_pattern =
        regex::Regex::new(r"(?i)(?:^|[^a-z0-9_-])((?:px|em|pt)\b|%)").map_err(display_error)?;
    let color_pattern = regex::Regex::new(r"(?i)(#[0-9a-f]{3,8}\b|rgba?\(|\b(?:black|white|red|blue|green|gray|grey|silver|yellow|orange|purple|pink|brown|transparent)\b)")
        .map_err(display_error)?;
    for captures in tag_pattern.captures_iter(html) {
        let tag = captures
            .get(1)
            .expect("tag capture")
            .as_str()
            .to_ascii_lowercase();
        let source = captures.get(0).expect("whole tag").as_str();
        tags.add(&tag, source);
        let attr_text = captures.get(2).expect("attribute capture").as_str();
        for attr in attribute_pattern.captures_iter(attr_text) {
            let name = attr
                .get(1)
                .expect("attribute name")
                .as_str()
                .to_ascii_lowercase();
            attributes.add(&name, source);
            if name == "style" {
                let value = attr
                    .get(2)
                    .map(|match_| match_.as_str().trim_matches(['\'', '\"']))
                    .unwrap_or("");
                for declaration in value.split(';') {
                    let Some((property, value)) = declaration.split_once(':') else {
                        continue;
                    };
                    let property = property.trim().to_ascii_lowercase();
                    let value = value.trim();
                    if property.is_empty() {
                        continue;
                    }
                    properties.add(&property, declaration.trim());
                    for unit in unit_pattern.captures_iter(value) {
                        units.add(&unit[1].to_ascii_lowercase(), declaration.trim());
                    }
                    for color in color_pattern.captures_iter(value) {
                        let form = color[1].to_ascii_lowercase();
                        let name = if form.starts_with('#') {
                            "hex"
                        } else if form.starts_with("rgb") {
                            "rgb"
                        } else {
                            "named"
                        };
                        colors.add(name, declaration.trim());
                    }
                }
            }
        }
    }
    Ok(())
}

fn write_reports(
    output: &Path,
    biology: &Path,
    biology_revision: &str,
    reports: &[GeneratorReport],
    fragments: &[Fragment],
    provenance: &Provenance,
    census: &Value,
) -> Result<(), String> {
    let unavailable = reports
        .iter()
        .filter(|report| report.status == "failed")
        .count();
    let generator_json = reports
        .iter()
        .map(|report| {
            json!({
                "generator": report.generator, "arguments": report.arguments,
                "status": report.status, "stderr": report.stderr,
                "source_sha256": report.source_sha256,
                "bbq_files": report.bbq_files,
                "assets": report.assets.iter().map(|asset| json!({"path": asset.path, "sha256": asset.sha256})).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        output.join("generator_report.json"),
        serde_json::to_vec_pretty(&generator_json).map_err(display_error)?,
    )
    .map_err(display_error)?;
    fs::write(
        output.join("census.json"),
        serde_json::to_vec_pretty(census).map_err(display_error)?,
    )
    .map_err(display_error)?;
    let manifest = json!({
        "command": "cargo xtask table-corpus",
        "fixed_seed": FIXED_SEED,
        "questions_per_generator": QUESTION_COUNT,
        "biology_problems": biology,
        "biology_problems_git_commit": biology_revision,
        "generator_count": reports.len(),
        "unavailable_generators": unavailable,
        "bbq_files": reports.iter().flat_map(|report| report.bbq_files.iter()).collect::<Vec<_>>(),
        "assets": reports.iter().flat_map(|report| report.assets.iter()).map(|asset| json!({"path": asset.path, "sha256": asset.sha256})).collect::<Vec<_>>(),
        "generators": generator_json,
        "fragments": fragments.iter().map(|fragment| json!({"family": fragment.family, "sha256": fragment.hash, "provenance": provenance[&format!("{}:{}", fragment.family, fragment.hash)]})).collect::<Vec<_>>(),
    });
    fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(display_error)?,
    )
    .map_err(display_error)?;
    fs::write(output.join("census.md"), census_markdown(census)).map_err(display_error)?;
    Ok(())
}

fn git_revision(directory: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", &directory.display().to_string(), "rev-parse", "HEAD"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "cannot record biology-problems revision: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(display_error)
        .map(|revision| revision.trim().to_owned())
}

fn bbq_provenance(reports: &[GeneratorReport]) -> BTreeMap<PathBuf, Value> {
    reports
        .iter()
        .flat_map(|report| {
            report.bbq_files.iter().map(move |file| {
                (
                    file.clone(),
                    json!({
                        "generator": report.generator,
                        "arguments": report.arguments,
                        "source_sha256": report.source_sha256,
                    }),
                )
            })
        })
        .collect()
}

fn census_markdown(census: &Value) -> String {
    let mut output =
        String::from("# Table corpus feature census\n\nGenerated by `cargo xtask table-corpus`.\n");
    for (title, key) in [
        ("Tags", "tags"),
        ("Attributes", "attributes"),
        ("CSS properties", "css_properties"),
        ("Units", "units"),
        ("Color forms", "color_forms"),
    ] {
        output.push_str(&format!(
            "\n## {title}\n\n| Feature | Count | Example |\n| --- | ---: | --- |\n"
        ));
        if let Some(map) = census[key].as_object() {
            for (name, details) in map {
                output.push_str(&format!(
                    "| `{name}` | {} | `{}` |\n",
                    details["count"],
                    details["example"].as_str().unwrap_or("").replace('`', "'")
                ));
            }
        }
    }
    output
}

fn shell_quote(path: &Path) -> String {
    shell_quote_text(&path.to_string_lossy())
}

fn shell_quote_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn corpus_import_path(sandbox: &Path, biology: &Path, generator_parent: Option<&Path>) -> String {
    let mut paths = vec![sandbox, biology];
    if let Some(parent) = generator_parent {
        paths.push(parent);
    }
    std::env::join_paths(paths)
        .map_err(display_error)
        .expect("corpus import paths contain no path separator")
        .to_string_lossy()
        .into_owned()
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn census_keeps_outer_and_nested_table_tags() {
        let fragments = vec![Fragment { family: "table".to_owned(), hash: "test".to_owned(), content: "<table style='border: 1px solid #fff'><tr><td><table><tr><td>x</td></tr></table></td></tr></table>".to_owned() }];
        let census = census_fragments(&fragments).expect("census succeeds");
        assert_eq!(census["tags"]["table"]["count"], 2);
        assert_eq!(census["css_properties"]["border"]["count"], 1);
        assert_eq!(census["color_forms"]["hex"]["count"], 1);
    }
}
