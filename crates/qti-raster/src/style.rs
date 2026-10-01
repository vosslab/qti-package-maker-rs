//! CSS-subset values, user-agent defaults, and the allowlisted inline cascade.

use cssparser::{Parser, ParserInput};

use crate::subset::{UnsupportedFeatureKind, UnsupportedTableFeature};

/// An RGBA color in source order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

/// A supported CSS length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Em(f32),
    Pt(f32),
    Zero,
}
/// The two bundled font families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamily {
    AtkinsonNext,
    AtkinsonMono,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Numeric(u16),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextAlign {
    Start,
    Left,
    Right,
    Center,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
    Baseline,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WhiteSpace {
    Normal,
    NoWrap,
    Pre,
}
/// The declared display behavior needed by the closed table subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayMode {
    Inline,
    Block,
    InlineBlock,
    InlineTable,
    Table,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptionSide {
    Top,
    Bottom,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorderStyle {
    None,
    /// CSS 2.1 collapsed-border winner: hides this edge and every competing border.
    Hidden,
    Solid,
    Dashed,
    Dotted,
    Double,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSide {
    pub width: CssLength,
    pub style: BorderStyle,
    pub color: Color,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Border {
    pub top: BorderSide,
    pub right: BorderSide,
    pub bottom: BorderSide,
    pub left: BorderSide,
}

/// The one outer shadow grammar used by harvested tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    pub x: CssLength,
    pub y: CssLength,
    pub blur: CssLength,
    pub spread: CssLength,
    pub color: Color,
}

/// Values inherited by inline descendants and box values consumed by later work packages.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyle {
    pub color: Color,
    pub font_family: FontFamily,
    pub font_size: CssLength,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub text_align: TextAlign,
    pub vertical_align: VerticalAlign,
    pub white_space: WhiteSpace,
    pub line_height: Option<CssLength>,
    /// Inherited extra advance after each glyph. The supported grammar is non-negative pixels.
    pub letter_spacing: CssLength,
    /// The sole observed bracket-span transform. It affects only inline horizontal glyph advance.
    pub inline_scale_x: Option<f32>,
    pub width: Option<CssLength>,
    pub min_width: Option<CssLength>,
    pub height: Option<CssLength>,
    pub max_width: Option<CssLength>,
    pub background: Option<Color>,
    pub border: Border,
    pub border_radius: Option<CssLength>,
    pub padding: [CssLength; 4],
    /// Whether each padding side was declared in inline CSS. Layout uses this to distinguish an
    /// explicit `padding: 0` from an omitted value that may inherit HTML `cellpadding`.
    pub padding_declared: [bool; 4],
    pub margin: [CssLength; 4],
    /// Exact `margin:0 auto` on a finite-width table; R2 resolves its centered table x offset.
    pub table_auto_horizontal_margins: bool,
    pub overflow_hidden: bool,
    pub table_layout_fixed: bool,
    pub border_collapse: bool,
    pub border_spacing: CssLength,
    pub display: DisplayMode,
    /// Hidden content retains layout dimensions but produces no paint operations.
    pub visible: bool,
    pub caption_side: CaptionSide,
    /// A single outer shadow resolved by paint before the element background and border.
    pub box_shadow: Option<BoxShadow>,
    /// `position: relative` is retained only until the parser verifies the inline-table rule.
    pub relative_positioned: bool,
    /// The paint-only top adjustment for the approved inline-table pattern.
    pub relative_top: Option<CssLength>,
}
const ZERO: CssLength = CssLength::Zero;
const BLACK: Color = Color(0, 0, 0, 255);
const NONE_BORDER: BorderSide = BorderSide {
    width: ZERO,
    style: BorderStyle::None,
    color: BLACK,
};

impl ComputedStyle {
    #[must_use]
    pub fn ua_defaults(tag: &str, parent: Option<&Self>) -> Self {
        let mut style = Self {
            color: BLACK,
            font_family: FontFamily::AtkinsonNext,
            font_size: CssLength::Px(16.0),
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            text_align: TextAlign::Start,
            vertical_align: VerticalAlign::Middle,
            white_space: WhiteSpace::Normal,
            line_height: None,
            letter_spacing: ZERO,
            inline_scale_x: None,
            width: None,
            min_width: None,
            height: None,
            max_width: None,
            background: None,
            border: Border {
                top: NONE_BORDER,
                right: NONE_BORDER,
                bottom: NONE_BORDER,
                left: NONE_BORDER,
            },
            border_radius: None,
            padding: [ZERO; 4],
            padding_declared: [false; 4],
            margin: [ZERO; 4],
            table_auto_horizontal_margins: false,
            overflow_hidden: false,
            table_layout_fixed: false,
            border_collapse: false,
            border_spacing: CssLength::Px(2.0),
            display: match tag {
                "table" => DisplayMode::Table,
                "caption" | "div" | "p" | "ul" | "ol" | "li" => DisplayMode::Block,
                _ => DisplayMode::Inline,
            },
            visible: true,
            caption_side: CaptionSide::Top,
            box_shadow: None,
            relative_positioned: false,
            relative_top: None,
        };
        // Only CSS's inherited text properties cross an element boundary. Box values must remain
        // local so a cell cannot accidentally acquire its table's border, background, or size.
        if let Some(parent) = parent {
            style.color = parent.color;
            style.font_family = parent.font_family;
            style.font_size = parent.font_size;
            style.font_weight = parent.font_weight;
            style.font_style = parent.font_style;
            style.text_align = parent.text_align;
            style.white_space = parent.white_space;
            style.line_height = parent.line_height;
            style.letter_spacing = parent.letter_spacing;
            // `visibility` is inherited: descendants of a hidden span keep their geometry but
            // cannot reappear unless they explicitly declare `visibility:visible`.
            style.visible = parent.visible;
        }
        if tag == "th" {
            style.font_weight = FontWeight::Bold;
            style.text_align = TextAlign::Center;
        }
        style
    }
}

pub(crate) fn apply_presentational(
    style: &mut ComputedStyle,
    name: &str,
    value: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    match name {
        "bgcolor" => {
            style.background = Some(
                legacy_color(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?,
            )
        }
        "align" => {
            style.text_align = text_align(value)
                .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?
        }
        "valign" => {
            style.vertical_align = vertical_align(value)
                .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?
        }
        "width" => {
            style.width = Some(
                presentational_length(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?,
            )
        }
        "height" => {
            style.height = Some(
                presentational_length(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?,
            )
        }
        "border" => apply_presentational_border(style, value, snippet)?,
        "cellpadding" | "cellspacing" | "colspan" | "rowspan" | "span" => {}
        _ => {
            return Err(unsupported(
                UnsupportedFeatureKind::Attribute,
                name,
                snippet,
            ));
        }
    };
    Ok(())
}

/// Applies the limited legacy `<font>` attributes recorded in the corpus.
pub(crate) fn apply_legacy_font(
    style: &mut ComputedStyle,
    name: &str,
    value: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    match name {
        "size" => {
            style.font_size = legacy_font_size(value, style.font_size)
                .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?;
        }
        "color" => {
            style.color = color(value)
                .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, name, snippet))?;
        }
        "face" => {
            style.font_family = if value.to_ascii_lowercase().contains("mono")
                || value.to_ascii_lowercase().contains("courier")
            {
                FontFamily::AtkinsonMono
            } else {
                FontFamily::AtkinsonNext
            };
        }
        _ => {
            return Err(unsupported(
                UnsupportedFeatureKind::Attribute,
                name,
                snippet,
            ));
        }
    }
    Ok(())
}

pub(crate) fn apply_inline(
    style: &mut ComputedStyle,
    input: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    // cssparser performs lexical validation; the allowlist below controls meaning (ASVS 1.3.5).
    let mut tokens = ParserInput::new(input);
    let mut parser = Parser::new(&mut tokens);
    while parser.next_including_whitespace_and_comments().is_ok() {}
    for declaration in input.split(';').filter(|part| !part.trim().is_empty()) {
        let Some((name, value)) = declaration.split_once(':') else {
            return Err(unsupported(
                UnsupportedFeatureKind::Property,
                declaration.trim(),
                snippet,
            ));
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "color" if bare_hex_color(value) => {
                // Corpus compatibility receipt: this malformed CSS declaration is ignored by the
                // browser, leaving the inherited color intact.
            }
            "color" => style.color = required_color(value, &name, snippet)?,
            "background" | "background-color" => {
                style.background = Some(required_color(value, &name, snippet)?)
            }
            "font-family" => {
                style.font_family = if value.contains("monospace") {
                    FontFamily::AtkinsonMono
                } else if value.contains("sans-serif") || value.contains("Atkinson") {
                    FontFamily::AtkinsonNext
                } else {
                    return Err(unsupported(
                        UnsupportedFeatureKind::Property,
                        &name,
                        snippet,
                    ));
                }
            }
            "font-size" => {
                style.font_size = font_size(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, &name, snippet))?
            }
            "font-weight" => {
                style.font_weight = font_weight(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, &name, snippet))?
            }
            "font-style" => {
                style.font_style = match value {
                    "normal" => FontStyle::Normal,
                    "italic" => FontStyle::Italic,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "text-align" => {
                style.text_align = text_align(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, &name, snippet))?
            }
            "vertical-align" => {
                style.vertical_align = vertical_align(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, &name, snippet))?
            }
            "white-space" => {
                style.white_space = match value {
                    "normal" => WhiteSpace::Normal,
                    "nowrap" => WhiteSpace::NoWrap,
                    "pre" => WhiteSpace::Pre,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "line-height" => {
                style.line_height =
                    Some(line_height_length(value).ok_or_else(|| {
                        unsupported(UnsupportedFeatureKind::Property, &name, snippet)
                    })?)
            }
            "letter-spacing" => {
                style.letter_spacing = nonnegative_px(value)
                    .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, &name, snippet))?
            }
            "width" if value == "auto" => style.width = None,
            "width" => style.width = Some(required_length(value, &name, snippet)?),
            "min-width" => style.min_width = Some(required_length(value, &name, snippet)?),
            "height" => {
                style.height =
                    Some(layout_length(value).ok_or_else(|| {
                        unsupported(UnsupportedFeatureKind::Property, &name, snippet)
                    })?)
            }
            "max-width" => style.max_width = Some(required_length(value, &name, snippet)?),
            "padding" => {
                style.padding = box_values(value, &name, snippet)?;
                style.padding_declared = [true; 4];
            }
            "margin" if value == "0 auto" => {
                style.margin = [ZERO; 4];
                style.table_auto_horizontal_margins = true;
            }
            "margin" => {
                style.margin = box_values(value, &name, snippet)?;
                style.table_auto_horizontal_margins = false;
            }
            "padding-top" => {
                style.padding[0] = required_length(value, &name, snippet)?;
                style.padding_declared[0] = true;
            }
            "padding-right" => {
                style.padding[1] = required_length(value, &name, snippet)?;
                style.padding_declared[1] = true;
            }
            "padding-bottom" => {
                style.padding[2] = required_length(value, &name, snippet)?;
                style.padding_declared[2] = true;
            }
            "padding-left" => {
                style.padding[3] = required_length(value, &name, snippet)?;
                style.padding_declared[3] = true;
            }
            "margin-top" => {
                style.margin[0] = required_length(value, &name, snippet)?;
                style.table_auto_horizontal_margins = false;
            }
            "margin-right" => {
                style.margin[1] = required_length(value, &name, snippet)?;
                style.table_auto_horizontal_margins = false;
            }
            "margin-bottom" => {
                style.margin[2] = required_length(value, &name, snippet)?;
                style.table_auto_horizontal_margins = false;
            }
            "margin-left" => {
                style.margin[3] = required_length(value, &name, snippet)?;
                style.table_auto_horizontal_margins = false;
            }
            "border" => apply_border(style, value, snippet)?,
            "border-top" => style.border.top = border_side(value, snippet)?,
            "border-right" => style.border.right = border_side(value, snippet)?,
            "border-bottom" => style.border.bottom = border_side(value, snippet)?,
            "border-left" => style.border.left = border_side(value, snippet)?,
            "border-width" => {
                let widths = box_values(value, &name, snippet)?;
                style.border.top.width = widths[0];
                style.border.right.width = widths[1];
                style.border.bottom.width = widths[2];
                style.border.left.width = widths[3];
            }
            "border-style" => {
                let styles = border_styles(value, &name, snippet)?;
                style.border.top.style = styles[0];
                style.border.right.style = styles[1];
                style.border.bottom.style = styles[2];
                style.border.left.style = styles[3];
            }
            "border-color" => {
                let colors = box_colors(value, &name, snippet)?;
                style.border.top.color = colors[0];
                style.border.right.color = colors[1];
                style.border.bottom.color = colors[2];
                style.border.left.color = colors[3];
            }
            "border-radius"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-left-radius"
            | "border-bottom-right-radius" => {
                style.border_radius = Some(required_length(
                    value.split_whitespace().next().unwrap_or_default(),
                    &name,
                    snippet,
                )?)
            }
            "overflow" => {
                style.overflow_hidden = match value {
                    "hidden" => true,
                    "visible" => false,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "table-layout" => {
                style.table_layout_fixed = match value {
                    "fixed" => true,
                    "auto" => false,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "border-collapse" => {
                style.border_collapse = match value {
                    "collapse" => true,
                    "separate" => false,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "border-spacing" => style.border_spacing = required_length(value, &name, snippet)?,
            "display" => {
                style.display = match value {
                    "inline" => DisplayMode::Inline,
                    "block" => DisplayMode::Block,
                    "inline-block" => DisplayMode::InlineBlock,
                    "inline-table" => DisplayMode::InlineTable,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "visibility" => {
                style.visible = match value {
                    "visible" => true,
                    "hidden" => false,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "caption-side" => {
                style.caption_side = match value {
                    "top" => CaptionSide::Top,
                    "bottom" => CaptionSide::Bottom,
                    _ => {
                        return Err(unsupported(
                            UnsupportedFeatureKind::Property,
                            &name,
                            snippet,
                        ));
                    }
                }
            }
            "box-shadow" => style.box_shadow = Some(box_shadow(value, snippet)?),
            "transform" if value == "scale(1.35)" => style.inline_scale_x = Some(1.35),
            "position" => {
                if value != "relative" {
                    return Err(unsupported(
                        UnsupportedFeatureKind::Property,
                        &name,
                        snippet,
                    ));
                }
                style.relative_positioned = true;
            }
            "top" => {
                style.relative_top =
                    Some(relative_top(value).ok_or_else(|| {
                        unsupported(UnsupportedFeatureKind::Property, &name, snippet)
                    })?)
            }
            // Corpus compatibility receipt: the generator's spelling is ignored by browsers.
            "vert-align" => {}
            // Corpus compatibility receipt: this generator-only property has no CSS meaning.
            "spacing" if value == "20px" => {}
            _ => {
                return Err(unsupported(
                    UnsupportedFeatureKind::Property,
                    &name,
                    snippet,
                ));
            }
        }
    }
    Ok(())
}

/// Verifies the few general-table extensions after all declarations have cascaded.
pub(crate) fn validate_general_position(
    style: &ComputedStyle,
    tag: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    if style.inline_scale_x.is_some()
        && !(tag == "span"
            && style.display == DisplayMode::InlineBlock
            && style.font_size == CssLength::Px(32.0))
    {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "transform",
            snippet,
        ));
    }
    if style.table_auto_horizontal_margins && tag != "table" {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "margin",
            snippet,
        ));
    }
    if !style.relative_positioned && style.relative_top.is_none() {
        return Ok(());
    }
    if tag == "table"
        && style.display == DisplayMode::InlineTable
        && style.relative_positioned
        && style.relative_top.is_some()
    {
        return Ok(());
    }
    Err(unsupported(
        UnsupportedFeatureKind::Property,
        "position/top",
        snippet,
    ))
}

fn unsupported(kind: UnsupportedFeatureKind, name: &str, snippet: &str) -> UnsupportedTableFeature {
    UnsupportedTableFeature::new(kind, name, snippet)
}
fn required_length(
    value: &str,
    name: &str,
    snippet: &str,
) -> Result<CssLength, UnsupportedTableFeature> {
    length(value).ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, snippet))
}
fn required_color(
    value: &str,
    name: &str,
    snippet: &str,
) -> Result<Color, UnsupportedTableFeature> {
    color(value).ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, snippet))
}
fn length(value: &str) -> Option<CssLength> {
    let value = value.trim();
    if value == "0" {
        return Some(ZERO);
    }
    for (suffix, make) in [
        ("px", CssLength::Px as fn(f32) -> _),
        ("%", CssLength::Percent),
        ("em", CssLength::Em),
        ("pt", CssLength::Pt),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            return number
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|number| number.is_finite())
                .map(make);
        }
    }
    None
}
fn nonnegative_px(value: &str) -> Option<CssLength> {
    let length = absolute_px(value)?;
    match length {
        CssLength::Px(number) if number >= 0.0 => Some(length),
        CssLength::Zero => Some(length),
        _ => None,
    }
}
fn absolute_px(value: &str) -> Option<CssLength> {
    let value = value.trim();
    if value == "0" {
        return Some(ZERO);
    }
    let number = value.strip_suffix("px")?.trim().parse::<f32>().ok()?;
    number.is_finite().then_some(CssLength::Px(number))
}
fn relative_top(value: &str) -> Option<CssLength> {
    let number = value
        .trim()
        .strip_suffix("em")?
        .trim()
        .parse::<f32>()
        .ok()?;
    number.is_finite().then_some(CssLength::Em(number))
}
/// The corpus has row-height declarations written as bare CSS numbers. The R1 ruling treats
/// these as CSS pixels for table geometry; other CSS length properties remain unit-strict.
fn layout_length(value: &str) -> Option<CssLength> {
    length(value).or_else(|| value.trim().parse().ok().map(CssLength::Px))
}
/// Unitless CSS `line-height` is a multiplier of the element's font size.
fn line_height_length(value: &str) -> Option<CssLength> {
    length(value).or_else(|| value.trim().parse().ok().map(CssLength::Em))
}
/// HTML presentational dimensions use CSS pixels when supplied as a bare number. Inline CSS
/// remains stricter and must provide a unit except for zero.
fn presentational_length(value: &str) -> Option<CssLength> {
    length(value).or_else(|| value.trim().parse().ok().map(CssLength::Px))
}
fn text_align(value: &str) -> Option<TextAlign> {
    match value {
        "left" => Some(TextAlign::Left),
        "right" => Some(TextAlign::Right),
        "center" => Some(TextAlign::Center),
        "start" => Some(TextAlign::Start),
        _ => None,
    }
}
fn vertical_align(value: &str) -> Option<VerticalAlign> {
    match value {
        "top" => Some(VerticalAlign::Top),
        "middle" => Some(VerticalAlign::Middle),
        "bottom" => Some(VerticalAlign::Bottom),
        "baseline" => Some(VerticalAlign::Baseline),
        _ => None,
    }
}
fn font_weight(value: &str) -> Option<FontWeight> {
    match value {
        "normal" => Some(FontWeight::Normal),
        "bold" => Some(FontWeight::Bold),
        _ => value.parse().ok().map(FontWeight::Numeric),
    }
}
fn font_size(value: &str) -> Option<CssLength> {
    match value {
        "xx-small" => Some(CssLength::Px(9.0)),
        "x-small" => Some(CssLength::Px(10.0)),
        "small" => Some(CssLength::Px(13.0)),
        "medium" => Some(CssLength::Px(16.0)),
        "large" => Some(CssLength::Px(18.0)),
        "x-large" => Some(CssLength::Px(24.0)),
        "xx-large" => Some(CssLength::Px(32.0)),
        _ => length(value),
    }
}
fn legacy_font_size(value: &str, inherited: CssLength) -> Option<CssLength> {
    let absolute = |size| match size {
        1 => Some(CssLength::Px(10.0)),
        2 => Some(CssLength::Px(13.0)),
        3 => Some(CssLength::Px(16.0)),
        4 => Some(CssLength::Px(18.0)),
        5 => Some(CssLength::Px(24.0)),
        6 => Some(CssLength::Px(32.0)),
        7 => Some(CssLength::Px(48.0)),
        _ => None,
    };
    let value = value.trim();
    if let Some(relative) = value.strip_prefix(['+', '-']) {
        let delta = relative.parse::<i32>().ok()? * if value.starts_with('-') { -1 } else { 1 };
        let inherited = match inherited {
            CssLength::Px(value) => value,
            _ => 16.0,
        };
        return Some(CssLength::Px(
            (inherited + delta as f32 * 3.0).clamp(1.0, 48.0),
        ));
    }
    absolute(value.parse().ok()?)
}
fn box_values(
    value: &str,
    name: &str,
    snippet: &str,
) -> Result<[CssLength; 4], UnsupportedTableFeature> {
    let values: Result<Vec<_>, _> = value
        .split_whitespace()
        .map(|part| required_length(part, name, snippet))
        .collect();
    let values = values?;
    match values.as_slice() {
        [a] => Ok([*a; 4]),
        [a, b] => Ok([*a, *b, *a, *b]),
        [a, b, c] => Ok([*a, *b, *c, *b]),
        [a, b, c, d] => Ok([*a, *b, *c, *d]),
        _ => Err(unsupported(UnsupportedFeatureKind::Property, name, snippet)),
    }
}
fn border_styles(
    value: &str,
    name: &str,
    snippet: &str,
) -> Result<[BorderStyle; 4], UnsupportedTableFeature> {
    let values: Result<Vec<_>, _> = value
        .split_whitespace()
        .map(|part| {
            match part {
                "none" => Some(BorderStyle::None),
                "hidden" => Some(BorderStyle::Hidden),
                "solid" => Some(BorderStyle::Solid),
                "dashed" => Some(BorderStyle::Dashed),
                "dotted" => Some(BorderStyle::Dotted),
                "double" => Some(BorderStyle::Double),
                _ => None,
            }
            .ok_or_else(|| unsupported(UnsupportedFeatureKind::Property, name, snippet))
        })
        .collect();
    four_values(&values?, name, snippet)
}
fn box_colors(
    value: &str,
    name: &str,
    snippet: &str,
) -> Result<[Color; 4], UnsupportedTableFeature> {
    let values: Result<Vec<_>, _> = value
        .split_whitespace()
        .map(|part| required_color(part, name, snippet))
        .collect();
    four_values(&values?, name, snippet)
}
fn four_values<T: Copy>(
    values: &[T],
    name: &str,
    snippet: &str,
) -> Result<[T; 4], UnsupportedTableFeature> {
    match values {
        [a] => Ok([*a; 4]),
        [a, b] => Ok([*a, *b, *a, *b]),
        [a, b, c] => Ok([*a, *b, *c, *b]),
        [a, b, c, d] => Ok([*a, *b, *c, *d]),
        _ => Err(unsupported(UnsupportedFeatureKind::Property, name, snippet)),
    }
}

fn box_shadow(value: &str, snippet: &str) -> Result<BoxShadow, UnsupportedTableFeature> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if !(4..=5).contains(&parts.len())
        || parts.iter().any(|part| part.eq_ignore_ascii_case("inset"))
    {
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "box-shadow",
            snippet,
        ));
    }
    let x = absolute_px(parts[0]);
    let y = absolute_px(parts[1]);
    let blur = nonnegative_px(parts[2]);
    let (spread, color) = if parts.len() == 4 {
        (Some(ZERO), color(parts[3]))
    } else {
        (absolute_px(parts[3]), color(parts[4]))
    };
    match (x, y, blur, spread, color) {
        (Some(x), Some(y), Some(blur), Some(spread), Some(color)) => Ok(BoxShadow {
            x,
            y,
            blur,
            spread,
            color,
        }),
        _ => Err(unsupported(
            UnsupportedFeatureKind::Property,
            "box-shadow",
            snippet,
        )),
    }
}

/// Legacy HTML `bgcolor` permits a six-digit hexadecimal value without `#`.
/// This does not extend CSS: malformed bare CSS color declarations remain a browser no-op.
fn legacy_color(value: &str) -> Option<Color> {
    color(value).or_else(|| {
        bare_hex_color(value).then(|| {
            let prefixed = format!("#{}", value.trim());
            color(&prefixed)
        })?
    })
}

fn apply_border(
    style: &mut ComputedStyle,
    value: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() == 1 && matches!(parts[0], "0" | "none") {
        style.border = Border {
            top: NONE_BORDER,
            right: NONE_BORDER,
            bottom: NONE_BORDER,
            left: NONE_BORDER,
        };
        return Ok(());
    }
    let side = border_side(value, snippet)?;
    style.border = Border {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
    Ok(())
}
fn apply_presentational_border(
    style: &mut ComputedStyle,
    value: &str,
    snippet: &str,
) -> Result<(), UnsupportedTableFeature> {
    if matches!(value.trim(), "0" | "none") {
        return apply_border(style, value, snippet);
    }
    let width = presentational_length(value)
        .ok_or_else(|| unsupported(UnsupportedFeatureKind::Attribute, "border", snippet))?;
    let side = BorderSide {
        width,
        style: BorderStyle::Solid,
        color: BLACK,
    };
    style.border = Border {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
    Ok(())
}
fn border_side(value: &str, snippet: &str) -> Result<BorderSide, UnsupportedTableFeature> {
    if matches!(value.trim(), "0" | "none") {
        return Ok(NONE_BORDER);
    }
    let mut width = None;
    let mut style = None;
    let mut color_value = None;
    for part in value.split_whitespace() {
        if width.is_none() {
            width = length(part);
            if width.is_some() {
                continue;
            }
        }
        if style.is_none() {
            style = match part {
                "none" => Some(BorderStyle::None),
                "hidden" => Some(BorderStyle::Hidden),
                "solid" => Some(BorderStyle::Solid),
                "dashed" => Some(BorderStyle::Dashed),
                "dotted" => Some(BorderStyle::Dotted),
                "double" => Some(BorderStyle::Double),
                _ => None,
            };
            if style.is_some() {
                continue;
            }
        }
        if color_value.is_none() {
            color_value = color(part);
            if color_value.is_some() {
                continue;
            }
        }
        return Err(unsupported(
            UnsupportedFeatureKind::Property,
            "border",
            snippet,
        ));
    }
    Ok(BorderSide {
        width: width.unwrap_or(ZERO),
        style: style.unwrap_or(BorderStyle::None),
        color: color_value.unwrap_or(BLACK),
    })
}

mod colors;
use colors::{bare_hex_color, color};

#[cfg(test)]
mod tests;
