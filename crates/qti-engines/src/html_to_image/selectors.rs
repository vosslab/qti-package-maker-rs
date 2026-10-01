//! HTML fragment selection for table and static RDKit canvas conversion.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use lol_html::html_content::ContentType;
use lol_html::{Settings, element, rewrite_str};
use qti_molecule::CanvasSource;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

use super::{CanvasScriptError, parse_canvas_script};

const RDKIT_LOADER_URL: &str = "https://unpkg.com/@rdkit/rdkit/dist/RDKit_minimal.js";

/// A selected HTML fragment. Conversion owns the serialized, ASCII-only text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FragmentId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum FragmentKind {
    Table,
    Canvas,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub id: FragmentId,
    pub html: String,
}

/// A canvas and its statically parsed molecule source, in canvas document order.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasTarget {
    pub id: FragmentId,
    pub canvas_id: String,
    pub source: CanvasSource,
}

/// Opaque, per-field selection result for two-phase conversion.
#[derive(Clone, Debug, PartialEq)]
pub enum PreparedFragment {
    Table(Fragment),
    Canvas(CanvasTarget),
}
impl PreparedFragment {
    pub fn id(&self) -> FragmentId {
        match self {
            Self::Table(value) => value.id,
            Self::Canvas(value) => value.id,
        }
    }
    pub fn kind(&self) -> FragmentKind {
        match self {
            Self::Table(_) => FragmentKind::Table,
            Self::Canvas(_) => FragmentKind::Canvas,
        }
    }
    pub fn table_html(&self) -> Option<&str> {
        match self {
            Self::Table(value) => Some(&value.html),
            Self::Canvas(_) => None,
        }
    }
    pub fn canvas_source(&self) -> Option<&CanvasSource> {
        match self {
            Self::Table(_) => None,
            Self::Canvas(value) => Some(&value.source),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentReplacement {
    pub id: FragmentId,
    pub html: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldConversionPlan {
    prepared_html: String,
    fragments: Vec<PreparedFragment>,
    canvas_scripts: HashMap<FragmentId, usize>,
}
impl FieldConversionPlan {
    pub fn prepare(original_html: &str) -> Result<Self, HtmlToImageError> {
        let prepared_html = remove_rdkit_loader_scripts(original_html)?.html;
        let mut fragments = find_table_fragments(&prepared_html)
            .into_iter()
            .map(PreparedFragment::Table)
            .collect::<Vec<_>>();
        let canvas_targets = find_canvas_targets_with_scripts(&prepared_html)?;
        let canvas_scripts = canvas_targets
            .iter()
            .map(|(target, script_index)| (target.id, *script_index))
            .collect();
        fragments.extend(
            canvas_targets
                .into_iter()
                .map(|(target, _)| PreparedFragment::Canvas(target)),
        );
        fragments.sort_by_key(PreparedFragment::id);
        Ok(Self {
            prepared_html,
            fragments,
            canvas_scripts,
        })
    }
    pub fn fragments(&self) -> &[PreparedFragment] {
        &self.fragments
    }
    /// Returns the loader-free, ASCII-normalized HTML used to prepare this plan.
    pub fn prepared_html(&self) -> &str {
        &self.prepared_html
    }
    pub fn apply_replacements(
        &self,
        replacements: &[FragmentReplacement],
    ) -> Result<String, HtmlToImageError> {
        let removed_scripts = replacements
            .iter()
            .filter_map(|replacement| self.canvas_scripts.get(&replacement.id))
            .copied()
            .collect();
        rewrite_replacements(&self.prepared_html, replacements, &removed_scripts)
    }
    /// Replans tables after canvas data-image replacements have been applied.
    pub fn after_canvas_replacements(
        &self,
        replacements: &[FragmentReplacement],
    ) -> Result<Self, HtmlToImageError> {
        Self::prepare(&self.apply_replacements(replacements)?)
    }
}

/// Failures while selecting fragments or validating a static canvas script.
#[derive(Debug, Error, PartialEq)]
pub enum HtmlToImageError {
    #[error("RDKit canvas '{0}' has no supported drawing script")]
    MissingCanvasScript(String),
    #[error("RDKit drawing script matches more than one canvas")]
    SharedCanvasScript,
    #[error("RDKit drawing script has no matching canvas")]
    OrphanScript,
    #[error(transparent)]
    CanvasScript(#[from] CanvasScriptError),
    #[error("HTML rewrite failed: {0}")]
    Rewrite(String),
}

/// Returns outermost table fragments in document order, keeping nested tables inside their parent.
pub fn find_table_fragments(html: &str) -> Vec<Fragment> {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("table").expect("static selector parses");
    document
        .select(&selector)
        .filter(|table| !has_table_ancestor(table))
        .map(|table| Fragment {
            id: fragment_id(&document, table),
            html: ascii_html(&table.html()),
        })
        .collect()
}

/// Returns all supported RDKit canvas targets in document order without executing JavaScript.
pub fn find_canvas_targets(html: &str) -> Result<Vec<CanvasTarget>, HtmlToImageError> {
    find_canvas_targets_with_scripts(html)
        .map(|targets| targets.into_iter().map(|(target, _)| target).collect())
}

fn find_canvas_targets_with_scripts(
    html: &str,
) -> Result<Vec<(CanvasTarget, usize)>, HtmlToImageError> {
    let document = Html::parse_fragment(html);
    let canvas_selector = Selector::parse("canvas").expect("static selector parses");
    let script_selector = Selector::parse("script").expect("static selector parses");
    let scripts = document.select(&script_selector).collect::<Vec<_>>();
    let mut used_scripts = Vec::new();
    let mut targets = Vec::new();
    for canvas in document.select(&canvas_selector) {
        let id = canvas.value().attr("id").unwrap_or_default().to_owned();
        let Some((script_index, script)) = following_rdkit_script(&canvas, &scripts, &id) else {
            if id.starts_with("canvas_") {
                return Err(HtmlToImageError::MissingCanvasScript(id));
            }
            continue;
        };
        if used_scripts.contains(&script_index) {
            return Err(HtmlToImageError::SharedCanvasScript);
        }
        let source = parse_canvas_script(
            script.text().collect::<String>().as_str(),
            canvas.value().attr("width"),
            canvas.value().attr("height"),
        )?;
        targets.push((
            CanvasTarget {
                id: fragment_id(&document, canvas),
                canvas_id: id,
                source,
            },
            script_index,
        ));
        used_scripts.push(script_index);
    }
    for (index, script) in scripts.iter().enumerate() {
        if is_rdkit_script(script) && !used_scripts.contains(&index) {
            return Err(HtmlToImageError::OrphanScript);
        }
    }
    Ok(targets)
}

/// Applies generated HTML by stable parsed-node ordinal, never by string search.
pub fn apply_replacements(
    original_html: &str,
    replacements: &[FragmentReplacement],
) -> Result<String, HtmlToImageError> {
    FieldConversionPlan::prepare(original_html)?.apply_replacements(replacements)
}

fn rewrite_replacements(
    original_html: &str,
    replacements: &[FragmentReplacement],
    removed_scripts: &HashSet<usize>,
) -> Result<String, HtmlToImageError> {
    let replacements = replacements
        .iter()
        .map(|value| (value.id, value.html.clone()))
        .collect::<HashMap<_, _>>();
    let fragment_ordinal = Rc::new(Cell::new(0_usize));
    let script_ordinal = Rc::new(Cell::new(0_usize));
    let fragment_counter = Rc::clone(&fragment_ordinal);
    let script_counter = Rc::clone(&script_ordinal);
    let removed_scripts = removed_scripts.clone();
    let settings = Settings::new()
        .append_element_content_handler(element!("table, canvas", move |element| {
            let id = FragmentId(fragment_counter.get());
            fragment_counter.set(id.0 + 1);
            if let Some(html) = replacements.get(&id) {
                element.replace(html, ContentType::Html);
            }
            Ok(())
        }))
        .append_element_content_handler(element!("script", move |element| {
            let index = script_counter.get();
            script_counter.set(index + 1);
            if removed_scripts.contains(&index) {
                element.remove();
            }
            Ok(())
        }));
    rewrite_str(original_html, settings)
        .map(|html| ascii_html(&html))
        .map_err(|error| HtmlToImageError::Rewrite(error.to_string()))
}

/// Removes only the known CDN loader while preserving other markup for later conversion.
pub fn remove_rdkit_loader_scripts(html: &str) -> Result<Fragment, HtmlToImageError> {
    let settings =
        Settings::new().append_element_content_handler(element!("script[src]", |element| {
            if element.get_attribute("src").as_deref() == Some(RDKIT_LOADER_URL) {
                element.remove();
            }
            Ok(())
        }));
    rewrite_str(html, settings)
        .map(|html| Fragment {
            id: FragmentId(usize::MAX),
            html: ascii_html(&html),
        })
        .map_err(|error| HtmlToImageError::Rewrite(error.to_string()))
}

fn has_table_ancestor(element: &ElementRef<'_>) -> bool {
    element
        .ancestors()
        .skip(1)
        .filter_map(ElementRef::wrap)
        .any(|ancestor| ancestor.value().name() == "table")
}
fn fragment_id(document: &Html, candidate: ElementRef<'_>) -> FragmentId {
    let selector = Selector::parse("table, canvas").expect("static selector parses");
    FragmentId(
        document
            .select(&selector)
            .position(|element| element.id() == candidate.id())
            .expect("selected node remains in document"),
    )
}

fn following_rdkit_script<'a>(
    canvas: &ElementRef<'a>,
    scripts: &[ElementRef<'a>],
    canvas_id: &str,
) -> Option<(usize, ElementRef<'a>)> {
    let direct =
        following_siblings(canvas).find_map(|sibling| matching_script(sibling, scripts, canvas_id));
    direct
        .or_else(|| {
            canvas
                .parent()
                .and_then(ElementRef::wrap)
                .and_then(|parent| {
                    following_siblings(&parent)
                        .find_map(|sibling| matching_script(sibling, scripts, canvas_id))
                })
        })
        .or_else(|| {
            scripts
                .iter()
                .enumerate()
                .find(|(_, script)| {
                    is_rdkit_script(script)
                        && script_canvas_id(script).as_deref() == Some(canvas_id)
                })
                .map(|(index, script)| (index, *script))
        })
}

fn following_siblings<'a>(element: &ElementRef<'a>) -> impl Iterator<Item = ElementRef<'a>> {
    element.next_siblings().filter_map(ElementRef::wrap)
}
fn matching_script<'a>(
    candidate: ElementRef<'a>,
    scripts: &[ElementRef<'a>],
    canvas_id: &str,
) -> Option<(usize, ElementRef<'a>)> {
    let index = scripts
        .iter()
        .position(|script| script.id() == candidate.id())?;
    (is_rdkit_script(&candidate)
        && script_canvas_id(&candidate).is_none_or(|target| target == canvas_id))
    .then_some((index, candidate))
}
fn is_rdkit_script(element: &ElementRef<'_>) -> bool {
    element.value().name() == "script" && {
        let text = element.text().collect::<String>();
        text.contains("RDKitModule") || text.contains("initRDKitModule")
    }
}
fn script_canvas_id(element: &ElementRef<'_>) -> Option<String> {
    regex::Regex::new(r#"getElementById\(\s*(?:\"([^\"]+)\"|'([^']+)')\s*\)"#)
        .expect("valid regex")
        .captures(&element.text().collect::<String>())
        .and_then(|capture| {
            capture
                .get(1)
                .or_else(|| capture.get(2))
                .map(|id| id.as_str().to_owned())
        })
}
fn ascii_html(html: &str) -> String {
    html.chars()
        .map(|character| {
            if character.is_ascii() {
                character.to_string()
            } else {
                format!("&#{};", character as u32)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn script(id: &str) -> String {
        format!(
            r#"<script>initRDKitModule();let smiles="CCO";let mol=RDKitModule.get_mol(smiles);let mdetails={{}};canvas=document.getElementById("{id}");mol.draw_to_canvas_with_highlights(canvas,JSON.stringify(mdetails));</script>"#
        )
    }
    #[test]
    fn selects_outermost_tables_in_document_order() {
        let fragments = find_table_fragments(
            "<table><tr><td>a<table><tr><td>b</td></tr></table></td></tr></table><table><tr><td>c</td></tr></table>",
        );
        assert_eq!(fragments.len(), 2);
        assert!(fragments[0].html.contains("b"));
        assert!(fragments[0].html.is_ascii());
    }
    #[test]
    fn finds_all_three_python_script_locations() {
        let html = format!(
            "<canvas id='canvas_direct' width='120' height='80'></canvas>{}<p><canvas id='canvas_parent' width='120' height='80'></canvas></p>{}<div><canvas id='canvas_deep' width='120' height='80'></canvas></div>{}",
            script("canvas_direct"),
            script("canvas_parent"),
            script("canvas_deep")
        );
        let targets = find_canvas_targets(&html).expect("static scripts accepted");
        assert_eq!(targets.len(), 3);
    }
    #[test]
    fn removes_only_known_loader_and_serializes_ascii() {
        let html = format!(
            "<script src='{RDKIT_LOADER_URL}'></script><script src='safe.js'></script><p>\u{a0}</p>"
        );
        let result = remove_rdkit_loader_scripts(&html).expect("loader removal succeeds");
        assert!(!result.html.contains("RDKit_minimal"));
        assert!(result.html.contains("safe.js"));
        assert!(result.html.is_ascii());
    }

    #[test]
    fn rejects_orphan_and_shared_rdkit_scripts() {
        let orphan = script("other");
        assert_eq!(
            find_canvas_targets(&orphan),
            Err(HtmlToImageError::OrphanScript)
        );
        let shared = format!(
            "<canvas id='canvas_a' width='120' height='80'></canvas><canvas id='canvas_b' width='120' height='80'></canvas>{}",
            script("")
        );
        assert_eq!(
            find_canvas_targets(&shared),
            Err(HtmlToImageError::SharedCanvasScript)
        );
        assert_eq!(
            find_canvas_targets("<canvas id='canvas_orphan'></canvas>"),
            Err(HtmlToImageError::MissingCanvasScript(
                "canvas_orphan".to_owned()
            ))
        );
    }

    #[test]
    fn plan_uses_unique_positions_for_identical_table_markup() {
        let plan = FieldConversionPlan::prepare(
            "<table><tr><td>same</td></tr></table><table><tr><td>same</td></tr></table>",
        )
        .expect("tables prepare");
        let ids = plan
            .fragments()
            .iter()
            .filter(|fragment| fragment.kind() == FragmentKind::Table)
            .map(PreparedFragment::id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![FragmentId(0), FragmentId(1)]);
    }

    #[test]
    fn prepared_html_is_a_distinct_cache_key_for_fields_without_fragments() {
        let first = FieldConversionPlan::prepare("<p>first</p>").expect("first field prepares");
        let second = FieldConversionPlan::prepare("<p>second</p>").expect("second field prepares");
        assert!(first.fragments().is_empty());
        assert!(second.fragments().is_empty());
        assert_ne!(first.prepared_html(), second.prepared_html());
    }

    #[test]
    fn canvas_replacement_removes_only_its_matched_script_and_replans_tables() {
        let html = format!(
            "<script src='{RDKIT_LOADER_URL}'></script><table><tr><td><canvas id='canvas_one' width='120' height='80'></canvas></td></tr></table>{}<canvas id='canvas_two' width='120' height='80'></canvas>{}",
            script("canvas_one"),
            script("canvas_two"),
        );
        let plan = FieldConversionPlan::prepare(&html).expect("field prepares");
        let canvas_one = plan
            .fragments()
            .iter()
            .find(|fragment| fragment.canvas_source().is_some() && fragment.id() == FragmentId(1))
            .expect("first canvas selected")
            .id();
        let replacements = [FragmentReplacement {
            id: canvas_one,
            html: "<img src='data:image/png;base64,abc' alt='molecule'>".to_owned(),
        }];

        let applied = plan
            .apply_replacements(&replacements)
            .expect("canvas replacement applies");
        assert!(applied.contains("data:image/png;base64,abc"));
        assert!(!applied.contains("canvas_one\");mol.draw"));
        assert!(applied.contains("canvas_two\");mol.draw"));
        assert!(!applied.contains("RDKit_minimal"));

        let after_canvas = plan
            .after_canvas_replacements(&replacements)
            .expect("table plan refreshes after canvas replacement");
        let table = after_canvas
            .fragments()
            .iter()
            .find_map(PreparedFragment::table_html)
            .expect("outer table remains selected");
        assert!(table.contains("data:image/png;base64,abc"));
        assert_eq!(
            after_canvas
                .fragments()
                .iter()
                .filter(|fragment| fragment.kind() == FragmentKind::Table)
                .count(),
            1
        );
    }
}
