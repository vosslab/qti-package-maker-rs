//! Strict recognition for positioned leaves embedded in ordinary table layout.
//!
//! This module does not lay out a table or provide general CSS positioning.  It recognizes only
//! the bounded pedigree-glyph grammar; other positioned source remains a typed parser error.

use std::collections::BTreeMap;

use scraper::{ElementRef, Node};

use crate::{
    CssLength, DisplayCommand, DisplayList, ElementKind, LayoutBox, RasterConfig, RasterError,
    SceneAnchor, SceneLeaf, SceneLeafKind, SceneText, StyledNode, StyledNodeKind, TextAlign,
    UnsupportedFeatureKind, UnsupportedTableFeature,
};

/// Recognizes a constrained positioned leaf before the general position validator runs.
///
/// `None` means the element has no scene-position signature. A signature that does not satisfy
/// the closed grammar returns a typed error rather than falling back to general positioning.
pub(crate) fn parse_candidate(
    element: ElementRef<'_>,
    parent: Option<&crate::ComputedStyle>,
    _depth: usize,
    _config: &RasterConfig,
) -> Result<Option<SceneLeaf>, RasterError> {
    let style_source = element.value().attr("style").unwrap_or("");
    if element.value().name() == "div" && is_titration_card(element) {
        return parse_titration_card(element, parent);
    }
    if !style_source.contains("position") && !style_source.contains("z-index") {
        return Ok(None);
    }
    if element.value().name() != "span" {
        return Ok(None);
    }
    if style_source.contains("position: absolute") || style_source.contains("position:absolute") {
        return Err(unsupported(
            "absolute positioned scene leaf",
            &element.html(),
        ));
    }
    let mut style = crate::ComputedStyle::ua_defaults("span", parent);
    crate::style::apply_inline(
        &mut style,
        &strip_scene_only_properties(style_source),
        &element.html(),
    )?;
    if style.display != crate::DisplayMode::InlineBlock || !style.relative_positioned {
        return Err(unsupported("scene leaf root", &element.html()));
    }
    let (Some(width), Some(height)) = (style.width, style.height) else {
        return Err(unsupported("scene leaf dimensions", &element.html()));
    };
    let (crate::CssLength::Px(width), crate::CssLength::Px(height)) = (width, height) else {
        return Err(unsupported("scene leaf pixel dimensions", &element.html()));
    };
    if !(width.is_finite()
        && height.is_finite()
        && (1.0..=128.0).contains(&width)
        && (1.0..=128.0).contains(&height))
    {
        return Err(unsupported(
            "scene leaf bounded dimensions",
            &element.html(),
        ));
    }
    let label = foreground_label(&element, &style)?;
    let bounds = LayoutBox {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };
    Ok(Some(SceneLeaf {
        kind: SceneLeafKind::PedigreeGlyph,
        intrinsic_width: width,
        intrinsic_height: height,
        display_list: DisplayList {
            bounds,
            commands: vec![DisplayCommand::Border { bounds, style }],
        },
        text_groups: vec![label],
    }))
}

fn foreground_label(
    element: &ElementRef<'_>,
    inherited: &crate::ComputedStyle,
) -> Result<SceneText, RasterError> {
    let children: Vec<_> = element.children().filter_map(ElementRef::wrap).collect();
    if children.len() != 1 || children[0].value().name() != "span" {
        return Err(unsupported("pedigree foreground label", &element.html()));
    }
    let label = children[0];
    let source = label.value().attr("style").unwrap_or("");
    if source.replace(' ', "").trim_end_matches(';') != "position:relative;z-index:1" {
        return Err(unsupported("pedigree foreground layer", &label.html()));
    }
    let text = direct_text(&label)?;
    let node = StyledNode {
        kind: StyledNodeKind::Text(text),
        style: inherited.clone(),
        attributes: BTreeMap::new(),
        children: Vec::new(),
        scene: None,
    };
    Ok(SceneText {
        node,
        anchor: SceneAnchor::Center,
        x_css: 0.0,
        y_css: 0.0,
    })
}

/// The harvested card is one replaced leaf: its normal table cell still controls the grid, while
/// the card itself supplies the title, border, and four reliably anchored chemical labels.
fn parse_titration_card(
    element: ElementRef<'_>,
    parent: Option<&crate::ComputedStyle>,
) -> Result<Option<SceneLeaf>, RasterError> {
    let children: Vec<_> = element.children().filter_map(ElementRef::wrap).collect();
    let heading = children[0];
    let tile = children[1];
    let outer_source = element.value().attr("style").unwrap_or("");
    let tile_source = tile.value().attr("style").unwrap_or("");
    let height = declaration(tile_source, "height")
        .and_then(px)
        .filter(|height| (1.0..=160.0).contains(height))
        .ok_or_else(|| unsupported("titration tile height", &element.html()))?;
    let groups: Vec<_> = tile.children().filter_map(ElementRef::wrap).collect();
    if groups.len() != 4 || groups.iter().any(|group| group.value().name() != "div") {
        return Err(unsupported("four titration text groups", &element.html()));
    }
    let mut card_style = crate::ComputedStyle::ua_defaults("div", parent);
    crate::style::apply_inline(&mut card_style, outer_source, &element.html())?;
    let content_width = match card_style.min_width {
        Some(CssLength::Px(width)) if (1.0..=256.0).contains(&width) => width,
        _ => return Err(unsupported("titration card min-width", &element.html())),
    };
    let horizontal_padding =
        px_padding(card_style.padding[1])? + px_padding(card_style.padding[3])?;
    let vertical_padding = px_padding(card_style.padding[0])? + px_padding(card_style.padding[2])?;
    let border = px_border(card_style.border.top.width)?;
    let heading_style = heading.value().attr("style").unwrap_or("");
    let heading_margin = declaration(heading_style, "margin-bottom")
        .and_then(px)
        .filter(|value| (0.0..=32.0).contains(value))
        .ok_or_else(|| unsupported("titration title margin", &heading.html()))?;
    let heading_node = inline_node_with_strip(&heading, &card_style, &["margin-bottom"])?;
    let heading_height = crate::layout_inline(&heading_node, content_width).height;
    let intrinsic_width = content_width + horizontal_padding + 2.0 * border;
    let intrinsic_height =
        height + heading_height + heading_margin + vertical_padding + 2.0 * border;
    if !(intrinsic_width.is_finite()
        && intrinsic_height.is_finite()
        && intrinsic_width <= 320.0
        && intrinsic_height <= 256.0)
    {
        return Err(unsupported("titration card bounds", &element.html()));
    }
    let content_x = border + px_padding(card_style.padding[3])?;
    let tile_y = border + px_padding(card_style.padding[0])? + heading_height + heading_margin;
    let mut text_groups = Vec::with_capacity(4);
    text_groups.push(SceneText {
        node: heading_node,
        anchor: SceneAnchor::TopLeft,
        x_css: content_x
            + (content_width
                - crate::layout_inline(
                    &inline_node_with_strip(&heading, &card_style, &["margin-bottom"])?,
                    content_width,
                )
                .width)
                / 2.0,
        y_css: border + px_padding(card_style.padding[0])?,
    });
    for group in groups {
        let style = group.value().attr("style").unwrap_or("");
        let anchor =
            anchor(style).ok_or_else(|| unsupported("titration group anchor", &group.html()))?;
        let mut node = inline_node(&group, &card_style)?;
        // R5 anchors the group's *natural* inline box at the right edge.  Keeping the source
        // `text-align:right` here makes `layout_inline` report the whole card width and sends
        // its glyphs past the scene clip. The anchor provides the right alignment instead.
        if matches!(anchor, SceneAnchor::TopRight | SceneAnchor::BottomRight) {
            normalize_text_alignment(&mut node);
        }
        text_groups.push(SceneText {
            node,
            anchor,
            x_css: content_x,
            y_css: match anchor {
                SceneAnchor::TopLeft | SceneAnchor::TopRight => tile_y,
                SceneAnchor::BottomLeft | SceneAnchor::BottomRight => {
                    border + px_padding(card_style.padding[2])?
                }
                SceneAnchor::Center => unreachable!("titration grammar has corner anchors"),
            },
        });
    }
    Ok(Some(SceneLeaf {
        kind: SceneLeafKind::TitrationStateTile,
        intrinsic_width,
        intrinsic_height,
        display_list: DisplayList {
            bounds: LayoutBox {
                x: 0.0,
                y: 0.0,
                width: intrinsic_width,
                height: intrinsic_height,
            },
            commands: vec![DisplayCommand::Border {
                bounds: LayoutBox {
                    x: 0.0,
                    y: 0.0,
                    width: intrinsic_width,
                    height: intrinsic_height,
                },
                style: card_style,
            }],
        },
        text_groups,
    }))
}

fn normalize_text_alignment(node: &mut StyledNode) {
    node.style.text_align = TextAlign::Start;
    for child in &mut node.children {
        normalize_text_alignment(child);
    }
}

fn is_titration_card(element: ElementRef<'_>) -> bool {
    let source = element.value().attr("style").unwrap_or("");
    if !source.contains("min-width") || !source.contains("border") {
        return false;
    }
    let children: Vec<_> = element.children().filter_map(ElementRef::wrap).collect();
    children.len() == 2
        && children.iter().all(|child| child.value().name() == "div")
        && children[1]
            .value()
            .attr("style")
            .is_some_and(|style| style.replace(' ', "").contains("position:relative"))
}

fn anchor(source: &str) -> Option<SceneAnchor> {
    let compact = source.replace(' ', "");
    match (
        compact.contains("left:0"),
        compact.contains("right:0"),
        compact.contains("top:0"),
        compact.contains("bottom:0"),
    ) {
        (true, false, true, false) => Some(SceneAnchor::TopLeft),
        (false, true, true, false) => Some(SceneAnchor::TopRight),
        (true, false, false, true) => Some(SceneAnchor::BottomLeft),
        (false, true, false, true) => Some(SceneAnchor::BottomRight),
        _ => None,
    }
}

fn inline_node(
    element: &ElementRef<'_>,
    parent: &crate::ComputedStyle,
) -> Result<StyledNode, RasterError> {
    let tag = element.value().name();
    let kind = match tag {
        "div" => ElementKind::Div,
        "span" => ElementKind::Span,
        "sub" => ElementKind::Sub,
        "sup" => ElementKind::Sup,
        _ => return Err(unsupported("titration inline tag", &element.html())),
    };
    let mut style = crate::ComputedStyle::ua_defaults(tag, Some(parent));
    if let Some(source) = element.value().attr("style") {
        crate::style::apply_inline(
            &mut style,
            &strip_position_properties(source),
            &element.html(),
        )?;
    }
    let mut children = Vec::new();
    for child in element.children() {
        match child.value() {
            Node::Text(text) if !text.trim().is_empty() => children.push(StyledNode {
                kind: StyledNodeKind::Text(text.to_string()),
                style: style.clone(),
                attributes: BTreeMap::new(),
                children: Vec::new(),
                scene: None,
            }),
            Node::Text(_) => {}
            Node::Element(_) => children.push(inline_node(
                &ElementRef::wrap(child).expect("element"),
                &style,
            )?),
            Node::Comment(_) => {}
            _ => return Err(unsupported("titration content", &element.html())),
        }
    }
    Ok(StyledNode {
        kind: StyledNodeKind::Element(kind),
        style,
        attributes: BTreeMap::new(),
        children,
        scene: None,
    })
}

fn inline_node_with_strip(
    element: &ElementRef<'_>,
    parent: &crate::ComputedStyle,
    extra_strip: &[&str],
) -> Result<StyledNode, RasterError> {
    let tag = element.value().name();
    let kind = match tag {
        "div" => ElementKind::Div,
        "span" => ElementKind::Span,
        "sub" => ElementKind::Sub,
        "sup" => ElementKind::Sup,
        _ => return Err(unsupported("titration inline tag", &element.html())),
    };
    let mut style = crate::ComputedStyle::ua_defaults(tag, Some(parent));
    if let Some(source) = element.value().attr("style") {
        let mut names = vec!["position", "left", "right", "top", "bottom"];
        names.extend_from_slice(extra_strip);
        crate::style::apply_inline(
            &mut style,
            &strip_properties(source, &names),
            &element.html(),
        )?;
    }
    let mut children = Vec::new();
    for child in element.children() {
        match child.value() {
            Node::Text(text) if !text.trim().is_empty() => children.push(StyledNode {
                kind: StyledNodeKind::Text(text.to_string()),
                style: style.clone(),
                attributes: BTreeMap::new(),
                children: Vec::new(),
                scene: None,
            }),
            Node::Text(_) | Node::Comment(_) => {}
            Node::Element(_) => children.push(inline_node(
                &ElementRef::wrap(child).expect("element"),
                &style,
            )?),
            _ => return Err(unsupported("titration content", &element.html())),
        }
    }
    Ok(StyledNode {
        kind: StyledNodeKind::Element(kind),
        style,
        attributes: BTreeMap::new(),
        children,
        scene: None,
    })
}

fn strip_position_properties(source: &str) -> String {
    strip_properties(source, &["position", "left", "right", "top", "bottom"])
}

fn strip_properties(source: &str, names: &[&str]) -> String {
    source
        .split(';')
        .filter(|declaration| {
            let name = declaration
                .split_once(':')
                .map_or("", |(name, _)| name.trim());
            !names.contains(&name)
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn px_padding(value: CssLength) -> Result<f32, RasterError> {
    match value {
        CssLength::Px(value) if (0.0..=64.0).contains(&value) => Ok(value),
        _ => Err(unsupported("titration card padding", "")),
    }
}

fn px_border(value: CssLength) -> Result<f32, RasterError> {
    match value {
        CssLength::Px(value) if (0.0..=8.0).contains(&value) => Ok(value),
        _ => Err(unsupported("titration card border", "")),
    }
}

fn declaration<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    source.split(';').find_map(|declaration| {
        let (key, value) = declaration.split_once(':')?;
        (key.trim() == name).then_some(value.trim())
    })
}

fn px(value: &str) -> Option<f32> {
    value.strip_suffix("px")?.parse().ok()
}

fn direct_text(element: &ElementRef<'_>) -> Result<String, RasterError> {
    let mut output = String::new();
    for child in element.children() {
        match child.value() {
            Node::Text(text) => output.push_str(text),
            _ => return Err(unsupported("pedigree label content", &element.html())),
        }
    }
    if output.is_empty() {
        return Err(unsupported("empty pedigree label", &element.html()));
    }
    Ok(output)
}

fn strip_scene_only_properties(source: &str) -> String {
    source
        .split(';')
        .filter(|declaration| {
            let name = declaration
                .split_once(':')
                .map_or("", |(name, _)| name.trim());
            name != "box-sizing"
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn unsupported(name: &str, source: &str) -> RasterError {
    UnsupportedTableFeature::new(UnsupportedFeatureKind::Property, name, source).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    #[test]
    fn recognizes_a_bounded_pedigree_glyph() {
        let html = Html::parse_fragment(
            "<span style='display:inline-block;position:relative;width:65px;height:65px;line-height:65px;text-align:center;font-weight:bold;font-size:39px;border:2px solid #000;box-sizing:border-box;overflow:hidden;background-color:#ffffff;color:#000000'><span style='position:relative;z-index:1'>&#160;</span></span>",
        );
        let root = html.root_element();
        let span = root.children().find_map(ElementRef::wrap).unwrap();
        let scene = parse_candidate(span, None, 0, &RasterConfig::default())
            .unwrap()
            .unwrap();
        assert_eq!(scene.kind, SceneLeafKind::PedigreeGlyph);
        assert_eq!(scene.intrinsic_width, 65.0);
        assert_eq!(scene.display_list.commands.len(), 1);
    }

    #[test]
    fn recognizes_four_anchored_titration_groups() {
        let html = Html::parse_fragment(
            "<div style='border:1px solid #ccc;border-radius:10px;padding:8px 10px;min-width:160px;background:#fff'><div style='text-align:center;font-size:13px;font-weight:700;margin-bottom:6px'>State 1</div><div style='position:relative;height:72px'><div style='position:absolute;left:0;top:0;font-size:14px'>NH<sub>3</sub><sup>+</sup></div><div style='position:absolute;right:0;top:0;font-size:14px'>CH<sub>3</sub></div><div style='position:absolute;left:0;bottom:0;font-size:14px'>H<sub>3</sub>N<sup>+</sup></div><div style='position:absolute;right:0;bottom:0;font-size:14px'>COO<sup>-</sup></div></div></div>",
        );
        let root = html.root_element();
        let div = root.children().find_map(ElementRef::wrap).unwrap();
        let scene = parse_candidate(div, None, 0, &RasterConfig::default())
            .unwrap()
            .unwrap();
        assert_eq!(scene.kind, SceneLeafKind::TitrationStateTile);
        assert_eq!(scene.intrinsic_width, 182.0);
        assert_eq!(scene.text_groups.len(), 5);
        assert_eq!(scene.text_groups[4].anchor, SceneAnchor::BottomRight);
        assert_eq!(scene.text_groups[2].node.style.text_align, TextAlign::Start);
        assert!(matches!(
            scene.text_groups[1].node.children[1].kind,
            StyledNodeKind::Element(ElementKind::Sub)
        ));
    }
}
