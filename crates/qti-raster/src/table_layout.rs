//! CSS 2.1 table-grid construction and geometry for the supported raster subset.
//!
//! This module deliberately deals in content metrics rather than glyphs.  The inline-layout
//! package supplies those metrics, and painting consumes the resulting stable rectangles.  That
//! separation keeps the table algorithm testable without a font or PNG backend.

use crate::{
    BorderSide, BorderStyle, CssLength, ElementKind, LayoutBox, RasterError, StyledNode,
    StyledNodeKind, StyledTree, VerticalAlign,
};

mod collapsed;

const DEFAULT_CELL_PADDING: f32 = 1.0;
const DEFAULT_BORDER_SPACING: f32 = 2.0;
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// Content dimensions supplied by the inline-layout package.
///
/// Widths exclude cell padding and borders. `min_width` is the widest unbreakable contribution;
/// `max_width` is the width with no soft wrapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntrinsicSize {
    pub min_width: f32,
    pub max_width: f32,
    pub height: f32,
}

impl IntrinsicSize {
    /// Normalizes a measurement supplied by a consumer at the table-layout boundary.
    #[must_use]
    pub fn normalized(self) -> Self {
        let min_width = finite_nonnegative(self.min_width);
        let max_width = finite_nonnegative(self.max_width).max(min_width);
        Self {
            min_width,
            max_width,
            height: finite_nonnegative(self.height),
        }
    }
}

/// A content rectangle after the cell's padding and resolved borders are removed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellContentBox {
    pub bounds: LayoutBox,
    pub vertical_align: VerticalAlign,
}

/// One source cell and its final placement in the CSS table grid.
#[derive(Clone, Debug, PartialEq)]
pub struct TableCellLayout {
    pub row: usize,
    pub column: usize,
    pub row_span: usize,
    pub column_span: usize,
    pub bounds: LayoutBox,
    pub content: CellContentBox,
    /// The strongest border for each whole cell side, retained for separated-border painting.
    /// In collapsed mode, use `border_segments` because a side can meet several neighbours.
    pub borders: [BorderSide; 4],
    /// Conflict-resolved collapsed-border segments for each side in top, right, bottom, left
    /// order. Segment coordinates are relative grid row or column offsets within this cell.
    pub border_segments: [Vec<CollapsedBorderSegment>; 4],
    pub node: StyledNode,
}

/// One conflict-resolved portion of a collapsed cell edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapsedBorderSegment {
    /// Inclusive grid offset at which this edge segment begins.
    pub start: usize,
    /// Exclusive grid offset at which this edge segment ends.
    pub end: usize,
    pub border: BorderSide,
}

/// Geometry of one table. Bounds include the table's outer border and separated-model spacing.
#[derive(Clone, Debug, PartialEq)]
pub struct TableLayout {
    pub bounds: LayoutBox,
    pub column_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub cells: Vec<TableCellLayout>,
    pub border_collapse: bool,
    pub border_spacing: f32,
}

/// Callback used by the layout engine to obtain intrinsic and final-width content dimensions.
///
/// The callback receives the complete cell node so inline layout can preserve its styled
/// descendants. The returned width is content-only; this module adds padding and borders.
pub trait CellMeasurer {
    fn intrinsic_size(&self, cell: &StyledNode) -> IntrinsicSize;
    fn layout_height(&self, cell: &StyledNode, content_width: f32) -> f32;
}

/// The production bridge from WP-R3's bundled-font text layout to this grid algorithm.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeCellMeasurer;

impl CellMeasurer for NativeCellMeasurer {
    fn intrinsic_size(&self, cell: &StyledNode) -> IntrinsicSize {
        let widths = crate::measure_inline(cell);
        IntrinsicSize {
            min_width: widths.min_width,
            max_width: widths.max_width,
            height: 0.0,
        }
    }

    fn layout_height(&self, cell: &StyledNode, content_width: f32) -> f32 {
        crate::layout_inline(cell, content_width).height
    }
}

impl<F, G> CellMeasurer for (F, G)
where
    F: Fn(&StyledNode) -> IntrinsicSize,
    G: Fn(&StyledNode, f32) -> f32,
{
    fn intrinsic_size(&self, cell: &StyledNode) -> IntrinsicSize {
        (self.0)(cell)
    }

    fn layout_height(&self, cell: &StyledNode, content_width: f32) -> f32 {
        (self.1)(cell, content_width)
    }
}

/// Builds a CSS 2.1 section 17 grid and lays it out within `available_width`.
///
/// The input must be the parsed outer table. Invalid span values are rejected at this trusted
/// boundary instead of being silently coerced, so malformed generator output cannot make the
/// renderer allocate an unbounded grid.
pub fn layout_table(
    tree: &StyledTree,
    available_width: f32,
    measurer: &impl CellMeasurer,
) -> Result<TableLayout, RasterError> {
    layout_table_node(&tree.root, available_width, measurer)
}

/// A deterministic fallback useful to tools which need geometry before the text backend is wired.
/// It is intentionally simple and is not the production text measurer.
pub fn layout_table_with_text_estimate(
    tree: &StyledTree,
    available_width: f32,
) -> Result<TableLayout, RasterError> {
    layout_table(
        tree,
        available_width,
        &(estimate_intrinsic, estimate_height),
    )
}

/// Lays out a parsed table with the bundled Atkinson text backend.
pub fn layout_table_with_native_text(
    tree: &StyledTree,
    available_width: f32,
) -> Result<TableLayout, RasterError> {
    layout_table(tree, available_width, &NativeCellMeasurer)
}

fn layout_table_node(
    table: &StyledNode,
    available_width: f32,
    measurer: &impl CellMeasurer,
) -> Result<TableLayout, RasterError> {
    require_kind(table, ElementKind::Table)?;
    let rows = collect_rows(table)?;
    let grid = build_grid(rows)?;
    let spacing = table_spacing(table);
    let table_cell_padding = table_cell_padding(table);
    let collapse = table.style.border_collapse;
    // CSS 2.1 only uses the fixed algorithm when the table itself has a non-auto width.
    // `table-layout: fixed` without a table width falls back to automatic layout.
    let fixed_algorithm = table.style.table_layout_fixed && table.style.width.is_some();
    let outer_border = border_widths(table);
    let mut columns = vec![ColumnRequirement::default(); grid.columns];

    // CSS automatic layout's first pass: one-column cell minima/maxima, including declared
    // widths, then span constraints distributed across participating columns.
    for placed in &grid.cells {
        // Fixed layout gets column widths from colgroups and the first row. Later rows can affect
        // height, but must not make the browser reflow the established column grid.
        let measurement = if fixed_algorithm && placed.row != 0 {
            IntrinsicSize::default()
        } else {
            cell_intrinsic(&placed.node, measurer)?.normalized()
        };
        let extras = cell_horizontal_extras(&placed.node, table_cell_padding);
        let required_min = measurement.min_width + extras;
        let required_max = measurement.max_width + extras;
        let declared = declared_minimum_width(&placed.node, available_width);
        distribute_requirement(
            &mut columns,
            placed.column,
            placed.column_span,
            required_min.max(declared.unwrap_or(0.0)),
            Requirement::Minimum,
        );
        distribute_requirement(
            &mut columns,
            placed.column,
            placed.column_span,
            required_max.max(declared.unwrap_or(0.0)),
            Requirement::Maximum,
        );
    }
    apply_colgroup_widths(table, &mut columns, available_width);

    let gap_total = if collapse {
        0.0
    } else {
        spacing * (grid.columns.saturating_add(1) as f32)
    };
    let border_total = outer_border[1] + outer_border[3];
    let min_total: f32 =
        columns.iter().map(|column| column.min).sum::<f32>() + gap_total + border_total;
    let max_total: f32 =
        columns.iter().map(|column| column.max).sum::<f32>() + gap_total + border_total;
    let specified = resolve_optional_width(table.style.width, available_width);
    let table_min_width = resolve_optional_width(table.style.min_width, available_width)
        .unwrap_or(0.0)
        .max(min_total);
    let table_width = match specified {
        Some(width) => width.max(table_min_width),
        None => max_total
            .min(available_width.max(table_min_width))
            .max(table_min_width),
    };
    let content_target = (table_width - gap_total - border_total).max(0.0);
    let column_widths = resolve_column_widths(&columns, content_target, fixed_algorithm);

    let mut row_heights = vec![0.0_f32; grid.rows.len()];
    for placed in &grid.cells {
        let cell_content_width = spanned_width(&column_widths, placed.column, placed.column_span)
            - cell_horizontal_extras(&placed.node, table_cell_padding);
        let measured = finite_nonnegative(cell_layout_height(
            &placed.node,
            cell_content_width.max(0.0),
            measurer,
        )?);
        let required = (measured + cell_vertical_extras(&placed.node, table_cell_padding))
            .max(resolve_optional_height(placed.node.style.height, available_width).unwrap_or(0.0));
        distribute_row_requirement(&mut row_heights, placed.row, placed.row_span, required);
    }
    for (index, row) in grid.rows.iter().enumerate() {
        if let Some(height) = resolve_optional_height(row.style.height, available_width) {
            row_heights[index] = row_heights[index].max(height);
        }
    }
    let declared_height = resolve_optional_height(table.style.height, available_width);
    if let Some(height) = declared_height {
        let required_rows = (height
            - outer_border[0]
            - outer_border[2]
            - vertical_gap_total(collapse, spacing, grid.rows.len()))
        .max(0.0);
        grow_equally(&mut row_heights, required_rows);
    }

    // `margin: 0 auto` centers a finite-width table in its containing block.  The layout owns
    // absolute table-relative geometry, so shift cell positions too rather than only moving the
    // outer bounds (which would detach borders and inline content from the table box).
    let table_x = if table.style.table_auto_horizontal_margins && table_width.is_finite() {
        ((finite_nonnegative(available_width) - table_width) / 2.0).max(0.0)
    } else {
        0.0
    };
    let mut x_positions = positions(&column_widths, outer_border[3], spacing, collapse);
    for position in &mut x_positions {
        *position += table_x;
    }
    let y_positions = positions(&row_heights, outer_border[0], spacing, collapse);
    let table_height = outer_border[0]
        + outer_border[2]
        + row_heights.iter().sum::<f32>()
        + vertical_gap_total(collapse, spacing, grid.rows.len());
    let geometry = BuildGeometry {
        widths: &column_widths,
        heights: &row_heights,
        xs: &x_positions,
        ys: &y_positions,
        spacing,
        collapse,
        table_cell_padding,
    };
    let provisional = build_cells(&grid.cells, geometry);
    let cells = if collapse {
        collapsed::resolve(provisional)
    } else {
        provisional
    };
    Ok(TableLayout {
        bounds: LayoutBox {
            x: table_x,
            y: 0.0,
            width: table_width,
            height: table_height,
        },
        column_widths,
        row_heights,
        cells,
        border_collapse: collapse,
        border_spacing: if collapse { 0.0 } else { spacing },
    })
}

#[derive(Clone, Debug)]
struct PlacedCell {
    row: usize,
    column: usize,
    row_span: usize,
    column_span: usize,
    node: StyledNode,
}

#[derive(Clone, Debug)]
struct Grid {
    rows: Vec<StyledNode>,
    columns: usize,
    cells: Vec<PlacedCell>,
}

fn collect_rows(table: &StyledNode) -> Result<Vec<StyledNode>, RasterError> {
    let mut rows = Vec::new();
    for child in &table.children {
        match child.kind {
            StyledNodeKind::Element(ElementKind::Tr) => rows.push(child.clone()),
            StyledNodeKind::Element(
                ElementKind::Thead | ElementKind::Tbody | ElementKind::Tfoot,
            ) => {
                for row in &child.children {
                    if matches!(row.kind, StyledNodeKind::Element(ElementKind::Tr)) {
                        rows.push(row.clone());
                    }
                }
            }
            StyledNodeKind::Element(ElementKind::Caption | ElementKind::Colgroup) => {}
            _ => {}
        }
    }
    if rows.is_empty() {
        return Err(layout_error("table", "table has no rows"));
    }
    Ok(rows)
}

fn build_grid(rows: Vec<StyledNode>) -> Result<Grid, RasterError> {
    let mut occupancy: Vec<Vec<bool>> = Vec::new();
    let mut cells = Vec::new();
    let mut widest = 0;
    for (row_index, row) in rows.iter().enumerate() {
        if occupancy.len() <= row_index {
            occupancy.push(Vec::new());
        }
        let mut column = 0;
        for child in &row.children {
            if !matches!(
                child.kind,
                StyledNodeKind::Element(ElementKind::Td | ElementKind::Th)
            ) {
                continue;
            }
            while occupancy[row_index].get(column).copied().unwrap_or(false) {
                column += 1;
            }
            let column_span = span(child, "colspan")?;
            let row_span = span(child, "rowspan")?;
            for target_row in row_index..row_index + row_span {
                while occupancy.len() <= target_row {
                    occupancy.push(Vec::new());
                }
                if occupancy[target_row].len() < column + column_span {
                    occupancy[target_row].resize(column + column_span, false);
                }
                if occupancy[target_row][column..column + column_span]
                    .iter()
                    .any(|occupied| *occupied)
                {
                    return Err(layout_error("rowspan", "cell spans overlap"));
                }
                occupancy[target_row][column..column + column_span].fill(true);
            }
            widest = widest.max(column + column_span);
            cells.push(PlacedCell {
                row: row_index,
                column,
                row_span,
                column_span,
                node: child.clone(),
            });
            column += column_span;
        }
    }
    if widest == 0 {
        return Err(layout_error("table", "table rows contain no cells"));
    }
    Ok(Grid {
        rows,
        columns: widest,
        cells,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct ColumnRequirement {
    min: f32,
    max: f32,
}

enum Requirement {
    Minimum,
    Maximum,
}

fn distribute_requirement(
    columns: &mut [ColumnRequirement],
    start: usize,
    span: usize,
    required: f32,
    kind: Requirement,
) {
    let range = &mut columns[start..start + span];
    let current: f32 = range
        .iter()
        .map(|column| match kind {
            Requirement::Minimum => column.min,
            Requirement::Maximum => column.max,
        })
        .sum();
    let deficit = (required - current).max(0.0);
    if deficit == 0.0 {
        return;
    }
    let share = deficit / span as f32;
    for column in range {
        match kind {
            Requirement::Minimum => {
                column.min += share;
                column.max = column.max.max(column.min);
            }
            Requirement::Maximum => column.max = (column.max + share).max(column.min),
        }
    }
}

fn apply_colgroup_widths(table: &StyledNode, columns: &mut [ColumnRequirement], available: f32) {
    let mut column = 0;
    for group in &table.children {
        if !matches!(
            group.kind,
            StyledNodeKind::Element(ElementKind::Colgroup | ElementKind::Col)
        ) {
            continue;
        }
        let children: Vec<&StyledNode> =
            if matches!(group.kind, StyledNodeKind::Element(ElementKind::Col)) {
                vec![group]
            } else {
                group
                    .children
                    .iter()
                    .filter(|child| matches!(child.kind, StyledNodeKind::Element(ElementKind::Col)))
                    .collect()
            };
        // HTML permits `<colgroup width='30'>` without explicit `<col>` children. Treat that
        // form as one column; this is present in the harvested gel tables.
        let nodes = if children.is_empty() {
            vec![group]
        } else {
            children
        };
        for node in nodes {
            let count = span(node, "span").unwrap_or(1);
            let Some(width) = resolve_optional_width(node.style.width, available) else {
                column += count;
                continue;
            };
            for item in columns.iter_mut().skip(column).take(count) {
                item.min = item.min.max(width / count as f32);
                item.max = item.max.max(item.min);
            }
            column += count;
        }
    }
}

fn resolve_column_widths(columns: &[ColumnRequirement], target: f32, fixed: bool) -> Vec<f32> {
    let mut result: Vec<f32> = columns.iter().map(|column| column.min).collect();
    let minimum: f32 = result.iter().sum();
    if target <= minimum {
        return result;
    }
    let mut extra = target - minimum;
    if !fixed {
        let flexibility: f32 = columns
            .iter()
            .map(|column| (column.max - column.min).max(0.0))
            .sum();
        if flexibility > 0.0 {
            let used = extra.min(flexibility);
            for (width, column) in result.iter_mut().zip(columns) {
                *width += used * (column.max - column.min).max(0.0) / flexibility;
            }
            extra -= used;
        }
    }
    if extra > 0.0 {
        let share = extra / result.len() as f32;
        for width in &mut result {
            *width += share;
        }
    }
    result
}

struct BuildGeometry<'a> {
    widths: &'a [f32],
    heights: &'a [f32],
    xs: &'a [f32],
    ys: &'a [f32],
    spacing: f32,
    collapse: bool,
    table_cell_padding: f32,
}

fn build_cells(cells: &[PlacedCell], geometry: BuildGeometry<'_>) -> Vec<TableCellLayout> {
    cells
        .iter()
        .map(|cell| {
            let borders = border_sides(&cell.node);
            let width = spanned_width(geometry.widths, cell.column, cell.column_span)
                + if geometry.collapse {
                    0.0
                } else {
                    geometry.spacing * (cell.column_span.saturating_sub(1) as f32)
                };
            let height = spanned_width(geometry.heights, cell.row, cell.row_span)
                + if geometry.collapse {
                    0.0
                } else {
                    geometry.spacing * (cell.row_span.saturating_sub(1) as f32)
                };
            let bounds = LayoutBox {
                x: geometry.xs[cell.column],
                y: geometry.ys[cell.row],
                width,
                height,
            };
            let padding = padding(&cell.node, geometry.table_cell_padding);
            let content_width =
                (width - padding[1] - padding[3] - borders[1].0 - borders[3].0).max(0.0);
            let content_height =
                (height - padding[0] - padding[2] - borders[0].0 - borders[2].0).max(0.0);
            let content_y = match cell.node.style.vertical_align {
                VerticalAlign::Top | VerticalAlign::Baseline => {
                    bounds.y + borders[0].0 + padding[0]
                }
                VerticalAlign::Middle => bounds.y + borders[0].0 + padding[0],
                VerticalAlign::Bottom => bounds.y + borders[0].0 + padding[0],
            };
            TableCellLayout {
                row: cell.row,
                column: cell.column,
                row_span: cell.row_span,
                column_span: cell.column_span,
                bounds,
                content: CellContentBox {
                    bounds: LayoutBox {
                        x: bounds.x + borders[3].0 + padding[3],
                        y: content_y,
                        width: content_width,
                        height: content_height,
                    },
                    vertical_align: cell.node.style.vertical_align,
                },
                borders: [
                    cell.node.style.border.top,
                    cell.node.style.border.right,
                    cell.node.style.border.bottom,
                    cell.node.style.border.left,
                ],
                border_segments: [
                    vec![CollapsedBorderSegment {
                        start: 0,
                        end: cell.column_span,
                        border: cell.node.style.border.top,
                    }],
                    vec![CollapsedBorderSegment {
                        start: 0,
                        end: cell.row_span,
                        border: cell.node.style.border.right,
                    }],
                    vec![CollapsedBorderSegment {
                        start: 0,
                        end: cell.column_span,
                        border: cell.node.style.border.bottom,
                    }],
                    vec![CollapsedBorderSegment {
                        start: 0,
                        end: cell.row_span,
                        border: cell.node.style.border.left,
                    }],
                ],
                node: cell.node.clone(),
            }
        })
        .collect()
}

fn positions(values: &[f32], border_start: f32, spacing: f32, collapse: bool) -> Vec<f32> {
    let mut cursor = border_start + if collapse { 0.0 } else { spacing };
    values
        .iter()
        .map(|value| {
            let current = cursor;
            cursor += *value + if collapse { 0.0 } else { spacing };
            current
        })
        .collect()
}

fn cell_intrinsic(
    cell: &StyledNode,
    measurer: &impl CellMeasurer,
) -> Result<IntrinsicSize, RasterError> {
    // Tables are block flow inside cells. Measure the ordinary inline flow with nested tables
    // removed, then compose every immediate nested table. This preserves text before and after a
    // nested table and does not conceal a malformed nested table behind an `.ok()` fallback.
    let inline = strip_nested_tables(cell);
    let inline_measurement = measurer.intrinsic_size(&inline).normalized();
    let mut min_width = inline_measurement.min_width;
    let mut max_width = inline_measurement.max_width;
    let mut height = inline_measurement.height;
    for table in nested_tables(cell) {
        let layout = layout_table_node(table, 10_000.0, measurer)?;
        min_width = min_width.max(layout.column_widths.iter().sum());
        max_width = max_width.max(layout.bounds.width);
        height += layout.bounds.height;
    }
    Ok(IntrinsicSize {
        min_width,
        max_width,
        height,
    })
}

fn cell_layout_height(
    cell: &StyledNode,
    content_width: f32,
    measurer: &impl CellMeasurer,
) -> Result<f32, RasterError> {
    let inline = strip_nested_tables(cell);
    let mut height = finite_nonnegative(measurer.layout_height(&inline, content_width));
    for table in nested_tables(cell) {
        height += layout_table_node(table, content_width.max(0.0), measurer)?
            .bounds
            .height;
    }
    Ok(height)
}

fn nested_tables(node: &StyledNode) -> Vec<&StyledNode> {
    let mut tables = Vec::new();
    collect_nested_tables(node, &mut tables);
    tables
}

fn collect_nested_tables<'a>(node: &'a StyledNode, tables: &mut Vec<&'a StyledNode>) {
    for child in &node.children {
        match child.kind {
            StyledNodeKind::Element(ElementKind::Table) => tables.push(child),
            StyledNodeKind::Element(_) => collect_nested_tables(child, tables),
            StyledNodeKind::Text(_) | StyledNodeKind::SceneLeaf(_) => {}
        }
    }
}

fn strip_nested_tables(node: &StyledNode) -> StyledNode {
    let children = node
        .children
        .iter()
        .filter_map(strip_nested_table_child)
        .collect();
    StyledNode {
        kind: node.kind.clone(),
        style: node.style.clone(),
        attributes: node.attributes.clone(),
        children,
        scene: node.scene.clone(),
    }
}

/// Removes nested tables from an inline-flow clone without leaving their now-empty block
/// wrappers behind.  A bare `<div><table>...</table></div>` does not create an anonymous text line
/// in browsers; retaining that empty `div` makes R3's block-end break inflate every containing
/// table row.  Atomic inline flow (images, breaks, and scene leaves) remains intact.
fn strip_nested_table_child(node: &StyledNode) -> Option<StyledNode> {
    if matches!(node.kind, StyledNodeKind::Element(ElementKind::Table)) {
        return None;
    }
    let children = node
        .children
        .iter()
        .filter_map(strip_nested_table_child)
        .collect::<Vec<_>>();
    let is_empty_container = children.is_empty()
        && matches!(node.kind, StyledNodeKind::Element(kind) if !matches!(kind, ElementKind::Br | ElementKind::Image));
    if is_empty_container {
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

fn table_spacing(table: &StyledNode) -> f32 {
    if table.style.border_collapse {
        return 0.0;
    }
    table
        .attributes
        .get("cellspacing")
        .and_then(|value| value.parse::<f32>().ok())
        .map(finite_nonnegative)
        .unwrap_or_else(|| {
            let resolved = length_px(table.style.border_spacing, DEFAULT_FONT_SIZE, 0.0);
            if resolved == 0.0 {
                DEFAULT_BORDER_SPACING
            } else {
                resolved
            }
        })
}

fn table_cell_padding(table: &StyledNode) -> f32 {
    table
        .attributes
        .get("cellpadding")
        .and_then(|value| value.parse::<f32>().ok())
        .map(finite_nonnegative)
        .unwrap_or(DEFAULT_CELL_PADDING)
}

fn padding(node: &StyledNode, table_cell_padding: f32) -> [f32; 4] {
    let declared = node.style.padding;
    std::array::from_fn(|side| {
        if node.style.padding_declared[side] {
            length_px(declared[side], DEFAULT_FONT_SIZE, table_cell_padding)
        } else {
            table_cell_padding
        }
    })
}

fn border_sides(node: &StyledNode) -> [(f32, BorderStyle); 4] {
    [
        node.style.border.top,
        node.style.border.right,
        node.style.border.bottom,
        node.style.border.left,
    ]
    .map(|side| (length_px(side.width, DEFAULT_FONT_SIZE, 0.0), side.style))
}

fn border_widths(node: &StyledNode) -> [f32; 4] {
    border_sides(node).map(|(width, _)| width)
}

fn cell_horizontal_extras(node: &StyledNode, table_cell_padding: f32) -> f32 {
    let padding = padding(node, table_cell_padding);
    let border = border_widths(node);
    padding[1] + padding[3] + border[1] + border[3]
}

fn cell_vertical_extras(node: &StyledNode, table_cell_padding: f32) -> f32 {
    let padding = padding(node, table_cell_padding);
    let border = border_widths(node);
    padding[0] + padding[2] + border[0] + border[2]
}

fn span(node: &StyledNode, name: &str) -> Result<usize, RasterError> {
    match node.attributes.get(name) {
        None => Ok(1),
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|span| (1..=1_000).contains(span))
            .ok_or_else(|| layout_error(name, "span must be an integer from 1 through 1000")),
    }
}

fn spanned_width(values: &[f32], start: usize, span: usize) -> f32 {
    values[start..start + span].iter().sum()
}

fn distribute_row_requirement(rows: &mut [f32], start: usize, span: usize, required: f32) {
    let current: f32 = rows[start..start + span].iter().sum();
    let deficit = (required - current).max(0.0);
    if deficit > 0.0 {
        let share = deficit / span as f32;
        for row in &mut rows[start..start + span] {
            *row += share;
        }
    }
}

fn grow_equally(rows: &mut [f32], required: f32) {
    let current: f32 = rows.iter().sum();
    if required > current && !rows.is_empty() {
        let share = (required - current) / rows.len() as f32;
        for row in rows {
            *row += share;
        }
    }
}

fn vertical_gap_total(collapse: bool, spacing: f32, rows: usize) -> f32 {
    if collapse {
        0.0
    } else {
        spacing * (rows.saturating_add(1) as f32)
    }
}

fn resolve_optional_width(length: Option<CssLength>, available: f32) -> Option<f32> {
    length.map(|length| length_px(length, DEFAULT_FONT_SIZE, available))
}

fn declared_minimum_width(node: &StyledNode, available: f32) -> Option<f32> {
    match (
        resolve_optional_width(node.style.width, available),
        resolve_optional_width(node.style.min_width, available),
    ) {
        (Some(width), Some(minimum)) => Some(width.max(minimum)),
        (Some(width), None) | (None, Some(width)) => Some(width),
        (None, None) => None,
    }
}

fn resolve_optional_height(length: Option<CssLength>, available: f32) -> Option<f32> {
    resolve_optional_width(length, available)
}

fn length_px(length: CssLength, em: f32, percent_reference: f32) -> f32 {
    match length {
        CssLength::Px(value) => finite_nonnegative(value),
        CssLength::Percent(value) => finite_nonnegative(percent_reference * value / 100.0),
        CssLength::Em(value) => finite_nonnegative(em * value),
        CssLength::Pt(value) => finite_nonnegative(value * 96.0 / 72.0),
        CssLength::Zero => 0.0,
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn require_kind(node: &StyledNode, kind: ElementKind) -> Result<(), RasterError> {
    if matches!(node.kind, StyledNodeKind::Element(actual) if actual == kind) {
        Ok(())
    } else {
        Err(layout_error("table", "layout root is not a table"))
    }
}

fn layout_error(name: &str, detail: &str) -> RasterError {
    RasterError::Unsupported(
        crate::UnsupportedFeatureKind::Attribute,
        name.to_owned(),
        detail.to_owned(),
    )
}

fn estimate_intrinsic(cell: &StyledNode) -> IntrinsicSize {
    let text = text_content(cell);
    let width = text
        .chars()
        .map(|character| if character.is_whitespace() { 4.0 } else { 8.0 })
        .sum::<f32>();
    let min_width = text
        .split_whitespace()
        .map(|word| word.chars().count() as f32 * 8.0)
        .fold(0.0, f32::max);
    IntrinsicSize {
        min_width,
        max_width: width,
        height: 19.0,
    }
}

fn estimate_height(cell: &StyledNode, width: f32) -> f32 {
    let intrinsic = estimate_intrinsic(cell);
    if width <= 0.0 || intrinsic.max_width == 0.0 {
        intrinsic.height
    } else {
        (intrinsic.max_width / width).ceil().max(1.0) * intrinsic.height
    }
}

fn text_content(node: &StyledNode) -> String {
    match &node.kind {
        StyledNodeKind::Text(text) => text.clone(),
        StyledNodeKind::Element(_) => node.children.iter().map(text_content).collect(),
        StyledNodeKind::SceneLeaf(_) => "\u{FFFC}".to_owned(),
    }
}

#[cfg(test)]
#[path = "table_layout/tests.rs"]
mod tests;
