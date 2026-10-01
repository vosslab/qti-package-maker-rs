//! Allowlisted HTML-subset parser and contracts shared by layout and painting.

use std::collections::BTreeMap;

use scraper::{ElementRef, Html, Node};
use thiserror::Error;

use crate::style::{self, ComputedStyle};

const MAX_FRAGMENT_BYTES: usize = 1_048_576;
const MAX_TREE_DEPTH: usize = 128;

/// Parser limits applied before building an owned tree.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterConfig {
    /// Viewport width used to resolve percentage table widths before device scaling.
    pub available_width_css_px: f32,
    /// Output pixels per CSS pixel. This is fixed by the renderer contract, not input markup.
    pub device_scale_factor: u32,
    pub max_fragment_bytes: usize,
    pub max_tree_depth: usize,
}
impl Default for RasterConfig {
    fn default() -> Self {
        Self {
            available_width_css_px: 1264.0,
            device_scale_factor: 2,
            max_fragment_bytes: MAX_FRAGMENT_BYTES,
            max_tree_depth: MAX_TREE_DEPTH,
        }
    }
}

/// The category of an unsupported source feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedFeatureKind {
    Tag,
    Attribute,
    Property,
}
/// A bounded diagnostic for an HTML or CSS construct outside the published subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedTableFeature {
    pub kind: UnsupportedFeatureKind,
    pub name: String,
    pub snippet: String,
}
impl UnsupportedTableFeature {
    #[must_use]
    pub fn new(kind: UnsupportedFeatureKind, name: impl Into<String>, snippet: &str) -> Self {
        Self {
            kind,
            name: name.into(),
            snippet: snippet.chars().take(160).collect(),
        }
    }
}

/// A parse or subset-contract failure.
#[derive(Debug, Error, PartialEq)]
pub enum RasterError {
    #[error("table fragment is {actual_bytes} bytes; maximum is {maximum_bytes}")]
    FragmentTooLarge {
        actual_bytes: usize,
        maximum_bytes: usize,
    },
    #[error("table fragment exceeds maximum tree depth {maximum_depth}")]
    TreeTooDeep { maximum_depth: usize },
    #[error("table fragment must contain one outer table")]
    MissingOuterTable,
    /// The configured viewport or resolved table bounds cannot form a PNG canvas.
    #[error(
        "invalid {stage} geometry: {css_width} by {css_height} CSS pixels at scale {device_scale_factor}"
    )]
    InvalidGeometry {
        stage: &'static str,
        css_width: f32,
        css_height: f32,
        device_scale_factor: u32,
    },
    /// A parser-approved embedded image could not be measured or painted.
    #[error("embedded image rendering failed: {reason}")]
    EmbeddedImage { reason: String },
    /// Native PNG painting or encoding failed after layout completed.
    #[error("native table painting failed: {reason}")]
    Paint { reason: String },
    #[error("unsupported {0:?} '{1}' near {2:?}")]
    Unsupported(UnsupportedFeatureKind, String, String),
}
impl From<UnsupportedTableFeature> for RasterError {
    fn from(value: UnsupportedTableFeature) -> Self {
        Self::Unsupported(value.kind, value.name, value.snippet)
    }
}

/// Supported HTML elements; the parser owns all tag spelling normalization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Table,
    Caption,
    Thead,
    Tbody,
    Tfoot,
    Tr,
    Td,
    Th,
    Colgroup,
    Col,
    Span,
    Bold,
    Italic,
    Sub,
    Sup,
    Br,
    Paragraph,
    Div,
    UnorderedList,
    OrderedList,
    ListItem,
    Image,
    Font,
    /// Noninteractive inline anchor content retained after the canvas replacement stage.
    Anchor,
}
/// One text or element node after cascade resolution.
#[derive(Clone, Debug, PartialEq)]
pub enum StyledNodeKind {
    Element(ElementKind),
    Text(String),
    /// A strictly validated positioned leaf embedded in an otherwise ordinary styled table tree.
    SceneLeaf(SceneLeafKind),
}

/// The two constrained positioned leaf grammars approved by the corpus ruling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneLeafKind {
    PedigreeGlyph,
    TitrationStateTile,
}

/// Anchor within a scene leaf after the parent grid resolves its border box.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

/// A text group retained as a styled node so inline layout can preserve sub/sup and font rules.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneText {
    pub node: StyledNode,
    pub anchor: SceneAnchor,
    pub x_css: f32,
    pub y_css: f32,
}

/// Resolved non-text marks and text-group geometry for one constrained scene leaf.
///
/// All geometry is in CSS pixels and leaf-relative. R5 applies device scaling only after R2
/// resolves the leaf border box in the ordinary table grid.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLeaf {
    pub kind: SceneLeafKind,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
    pub display_list: DisplayList,
    pub text_groups: Vec<SceneText>,
}
/// An owned styled node consumed by later work packages.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledNode {
    pub kind: StyledNodeKind,
    pub style: ComputedStyle,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<Self>,
    /// Present only when [`Self::kind`] is a validated [`StyledNodeKind::SceneLeaf`].
    pub scene: Option<SceneLeaf>,
}
/// A parsed outer table and its computed styles.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledTree {
    pub root: StyledNode,
}

/// Geometry contract filled by WP-R2/R3.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
/// Resolved paint operations produced after table and inline layout.
///
/// Coordinates and radii are physical pixels here. CSS lengths are resolved before this boundary,
/// so painting does not repeat layout decisions.
#[derive(Clone, Debug, PartialEq)]
pub enum DisplayCommand {
    Fill {
        bounds: LayoutBox,
        color: crate::Color,
    },
    Border {
        bounds: LayoutBox,
        style: ComputedStyle,
    },
    Text {
        bounds: LayoutBox,
        text: String,
        style: ComputedStyle,
    },
    /// Begins clipping subsequent commands to a resolved rectangular or rounded region.
    PushClip {
        bounds: LayoutBox,
        radius: Option<f32>,
    },
    /// Restores the previous clip established by [`Self::PushClip`].
    PopClip,
    /// Paints a parser-validated `data:image/png` or `data:image/svg+xml` image.
    Image { bounds: LayoutBox, data_url: String },
    /// Paints a scene-resolved finite line segment in physical raster coordinates.
    StrokeLine {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        color: crate::Color,
    },
    /// Paints glyphs shaped by WP-R3. The run carries its resolved bounds and baseline.
    GlyphRun { run: crate::TextRun },
}
/// A paint sequence bounded by the resolved outer table border box.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub bounds: LayoutBox,
    pub commands: Vec<DisplayCommand>,
}

/// Parses one table fragment under the published supported subset.
///
/// The input is never executed or fetched. It is parsed into an owned allowlisted tree, and each
/// unsupported tag, attribute, or CSS property is rejected at this boundary (ASVS 1.3.1, 1.3.5,
/// and 2.2.1).
pub fn parse_fragment(fragment: &str) -> Result<StyledTree, RasterError> {
    parse_fragment_with_config(fragment, &RasterConfig::default())
}

/// Parses one table fragment with explicit resource limits.
pub fn parse_fragment_with_config(
    fragment: &str,
    config: &RasterConfig,
) -> Result<StyledTree, RasterError> {
    if fragment.len() > config.max_fragment_bytes {
        return Err(RasterError::FragmentTooLarge {
            actual_bytes: fragment.len(),
            maximum_bytes: config.max_fragment_bytes,
        });
    }
    validate_source_structure(fragment)?;
    let document = Html::parse_fragment(fragment);
    let mut tables = Vec::new();
    for child in document.root_element().children() {
        match child.value() {
            Node::Text(text) if text.trim().is_empty() => {}
            Node::Comment(_) => {}
            Node::Text(_) => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "content outside outer table",
                    fragment,
                ));
            }
            Node::Element(_) => {
                let element = ElementRef::wrap(child).expect("element node wraps");
                if element.value().name() == "table" {
                    tables.push(element);
                } else {
                    return Err(unsupported(
                        UnsupportedFeatureKind::Tag,
                        element.value().name(),
                        &element.html(),
                    ));
                }
            }
            _ => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "non-content node",
                    fragment,
                ));
            }
        }
    }
    let [root] = tables.as_slice() else {
        return if tables.is_empty() {
            Err(RasterError::MissingOuterTable)
        } else {
            Err(unsupported(
                UnsupportedFeatureKind::Tag,
                "multiple outer tables",
                fragment,
            ))
        };
    };
    Ok(StyledTree {
        root: parse_element(*root, None, 0, config)?,
    })
}

fn parse_element(
    element: ElementRef<'_>,
    parent: Option<&ComputedStyle>,
    depth: usize,
    config: &RasterConfig,
) -> Result<StyledNode, RasterError> {
    if depth > config.max_tree_depth {
        return Err(RasterError::TreeTooDeep {
            maximum_depth: config.max_tree_depth,
        });
    }
    let tag = element.value().name();
    let kind = element_kind(tag)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Tag, tag, &element.html()))?;
    let snippet = element.html();
    if let Some(scene) = crate::scene_leaf::parse_candidate(element, parent, depth, config)? {
        let mut style = ComputedStyle::ua_defaults(tag, parent);
        style.display = match scene.kind {
            SceneLeafKind::PedigreeGlyph => crate::DisplayMode::InlineBlock,
            SceneLeafKind::TitrationStateTile => crate::DisplayMode::Block,
        };
        style.width = Some(crate::CssLength::Px(scene.intrinsic_width));
        style.height = Some(crate::CssLength::Px(scene.intrinsic_height));
        return Ok(StyledNode {
            kind: StyledNodeKind::SceneLeaf(scene.kind),
            style,
            attributes: BTreeMap::new(),
            children: Vec::new(),
            scene: Some(scene),
        });
    }
    let mut style = ComputedStyle::ua_defaults(tag, parent);
    let mut attributes = BTreeMap::new();
    for (key, value) in &element.value().attrs {
        let name = key.local.as_ref();
        let value = value.as_ref();
        if name == "style" {
            style::apply_inline(&mut style, value, &snippet)?;
        } else if kind == ElementKind::Font {
            style::apply_legacy_font(&mut style, name, value, &snippet)?;
        } else if metadata_attribute(kind, name) {
            // Metadata is retained for accessibility and the constrained boxplot route. It never
            // enables selector matching or changes general-table painting.
        } else if (kind == ElementKind::Anchor || (kind == ElementKind::Image && name == "src"))
            && allowed_attribute(kind, name)
        {
            // Prepared-canvas links contribute ordinary inline text only. Image source data is
            // validated below. Neither path is a presentational HTML attribute.
        } else if allowed_attribute(kind, name) {
            style::apply_presentational(&mut style, name, value, &snippet)?;
        } else {
            return Err(unsupported(
                UnsupportedFeatureKind::Attribute,
                name,
                &snippet,
            ));
        }
        if name != "style" && kind != ElementKind::Anchor {
            if kind == ElementKind::Image && name == "src" && !data_image(value) {
                return Err(unsupported(
                    UnsupportedFeatureKind::Attribute,
                    name,
                    &snippet,
                ));
            }
            attributes.insert(name.to_owned(), value.to_owned());
        }
    }
    if let Some(scene) = crate::scene_leaf::parse_candidate(element, parent, depth, config)? {
        return Ok(StyledNode {
            kind: StyledNodeKind::SceneLeaf(scene.kind),
            style,
            attributes,
            children: Vec::new(),
            scene: Some(scene),
        });
    }
    style::validate_general_position(&style, tag, &snippet)?;
    let mut children = Vec::new();
    for child in element.children() {
        match child.value() {
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    children.push(StyledNode {
                        kind: StyledNodeKind::Text(text.to_string()),
                        style: style.clone(),
                        attributes: BTreeMap::new(),
                        children: Vec::new(),
                        scene: None,
                    });
                }
            }
            Node::Element(_) => {
                let child = ElementRef::wrap(child).expect("element node wraps");
                let child = parse_element(child, Some(&style), depth + 1, config)?;
                validate_child(kind, &child, &snippet)?;
                children.push(child);
            }
            Node::Comment(_) => {}
            _ => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "non-content node",
                    &snippet,
                ));
            }
        }
    }
    Ok(StyledNode {
        kind: StyledNodeKind::Element(kind),
        style,
        attributes,
        children,
        scene: None,
    })
}

/// Rejects table structures that later layout would otherwise skip. This runs after HTML5 parsing
/// so foster-parented content is either found at the fragment boundary or rejected here before
/// R2's row collector sees the tree.
fn validate_child(
    parent: ElementKind,
    child: &StyledNode,
    snippet: &str,
) -> Result<(), RasterError> {
    let StyledNodeKind::Element(child) = child.kind else {
        return Ok(());
    };
    let allowed = valid_child_kind(parent, child);
    if allowed {
        Ok(())
    } else {
        Err(unsupported(
            UnsupportedFeatureKind::Tag,
            &format!("{} inside {}", element_name(child), element_name(parent)),
            snippet,
        ))
    }
}

fn valid_child_kind(parent: ElementKind, child: ElementKind) -> bool {
    match parent {
        ElementKind::Table => matches!(
            child,
            ElementKind::Caption
                | ElementKind::Colgroup
                | ElementKind::Col
                | ElementKind::Thead
                | ElementKind::Tbody
                | ElementKind::Tfoot
                | ElementKind::Tr
        ),
        ElementKind::Colgroup => child == ElementKind::Col,
        ElementKind::Thead | ElementKind::Tbody | ElementKind::Tfoot => child == ElementKind::Tr,
        ElementKind::Tr => matches!(child, ElementKind::Td | ElementKind::Th),
        ElementKind::Col | ElementKind::Image | ElementKind::Br => false,
        ElementKind::UnorderedList | ElementKind::OrderedList => child == ElementKind::ListItem,
        _ => true,
    }
}

/// Validates source nesting before HTML5 recovery can foster-parent or reorder a node. The
/// bounded scanner only recognizes tag boundaries and quoted attributes; HTML parsing and style
/// resolution remain the responsibility of `scraper`.
fn validate_source_structure(fragment: &str) -> Result<(), RasterError> {
    let mut stack = Vec::<ElementKind>::new();
    let bytes = fragment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if fragment[index..].starts_with("<!--") {
            let Some(end) = fragment[index + 4..].find("-->") else {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "unterminated comment",
                    fragment,
                ));
            };
            index += 4 + end + 3;
            continue;
        }
        let end = tag_end(fragment, index + 1).ok_or_else(|| {
            unsupported(UnsupportedFeatureKind::Tag, "unterminated tag", fragment)
        })?;
        let source = fragment[index + 1..end].trim();
        index = end + 1;
        if source.starts_with('!') || source.starts_with('?') {
            continue;
        }
        let closing = source.starts_with('/');
        let source = source.trim_start_matches('/').trim();
        let name = source
            .split(|character: char| character.is_whitespace() || character == '/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let kind = element_kind(&name)
            .ok_or_else(|| unsupported(UnsupportedFeatureKind::Tag, &name, fragment))?;
        if closing {
            if stack.pop() != Some(kind) {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "misnested closing tag",
                    fragment,
                ));
            }
            continue;
        }
        if let Some(parent) = stack.last().copied() {
            if !valid_child_kind(parent, kind) {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    &format!("{} inside {}", element_name(kind), element_name(parent)),
                    fragment,
                ));
            }
        } else if kind != ElementKind::Table {
            return Err(unsupported(UnsupportedFeatureKind::Tag, &name, fragment));
        }
        if !matches!(
            kind,
            ElementKind::Br | ElementKind::Col | ElementKind::Image
        ) && !source.ends_with('/')
        {
            stack.push(kind);
        }
    }
    if stack.is_empty() {
        Ok(())
    } else {
        Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "unclosed tag",
            fragment,
        ))
    }
}

fn tag_end(fragment: &str, start: usize) -> Option<usize> {
    let bytes = fragment.as_bytes();
    let mut quote = None;
    for (offset, byte) in bytes[start..].iter().enumerate() {
        match (quote, *byte) {
            (Some(delimiter), byte) if byte == delimiter => quote = None,
            (None, b'\'' | b'"') => quote = Some(*byte),
            (None, b'>') => return Some(start + offset),
            _ => {}
        }
    }
    None
}

fn element_name(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Table => "table",
        ElementKind::Caption => "caption",
        ElementKind::Thead => "thead",
        ElementKind::Tbody => "tbody",
        ElementKind::Tfoot => "tfoot",
        ElementKind::Tr => "tr",
        ElementKind::Td => "td",
        ElementKind::Th => "th",
        ElementKind::Colgroup => "colgroup",
        ElementKind::Col => "col",
        ElementKind::Span => "span",
        ElementKind::Bold => "b",
        ElementKind::Italic => "i",
        ElementKind::Sub => "sub",
        ElementKind::Sup => "sup",
        ElementKind::Br => "br",
        ElementKind::Paragraph => "p",
        ElementKind::Div => "div",
        ElementKind::UnorderedList => "ul",
        ElementKind::OrderedList => "ol",
        ElementKind::ListItem => "li",
        ElementKind::Image => "img",
        ElementKind::Font => "font",
        ElementKind::Anchor => "a",
    }
}

fn element_kind(tag: &str) -> Option<ElementKind> {
    Some(match tag {
        "table" => ElementKind::Table,
        "caption" => ElementKind::Caption,
        "thead" => ElementKind::Thead,
        "tbody" => ElementKind::Tbody,
        "tfoot" => ElementKind::Tfoot,
        "tr" => ElementKind::Tr,
        "td" => ElementKind::Td,
        "th" => ElementKind::Th,
        "colgroup" => ElementKind::Colgroup,
        "col" => ElementKind::Col,
        "span" => ElementKind::Span,
        "b" | "strong" => ElementKind::Bold,
        "i" | "em" => ElementKind::Italic,
        "sub" => ElementKind::Sub,
        "sup" => ElementKind::Sup,
        "br" => ElementKind::Br,
        "p" => ElementKind::Paragraph,
        "div" => ElementKind::Div,
        "ul" => ElementKind::UnorderedList,
        "ol" => ElementKind::OrderedList,
        "li" => ElementKind::ListItem,
        "img" => ElementKind::Image,
        "font" => ElementKind::Font,
        "a" => ElementKind::Anchor,
        _ => return None,
    })
}
fn allowed_attribute(kind: ElementKind, name: &str) -> bool {
    match kind {
        ElementKind::Table => matches!(
            name,
            "bgcolor"
                | "align"
                | "valign"
                | "width"
                | "height"
                | "border"
                | "cellpadding"
                | "cellspacing"
        ),
        ElementKind::Colgroup | ElementKind::Col => matches!(name, "width" | "span"),
        ElementKind::Td | ElementKind::Th => {
            matches!(
                name,
                "bgcolor" | "align" | "valign" | "width" | "height" | "colspan" | "rowspan"
            )
        }
        ElementKind::Image => matches!(name, "src" | "alt"),
        ElementKind::Font => matches!(name, "size" | "color" | "face"),
        ElementKind::Anchor => matches!(name, "href" | "target" | "rel"),
        _ => false,
    }
}
fn metadata_attribute(kind: ElementKind, name: &str) -> bool {
    matches!(
        (kind, name),
        (ElementKind::Table, "class" | "role" | "aria-label")
            | (ElementKind::Th, "scope")
            // Alternative text remains item metadata; painting uses only the validated data URL.
            | (ElementKind::Image, "alt")
            | (ElementKind::Span, "aria-hidden")
    )
}
fn data_image(value: &str) -> bool {
    value.starts_with("data:image/png;base64,")
        || value.starts_with("data:image/svg+xml,")
        || value.starts_with("data:image/svg+xml;base64,")
}
fn unsupported(kind: UnsupportedFeatureKind, name: &str, snippet: &str) -> RasterError {
    UnsupportedTableFeature::new(kind, name, snippet).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cascades_presentational_then_inline_style() {
        let tree = parse_fragment("<table bgcolor='#fff' style='color:#123456'><tr><td align='right' style='text-align:center'>x</td></tr></table>").unwrap();
        let cell = &tree.root.children[0].children[0].children[0];
        assert_eq!(cell.style.text_align, crate::TextAlign::Center);
        assert_eq!(cell.style.color, crate::Color(18, 52, 86, 255));
    }
    #[test]
    fn rejects_script_at_boundary() {
        assert!(matches!(
            parse_fragment("<table><script>x</script></table>"),
            Err(RasterError::Unsupported(UnsupportedFeatureKind::Tag, _, _))
        ));
    }
    #[test]
    fn preserves_colspan_and_mono_style() {
        let tree = parse_fragment(
            "<table><tr><th colspan='4' style='font-family:monospace'>Tetrad</th></tr></table>",
        )
        .unwrap();
        let head = &tree.root.children[0].children[0].children[0];
        assert_eq!(head.style.font_family, crate::FontFamily::AtkinsonMono);
    }
    #[test]
    fn rejects_network_image_sources_without_fetching() {
        assert!(matches!(
            parse_fragment(
                "<table><tr><td><img src='https://example.test/x.png'></td></tr></table>"
            ),
            Err(RasterError::Unsupported(
                UnsupportedFeatureKind::Attribute,
                _,
                _
            ))
        ));
    }

    #[test]
    fn preserves_prepared_molecule_image_alt_metadata() {
        let tree = parse_fragment(
            "<table><tr><td><img alt='Molecule canvas' src='data:image/png;base64,AA=='></td></tr></table>",
        )
        .unwrap();
        let image = &tree.root.children[0].children[0].children[0].children[0];
        assert_eq!(
            image.attributes.get("alt"),
            Some(&"Molecule canvas".to_owned())
        );
        assert_eq!(
            image.attributes.get("src"),
            Some(&"data:image/png;base64,AA==".to_owned())
        );
    }

    #[test]
    fn retains_decorated_non_cell_boxes_for_later_layout_and_paint() {
        let tree = parse_fragment(
            "<table><tr><td><div style='height:7px;background-color:#99dbfb;border-radius:4px;box-shadow:0 0 2px #99dbfb'></div><span style='display:inline-block;width:38px;height:38px;border-radius:50%;background:#5e6312;border:2px solid #333'>I</span></td></tr></table>",
        )
        .unwrap();
        let cell = &tree.root.children[0].children[0].children[0];
        let band = &cell.children[0].style;
        let badge = &cell.children[1].style;
        assert_eq!(band.height, Some(crate::CssLength::Px(7.0)));
        assert_eq!(band.background, Some(crate::Color(153, 219, 251, 255)));
        assert!(band.box_shadow.is_some());
        assert_eq!(badge.width, Some(crate::CssLength::Px(38.0)));
        assert_eq!(badge.height, Some(crate::CssLength::Px(38.0)));
        assert_eq!(badge.background, Some(crate::Color(94, 99, 18, 255)));
    }

    #[test]
    fn preserves_prepared_canvas_anchor_text_without_navigation_metadata() {
        let tree = parse_fragment(
            "<table><tr><td><a href='https://example.test/image' target='_blank' rel='noopener'>link to static image</a></td></tr></table>",
        )
        .unwrap();
        let anchor = &tree.root.children[0].children[0].children[0].children[0];
        assert!(matches!(
            anchor.kind,
            StyledNodeKind::Element(ElementKind::Anchor)
        ));
        assert!(anchor.attributes.is_empty());
        assert!(matches!(
            anchor.children[0].kind,
            StyledNodeKind::Text(ref text) if text == "link to static image"
        ));
    }
    #[test]
    fn rejects_non_table_siblings_instead_of_silently_ignoring_them() {
        assert!(matches!(
            parse_fragment("<div>outside</div><table><tr><td>x</td></tr></table>"),
            Err(RasterError::Unsupported(UnsupportedFeatureKind::Tag, _, _))
        ));
        assert!(matches!(
            parse_fragment("<table></table><table></table>"),
            Err(RasterError::Unsupported(UnsupportedFeatureKind::Tag, _, _))
        ));
    }
    #[test]
    fn accepts_col_span_as_structural_colgroup_metadata() {
        let tree =
            parse_fragment("<table><colgroup span='2' width='40'/><tr><td>x</td></tr></table>")
                .unwrap();
        let group = &tree.root.children[0];
        assert_eq!(group.attributes.get("span"), Some(&"2".to_owned()));
    }
    #[test]
    fn defaults_use_the_fixed_raster_viewport_and_scale() {
        let config = RasterConfig::default();
        assert_eq!(config.available_width_css_px, 1264.0);
        assert_eq!(config.device_scale_factor, 2);
    }
    #[test]
    fn rejects_malformed_table_children_before_layout_can_skip_them() {
        for fragment in [
            "<table><colgroup><span>x</span></colgroup><tr><td>x</td></tr></table>",
            "<table><tbody><div>x</div></tbody></table>",
            "<table><tr><td>x</td><caption>late</caption></tr></table>",
        ] {
            assert!(
                matches!(
                    parse_fragment(fragment),
                    Err(RasterError::Unsupported(UnsupportedFeatureKind::Tag, _, _))
                ),
                "{fragment:?} parsed as {:?}",
                parse_fragment(fragment)
            );
        }
    }

    #[test]
    fn legacy_font_is_preserved_as_inherited_text_style() {
        let tree = parse_fragment(
            "<table><tr><td><font size='-2' color='red' face='Courier'>x</font></td></tr></table>",
        )
        .unwrap();
        let font = &tree.root.children[0].children[0].children[0].children[0];
        assert_eq!(font.style.font_family, crate::FontFamily::AtkinsonMono);
        assert_eq!(font.style.color, crate::Color(255, 0, 0, 255));
    }

    #[test]
    fn accepts_only_the_declared_inline_table_relative_offset() {
        let valid = parse_fragment(
            "<table style='display:inline-table;position:relative;top:-.2em'><tr><td>x</td></tr></table>",
        )
        .unwrap();
        assert_eq!(
            valid.root.style.relative_top,
            Some(crate::CssLength::Em(-0.2))
        );
        for invalid in [
            "<table style='position:relative;top:-.2em'><tr><td>x</td></tr></table>",
            "<table style='display:inline-table;top:-.2em'><tr><td>x</td></tr></table>",
            "<table><tr><td style='position:relative;top:-.2em'>x</td></tr></table>",
        ] {
            assert!(matches!(
                parse_fragment(invalid),
                Err(RasterError::Unsupported(
                    UnsupportedFeatureKind::Property,
                    _,
                    _
                ))
            ));
        }
    }

    #[test]
    fn accepts_only_the_exact_table_margin_and_bracket_scale_extensions() {
        let centered =
            parse_fragment("<table style='width:200px;margin:0 auto'><tr><td>x</td></tr></table>")
                .unwrap();
        assert!(centered.root.style.table_auto_horizontal_margins);
        let scaled = parse_fragment(
            "<table><tr><td><span style='font-size:xx-large;transform:scale(1.35);display:inline-block'>\u{27ee}</span></td></tr></table>",
        )
        .unwrap();
        assert_eq!(
            scaled.root.children[0].children[0].children[0].children[0]
                .style
                .inline_scale_x,
            Some(1.35)
        );
        for fragment in [
            "<table><tr><td style='margin:0 auto'>x</td></tr></table>",
            "<table><tr><td><span style='font-size:xx-large;transform:scale(1.4);display:inline-block'>\u{27ee}</span></td></tr></table>",
        ] {
            assert!(parse_fragment(fragment).is_err(), "{fragment}");
        }
    }

    #[test]
    fn embeds_a_validated_pedigree_leaf_without_replacing_the_table_grid() {
        let tree = parse_fragment(
            "<table><tr><td><span style='display:inline-block;position:relative;width:65px;height:65px;line-height:65px;text-align:center;font-weight:bold;font-size:39px;border:2px solid #000;box-sizing:border-box;overflow:hidden;background-color:#ffffff;color:#000000'><span style='position:relative;z-index:1'>&#160;</span></span></td><td>grid text</td></tr></table>",
        )
        .unwrap();
        let first_cell = &tree.root.children[0].children[0].children[0];
        assert!(matches!(
            first_cell.children[0].kind,
            StyledNodeKind::SceneLeaf(SceneLeafKind::PedigreeGlyph)
        ));
        assert!(first_cell.children[0].scene.is_some());
        assert!(matches!(
            tree.root.children[0].children[0].children[1].kind,
            StyledNodeKind::Element(ElementKind::Td)
        ));
    }
}
