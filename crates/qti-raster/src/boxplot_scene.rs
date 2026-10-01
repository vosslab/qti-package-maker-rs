//! A deliberately small declarative scene for the harvested `table.boxplot` figures.
//!
//! This is not a CSS positioning engine.  It accepts only the generator's fixed table shape and
//! its literal, bounded style grammar, then resolves every mark to regular display commands.

use std::collections::{BTreeMap, BTreeSet};

use scraper::{ElementRef, Html, Node};

use crate::{
    Border, BorderSide, BorderStyle, Color, ComputedStyle, CssLength, DisplayCommand, DisplayList,
    LayoutBox, RasterConfig, RasterError, TextAlign, UnsupportedFeatureKind,
    UnsupportedTableFeature,
};

const SCENE_WIDTH: f32 = 480.0;
const SCENE_HEIGHT: f32 = 84.0;
const PLOT_X: f32 = 24.0;
const PLOT_Y: f32 = 4.0;
const PLOT_WIDTH: f32 = 432.0;
const MAX_MARKS: usize = 40;

/// Parses the known `table.boxplot` generator output into resolved display commands.
///
/// The input remains bounded by [`RasterConfig`] and is never executed, fetched, or delegated to
/// a general CSS interpreter.  This closed grammar is the validation boundary for the exception
/// approved in the R1 corpus ruling (ASVS 1.3.1 and 2.2.1).
pub fn parse_boxplot_fragment(
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
    let table = one_element(&root, "table", fragment)?;
    require_class(&table, "boxplot")?;
    validate_attributes(&table, &["class", "role", "style"])?;
    if table.value().attr("role") != Some("presentation") {
        return Err(unsupported(
            UnsupportedFeatureKind::Attribute,
            "role",
            &table.html(),
        ));
    }
    let table_style = properties(&table)?;
    require_exact(&table_style, "display", "block", &table)?;
    require_exact(&table_style, "border-collapse", "collapse", &table)?;
    require_exact(&table_style, "max-width", "480px", &table)?;
    require_exact(&table_style, "background-color", "white", &table)?;
    let table_color = color(required(&table_style, "color", &table)?)?;
    require_exact(&table_style, "margin", "4px 0", &table)?;
    allow_only(
        &table_style,
        &[
            "display",
            "border-collapse",
            "max-width",
            "background-color",
            "color",
            "margin",
        ],
        &table,
    )?;

    let tbody = only_child(&table, "tbody")?;
    validate_attributes(&tbody, &["style"])?;
    require_block(&tbody)?;
    let tr = only_child(&tbody, "tr")?;
    validate_attributes(&tr, &["style"])?;
    require_block(&tr)?;
    let td = only_child(&tr, "td")?;
    validate_attributes(&td, &["style"])?;
    let td_style = properties(&td)?;
    require_exact(&td_style, "display", "block", &td)?;
    require_exact(&td_style, "padding", "4px 24px", &td)?;
    allow_only(&td_style, &["display", "padding"], &td)?;

    let plot = direct_elements(&td)
        .into_iter()
        .find(|element| element.value().name() == "div")
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Tag, "relative plot div", &td.html()))?;
    let plot_style = properties(&plot)?;
    validate_attributes(&plot, &["style"])?;
    require_exact(&plot_style, "position", "relative", &plot)?;
    require_exact(&plot_style, "height", "76px", &plot)?;
    require_exact(&plot_style, "width", "100%", &plot)?;
    allow_only(&plot_style, &["position", "height", "width"], &plot)?;

    let scale = config.device_scale_factor as f32;
    let bounds = scaled_box(0.0, 0.0, SCENE_WIDTH, SCENE_HEIGHT, scale);
    let mut commands = vec![DisplayCommand::Fill {
        bounds,
        color: Color(255, 255, 255, 255),
    }];
    let marks = direct_elements(&plot);
    if marks.len() > MAX_MARKS {
        return Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "too many boxplot marks",
            &plot.html(),
        ));
    }
    for mark in marks {
        let tag = mark.value().name();
        if tag != "div" && tag != "span" {
            return Err(unsupported(UnsupportedFeatureKind::Tag, tag, &mark.html()));
        }
        commands.extend(parse_mark(&mark, table_color, scale)?);
    }
    Ok(DisplayList { bounds, commands })
}

fn parse_mark(
    element: &ElementRef<'_>,
    inherited_color: Color,
    scale: f32,
) -> Result<Vec<DisplayCommand>, RasterError> {
    validate_attributes(element, &["style"])?;
    let props = properties(element)?;
    require_exact(&props, "position", "absolute", element)?;
    let left = percent(required(&props, "left", element)?)?;
    let top = px(required(&props, "top", element)?)?;
    let width = props
        .get("width")
        .map_or(Ok(0.0), |value| length(value, PLOT_WIDTH))?;
    let height = props.get("height").map_or(Ok(0.0), |value| px(value))?;
    let margin_left = props
        .get("margin-left")
        .map_or(Ok(0.0), |value| px(value))?;
    let bounds = scaled_box(
        PLOT_X + PLOT_WIDTH * left + margin_left,
        PLOT_Y + top,
        width,
        height,
        scale,
    );
    let mut style = ComputedStyle::ua_defaults("span", None);
    style.color = props
        .get("color")
        .map_or(Ok(inherited_color), |value| color(value))?;
    style.background = props
        .get("background-color")
        .map_or(Ok(None), |value| color(value).map(Some))?;
    style.text_align = match props.get("text-align") {
        None => TextAlign::Start,
        Some(value) if value == "center" => TextAlign::Center,
        Some(value) => {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                "text-align",
                value,
            ));
        }
    };
    if let Some(font) = props.get("font") {
        apply_font(&mut style, font, scale)?;
    }
    if let Some(size) = props.get("font-size") {
        style.font_size = CssLength::Px(px(size)? * scale);
    }
    if let Some(line_height) = props.get("line-height") {
        style.line_height = Some(CssLength::Px(px(line_height)? * scale));
    }
    // The browser's Arial figures leave a visible gap in two-digit tick labels.  The bundled
    // accessible face has tighter numeric side bearings, so retain one CSS pixel of the observed
    // label breathing room after device scaling. This applies only to scene text, never table
    // prose or numeric answer content.
    if element.value().name() == "span" {
        style.letter_spacing = CssLength::Px(scale);
    }
    apply_borders(&mut style, &props, scale)?;
    validate_mark_properties(&props, element.value().name(), element)?;

    let mut commands = Vec::new();
    if style.background.is_some() || has_border(&style.border) {
        commands.push(DisplayCommand::Border {
            bounds,
            style: style.clone(),
        });
    }
    if element.value().name() == "span" {
        let text = literal_text(element)?;
        if text.is_empty() {
            return Err(unsupported(
                UnsupportedFeatureKind::Tag,
                "empty boxplot label",
                &element.html(),
            ));
        }
        commands.push(DisplayCommand::Text {
            bounds,
            text,
            style,
        });
    } else if !only_nbsp(element) {
        return Err(unsupported(
            UnsupportedFeatureKind::Tag,
            "boxplot mark content",
            &element.html(),
        ));
    }
    Ok(commands)
}

fn validate_mark_properties(
    props: &BTreeMap<String, String>,
    tag: &str,
    element: &ElementRef<'_>,
) -> Result<(), RasterError> {
    let div = [
        "position",
        "left",
        "top",
        "width",
        "height",
        "margin-left",
        "box-sizing",
        "font-size",
        "line-height",
        "background-color",
        "border",
        "border-top",
        "border-left",
    ];
    let span = [
        "position",
        "left",
        "top",
        "width",
        "height",
        "margin-left",
        "text-align",
        "color",
        "background-color",
        "font",
    ];
    allow_only(props, if tag == "div" { &div } else { &span }, element)?;
    if tag == "div" {
        require_exact(props, "box-sizing", "border-box", element)?;
    }
    Ok(())
}

fn apply_borders(
    style: &mut ComputedStyle,
    props: &BTreeMap<String, String>,
    scale: f32,
) -> Result<(), RasterError> {
    if let Some(value) = props.get("border") {
        let side = border(value, scale)?;
        style.border = Border {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
    }
    for (name, target) in [("border-top", 0), ("border-left", 3)] {
        if let Some(value) = props.get(name) {
            let side = border(value, scale)?;
            match target {
                0 => style.border.top = side,
                3 => style.border.left = side,
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

fn apply_font(style: &mut ComputedStyle, value: &str, scale: f32) -> Result<(), RasterError> {
    let value = value.strip_prefix("normal ").unwrap_or(value);
    let size = value
        .split_whitespace()
        .next()
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "font", value))?;
    let (size, line) = size
        .split_once('/')
        .map_or((size, None), |(size, line)| (size, Some(line)));
    style.font_size = CssLength::Px(px(size)? * scale);
    if let Some(line) = line {
        style.line_height = Some(CssLength::Px(px(line)? * scale));
    }
    if !value.contains("Arial, sans-serif") {
        return Err(unsupported(UnsupportedFeatureKind::Property, "font", value));
    }
    Ok(())
}

fn border(value: &str, scale: f32) -> Result<BorderSide, RasterError> {
    let mut fields = value.split_ascii_whitespace();
    let width = px(fields
        .next()
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "border", value))?)?;
    let style = match fields.next() {
        Some("solid") => BorderStyle::Solid,
        _ => {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                "border",
                value,
            ));
        }
    };
    let color = color(
        fields
            .next()
            .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "border", value))?,
    )?;
    if fields.next().is_some() {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "border",
            value,
        ));
    }
    Ok(BorderSide {
        width: CssLength::Px(width * scale),
        style,
        color,
    })
}

fn direct_elements<'a>(element: &ElementRef<'a>) -> Vec<ElementRef<'a>> {
    element.children().filter_map(ElementRef::wrap).collect()
}

fn one_element<'a>(
    root: &'a ElementRef<'a>,
    name: &str,
    snippet: &str,
) -> Result<ElementRef<'a>, RasterError> {
    let elements = direct_elements(root);
    if elements.len() == 1 && elements[0].value().name() == name {
        Ok(elements[0])
    } else {
        Err(unsupported(UnsupportedFeatureKind::Tag, name, snippet))
    }
}

fn only_child<'a>(parent: &'a ElementRef<'a>, name: &str) -> Result<ElementRef<'a>, RasterError> {
    let elements = direct_elements(parent);
    if elements.len() == 1 && elements[0].value().name() == name {
        Ok(elements[0])
    } else {
        Err(unsupported(
            UnsupportedFeatureKind::Tag,
            name,
            &parent.html(),
        ))
    }
}

fn require_block(element: &ElementRef<'_>) -> Result<(), RasterError> {
    let props = properties(element)?;
    require_exact(&props, "display", "block", element)?;
    allow_only(&props, &["display"], element)
}

fn require_class(element: &ElementRef<'_>, class: &str) -> Result<(), RasterError> {
    let classes = element
        .value()
        .attr("class")
        .unwrap_or("")
        .split_ascii_whitespace()
        .collect::<BTreeSet<_>>();
    if classes.len() == 1 && classes.contains(class) {
        Ok(())
    } else {
        Err(unsupported(
            UnsupportedFeatureKind::Attribute,
            "class",
            &element.html(),
        ))
    }
}

fn validate_attributes(element: &ElementRef<'_>, allowed: &[&str]) -> Result<(), RasterError> {
    for (key, _) in &element.value().attrs {
        if !allowed
            .iter()
            .any(|allowed_name| *allowed_name == key.local.as_ref())
        {
            return Err(unsupported(
                UnsupportedFeatureKind::Attribute,
                key.local.as_ref(),
                &element.html(),
            ));
        }
    }
    Ok(())
}

fn properties(element: &ElementRef<'_>) -> Result<BTreeMap<String, String>, RasterError> {
    let source = element
        .value()
        .attr("style")
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, "style", &element.html()))?;
    let mut properties = BTreeMap::new();
    for declaration in source.split(';').filter(|value| !value.trim().is_empty()) {
        let (name, value) = declaration.split_once(':').ok_or_else(|| {
            unsupported(
                UnsupportedFeatureKind::Property,
                declaration,
                &element.html(),
            )
        })?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name.is_empty() || value.is_empty() || properties.insert(name.clone(), value).is_some() {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                name,
                &element.html(),
            ));
        }
    }
    Ok(properties)
}

fn allow_only(
    props: &BTreeMap<String, String>,
    allowed: &[&str],
    element: &ElementRef<'_>,
) -> Result<(), RasterError> {
    for name in props.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                name,
                &element.html(),
            ));
        }
    }
    Ok(())
}

fn require_exact(
    props: &BTreeMap<String, String>,
    name: &str,
    expected: &str,
    element: &ElementRef<'_>,
) -> Result<(), RasterError> {
    if props.get(name).is_some_and(|value| value == expected) {
        Ok(())
    } else {
        Err(unsupported(
            UnsupportedFeatureKind::Property,
            name,
            &element.html(),
        ))
    }
}

fn required<'a>(
    props: &'a BTreeMap<String, String>,
    name: &str,
    element: &ElementRef<'_>,
) -> Result<&'a str, RasterError> {
    props
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, &element.html()))
}

fn only_nbsp(element: &ElementRef<'_>) -> bool {
    literal_text(element).is_ok_and(|text| text == "\u{00A0}")
}

fn literal_text(element: &ElementRef<'_>) -> Result<String, RasterError> {
    let mut text = String::new();
    for child in element.children() {
        match child.value() {
            Node::Text(value) => text.push_str(value),
            _ => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Tag,
                    "nested boxplot content",
                    &element.html(),
                ));
            }
        }
    }
    Ok(text)
}

fn percent(value: &str) -> Result<f32, RasterError> {
    value
        .strip_suffix('%')
        .and_then(|number| number.parse().ok())
        .filter(|value: &f32| value.is_finite() && (0.0..=100.0).contains(value))
        .map(|value| value / 100.0)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "percent", value))
}
fn px(value: &str) -> Result<f32, RasterError> {
    if value == "0" {
        return Ok(0.0);
    }
    value
        .strip_suffix("px")
        .and_then(|number| number.parse().ok())
        .filter(|value: &f32| value.is_finite() && (-64.0..=512.0).contains(value))
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, "px length", value))
}
fn length(value: &str, percentage_base: f32) -> Result<f32, RasterError> {
    if value.ends_with('%') {
        percent(value).map(|value| value * percentage_base)
    } else {
        px(value)
    }
}
fn color(value: &str) -> Result<Color, RasterError> {
    match value {
        "white" => Ok(Color(255, 255, 255, 255)),
        "black" => Ok(Color(0, 0, 0, 255)),
        _ if value.len() == 7 && value.starts_with('#') => u32::from_str_radix(&value[1..], 16)
            .map(|rgb| Color((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255))
            .map_err(|_| unsupported(UnsupportedFeatureKind::Property, "color", value)),
        _ => Err(unsupported(
            UnsupportedFeatureKind::Property,
            "color",
            value,
        )),
    }
}
fn has_border(border: &Border) -> bool {
    border.top.style != BorderStyle::None
        || border.right.style != BorderStyle::None
        || border.bottom.style != BorderStyle::None
        || border.left.style != BorderStyle::None
}
fn scaled_box(x: f32, y: f32, width: f32, height: f32, scale: f32) -> LayoutBox {
    LayoutBox {
        x: x * scale,
        y: y * scale,
        width: width * scale,
        height: height * scale,
    }
}
fn unsupported(
    kind: UnsupportedFeatureKind,
    name: impl Into<String>,
    snippet: &str,
) -> RasterError {
    UnsupportedTableFeature::new(kind, name, snippet).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_SCENE: &str = "<table class='boxplot' role='presentation' style='display:block;border-collapse:collapse;max-width:480px;background-color:white;color:#172333;margin:4px 0'><tbody style='display:block'><tr style='display:block'><td style='display:block;padding:4px 24px'><div style='position:relative;height:76px;width:100%'><div style='position:absolute;left:10%;width:20%;top:6px;height:32px;box-sizing:border-box;font-size:0;line-height:0;background-color:#dceaf5;border:2px solid #777777'>&#160;</div><div style='position:absolute;left:0%;width:100%;top:50px;height:0px;box-sizing:border-box;font-size:0;line-height:0;border-top:1px solid #172333'>&#160;</div><span style='position:absolute;left:50%;top:60px;width:40px;margin-left:-20px;text-align:center;font:13px Arial, sans-serif'>10</span></div></td></tr></tbody></table>";

    #[test]
    fn resolves_geometry_and_renders_a_valid_boxplot_png() {
        let list = parse_boxplot_fragment(MINIMAL_SCENE, &RasterConfig::default()).unwrap();
        assert_eq!(
            list.bounds,
            LayoutBox {
                x: 0.0,
                y: 0.0,
                width: 960.0,
                height: 168.0,
            }
        );
        assert_eq!(list.commands.len(), 4);
        let png = crate::paint_display_list(&list, 960, 168).unwrap();
        let image = tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!(
            &image.data()[(36 * 960 + 180) * 4..(36 * 960 + 181) * 4],
            &[220, 234, 245, 255]
        );
    }

    #[test]
    fn rejects_positioning_outside_the_closed_grammar() {
        let source = "<table class='boxplot' role='presentation' style='display:block;border-collapse:collapse;max-width:480px;background-color:white;color:#172333;margin:4px 0'><tbody style='display:block'><tr style='display:block'><td style='display:block;padding:4px 24px'><div style='position:relative;height:76px;width:100%'><span style='position:absolute;left:0%;top:1px;transform:rotate(2deg)'>x</span></div></td></tr></tbody></table>";
        let result = parse_boxplot_fragment(source, &RasterConfig::default());
        assert!(matches!(
            result,
            Err(RasterError::Unsupported(
                UnsupportedFeatureKind::Property,
                _,
                _
            ))
        ));
    }
}
