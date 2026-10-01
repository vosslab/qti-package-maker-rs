//! Display-list assembly for the native table renderer.

use std::collections::BTreeMap;

use crate::{
    Border, CaptionSide, CssLength, DisplayCommand, DisplayList, ElementKind, InlineLayout,
    LayoutBox, RasterError, StyledNode, StyledNodeKind, StyledTree, TableLayout, VerticalAlign,
    layout_inline, layout_table, place_inline,
};

use super::RenderMeasurer;

pub(super) fn build_display_list(
    tree: &StyledTree,
    layout: &TableLayout,
    measurer: &RenderMeasurer<'_>,
) -> Result<DisplayList, RasterError> {
    let mut commands = Vec::new();
    let caption = table_caption(&tree.root);
    let mut caption_layout = caption.map(|node| layout_inline(node, layout.bounds.width));
    let top_height = match (caption, &caption_layout) {
        (Some(node), Some(caption))
            if node.style.visible && node.style.caption_side == CaptionSide::Top =>
        {
            caption.height
        }
        _ => 0.0,
    };
    let bottom_height = match (caption, &caption_layout) {
        (Some(node), Some(caption))
            if node.style.visible && node.style.caption_side == CaptionSide::Bottom =>
        {
            caption.height
        }
        _ => 0.0,
    };
    if let (Some(node), Some(caption)) = (caption, caption_layout.as_mut()) {
        if node.style.visible {
            let bounds = LayoutBox {
                x: 0.0,
                y: if node.style.caption_side == CaptionSide::Top {
                    0.0
                } else {
                    top_height + layout.bounds.height
                },
                width: layout.bounds.width,
                height: caption.height,
            };
            commands.push(DisplayCommand::Border {
                bounds,
                style: node.style.clone(),
            });
            place_inline(caption, bounds, node.style.vertical_align);
            commands.extend(
                caption
                    .runs
                    .iter()
                    .cloned()
                    .map(|run| DisplayCommand::GlyphRun { run }),
            );
        }
    }
    emit_table(
        &tree.root,
        layout,
        LayoutBox {
            x: 0.0,
            y: top_height,
            width: 0.0,
            height: 0.0,
        },
        measurer,
        &mut commands,
    )?;
    Ok(DisplayList {
        bounds: LayoutBox {
            height: top_height + layout.bounds.height + bottom_height,
            ..layout.bounds
        },
        commands,
    })
}

fn table_caption(table: &StyledNode) -> Option<&StyledNode> {
    table
        .children
        .iter()
        .find(|node| matches!(node.kind, StyledNodeKind::Element(ElementKind::Caption)))
}

fn emit_table(
    table: &StyledNode,
    layout: &TableLayout,
    offset: LayoutBox,
    measurer: &RenderMeasurer<'_>,
    commands: &mut Vec<DisplayCommand>,
) -> Result<(), RasterError> {
    if !table.style.visible {
        return Ok(());
    }
    let table_bounds = offset_box(layout.bounds, offset.x, offset.y);
    commands.push(DisplayCommand::Border {
        bounds: table_bounds,
        style: table.style.clone(),
    });
    if table.style.overflow_hidden {
        commands.push(DisplayCommand::PushClip {
            bounds: table_bounds,
            radius: None,
        });
    }

    for cell in &layout.cells {
        if !cell.node.style.visible {
            continue;
        }
        let bounds = offset_box(cell.bounds, offset.x, offset.y);
        let content = offset_box(cell.content.bounds, offset.x, offset.y);
        let mut style = cell.node.style.clone();
        style.border = if layout.border_collapse {
            no_borders(style.color)
        } else {
            Border {
                top: cell.borders[0],
                right: cell.borders[1],
                bottom: cell.borders[2],
                left: cell.borders[3],
            }
        };
        commands.push(DisplayCommand::Border { bounds, style });
        if layout.border_collapse {
            emit_collapsed_border_segments(cell, layout, offset, commands);
        }
        if cell.node.style.overflow_hidden {
            commands.push(DisplayCommand::PushClip {
                bounds,
                radius: None,
            });
        }

        let inline_node = without_nested_tables(&cell.node);
        let mut inline = layout_inline(&inline_node, content.width);
        place_inline(&mut inline, content, cell.content.vertical_align);
        emit_decorated_boxes(
            &inline.decorated_empty_blocks,
            &inline.decorated_inline_boxes,
            commands,
        );
        commands.extend(
            inline
                .runs
                .iter()
                .cloned()
                .map(|run| DisplayCommand::GlyphRun { run }),
        );
        emit_scene_objects(&inline.scene_objects, commands);
        emit_images(&cell.node, content, &inline, measurer, commands)?;
        emit_nested_tables(&cell.node, content, inline.height, measurer, commands)?;

        if cell.node.style.overflow_hidden {
            commands.push(DisplayCommand::PopClip);
        }
    }
    if table.style.overflow_hidden {
        commands.push(DisplayCommand::PopClip);
    }
    Ok(())
}

/// Assembles each bounded scene leaf after ordinary inline and table layout places its border
/// box. The containing table grid remains a general-table concern.
fn emit_scene_objects(
    objects: &[crate::PositionedSceneObject],
    commands: &mut Vec<DisplayCommand>,
) {
    for object in objects {
        commands.extend(
            object
                .scene
                .display_list
                .commands
                .iter()
                .cloned()
                .map(|command| {
                    translate_display_command(command, object.bounds.x, object.bounds.y)
                }),
        );
        for group in &object.scene.text_groups {
            let mut text = layout_inline(&group.node, object.bounds.width);
            let (x, y) = scene_text_origin(object.bounds, &text, group);
            let text_width = text.width;
            let text_height = text.height;
            place_inline(
                &mut text,
                LayoutBox {
                    x,
                    y,
                    width: text_width,
                    height: text_height,
                },
                crate::VerticalAlign::Top,
            );
            commands.extend(
                text.runs
                    .into_iter()
                    .map(|run| DisplayCommand::GlyphRun { run }),
            );
        }
    }
}

/// Emits bounded non-cell boxes after ordinary inline placement. The enclosing cell clip is
/// already active at this point, so shadows and child content cannot escape its owning cell.
fn emit_decorated_boxes(
    empty_blocks: &[crate::inline_layout::DecoratedEmptyBlock],
    inline_boxes: &[crate::inline_layout::DecoratedInlineBox],
    commands: &mut Vec<DisplayCommand>,
) {
    for object in empty_blocks {
        commands.push(DisplayCommand::Border {
            bounds: object.bounds,
            style: object.style.clone(),
        });
    }
    for object in inline_boxes {
        commands.push(DisplayCommand::Border {
            bounds: object.bounds,
            style: object.style.clone(),
        });
        let content = decorated_content_bounds(object.bounds, &object.style);
        let node = decoration_children_node(&object.style, &object.children);
        let mut children = layout_inline(&node, content.width);
        place_inline(&mut children, content, VerticalAlign::Top);
        emit_decorated_boxes(
            &children.decorated_empty_blocks,
            &children.decorated_inline_boxes,
            commands,
        );
        commands.extend(
            children
                .runs
                .into_iter()
                .map(|run| DisplayCommand::GlyphRun { run }),
        );
        emit_scene_objects(&children.scene_objects, commands);
    }
}

fn decoration_children_node(style: &crate::ComputedStyle, children: &[StyledNode]) -> StyledNode {
    let mut content_style = style.clone();
    content_style.width = None;
    content_style.min_width = None;
    content_style.max_width = None;
    content_style.height = None;
    content_style.padding = [CssLength::Zero; 4];
    content_style.border = no_borders(content_style.color);
    content_style.border_radius = None;
    content_style.background = None;
    content_style.box_shadow = None;
    StyledNode {
        kind: StyledNodeKind::Element(ElementKind::Span),
        style: content_style,
        attributes: BTreeMap::new(),
        children: children.to_vec(),
        scene: None,
    }
}

fn decorated_content_bounds(bounds: LayoutBox, style: &crate::ComputedStyle) -> LayoutBox {
    let font_size = resolve_box_length(style.font_size, bounds.width, 16.0);
    let top = resolve_box_length(style.border.top.width, bounds.width, font_size)
        + resolve_box_length(style.padding[0], bounds.width, font_size);
    let right = resolve_box_length(style.border.right.width, bounds.width, font_size)
        + resolve_box_length(style.padding[1], bounds.width, font_size);
    let bottom = resolve_box_length(style.border.bottom.width, bounds.width, font_size)
        + resolve_box_length(style.padding[2], bounds.width, font_size);
    let left = resolve_box_length(style.border.left.width, bounds.width, font_size)
        + resolve_box_length(style.padding[3], bounds.width, font_size);
    LayoutBox {
        x: bounds.x + left,
        y: bounds.y + top,
        width: (bounds.width - left - right).max(0.0),
        height: (bounds.height - top - bottom).max(0.0),
    }
}

fn resolve_box_length(length: CssLength, available_width: f32, font_size: f32) -> f32 {
    match length {
        CssLength::Px(value) => value,
        CssLength::Pt(value) => value * (96.0 / 72.0),
        CssLength::Em(value) => value * font_size,
        CssLength::Percent(value) => available_width * value / 100.0,
        CssLength::Zero => 0.0,
    }
    .max(0.0)
}

fn scene_text_origin(
    bounds: LayoutBox,
    text: &InlineLayout,
    group: &crate::SceneText,
) -> (f32, f32) {
    use crate::SceneAnchor;

    let x = match group.anchor {
        SceneAnchor::TopLeft | SceneAnchor::BottomLeft => bounds.x + group.x_css,
        SceneAnchor::TopRight | SceneAnchor::BottomRight => {
            bounds.x + bounds.width - text.width - group.x_css
        }
        SceneAnchor::Center => bounds.x + (bounds.width - text.width) / 2.0 + group.x_css,
    };
    let y = match group.anchor {
        SceneAnchor::TopLeft | SceneAnchor::TopRight => bounds.y + group.y_css,
        SceneAnchor::BottomLeft | SceneAnchor::BottomRight => {
            bounds.y + bounds.height - text.height - group.y_css
        }
        SceneAnchor::Center => bounds.y + (bounds.height - text.height) / 2.0 + group.y_css,
    };
    (x, y)
}

fn translate_display_command(command: DisplayCommand, x: f32, y: f32) -> DisplayCommand {
    match command {
        DisplayCommand::Fill { bounds, color } => DisplayCommand::Fill {
            bounds: offset_box(bounds, x, y),
            color,
        },
        DisplayCommand::Border { bounds, style } => DisplayCommand::Border {
            bounds: offset_box(bounds, x, y),
            style,
        },
        DisplayCommand::Text {
            bounds,
            text,
            style,
        } => DisplayCommand::Text {
            bounds: offset_box(bounds, x, y),
            text,
            style,
        },
        DisplayCommand::PushClip { bounds, radius } => DisplayCommand::PushClip {
            bounds: offset_box(bounds, x, y),
            radius,
        },
        DisplayCommand::PopClip => DisplayCommand::PopClip,
        DisplayCommand::Image { bounds, data_url } => DisplayCommand::Image {
            bounds: offset_box(bounds, x, y),
            data_url,
        },
        DisplayCommand::StrokeLine {
            from,
            to,
            width,
            color,
        } => DisplayCommand::StrokeLine {
            from: (from.0 + x, from.1 + y),
            to: (to.0 + x, to.1 + y),
            width,
            color,
        },
        DisplayCommand::GlyphRun { mut run } => {
            run.bounds = offset_box(run.bounds, x, y);
            run.baseline_y += y;
            for glyph in &mut run.glyphs {
                glyph.x += x.round() as i32;
                glyph.y += y.round() as i32;
            }
            DisplayCommand::GlyphRun { run }
        }
    }
}

/// Emits resolved collapsed borders per grid segment. A spanning cell can meet several smaller
/// neighbours, so its strongest whole-side border is insufficient for correct edge painting.
fn emit_collapsed_border_segments(
    cell: &crate::TableCellLayout,
    layout: &TableLayout,
    offset: LayoutBox,
    commands: &mut Vec<DisplayCommand>,
) {
    for (side_index, segments) in cell.border_segments.iter().enumerate() {
        for segment in segments {
            if matches!(
                segment.border.style,
                crate::BorderStyle::None | crate::BorderStyle::Hidden
            ) {
                continue;
            }
            let (bounds, side) =
                collapsed_segment_bounds(cell, layout, offset, side_index, segment);
            let mut style = cell.node.style.clone();
            style.background = None;
            style.box_shadow = None;
            style.border_radius = None;
            style.border = no_borders(style.color);
            match side {
                0 => style.border.top = segment.border,
                1 => style.border.right = segment.border,
                2 => style.border.bottom = segment.border,
                3 => style.border.left = segment.border,
                _ => unreachable!("cell border side is one of four values"),
            }
            commands.push(DisplayCommand::Border { bounds, style });
        }
    }
}

fn collapsed_segment_bounds(
    cell: &crate::TableCellLayout,
    layout: &TableLayout,
    offset: LayoutBox,
    side: usize,
    segment: &crate::CollapsedBorderSegment,
) -> (LayoutBox, usize) {
    let cell_bounds = offset_box(cell.bounds, offset.x, offset.y);
    match side {
        0 | 2 => {
            let x = cell_bounds.x
                + layout.column_widths[cell.column..cell.column + segment.start]
                    .iter()
                    .sum::<f32>();
            let width = layout.column_widths
                [cell.column + segment.start..cell.column + segment.end]
                .iter()
                .sum();
            (
                LayoutBox {
                    x,
                    width,
                    ..cell_bounds
                },
                side,
            )
        }
        1 | 3 => {
            let y = cell_bounds.y
                + layout.row_heights[cell.row..cell.row + segment.start]
                    .iter()
                    .sum::<f32>();
            let height = layout.row_heights[cell.row + segment.start..cell.row + segment.end]
                .iter()
                .sum();
            (
                LayoutBox {
                    y,
                    height,
                    ..cell_bounds
                },
                side,
            )
        }
        _ => unreachable!("cell border side is one of four values"),
    }
}

fn no_borders(color: crate::Color) -> Border {
    let none = crate::BorderSide {
        width: CssLength::Zero,
        style: crate::BorderStyle::None,
        color,
    };
    Border {
        top: none,
        right: none,
        bottom: none,
        left: none,
    }
}

fn emit_images(
    node: &StyledNode,
    content: LayoutBox,
    inline: &InlineLayout,
    measurer: &RenderMeasurer<'_>,
    commands: &mut Vec<DisplayCommand>,
) -> Result<(), RasterError> {
    let mut images = Vec::new();
    collect_direct_images(node, &mut images);
    let mut y = content.y + inline.height;
    for image in images {
        let source = image
            .attributes
            .get("src")
            .expect("parser requires img src");
        let (natural_width, natural_height) = measurer.image_dimensions(source)?;
        let width = natural_width.min(content.width);
        let height = if natural_width > content.width {
            natural_height * content.width / natural_width
        } else {
            natural_height
        };
        commands.push(DisplayCommand::Image {
            bounds: LayoutBox {
                x: content.x,
                y,
                width,
                height,
            },
            data_url: source.clone(),
        });
        y += height;
    }
    Ok(())
}

fn emit_nested_tables(
    node: &StyledNode,
    content: LayoutBox,
    inline_height: f32,
    measurer: &RenderMeasurer<'_>,
    commands: &mut Vec<DisplayCommand>,
) -> Result<(), RasterError> {
    let mut tables = Vec::new();
    collect_nested_tables(node, &mut tables);
    let mut y = content.y + inline_height;
    for table in tables {
        let layout = layout_table(
            &StyledTree {
                root: table.clone(),
            },
            content.width,
            measurer,
        )?;
        emit_table(
            table,
            &layout,
            LayoutBox {
                x: content.x,
                y,
                width: 0.0,
                height: 0.0,
            },
            measurer,
            commands,
        )?;
        y += layout.bounds.height;
    }
    Ok(())
}

pub(super) fn collect_direct_images<'a>(node: &'a StyledNode, output: &mut Vec<&'a StyledNode>) {
    for child in &node.children {
        if !child.style.visible {
            continue;
        }
        if matches!(child.kind, StyledNodeKind::Element(ElementKind::Table)) {
            continue;
        }
        if matches!(child.kind, StyledNodeKind::Element(ElementKind::Image)) {
            output.push(child);
        }
        collect_direct_images(child, output);
    }
}

fn collect_nested_tables<'a>(node: &'a StyledNode, output: &mut Vec<&'a StyledNode>) {
    for child in &node.children {
        if !child.style.visible {
            continue;
        }
        if matches!(child.kind, StyledNodeKind::Element(ElementKind::Table)) {
            output.push(child);
        } else {
            collect_nested_tables(child, output);
        }
    }
}

pub(super) fn without_nested_tables(node: &StyledNode) -> StyledNode {
    let children = node
        .children
        .iter()
        .filter_map(without_nested_table_child)
        .collect();
    StyledNode {
        kind: node.kind.clone(),
        style: node.style.clone(),
        attributes: node.attributes.clone(),
        children,
        scene: node.scene.clone(),
    }
}

/// Mirrors R2's inline-flow clone: extracting a nested table also removes a wrapper which has
/// become empty, avoiding an artificial block-end line before the table is emitted below.
fn without_nested_table_child(node: &StyledNode) -> Option<StyledNode> {
    if !node.style.visible || matches!(node.kind, StyledNodeKind::Element(ElementKind::Table)) {
        return None;
    }
    let children = node
        .children
        .iter()
        .filter_map(without_nested_table_child)
        .collect::<Vec<_>>();
    let empty_container = children.is_empty()
        && matches!(node.kind, StyledNodeKind::Element(kind) if !matches!(kind, ElementKind::Br | ElementKind::Image))
        && !retains_own_decoration(node);
    if empty_container {
        return None;
    }
    Some(StyledNode {
        kind: node.kind.clone(),
        style: node.style.clone(),
        attributes: node.attributes.clone(),
        children,
        scene: node.scene.clone(),
    })
}

/// Nested-table extraction may remove inert wrappers, but an empty finite box still carries a
/// visible background, shadow, or border and must reach R3/R5 decoration assembly.
fn retains_own_decoration(node: &StyledNode) -> bool {
    let style = &node.style;
    style.background.is_some()
        || style.box_shadow.is_some()
        || !matches!(style.border.top.style, crate::BorderStyle::None)
        || !matches!(style.border.right.style, crate::BorderStyle::None)
        || !matches!(style.border.bottom.style, crate::BorderStyle::None)
        || !matches!(style.border.left.style, crate::BorderStyle::None)
}

fn offset_box(bounds: LayoutBox, x: f32, y: f32) -> LayoutBox {
    LayoutBox {
        x: bounds.x + x,
        y: bounds.y + y,
        ..bounds
    }
}
