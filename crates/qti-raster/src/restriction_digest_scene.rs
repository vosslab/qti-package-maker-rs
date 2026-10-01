//! Constrained native scene parser for the two restriction-digest DNA maps.
//!
//! The source grammar is deliberately limited to the harvested fixed canvas. It resolves
//! coordinates and transforms before handing regular display commands to the painter.

use std::collections::BTreeMap;

use scraper::{ElementRef, Html, Node};

use crate::{
    Border, BorderSide, BorderStyle, Color, ComputedStyle, CssLength, DisplayCommand, DisplayList,
    LayoutBox, RasterConfig, RasterError, UnsupportedFeatureKind, UnsupportedTableFeature,
};

const LINEAR_LABEL: &str = "Linear restriction-digest DNA map.";
const CIRCULAR_LABEL: &str = "Circular restriction-digest DNA map.";
const MAX_MARKS: usize = 64;

/// Parses one harvested fixed-canvas restriction map without enabling general positioning.
pub fn parse_restriction_digest_fragment(
    fragment: &str,
    config: &RasterConfig,
) -> Result<DisplayList, RasterError> {
    if fragment.len() > config.max_fragment_bytes {
        return Err(RasterError::FragmentTooLarge {
            actual_bytes: fragment.len(),
            maximum_bytes: config.max_fragment_bytes,
        });
    }
    let document = Html::parse_fragment(fragment);
    let root = document.root_element();
    let table = one_child(&root, "table", fragment)?;
    attrs(
        &table,
        &[
            "align",
            "border",
            "cellpadding",
            "cellspacing",
            "role",
            "aria-label",
            "style",
        ],
    )?;
    let label = table.value().attr("aria-label").unwrap_or("");
    if table.value().attr("role") != Some("img") || !matches!(label, LINEAR_LABEL | CIRCULAR_LABEL)
    {
        return Err(unsupported(
            UnsupportedFeatureKind::Attribute,
            "restriction map role/label",
            &table.html(),
        ));
    }
    let tbody = one_child(&table, "tbody", &table.html())?;
    let tr = one_child(&tbody, "tr", &tbody.html())?;
    let td = one_child(&tr, "td", &tr.html())?;
    let canvas = one_child(&td, "div", &td.html())?;
    let canvas_style = properties(&canvas)?;
    let width = pixels(required(&canvas_style, "width", &canvas)?)?;
    let height = pixels(required(&canvas_style, "height", &canvas)?)?;
    if !(width.is_finite()
        && height.is_finite()
        && (1.0..=800.0).contains(&width)
        && (1.0..=400.0).contains(&height))
    {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "canvas dimensions",
            &canvas.html(),
        ));
    }
    if required(&canvas_style, "position", &canvas)? != "relative" {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "position",
            &canvas.html(),
        ));
    }
    allow(&canvas_style, &["width", "height", "position"], &canvas)?;

    let scale = config.device_scale_factor as f32;
    let bounds = box_px(0.0, 0.0, width, height, scale);
    let mut commands = vec![DisplayCommand::Fill {
        bounds,
        color: Color(255, 255, 255, 255),
    }];
    let marks = elements(&canvas);
    if marks.len() > MAX_MARKS {
        return Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "too many restriction marks",
            &canvas.html(),
        ));
    }
    for mark in marks {
        if mark.value().name() != "span" {
            return Err(unsupported(
                UnsupportedFeatureKind::Tag,
                mark.value().name(),
                &mark.html(),
            ));
        }
        commands.extend(parse_mark(&mark, scale)?);
    }
    Ok(DisplayList { bounds, commands })
}

fn parse_mark(mark: &ElementRef<'_>, scale: f32) -> Result<Vec<DisplayCommand>, RasterError> {
    attrs(mark, &["style"])?;
    let outer = properties(mark)?;
    let x = pixels(required(&outer, "left", mark)?)?;
    let y = pixels(required(&outer, "top", mark)?)?;
    if required(&outer, "position", mark)? != "absolute" {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "position",
            &mark.html(),
        ));
    }
    let transform = outer.get("transform").map(String::as_str).unwrap_or("none");
    let children = elements(mark);
    if children.is_empty() {
        if outer.contains_key("background-color") {
            return parse_fill(mark, &outer, x, y, scale);
        }
        let text = text_content(mark)?;
        if !text.trim().is_empty() {
            allow(&outer, &["left", "top", "position", "transform"], mark)?;
            // This literal is the linear map's centered scale caption.  Scene coordinates are
            // converted to physical pixels below, so its otherwise-UA font must be converted
            // too; leaving it at CSS pixels made only this caption visibly tiny at 2x output.
            let mut style = ComputedStyle::ua_defaults("span", None);
            style.font_size = CssLength::Px(16.0 * scale);
            let width = crate::layout_inline(
                &crate::StyledNode {
                    kind: crate::StyledNodeKind::Text(text.clone()),
                    style: style.clone(),
                    attributes: BTreeMap::new(),
                    children: Vec::new(),
                    scene: None,
                },
                1000.0,
            )
            .width;
            return Ok(vec![DisplayCommand::Text {
                bounds: box_px(
                    aligned_x(x, width / scale, transform)?,
                    y,
                    width / scale,
                    24.0,
                    scale,
                ),
                text,
                style,
            }]);
        }
        return parse_outline(mark, &outer, x, y, scale);
    }
    if children.len() != 1 || children[0].value().name() != "span" {
        return Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "restriction mark content",
            &mark.html(),
        ));
    }
    allow(&outer, &["left", "top", "position", "transform"], mark)?;
    let inner = children[0];
    attrs(&inner, &["style"])?;
    let style = properties(&inner)?;
    if style.contains_key("background-color") {
        return parse_tick(&inner, &style, x, y, transform, scale);
    }
    let text = text_content(&inner)?;
    if text.trim().is_empty() {
        return Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "empty restriction label",
            &inner.html(),
        ));
    }
    let mut text_style = text_style(&inner, &style, scale)?;
    let width = crate::layout_inline(
        &crate::StyledNode {
            kind: crate::StyledNodeKind::Text(text.clone()),
            style: text_style.clone(),
            attributes: BTreeMap::new(),
            children: Vec::new(),
            scene: None,
        },
        1000.0,
    )
    .width;
    let text_width = width / scale;
    let x = aligned_x(x, text_width, transform)?;
    let bounds = box_px(x, aligned_y(y, transform), text_width, 24.0, scale);
    text_style.white_space = crate::WhiteSpace::NoWrap;
    Ok(vec![DisplayCommand::Text {
        bounds,
        text,
        style: text_style,
    }])
}

fn parse_fill(
    mark: &ElementRef<'_>,
    style: &BTreeMap<String, String>,
    x: f32,
    y: f32,
    scale: f32,
) -> Result<Vec<DisplayCommand>, RasterError> {
    allow(
        style,
        &[
            "left",
            "top",
            "position",
            "width",
            "height",
            "background-color",
        ],
        mark,
    )?;
    Ok(vec![DisplayCommand::Fill {
        bounds: box_px(
            x,
            y,
            pixels(required(style, "width", mark)?)?,
            pixels(required(style, "height", mark)?)?,
            scale,
        ),
        color: hex_color(required(style, "background-color", mark)?)?,
    }])
}

fn parse_outline(
    mark: &ElementRef<'_>,
    style: &BTreeMap<String, String>,
    x: f32,
    y: f32,
    scale: f32,
) -> Result<Vec<DisplayCommand>, RasterError> {
    allow(
        style,
        &[
            "left",
            "top",
            "position",
            "width",
            "height",
            "border",
            "border-radius",
            "box-sizing",
        ],
        mark,
    )?;
    let mut output = ComputedStyle::ua_defaults("span", None);
    output.border = uniform_border(required(style, "border", mark)?, scale)?;
    output.border_radius = Some(CssLength::Px(
        pixels(required(style, "border-radius", mark)?)? * scale,
    ));
    Ok(vec![DisplayCommand::Border {
        bounds: box_px(
            x,
            y,
            pixels(required(style, "width", mark)?)?,
            pixels(required(style, "height", mark)?)?,
            scale,
        ),
        style: output,
    }])
}

fn parse_tick(
    inner: &ElementRef<'_>,
    style: &BTreeMap<String, String>,
    x: f32,
    y: f32,
    outer_transform: &str,
    scale: f32,
) -> Result<Vec<DisplayCommand>, RasterError> {
    allow(
        style,
        &[
            "background-color",
            "display",
            "font-size",
            "height",
            "line-height",
            "transform",
            "width",
        ],
        inner,
    )?;
    if required(style, "display", inner)? != "inline-block"
        || required(style, "font-size", inner)? != "0"
        || required(style, "line-height", inner)? != "0"
    {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "tick typography",
            &inner.html(),
        ));
    }
    let color = hex_color(required(style, "background-color", inner)?)?;
    let length = pixels(required(style, "height", inner)?)?;
    let thickness = pixels(required(style, "width", inner)?)?;
    let angle = style
        .get("transform")
        .map_or(Ok(0.0), |value| rotation(value))?;
    let (x, y) = translated_center(x, y, outer_transform)?;
    let radians = angle.to_radians();
    let dx = radians.sin() * length / 2.0;
    let dy = -radians.cos() * length / 2.0;
    Ok(vec![DisplayCommand::StrokeLine {
        from: ((x - dx) * scale, (y - dy) * scale),
        to: ((x + dx) * scale, (y + dy) * scale),
        width: thickness * scale,
        color,
    }])
}

fn text_style(
    element: &ElementRef<'_>,
    style: &BTreeMap<String, String>,
    scale: f32,
) -> Result<ComputedStyle, RasterError> {
    allow(style, &["color", "font-size", "white-space"], element)?;
    let mut result = ComputedStyle::ua_defaults("span", None);
    result.color = hex_color(required_plain(style, "color")?)?;
    result.font_size = CssLength::Px(pixels(required_plain(style, "font-size")?)? * scale);
    if elements(element)
        .iter()
        .any(|child| child.value().name() == "i")
    {
        result.font_style = crate::FontStyle::Italic;
    }
    Ok(result)
}

fn aligned_x(x: f32, width: f32, transform: &str) -> Result<f32, RasterError> {
    match transform {
        "translateX(-50%)" | "translate(-50%, -50%)" => Ok(x - width / 2.0),
        "none" => Ok(x),
        _ => Err(unsupported(
            UnsupportedFeatureKind::Property,
            "transform",
            transform,
        )),
    }
}
fn aligned_y(y: f32, transform: &str) -> f32 {
    if transform == "translate(-50%, -50%)" {
        y - 12.0
    } else {
        y
    }
}
fn translated_center(x: f32, y: f32, transform: &str) -> Result<(f32, f32), RasterError> {
    match transform {
        "translate(-50%, -50%)" => Ok((x, y)),
        "none" => Ok((x + 1.5, y + 8.0)),
        _ => Err(unsupported(
            UnsupportedFeatureKind::Property,
            "transform",
            transform,
        )),
    }
}
fn rotation(value: &str) -> Result<f32, RasterError> {
    value
        .strip_prefix("rotate(")
        .and_then(|v| v.strip_suffix("deg)"))
        .and_then(|v| v.parse().ok())
        .filter(|v: &f32| v.is_finite() && (-360.0..=360.0).contains(v))
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "transform", value))
}
fn uniform_border(value: &str, scale: f32) -> Result<Border, RasterError> {
    let mut parts = value.split_ascii_whitespace();
    let width = pixels(
        parts
            .next()
            .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "border", value))?,
    )?;
    if parts.next() != Some("solid") {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "border",
            value,
        ));
    }
    let side = BorderSide {
        width: CssLength::Px(width * scale),
        style: BorderStyle::Solid,
        color: hex_color(
            parts
                .next()
                .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "border", value))?,
        )?,
    };
    if parts.next().is_some() {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "border",
            value,
        ));
    }
    Ok(Border {
        top: side,
        right: side,
        bottom: side,
        left: side,
    })
}
fn elements<'a>(node: &ElementRef<'a>) -> Vec<ElementRef<'a>> {
    node.children().filter_map(ElementRef::wrap).collect()
}
fn one_child<'a>(
    node: &'a ElementRef<'a>,
    name: &str,
    source: &str,
) -> Result<ElementRef<'a>, RasterError> {
    let nodes = elements(node);
    if nodes.len() == 1 && nodes[0].value().name() == name {
        Ok(nodes[0])
    } else {
        Err(unsupported(UnsupportedFeatureKind::Tag, name, source))
    }
}
fn attrs(node: &ElementRef<'_>, allowed: &[&str]) -> Result<(), RasterError> {
    for (key, _) in &node.value().attrs {
        if !allowed.contains(&key.local.as_ref()) {
            return Err(unsupported(
                UnsupportedFeatureKind::Attribute,
                key.local.as_ref(),
                &node.html(),
            ));
        }
    }
    Ok(())
}
fn properties(node: &ElementRef<'_>) -> Result<BTreeMap<String, String>, RasterError> {
    let source = node
        .value()
        .attr("style")
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, "style", &node.html()))?;
    let mut values = BTreeMap::new();
    for declaration in source.split(';').filter(|v| !v.trim().is_empty()) {
        let (name, value) = declaration.split_once(':').ok_or_else(|| {
            unsupported(UnsupportedFeatureKind::Property, declaration, &node.html())
        })?;
        if values
            .insert(name.trim().to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                name,
                &node.html(),
            ));
        }
    }
    Ok(values)
}
fn allow(
    props: &BTreeMap<String, String>,
    allowed: &[&str],
    node: &ElementRef<'_>,
) -> Result<(), RasterError> {
    for name in props.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                name,
                &node.html(),
            ));
        }
    }
    Ok(())
}
fn required<'a>(
    props: &'a BTreeMap<String, String>,
    name: &str,
    node: &ElementRef<'_>,
) -> Result<&'a str, RasterError> {
    props
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, &node.html()))
}
fn required_plain<'a>(
    props: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, RasterError> {
    props
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, "restriction mark"))
}
fn pixels(value: &str) -> Result<f32, RasterError> {
    value
        .strip_suffix("px")
        .or_else(|| (value == "0").then_some("0"))
        .and_then(|v| v.parse().ok())
        .filter(|v: &f32| v.is_finite() && (-16.0..=800.0).contains(v))
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "pixel length", value))
}
fn hex_color(value: &str) -> Result<Color, RasterError> {
    u32::from_str_radix(
        value
            .strip_prefix('#')
            .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "color", value))?,
        16,
    )
    .ok()
    .filter(|_| value.len() == 7)
    .map(|v| Color((v >> 16) as u8, (v >> 8) as u8, v as u8, 255))
    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "color", value))
}
fn text_content(node: &ElementRef<'_>) -> Result<String, RasterError> {
    let mut text = String::new();
    for child in node.children() {
        match child.value() {
            Node::Text(value) => text.push_str(value),
            Node::Element(element) if element.name() == "i" => {
                let child = ElementRef::wrap(child).expect("element");
                text.push_str(&text_content(&child)?);
            }
            _ => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "restriction label content",
                    &node.html(),
                ));
            }
        }
    }
    Ok(text)
}
fn box_px(x: f32, y: f32, width: f32, height: f32, scale: f32) -> LayoutBox {
    LayoutBox {
        x: x * scale,
        y: y * scale,
        width: width * scale,
        height: height * scale,
    }
}
fn unsupported(kind: UnsupportedFeatureKind, name: impl Into<String>, source: &str) -> RasterError {
    UnsupportedTableFeature::new(kind, name, source).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_LINEAR: &str = "<table align='center' border='0' cellpadding='0' cellspacing='0' role='img' aria-label='Linear restriction-digest DNA map.' style='border-collapse:collapse;table-layout:fixed'><tbody><tr><td style='background-color:#ffffff;height:136px;padding:0;width:760px'><div style='height:136px;position:relative;width:760px'><span style='background-color:#b74300;height:6px;left:20px;position:absolute;top:68px;width:720px'>&#160;</span><span style='left:140px;position:absolute;top:59px;transform:none'><span style='background-color:#0067cc;display:inline-block;font-size:0;height:20px;line-height:0;width:3px'>&#160;</span></span><span style='left:140px;position:absolute;top:20px;transform:translateX(-50%)'><span style='color:#0067cc;font-size:18px;white-space:nowrap'><i>PmlI</i></span></span></div></td></tr></tbody></table>";

    #[test]
    fn resolves_linear_marks_to_native_commands_and_png() {
        let list = parse_restriction_digest_fragment(MINIMAL_LINEAR, &RasterConfig::default())
            .expect("declared linear scene parses");
        assert_eq!(list.bounds.width, 1520.0);
        assert_eq!(list.bounds.height, 272.0);
        assert!(matches!(list.commands[1], DisplayCommand::Fill { .. }));
        assert!(matches!(
            list.commands[2],
            DisplayCommand::StrokeLine { .. }
        ));
        assert!(matches!(list.commands[3], DisplayCommand::Text { .. }));
        let png = crate::paint_display_list(&list, 1520, 272).expect("scene paints");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn rejects_a_transform_outside_the_closed_map_grammar() {
        let invalid = MINIMAL_LINEAR.replace("transform:none", "transform:skewX(12deg)");
        assert!(matches!(
            parse_restriction_digest_fragment(&invalid, &RasterConfig::default()),
            Err(RasterError::Unsupported(UnsupportedFeatureKind::Property, name, _)) if name == "transform"
        ));
    }
}
