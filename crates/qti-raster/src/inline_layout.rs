//! Intrinsic measurement and line layout for the table renderer's inline subset.
//!
//! This module deliberately accepts the already allowlisted [`StyledNode`] tree.  It does not
//! parse HTML or CSS, fetch fonts, or inspect host font directories.  Its output is stable layout
//! geometry and glyph identifiers for the paint phase.

use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, Metrics, Shaping, Style as CosmicStyle, Weight, Wrap,
};

use crate::fonts::{ATKINSON_MONO_FAMILY, ATKINSON_NEXT_FAMILY, with_font_system};
use crate::{
    Color, ComputedStyle, CssLength, DisplayMode, ElementKind, FontFamily, FontStyle, FontWeight,
    LayoutBox, SceneLeaf, StyledNode, StyledNodeKind, TextAlign, VerticalAlign, WhiteSpace,
};

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT_FACTOR: f32 = 1.2;
const SUB_SUP_SIZE_FACTOR: f32 = 0.83;
const SUB_SHIFT_FACTOR: f32 = 0.2;
const SUP_SHIFT_FACTOR: f32 = -0.33;

/// The content-only widths consumed by the CSS table layout algorithm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntrinsicWidths {
    /// Smallest width that permits only CSS-allowed break opportunities.
    pub min_width: f32,
    /// Width of the content with normal wrapping disabled.
    pub max_width: f32,
}

/// A bundled face selected for a painted glyph run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontFaceKey {
    /// Atkinson face family selected from CSS `font-family`.
    pub family: FontFamily,
    /// CSS weight resolved to a variable-font axis value.
    pub weight: u16,
    /// CSS normal or italic font style.
    pub style: FontStyle,
    /// The rendered font size in device-independent pixels.
    pub size: f32,
}

/// One paint-ready glyph in the thread-local bundled-font cache.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// Cosmic Text cache key, including the resolved bundled font and variation axes.
    pub cache_key: CacheKey,
    /// Pixel origin of the rasterized glyph in cell-content coordinates.
    pub x: i32,
    /// Pixel origin of the rasterized glyph in cell-content coordinates.
    pub y: i32,
}

/// A paint-ready consecutive glyph run with one CSS face and color.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    /// Bounding rectangle in cell-content coordinates.
    pub bounds: LayoutBox,
    /// Baseline in cell-content coordinates.
    pub baseline_y: f32,
    /// Foreground color after CSS inheritance.
    pub color: Color,
    /// Face data required to recover the bundled font when painting.
    pub face: FontFaceKey,
    /// Horizontal paint scale for the allowlisted inline bracket transform.
    ///
    /// Glyph origins and bounds already include this scale in their inline advance.  The painter
    /// applies it to each cached glyph bitmap about that origin.
    pub scale_x: f32,
    /// Glyphs positioned relative to `bounds`.
    pub glyphs: Vec<PositionedGlyph>,
}

/// One final visual text line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineLine {
    /// Content-relative line rectangle.
    pub bounds: LayoutBox,
    /// Content-relative baseline used for vertical alignment by table layout.
    pub baseline_y: f32,
}

/// The final layout of a cell's inline and block content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InlineLayout {
    /// Final occupied content width.
    pub width: f32,
    /// Final occupied content height, including block margins.
    pub height: f32,
    /// Individual visual lines, in source order.
    pub lines: Vec<InlineLine>,
    /// Paint-ready glyph runs, in visual order.
    pub runs: Vec<TextRun>,
    /// Validated positioned leaves retained for R5 after normal table grid placement.
    pub scene_objects: Vec<PositionedSceneObject>,
    /// Finite inline boxes whose decoration and child content must be emitted after placement.
    pub decorated_inline_boxes: Vec<DecoratedInlineBox>,
    /// Empty block decorations whose auto width resolves against the final content rectangle.
    pub decorated_empty_blocks: Vec<DecoratedEmptyBlock>,
}

/// A validated positioned scene leaf placed as an inline replaced object.
#[derive(Clone, Debug, PartialEq)]
pub struct PositionedSceneObject {
    /// Final cell-content bounds of the leaf. Its display list remains leaf-relative.
    pub bounds: LayoutBox,
    /// Baseline used when this replacement object shares an inline line with text.
    pub baseline_y: f32,
    /// The constrained scene payload, including non-text marks and styled text groups.
    pub scene: SceneLeaf,
}

/// A finite decorated inline box, including the nodes that paint inside its border box.
#[derive(Clone, Debug, PartialEq)]
pub struct DecoratedInlineBox {
    /// Final border-box geometry in cell-content coordinates.
    pub bounds: LayoutBox,
    /// Resolved CSS styling retained for paint.
    pub style: ComputedStyle,
    /// Inline descendants laid out by R5 within this box after it emits the decoration.
    pub children: Vec<StyledNode>,
}

/// An empty decorated block with final border-box geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct DecoratedEmptyBlock {
    /// Final border-box geometry in cell-content coordinates.
    pub bounds: LayoutBox,
    /// Resolved CSS styling retained for paint.
    pub style: ComputedStyle,
}

#[derive(Clone, Debug)]
struct Span {
    text: String,
    style: ComputedStyle,
    baseline_shift: f32,
    size_scale: f32,
    visible: bool,
    scene: Option<SceneLeaf>,
    decorated_inline: Option<DecoratedInlineSpec>,
    decorated_empty_block: Option<ComputedStyle>,
}

#[derive(Clone, Debug)]
struct DecoratedInlineSpec {
    style: ComputedStyle,
    children: Vec<StyledNode>,
}

struct AppendState<'a> {
    runs: &'a mut Vec<TextRun>,
    scene_objects: &'a mut Vec<PositionedSceneObject>,
    decorated_inline_boxes: &'a mut Vec<DecoratedInlineBox>,
    decorated_empty_blocks: &'a mut Vec<DecoratedEmptyBlock>,
    content_width: f32,
    final_width: bool,
}

/// Measures the content-only CSS intrinsic widths of a cell or another inline root.
#[must_use]
pub fn measure_inline(node: &StyledNode) -> IntrinsicWidths {
    let spans = collect_spans(node);
    if spans.iter().all(|span| !occupies_space(span)) {
        return IntrinsicWidths {
            min_width: 0.0,
            max_width: 0.0,
        };
    }

    let max_width = layout_spans(&spans, f32::MAX, false).width;
    let mut min_width = 0.0_f32;
    for group in unbreakable_groups(&spans) {
        min_width = min_width.max(layout_spans(&group, f32::MAX, false).width);
    }
    IntrinsicWidths {
        min_width,
        max_width: max_width.max(min_width),
    }
}

/// Lays out a cell's inline content at its final content width.
///
/// `content_width` excludes cell padding and borders.  It is clamped to zero to keep malformed
/// upstream dimensions from producing negative geometry.
#[must_use]
pub fn layout_inline(node: &StyledNode, content_width: f32) -> InlineLayout {
    layout_spans(&collect_spans(node), content_width.max(0.0), true)
}

/// Places a relative inline layout in a final cell content rectangle.
///
/// Table layout owns row-baseline distribution; `Baseline` therefore begins at the content top
/// until that phase supplies a shared row offset.  All glyph cache origins move with their runs.
pub fn place_inline(layout: &mut InlineLayout, content: LayoutBox, vertical_align: VerticalAlign) {
    let remaining = (content.height - layout.height).max(0.0);
    let y_offset = match vertical_align {
        VerticalAlign::Bottom => remaining,
        VerticalAlign::Middle => remaining / 2.0,
        VerticalAlign::Top | VerticalAlign::Baseline => 0.0,
    };
    for line in &mut layout.lines {
        line.bounds.x += content.x;
        line.bounds.y += content.y + y_offset;
        line.baseline_y += content.y + y_offset;
    }
    for run in &mut layout.runs {
        run.bounds.x += content.x;
        run.bounds.y += content.y + y_offset;
        run.baseline_y += content.y + y_offset;
        for glyph in &mut run.glyphs {
            glyph.x += content.x.round() as i32;
            glyph.y += (content.y + y_offset).round() as i32;
        }
    }
    for object in &mut layout.scene_objects {
        object.bounds.x += content.x;
        object.bounds.y += content.y + y_offset;
        object.baseline_y += content.y + y_offset;
    }
    for object in &mut layout.decorated_inline_boxes {
        object.bounds.x += content.x;
        object.bounds.y += content.y + y_offset;
    }
    for object in &mut layout.decorated_empty_blocks {
        object.bounds.x += content.x;
        object.bounds.y += content.y + y_offset;
    }
}

fn collect_spans(node: &StyledNode) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_node(node, 0.0, 1.0, &mut spans);
    trim_leading_collapsible_whitespace(&mut spans);
    spans
}

/// Browsers discard collapsible leading whitespace at the start of an inline formatting context.
///
/// Scraper retains indentation around comments as text nodes, so without this normalization an
/// inline scene leaf at a cell's start gains a visible one-space offset. Whitespace between
/// content remains intact.
fn trim_leading_collapsible_whitespace(spans: &mut Vec<Span>) {
    let mut at_line_start = true;
    spans.retain_mut(|span| {
        if span.text.starts_with('\u{000B}') {
            return true;
        }
        if span.text == "\n" {
            at_line_start = true;
            return true;
        }
        if at_line_start && span.style.white_space == WhiteSpace::Normal {
            span.text = span.text.trim_start_matches(' ').to_owned();
        }
        if !span.text.is_empty() {
            at_line_start = false;
            true
        } else {
            false
        }
    });
}

fn collect_node(node: &StyledNode, shift: f32, size_scale: f32, spans: &mut Vec<Span>) {
    match &node.kind {
        StyledNodeKind::Text(text) => spans.push(Span {
            text: normalize_text(text, node.style.white_space),
            style: node.style.clone(),
            baseline_shift: shift,
            size_scale,
            visible: node.style.visible,
            scene: None,
            decorated_inline: None,
            decorated_empty_block: None,
        }),
        StyledNodeKind::SceneLeaf(_) => {
            let Some(scene) = node.scene.clone() else {
                return;
            };
            spans.push(Span {
                text: "\u{FFFC}".to_owned(),
                style: node.style.clone(),
                baseline_shift: shift,
                size_scale,
                visible: node.style.visible,
                scene: Some(scene),
                decorated_inline: None,
                decorated_empty_block: None,
            });
        }
        StyledNodeKind::Element(kind) => {
            if decorated_inline(node) {
                spans.push(Span {
                    text: "\u{FFFC}".to_owned(),
                    style: node.style.clone(),
                    baseline_shift: shift,
                    size_scale,
                    visible: node.style.visible,
                    scene: None,
                    decorated_inline: Some(DecoratedInlineSpec {
                        style: node.style.clone(),
                        children: node.children.clone(),
                    }),
                    decorated_empty_block: None,
                });
                return;
            }
            if decorated_empty_block(node) {
                block_start(kind, &node.style, spans);
                spans.push(Span {
                    text: "\u{FFFC}".to_owned(),
                    style: node.style.clone(),
                    baseline_shift: shift,
                    size_scale,
                    visible: node.style.visible,
                    scene: None,
                    decorated_inline: None,
                    decorated_empty_block: Some(node.style.clone()),
                });
                block_end(kind, &node.style, spans);
                return;
            }
            let is_block = matches!(node.style.display, DisplayMode::Block | DisplayMode::Table);
            if is_block {
                block_start(kind, &node.style, spans);
            }
            if *kind == ElementKind::Br {
                hard_break(&node.style, spans);
                return;
            }
            let (shift, size_scale) = match kind {
                ElementKind::Sub => (
                    shift + span_font_size(&node.style, size_scale) * SUB_SHIFT_FACTOR,
                    size_scale * SUB_SUP_SIZE_FACTOR,
                ),
                ElementKind::Sup => (
                    shift + span_font_size(&node.style, size_scale) * SUP_SHIFT_FACTOR,
                    size_scale * SUB_SUP_SIZE_FACTOR,
                ),
                _ => (shift, size_scale),
            };
            for child in &node.children {
                collect_node(child, shift, size_scale, spans);
            }
            if is_block {
                block_end(kind, &node.style, spans);
            }
        }
    }
}

fn decorated_inline(node: &StyledNode) -> bool {
    node.style.display == DisplayMode::InlineBlock
        && node.style.width.is_some()
        && node.style.height.is_some()
        && has_decoration(&node.style)
}

fn decorated_empty_block(node: &StyledNode) -> bool {
    node.style.display == DisplayMode::Block
        && node.children.is_empty()
        && node.style.height.is_some()
        && has_decoration(&node.style)
}

fn has_decoration(style: &ComputedStyle) -> bool {
    style.background.is_some()
        || style.border_radius.is_some()
        || style.box_shadow.is_some()
        || style.border.top.style != crate::BorderStyle::None
        || style.border.right.style != crate::BorderStyle::None
        || style.border.bottom.style != crate::BorderStyle::None
        || style.border.left.style != crate::BorderStyle::None
}

fn block_margins(kind: &ElementKind, style: &ComputedStyle) -> (f32, f32) {
    let margins = [
        resolve_length(style.margin[0], font_size(style)),
        resolve_length(style.margin[2], font_size(style)),
    ];
    // HTML's UA stylesheet gives paragraphs one em block margins.  Divisions deliberately retain
    // zero margins unless the source CSS supplies them.
    let default_paragraph_margin = if *kind == ElementKind::Paragraph && margins == [0.0, 0.0] {
        font_size(style)
    } else {
        0.0
    };
    (
        margins[0].max(default_paragraph_margin),
        margins[1].max(default_paragraph_margin),
    )
}

fn block_start(kind: &ElementKind, style: &ComputedStyle, spans: &mut Vec<Span>) {
    let (top, _) = block_margins(kind, style);
    if top > 0.0 {
        spans.push(control_span(format!("\u{000B}{top}"), style));
    }
    if spans.len() > usize::from(top > 0.0) {
        hard_break(style, spans);
    }
}

fn block_end(kind: &ElementKind, style: &ComputedStyle, spans: &mut Vec<Span>) {
    let (_, bottom) = block_margins(kind, style);
    hard_break(style, spans);
    if bottom > 0.0 {
        spans.push(control_span(format!("\u{000B}{bottom}"), style));
    }
}

fn hard_break(style: &ComputedStyle, spans: &mut Vec<Span>) {
    spans.push(control_span("\n".to_owned(), style));
}

fn control_span(text: String, style: &ComputedStyle) -> Span {
    Span {
        text,
        style: style.clone(),
        baseline_shift: 0.0,
        size_scale: 1.0,
        visible: style.visible,
        scene: None,
        decorated_inline: None,
        decorated_empty_block: None,
    }
}

fn normalize_text(text: &str, white_space: WhiteSpace) -> String {
    if white_space == WhiteSpace::Pre {
        return text.replace("\r\n", "\n").replace('\r', "\n");
    }
    let mut normalized = String::with_capacity(text.len());
    let mut last_space = false;
    for character in text.chars() {
        if character.is_whitespace() && character != '\u{00A0}' {
            if !last_space {
                normalized.push(' ');
            }
            last_space = true;
        } else {
            normalized.push(character);
            last_space = false;
        }
    }
    if white_space == WhiteSpace::NoWrap {
        normalized.replace(' ', "\u{00A0}")
    } else {
        normalized
    }
}

fn occupies_space(span: &Span) -> bool {
    span_font_size(&span.style, span.size_scale) > 0.0
        && span_line_height(&span.style, span.size_scale) > 0.0
        && span.text.chars().any(|character| !character.is_control())
}

fn unbreakable_groups(spans: &[Span]) -> Vec<Vec<Span>> {
    let mut groups = Vec::new();
    let mut group = Vec::new();
    for span in spans {
        if span.text.starts_with('\u{000B}') {
            continue;
        }
        let breakable = span.style.white_space == WhiteSpace::Normal;
        if !breakable {
            if span.text.contains('\n') {
                flush_group(&mut groups, &mut group);
                for line in span.text.split('\n') {
                    if !line.is_empty() {
                        groups.push(vec![Span {
                            text: line.to_owned(),
                            style: span.style.clone(),
                            baseline_shift: span.baseline_shift,
                            size_scale: span.size_scale,
                            visible: span.visible,
                            scene: span.scene.clone(),
                            decorated_inline: span.decorated_inline.clone(),
                            decorated_empty_block: span.decorated_empty_block.clone(),
                        }]);
                    }
                }
            } else {
                group.push(span.clone());
            }
            continue;
        }
        for word in span.text.split_inclusive(char::is_whitespace) {
            let trimmed = word.trim_matches(char::is_whitespace);
            if !trimmed.is_empty() {
                group.push(Span {
                    text: trimmed.to_owned(),
                    style: span.style.clone(),
                    baseline_shift: span.baseline_shift,
                    size_scale: span.size_scale,
                    visible: span.visible,
                    scene: span.scene.clone(),
                    decorated_inline: span.decorated_inline.clone(),
                    decorated_empty_block: span.decorated_empty_block.clone(),
                });
            }
            if word.chars().last().is_some_and(char::is_whitespace) {
                flush_group(&mut groups, &mut group);
            }
        }
    }
    flush_group(&mut groups, &mut group);
    groups
}

fn flush_group(groups: &mut Vec<Vec<Span>>, group: &mut Vec<Span>) {
    if !group.is_empty() {
        groups.push(std::mem::take(group));
    }
}

fn layout_spans(spans: &[Span], width: f32, final_width: bool) -> InlineLayout {
    let mut layout = InlineLayout::default();
    let mut visual_spans = Vec::new();
    let mut extra_vertical = 0.0;
    for span in spans {
        if let Some(value) = span.text.strip_prefix('\u{000B}') {
            extra_vertical += value.parse::<f32>().unwrap_or(0.0).max(0.0);
        } else if occupies_space(span) || span.text.contains('\n') {
            visual_spans.push(span.clone());
        }
    }
    if visual_spans.is_empty() {
        layout.height = extra_vertical;
        return layout;
    }

    let default = attrs_for(&visual_spans[0], 0);
    let metrics = Metrics::new(
        font_size(&visual_spans[0].style).max(0.01),
        line_height(&visual_spans[0].style).max(0.01),
    );
    with_font_system(|font_system| {
        let mut buffer = Buffer::new(font_system, metrics);
        let mut borrowed = buffer.borrow_with(font_system);
        borrowed.set_size(final_width.then_some(width), None);
        borrowed.set_wrap(
            if visual_spans
                .iter()
                .any(|span| span.style.white_space == WhiteSpace::Normal)
            {
                Wrap::Word
            } else {
                Wrap::None
            },
        );
        let rich: Vec<_> = visual_spans
            .iter()
            .enumerate()
            .map(|(index, span)| (span.text.as_str(), attrs_for(span, index)))
            .collect();
        borrowed.set_rich_text(rich, &default, Shaping::Advanced, None);
        borrowed.shape_until_scroll(true);
        for line in borrowed.layout_runs() {
            let line_width = resolved_line_width(&line, &visual_spans);
            let line_x = alignment_offset(
                visual_spans[0].style.text_align,
                width,
                line_width,
                final_width,
            );
            let bounds = LayoutBox {
                x: line_x,
                y: line.line_top + extra_vertical,
                width: line_width,
                height: line.line_height.max(0.0),
            };
            layout.lines.push(InlineLine {
                bounds,
                baseline_y: line.line_y + extra_vertical,
            });
            let mut state = AppendState {
                runs: &mut layout.runs,
                scene_objects: &mut layout.scene_objects,
                decorated_inline_boxes: &mut layout.decorated_inline_boxes,
                decorated_empty_blocks: &mut layout.decorated_empty_blocks,
                content_width: width,
                final_width,
            };
            let (resolved_width, resolved_height) =
                append_runs(&mut state, line, &visual_spans, line_x, extra_vertical);
            layout.width = layout.width.max(resolved_width);
            layout.height = layout.height.max(resolved_height);
            if let Some(resolved_line) = layout.lines.last_mut() {
                resolved_line.bounds.width = resolved_width;
                resolved_line.bounds.height =
                    (resolved_height - resolved_line.bounds.y).max(resolved_line.bounds.height);
            }
        }
    });
    layout.height = layout.height.max(extra_vertical);
    layout
}

fn append_runs(
    state: &mut AppendState<'_>,
    line: cosmic_text::LayoutRun<'_>,
    spans: &[Span],
    x_offset: f32,
    y_offset: f32,
) -> (f32, f32) {
    let mut horizontal_adjustment = 0.0;
    let mut resolved_width = resolved_line_width(&line, spans);
    let mut resolved_height = line.line_top + line.line_height + y_offset;
    let mut scene_since_last_text = false;
    for glyph in line.glyphs {
        let span = spans.get(glyph.metadata).unwrap_or(&spans[0]);
        let glyph_x = glyph.x + x_offset + horizontal_adjustment;
        if let Some(spec) = &span.decorated_inline {
            let box_width = declared_width(&spec.style, span.size_scale);
            let box_height = declared_height(&spec.style, span.size_scale);
            let top = line.line_top + y_offset;
            if span.visible {
                state.decorated_inline_boxes.push(DecoratedInlineBox {
                    bounds: LayoutBox {
                        x: glyph_x,
                        y: top,
                        width: box_width,
                        height: box_height,
                    },
                    style: spec.style.clone(),
                    children: spec.children.clone(),
                });
            }
            horizontal_adjustment += box_width - glyph.w;
            resolved_width = (line.line_w + horizontal_adjustment).max(resolved_width);
            resolved_height = resolved_height.max(top + box_height);
            scene_since_last_text = true;
            continue;
        }
        if let Some(style) = &span.decorated_empty_block {
            let box_width = style
                .width
                .map(|value| resolve_length(value, span_font_size(style, span.size_scale)))
                .unwrap_or_else(|| {
                    if state.final_width {
                        state.content_width
                    } else {
                        0.0
                    }
                });
            let box_height = declared_height(style, span.size_scale);
            let top = line.line_top + y_offset;
            if span.visible {
                state.decorated_empty_blocks.push(DecoratedEmptyBlock {
                    bounds: LayoutBox {
                        // A block's auto width resolves from the cell content edge, independently
                        // of any inherited inline text alignment.
                        x: 0.0,
                        y: top,
                        width: box_width,
                        height: box_height,
                    },
                    style: style.clone(),
                });
            }
            horizontal_adjustment += box_width - glyph.w;
            resolved_width = (line.line_w + horizontal_adjustment).max(resolved_width);
            resolved_height = resolved_height.max(top + box_height);
            scene_since_last_text = true;
            continue;
        }
        if let Some(scene) = &span.scene {
            let scene_width = scene.intrinsic_width.max(0.0);
            let scene_height = scene.intrinsic_height.max(0.0);
            let relative_top = span
                .style
                .relative_top
                .map(|value| resolve_length(value, span_font_size(&span.style, span.size_scale)))
                .unwrap_or(0.0);
            let top = line.line_top + y_offset + relative_top;
            let baseline = top + scene_height;
            if span.visible {
                state.scene_objects.push(PositionedSceneObject {
                    bounds: LayoutBox {
                        x: glyph_x,
                        y: top,
                        width: scene_width,
                        height: scene_height,
                    },
                    baseline_y: baseline,
                    scene: scene.clone(),
                });
            }
            horizontal_adjustment += scene_width - glyph.w;
            resolved_width = (line.line_w + horizontal_adjustment).max(resolved_width);
            resolved_height = resolved_height.max(top + scene_height);
            scene_since_last_text = true;
            continue;
        }
        let scale_x = span.style.inline_scale_x.unwrap_or(1.0);
        if !span.visible {
            horizontal_adjustment += glyph.w * (scale_x - 1.0);
            continue;
        }
        let face = face_key(span);
        let glyph_y = glyph.y + y_offset + span.baseline_shift;
        let physical = glyph.physical(
            (
                x_offset + horizontal_adjustment,
                line.line_y + y_offset + span.baseline_shift,
            ),
            1.0,
        );
        let glyph_bounds = LayoutBox {
            x: glyph_x,
            y: glyph_y,
            width: (glyph.w * scale_x).max(0.0),
            height: line.line_height.max(0.0),
        };
        let can_append = !scene_since_last_text
            && state.runs.last().is_some_and(|run| {
                run.color == span.style.color
                    && run.face == face
                    && (run.scale_x - scale_x).abs() < f32::EPSILON
                    && (run.baseline_y - (line.line_y + y_offset + span.baseline_shift)).abs()
                        < f32::EPSILON
            });
        if can_append {
            let run = state.runs.last_mut().expect("checked above");
            run.glyphs.push(PositionedGlyph {
                cache_key: physical.cache_key,
                x: physical.x,
                y: physical.y,
            });
            let right = (glyph_x + glyph.w * scale_x).max(run.bounds.x + run.bounds.width);
            run.bounds.width = right - run.bounds.x;
        } else {
            state.runs.push(TextRun {
                bounds: glyph_bounds,
                baseline_y: line.line_y + y_offset + span.baseline_shift,
                color: span.style.color,
                face,
                scale_x,
                glyphs: vec![PositionedGlyph {
                    cache_key: physical.cache_key,
                    x: physical.x,
                    y: physical.y,
                }],
            });
        }
        horizontal_adjustment += glyph.w * (scale_x - 1.0);
        scene_since_last_text = false;
    }
    resolved_width = resolved_width.max((line.line_w + horizontal_adjustment).max(0.0));
    (resolved_width, resolved_height)
}

fn resolved_line_width(line: &cosmic_text::LayoutRun<'_>, spans: &[Span]) -> f32 {
    let scaled_advance = line
        .glyphs
        .iter()
        .map(|glyph| {
            spans
                .get(glyph.metadata)
                .map_or(0.0, |span| object_advance(span, glyph.w) - glyph.w)
        })
        .sum::<f32>();
    (line.line_w + scaled_advance).max(0.0)
}

fn object_advance(span: &Span, glyph_width: f32) -> f32 {
    if let Some(scene) = &span.scene {
        return scene.intrinsic_width.max(0.0);
    }
    if let Some(spec) = &span.decorated_inline {
        return declared_width(&spec.style, span.size_scale);
    }
    if span.decorated_empty_block.is_some() {
        return 0.0;
    }
    glyph_width * span.style.inline_scale_x.unwrap_or(1.0)
}

fn declared_width(style: &ComputedStyle, size_scale: f32) -> f32 {
    style
        .width
        .map(|value| resolve_length(value, span_font_size(style, size_scale)))
        .unwrap_or(0.0)
}

fn declared_height(style: &ComputedStyle, size_scale: f32) -> f32 {
    style
        .height
        .map(|value| resolve_length(value, span_font_size(style, size_scale)))
        .unwrap_or(0.0)
}

fn attrs_for(span: &Span, metadata: usize) -> Attrs<'static> {
    Attrs::new()
        .family(Family::Name(match span.style.font_family {
            FontFamily::AtkinsonNext => ATKINSON_NEXT_FAMILY,
            FontFamily::AtkinsonMono => ATKINSON_MONO_FAMILY,
        }))
        .weight(Weight(weight(&span.style.font_weight)))
        .style(match span.style.font_style {
            FontStyle::Normal => CosmicStyle::Normal,
            FontStyle::Italic => CosmicStyle::Italic,
        })
        .metrics(Metrics::new(
            span_font_size(&span.style, span.size_scale),
            span_line_height(&span.style, span.size_scale),
        ))
        .letter_spacing(
            resolve_length(
                span.style.letter_spacing,
                span_font_size(&span.style, span.size_scale),
            ) / span_font_size(&span.style, span.size_scale).max(0.01),
        )
        .metadata(metadata)
}

fn face_key(span: &Span) -> FontFaceKey {
    FontFaceKey {
        family: span.style.font_family,
        weight: weight(&span.style.font_weight),
        style: span.style.font_style,
        size: span_font_size(&span.style, span.size_scale),
    }
}

fn font_size(style: &ComputedStyle) -> f32 {
    resolve_length(style.font_size, DEFAULT_FONT_SIZE)
}

fn line_height(style: &ComputedStyle) -> f32 {
    style
        .line_height
        .map(|length| resolve_length(length, font_size(style)))
        .unwrap_or_else(|| font_size(style) * DEFAULT_LINE_HEIGHT_FACTOR)
}

fn span_font_size(style: &ComputedStyle, size_scale: f32) -> f32 {
    font_size(style) * size_scale
}

fn span_line_height(style: &ComputedStyle, size_scale: f32) -> f32 {
    line_height(style) * size_scale
}

fn resolve_length(length: CssLength, em: f32) -> f32 {
    match length {
        CssLength::Px(value) => value,
        CssLength::Pt(value) => value * (96.0 / 72.0),
        CssLength::Em(value) => value * em,
        CssLength::Percent(value) => value * em / 100.0,
        CssLength::Zero => 0.0,
    }
    .max(0.0)
}

fn weight(weight: &FontWeight) -> u16 {
    match weight {
        FontWeight::Normal => 400,
        FontWeight::Bold => 700,
        FontWeight::Numeric(value) => *value,
    }
}

fn alignment_offset(align: TextAlign, width: f32, line_width: f32, final_width: bool) -> f32 {
    if !final_width {
        return 0.0;
    }
    match align {
        TextAlign::Start | TextAlign::Left => 0.0,
        TextAlign::Right => (width - line_width).max(0.0),
        TextAlign::Center => ((width - line_width) / 2.0).max(0.0),
    }
}

#[cfg(test)]
#[path = "inline_layout_tests.rs"]
mod tests;
