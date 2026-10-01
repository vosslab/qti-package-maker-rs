//! Per-grid-segment conflict resolution for CSS collapsed table borders.

use super::{CollapsedBorderSegment, TableCellLayout};
use crate::{BorderSide, BorderStyle, CssLength};
use std::cmp::Ordering;

pub(super) fn resolve(mut cells: Vec<TableCellLayout>) -> Vec<TableCellLayout> {
    for first in 0..cells.len() {
        for second in first + 1..cells.len() {
            resolve_pair_segments(&mut cells, first, second);
        }
    }
    for cell in &mut cells {
        cell.borders = std::array::from_fn(|side| strongest_segment(&cell.border_segments[side]));
    }
    cells
}

fn resolve_pair_segments(cells: &mut [TableCellLayout], first: usize, second: usize) {
    let (left, right) = cells.split_at_mut(second);
    let a = &mut left[first];
    let b = &mut right[0];
    if a.column + a.column_span == b.column {
        resolve_vertical_edge(a, b, 1, 3);
    }
    if b.column + b.column_span == a.column {
        resolve_vertical_edge(b, a, 1, 3);
    }
    if a.row + a.row_span == b.row {
        resolve_horizontal_edge(a, b, 2, 0);
    }
    if b.row + b.row_span == a.row {
        resolve_horizontal_edge(b, a, 2, 0);
    }
}

fn resolve_vertical_edge(
    left: &mut TableCellLayout,
    right: &mut TableCellLayout,
    left_side: usize,
    right_side: usize,
) {
    let start = left.row.max(right.row);
    let end = (left.row + left.row_span).min(right.row + right.row_span);
    if start >= end {
        return;
    }
    let winner = stronger_border(left.borders[left_side], right.borders[right_side]);
    replace_segment(
        &mut left.border_segments[left_side],
        start - left.row,
        end - left.row,
        winner,
    );
    replace_segment(
        &mut right.border_segments[right_side],
        start - right.row,
        end - right.row,
        winner,
    );
}

fn resolve_horizontal_edge(
    top: &mut TableCellLayout,
    bottom: &mut TableCellLayout,
    top_side: usize,
    bottom_side: usize,
) {
    let start = top.column.max(bottom.column);
    let end = (top.column + top.column_span).min(bottom.column + bottom.column_span);
    if start >= end {
        return;
    }
    let winner = stronger_border(top.borders[top_side], bottom.borders[bottom_side]);
    replace_segment(
        &mut top.border_segments[top_side],
        start - top.column,
        end - top.column,
        winner,
    );
    replace_segment(
        &mut bottom.border_segments[bottom_side],
        start - bottom.column,
        end - bottom.column,
        winner,
    );
}

fn replace_segment(
    segments: &mut Vec<CollapsedBorderSegment>,
    start: usize,
    end: usize,
    border: BorderSide,
) {
    let mut replacement = Vec::with_capacity(segments.len() + 2);
    for segment in segments.drain(..) {
        if segment.end <= start || end <= segment.start {
            replacement.push(segment);
            continue;
        }
        if segment.start < start {
            replacement.push(CollapsedBorderSegment {
                end: start,
                ..segment
            });
        }
        replacement.push(CollapsedBorderSegment {
            start: segment.start.max(start),
            end: segment.end.min(end),
            border,
        });
        if end < segment.end {
            replacement.push(CollapsedBorderSegment {
                start: end,
                ..segment
            });
        }
    }
    replacement.sort_by_key(|segment| segment.start);
    *segments = coalesce_segments(replacement);
}

fn coalesce_segments(segments: Vec<CollapsedBorderSegment>) -> Vec<CollapsedBorderSegment> {
    let mut output: Vec<CollapsedBorderSegment> = Vec::with_capacity(segments.len());
    for segment in segments {
        let joins_previous = output.last().is_some_and(|previous| {
            previous.end == segment.start && previous.border == segment.border
        });
        if joins_previous {
            output.last_mut().expect("checked above").end = segment.end;
            continue;
        }
        output.push(segment);
    }
    output
}

fn strongest_segment(segments: &[CollapsedBorderSegment]) -> BorderSide {
    segments
        .iter()
        .map(|segment| segment.border)
        .reduce(stronger_border)
        .expect("each cell edge starts with one segment")
}

fn stronger_border(a: BorderSide, b: BorderSide) -> BorderSide {
    // CSS 2.1 makes `hidden` the highest-priority collapsed-border value.  Retaining it in
    // the resolved segment lets painting suppress the edge without falling through to the
    // otherwise visible competing border.
    match (
        a.style == BorderStyle::Hidden,
        b.style == BorderStyle::Hidden,
    ) {
        (true, false) => return a,
        (false, true) => return b,
        (true, true) => return a,
        (false, false) => {}
    }
    match border_strength(a)
        .partial_cmp(&border_strength(b))
        .unwrap_or(Ordering::Equal)
    {
        Ordering::Less => b,
        Ordering::Greater | Ordering::Equal => a,
    }
}

fn border_strength(side: BorderSide) -> f32 {
    let rank = match side.style {
        BorderStyle::None => 0.0,
        BorderStyle::Dotted => 1.0,
        BorderStyle::Dashed => 2.0,
        BorderStyle::Solid => 3.0,
        BorderStyle::Double => 4.0,
        BorderStyle::Hidden => 5.0,
    };
    css_length_px(side.width) * 10.0 + rank
}

fn css_length_px(length: CssLength) -> f32 {
    match length {
        CssLength::Px(value) => value.max(0.0),
        CssLength::Em(value) => (value * 16.0).max(0.0),
        CssLength::Pt(value) => (value * 96.0 / 72.0).max(0.0),
        CssLength::Percent(_) | CssLength::Zero => 0.0,
    }
}
