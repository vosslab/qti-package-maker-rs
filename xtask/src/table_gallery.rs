//! Generate the human-review gallery using the production Chromium renderer.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use qti_engines::html_to_image::{ChromiumFragmentRenderer, FragmentRenderer};
use serde::Serialize;

const PINNED_ORACLE_REVISION: &str = "55e5f368777f7809fe2e91b5d070caf6df0cb581";

/// Generate browser-safe live-source, Python-reference, and Chromium columns for the corpus.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = repository_root()?;
    let corpus = parse_corpus(&repository, arguments)?;
    let tables = corpus.join("tables");
    if !tables.is_dir() {
        return Err(format!(
            "table corpus is unavailable: {}; run cargo xtask table-corpus first",
            tables.display()
        ));
    }
    let output = gallery_run_directory(&repository)?;
    fs::create_dir_all(output.join("rust")).map_err(display_error)?;
    fs::create_dir_all(output.join("python")).map_err(display_error)?;

    let sources = table_sources(&tables)?;
    let python = run_python_references(&repository, &sources, &output)?;
    let mut entries = Vec::with_capacity(sources.len());
    let renderer = ChromiumFragmentRenderer::default();
    for source in &sources {
        let html = fs::read_to_string(source).map_err(display_error)?;
        let name = source
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid corpus filename: {}", source.display()))?;
        let native = match prepare_native_table(&html).and_then(|prepared| {
            renderer
                .render_table(&prepared)
                .map(|png| png.bytes)
                .map_err(|error| error.to_string())
        }) {
            Ok(png) => {
                fs::write(output.join("rust").join(format!("{name}.png")), png)
                    .map_err(display_error)?;
                GalleryResult::Image(format!("rust/{name}.png"))
            }
            Err(error) => GalleryResult::Error(error),
        };
        let python = python
            .iter()
            .find(|result| result.name == name)
            .map_or_else(
                || GalleryResult::Error("Python reference result is absent".to_owned()),
                |result| match &result.error {
                    Some(error) => GalleryResult::Error(error.clone()),
                    None => GalleryResult::Image(format!("python/{name}.png")),
                },
            );
        entries.push(GalleryEntry {
            name: name.to_owned(),
            html,
            native,
            python,
        });
    }
    let page = render_gallery(&entries);
    fs::write(output.join("index.html"), page).map_err(display_error)?;
    let receipt = entries.iter().map(GalleryReceipt::from).collect::<Vec<_>>();
    fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).map_err(display_error)?,
    )
    .map_err(display_error)?;
    let native_errors = receipt
        .iter()
        .filter(|entry| entry.native_result != "png")
        .count();
    println!(
        "table gallery: {} of {} entries rendered with Chromium; receipt: {}",
        receipt.len() - native_errors,
        entries.len(),
        output.join("receipt.json").display()
    );
    Ok(())
}

/// Creates a fresh gallery run so prior visual-review evidence remains available.
fn gallery_run_directory(repository: &Path) -> Result<PathBuf, String> {
    let base = repository.join("output_tables").join("gallery");
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(display_error)?
        .as_secs();
    for suffix in 0_u32.. {
        let name = if suffix == 0 {
            format!("run-{seconds}")
        } else {
            format!("run-{seconds}-{suffix}")
        };
        let output = base.join(name);
        fs::create_dir_all(&base).map_err(display_error)?;
        match fs::create_dir(&output) {
            Ok(()) => {
                fs::create_dir_all(output.join("rust")).map_err(display_error)?;
                fs::create_dir_all(output.join("python")).map_err(display_error)?;
                return Ok(output);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(display_error(error)),
        }
    }
    unreachable!("u32 suffix range is exhaustive")
}

#[derive(Debug)]
struct GalleryEntry {
    name: String,
    html: String,
    native: GalleryResult,
    python: GalleryResult,
}

#[derive(Debug)]
enum GalleryResult {
    Image(String),
    Error(String),
}

#[derive(Serialize)]
struct GalleryReceipt {
    source_hash: String,
    route: String,
    native_result: String,
    python_result: String,
    compatibility_noops: Vec<&'static str>,
}

impl From<&GalleryEntry> for GalleryReceipt {
    fn from(entry: &GalleryEntry) -> Self {
        Self {
            source_hash: entry.name.clone(),
            route: route_for(&entry.html).to_owned(),
            native_result: result_for(&entry.native),
            python_result: result_for(&entry.python),
            compatibility_noops: compatibility_noops(&entry.html),
        }
    }
}

/// Replaces only statically parsed RDKit canvases before native table rendering.
///
/// The shared selector validates the constrained script grammar without evaluating authored
/// JavaScript. A native RDKit image becomes a `data:` image inside the table render.
fn prepare_native_table(html: &str) -> Result<String, String> {
    use qti_engines::html_to_image::{FieldConversionPlan, FragmentReplacement, PreparedFragment};

    let plan = FieldConversionPlan::prepare(html).map_err(|error| error.to_string())?;
    let replacements = plan
        .fragments()
        .iter()
        .filter_map(|fragment| match fragment {
            PreparedFragment::Canvas(target) => Some(target),
            PreparedFragment::Table(_) => None,
        })
        .map(|target| {
            let png = qti_molecule::render_canvas_png(&target.source)
                .map_err(|error| error.to_string())?;
            let source = base64::engine::general_purpose::STANDARD.encode(png);
            Ok(FragmentReplacement {
                id: target.id,
                html: format!(
                    "<img src=\"data:image/png;base64,{source}\" alt=\"Molecule canvas\">"
                ),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let plan = plan
        .after_canvas_replacements(&replacements)
        .map_err(|error| error.to_string())?;
    let tables = plan
        .fragments()
        .iter()
        .filter_map(PreparedFragment::table_html)
        .collect::<Vec<_>>();
    match tables.as_slice() {
        [table] => Ok((*table).to_owned()),
        [] => Err("prepared source has no outer table".to_owned()),
        _ => Err("prepared source has multiple outer tables".to_owned()),
    }
}

#[derive(Serialize)]
struct PythonJob {
    name: String,
    source: String,
    output: String,
}

#[derive(serde::Deserialize)]
struct PythonResult {
    name: String,
    error: Option<String>,
}

fn parse_corpus(repository: &Path, arguments: &[String]) -> Result<PathBuf, String> {
    let mut corpus = repository.join("output_tables").join("corpus");
    let mut iterator = arguments.iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--corpus" => corpus = PathBuf::from(iterator.next().ok_or("--corpus needs a path")?),
            "--help" | "-h" => {
                return Err("Usage: cargo xtask table-gallery [--corpus PATH]".to_owned());
            }
            _ => return Err(format!("unknown table-gallery argument: {argument}")),
        }
    }
    Ok(corpus)
}

fn repository_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(display_error)?;
    if !output.status.success() {
        return Err("cargo xtask table-gallery must run inside the Rust workspace".to_owned());
    }
    let root = String::from_utf8(output.stdout).map_err(display_error)?;
    Ok(PathBuf::from(root.trim()))
}

fn table_sources(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = fs::read_dir(directory)
        .map_err(display_error)?
        .map(|entry| entry.map_err(display_error).map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    result.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "html")
    });
    result.sort();
    if result.is_empty() {
        return Err(format!(
            "table corpus has no HTML fragments: {}",
            directory.display()
        ));
    }
    Ok(result)
}

fn run_python_references(
    repository: &Path,
    sources: &[PathBuf],
    output: &Path,
) -> Result<Vec<PythonResult>, String> {
    let jobs = sources
        .iter()
        .map(|source| {
            let name = source
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("invalid corpus filename: {}", source.display()))?;
            Ok(PythonJob {
                name: name.to_owned(),
                source: source.display().to_string(),
                output: output
                    .join("python")
                    .join(format!("{name}.png"))
                    .display()
                    .to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let jobs_path = output.join("python_jobs.json");
    let report_path = output.join("python_results.json");
    let script_path = output.join("render_python_references.py");
    fs::write(
        &jobs_path,
        serde_json::to_vec_pretty(&jobs).map_err(display_error)?,
    )
    .map_err(display_error)?;
    fs::write(&script_path, PYTHON_REFERENCE_SCRIPT).map_err(display_error)?;
    let oracle = repository
        .join("output_tables")
        .join("oracle_snapshot")
        .join(PINNED_ORACLE_REVISION);
    if !oracle.join("qti_package_maker").is_dir() {
        return Err(format!(
            "pinned Python oracle is unavailable: {}; run cargo xtask oracle-crosscheck first",
            oracle.display()
        ));
    }
    let command = format!(
        "source {} && PYTHONPATH={} python3 {} {} {}",
        shell_quote(&repository.join("source_me.sh")),
        shell_quote(&oracle),
        shell_quote(&script_path),
        shell_quote(&jobs_path),
        shell_quote(&report_path),
    );
    let status = Command::new("bash")
        .args(["-lc", &command])
        .current_dir(repository)
        .status()
        .map_err(display_error)?;
    if !status.success() {
        return Err(format!(
            "Python gallery references failed with status {status}"
        ));
    }
    serde_json::from_slice(&fs::read(report_path).map_err(display_error)?).map_err(display_error)
}

fn render_gallery(entries: &[GalleryEntry]) -> String {
    let mut page = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>QTI table raster gallery</title><style>body{font-family:system-ui,sans-serif;margin:1rem;background:#f5f6f7}h1{margin-bottom:0}.note{max-width:90rem}.entry{background:white;border:1px solid #b8bdc4;border-radius:.35rem;margin:1rem 0;padding:1rem}.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:1rem}.pane{min-width:0}.pane h2{font-size:1rem}.source{width:100%;min-height:220px;border:1px solid #88919b;background:white}.image{max-width:100%;height:auto;border:1px solid #88919b;background:white}.error{white-space:pre-wrap;color:#9d1c1c;background:#fff1f1;border:1px solid #d8a3a3;padding:.5rem}</style><h1>QTI Chromium table gallery</h1><p class=\"note\">Each source preview runs in a sandbox without scripts. The Python column is produced by the pinned package renderer with its actual wrapper and canvas options. Judge readability and source content. Python images are diagnostic references, not visual acceptance targets. Rendering errors remain visible.</p>",
    );
    for entry in entries {
        page.push_str("<section class=\"entry\"><h2>");
        page.push_str(&escape_html(&entry.name));
        page.push_str("</h2><p>Route: <code>");
        page.push_str(route_for(&entry.html));
        page.push_str("</code>");
        let noops = compatibility_noops(&entry.html);
        if !noops.is_empty() {
            page.push_str("; compatibility: ");
            page.push_str(&noops.join(", "));
        }
        page.push_str("</p><div class=\"grid\"><div class=\"pane\"><h2>Live source</h2><iframe class=\"source\" sandbox srcdoc=\"");
        page.push_str(&escape_attribute(&live_source_document(&entry.html)));
        page.push_str("\"></iframe></div><div class=\"pane\"><h2>Rust Chromium PNG</h2>");
        append_result(&mut page, &entry.native);
        page.push_str("</div><div class=\"pane\"><h2>Python PNG</h2>");
        append_result(&mut page, &entry.python);
        page.push_str("</div></div></section>");
    }
    page.push_str("</html>");
    page
}

fn route_for(html: &str) -> &'static str {
    if html.contains("class=\"boxplot\"") || html.contains("class='boxplot'") {
        "boxplot_scene"
    } else if is_restriction_digest_source(html) {
        "restriction_digest_scene"
    } else if is_pedigree_leaf_source(html) {
        "general_table+pedigree_scene_leaf"
    } else if is_titration_leaf_source(html) {
        "general_table+titration_scene_leaf"
    } else {
        "general_table"
    }
}

fn is_pedigree_leaf_source(html: &str) -> bool {
    html.contains("box-sizing: border-box")
        && html.contains("display: inline-block")
        && html.contains("position: relative")
        && html.contains("width: 65px")
        && html.contains("height: 65px")
}

fn is_titration_leaf_source(html: &str) -> bool {
    html.contains("position:relative;height:72px")
}

fn is_restriction_digest_source(html: &str) -> bool {
    html.contains("role=\"img\"")
        && (html.contains("aria-label=\"Linear restriction-digest DNA map.\"")
            || html.contains("aria-label=\"Circular restriction-digest DNA map.\""))
}

fn compatibility_noops(html: &str) -> Vec<&'static str> {
    let mut noops = Vec::new();
    if html.contains("vert-align") {
        noops.push("vert-align");
    }
    if ["ff0303", "ff9000", "b9e710", "1c7d72", "6d1685"]
        .iter()
        .any(|color| html.contains(color))
    {
        noops.push("unprefixed-six-digit-color");
    }
    noops
}

fn result_for(result: &GalleryResult) -> String {
    match result {
        GalleryResult::Image(_) => "png".to_owned(),
        GalleryResult::Error(error) => format!("error: {error}"),
    }
}

fn append_result(page: &mut String, result: &GalleryResult) {
    match result {
        GalleryResult::Image(path) => {
            page.push_str("<img class=\"image\" src=\"");
            page.push_str(&escape_attribute(path));
            page.push_str("\" alt=\"Rendered table\">");
        }
        GalleryResult::Error(error) => {
            page.push_str("<div class=\"error\">");
            page.push_str(&escape_html(error));
            page.push_str("</div>");
        }
    }
}

fn live_source_document(html: &str) -> String {
    format!(
        "<!doctype html><meta charset=\"utf-8\"><style>body{{margin:8px;background:#fff;font-family:'Atkinson Hyperlegible Next',sans-serif}}</style>{html}"
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attribute(input: &str) -> String {
    escape_html(input)
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

const PYTHON_REFERENCE_SCRIPT: &str = r#"
import json
import sys
from pathlib import Path

from qti_package_maker.html_to_image.render_table import TableRenderer

jobs = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
results = []
with TableRenderer() as renderer:
    for job in jobs:
        try:
            png = renderer.render_table_png(Path(job["source"]).read_text(encoding="utf-8"))
            Path(job["output"]).write_bytes(png)
            results.append({"name": job["name"], "error": None})
        except Exception as error:
            results.append({"name": job["name"], "error": f"{type(error).__name__}: {error}"})
Path(sys.argv[2]).write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
"#;

#[cfg(test)]
mod tests {
    use super::{GalleryEntry, GalleryResult, render_gallery, route_for};

    #[test]
    fn gallery_sandboxes_source_and_keeps_native_errors_visible() {
        let page = render_gallery(&[GalleryEntry {
            name: "example".to_owned(),
            html: "<table><script>bad()</script></table>".to_owned(),
            native: GalleryResult::Error("unsupported tag".to_owned()),
            python: GalleryResult::Image("python/example.png".to_owned()),
        }]);
        assert!(page.contains("iframe class=\"source\" sandbox"));
        assert!(page.contains("&lt;script&gt;bad()&lt;/script&gt;"));
        assert!(page.contains("unsupported tag"));
    }

    #[test]
    fn gallery_labels_leaf_routes_without_reclassifying_the_outer_table() {
        assert_eq!(
            route_for(
                "<table><tr><td><span style='display: inline-block; position: relative; width: 65px; height: 65px; box-sizing: border-box'></span></td></tr></table>"
            ),
            "general_table+pedigree_scene_leaf"
        );
        assert_eq!(
            route_for(
                "<table><tr><td><div style='position:relative;height:72px'></div></td></tr></table>"
            ),
            "general_table+titration_scene_leaf"
        );
    }
}
