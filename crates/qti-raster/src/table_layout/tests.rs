use super::*;
use crate::{DisplayList, SceneLeaf, SceneLeafKind, StyledNode, StyledNodeKind, parse_fragment};

fn measure(_: &StyledNode) -> IntrinsicSize {
    IntrinsicSize {
        min_width: 10.0,
        max_width: 20.0,
        height: 10.0,
    }
}
fn height(_: &StyledNode, _: f32) -> f32 {
    10.0
}

#[test]
fn colspan_spans_exactly_its_columns() {
    let tree = parse_fragment("<table cellspacing='0'><tr><td width='20'>a</td><td width='30'>b</td></tr><tr><td colspan='2'>c</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 200.0, &(measure, height)).unwrap();
    let spanning = &layout.cells[2];
    assert_eq!(spanning.column_span, 2);
    assert!((spanning.bounds.width - layout.column_widths.iter().sum::<f32>()).abs() < 0.01);
}

#[test]
fn rowspan_covers_its_rows() {
    let tree = parse_fragment("<table cellspacing='0'><tr><td rowspan='2' height='40'>a</td><td>b</td></tr><tr><td>c</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 200.0, &(measure, height)).unwrap();
    let spanning = &layout.cells[0];
    assert_eq!(spanning.row_span, 2);
    assert!((spanning.bounds.height - layout.row_heights.iter().sum::<f32>()).abs() < 0.01);
    assert!(spanning.bounds.height >= 40.0);
}

#[test]
fn declared_cell_size_is_a_minimum() {
    let tree = parse_fragment("<table><tr><td width='75' height='30'>x</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 100.0, &(measure, height)).unwrap();
    assert!(layout.cells[0].bounds.width >= 75.0);
    assert!(layout.cells[0].bounds.height >= 30.0);
}

#[test]
fn collapsed_shared_edge_uses_stronger_border() {
    let tree = parse_fragment("<table style='border-collapse:collapse'><tr><td style='border-right:1px solid black'>a</td><td style='border-left:3px dotted black'>b</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 100.0, &(measure, height)).unwrap();
    assert_eq!(layout.cells[0].borders[1].width, CssLength::Px(3.0));
    assert_eq!(layout.cells[1].borders[3].width, CssLength::Px(3.0));
}

#[test]
fn collapsed_hidden_border_suppresses_a_visible_competing_edge() {
    let tree = parse_fragment("<table style='border-collapse:collapse'><tr><td style='border-right:1px solid black'>a</td><td style='border-left:hidden'>b</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 100.0, &(measure, height)).unwrap();
    assert_eq!(
        layout.cells[0].border_segments[1][0].border.style,
        BorderStyle::Hidden
    );
    assert_eq!(
        layout.cells[1].border_segments[3][0].border.style,
        BorderStyle::Hidden
    );
}

#[test]
fn finite_margin_auto_table_centers_its_cells_with_its_bounds() {
    let tree = parse_fragment(
        "<table style='width:80px;margin:0 auto' cellspacing='0'><tr><td>a</td></tr></table>",
    )
    .unwrap();
    let layout = layout_table(&tree, 200.0, &(measure, height)).unwrap();
    assert_eq!(layout.bounds.width, 80.0);
    assert_eq!(layout.bounds.x, 60.0);
    assert!(layout.cells[0].bounds.x >= layout.bounds.x);
}

#[test]
fn separated_model_applies_cellspacing() {
    let tree =
        parse_fragment("<table cellspacing='5'><tr><td>a</td><td>b</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 100.0, &(measure, height)).unwrap();
    assert_eq!(layout.border_spacing, 5.0);
    assert!(
        (layout.cells[1].bounds.x
            - (layout.cells[0].bounds.x + layout.cells[0].bounds.width)
            - 5.0)
            .abs()
            < 0.01
    );
}

#[test]
fn explicit_zero_padding_overrides_table_cellpadding() {
    let tree = parse_fragment(
        "<table cellpadding='4' cellspacing='0'><tr><td style='padding:0'>a</td></tr></table>",
    )
    .unwrap();
    let layout = layout_table(&tree, 100.0, &(measure, height)).unwrap();
    let cell = &layout.cells[0];
    assert_eq!(cell.content.bounds.x, cell.bounds.x);
    assert_eq!(cell.content.bounds.y, cell.bounds.y);
}

#[test]
fn nested_tables_compose_with_before_after_text_and_each_other() {
    let text_only = parse_fragment("<table><tr><td>before after</td></tr></table>").unwrap();
    let one_nested = parse_fragment(
        "<table><tr><td>before<table><tr><td>nested</td></tr></table>after</td></tr></table>",
    )
    .unwrap();
    let two_nested = parse_fragment("<table><tr><td>before<table><tr><td>one</td></tr></table>middle<table><tr><td>two</td></tr></table>after</td></tr></table>").unwrap();
    let plain_height = layout_table(&text_only, 200.0, &(measure, height))
        .unwrap()
        .row_heights[0];
    let one_height = layout_table(&one_nested, 200.0, &(measure, height))
        .unwrap()
        .row_heights[0];
    let two_height = layout_table(&two_nested, 200.0, &(measure, height))
        .unwrap()
        .row_heights[0];
    assert!(one_height > plain_height);
    assert!(two_height > one_height);
}

#[test]
fn empty_block_wrapper_around_nested_table_does_not_add_an_inline_line() {
    let direct = parse_fragment(
        "<table cellspacing='0'><tr><td><table cellspacing='0'><tr><td>x</td></tr></table></td></tr></table>",
    )
    .unwrap();
    let wrapped = parse_fragment(
        "<table cellspacing='0'><tr><td><div style='padding-left:5px; padding-right:5px'><table cellspacing='0'><tr><td>x</td></tr></table></div></td></tr></table>",
    )
    .unwrap();
    let direct_height = layout_table_with_native_text(&direct, 200.0)
        .unwrap()
        .row_heights[0];
    let wrapped_height = layout_table_with_native_text(&wrapped, 200.0)
        .unwrap()
        .row_heights[0];
    assert!(
        (wrapped_height - direct_height).abs() < 0.01,
        "empty wrapper must not add a line: direct={direct_height}, wrapped={wrapped_height}"
    );
}

#[test]
fn malformed_nested_table_is_not_silently_ignored() {
    let tree = parse_fragment("<table><tr><td>x<table></table>y</td></tr></table>").unwrap();
    assert!(layout_table(&tree, 200.0, &(measure, height)).is_err());
}

#[test]
fn scene_leaf_contributes_native_cell_metrics_and_survives_table_layout() {
    let mut tree = parse_fragment("<table cellspacing='0'><tr><td></td></tr></table>").unwrap();
    let cell = &mut tree.root.children[0].children[0].children[0];
    cell.children = vec![StyledNode {
        kind: StyledNodeKind::SceneLeaf(SceneLeafKind::PedigreeGlyph),
        style: cell.style.clone(),
        attributes: Default::default(),
        children: Vec::new(),
        scene: Some(SceneLeaf {
            kind: SceneLeafKind::PedigreeGlyph,
            intrinsic_width: 47.0,
            intrinsic_height: 31.0,
            display_list: DisplayList::default(),
            text_groups: Vec::new(),
        }),
    }];

    let inline_widths = crate::measure_inline(cell);
    let layout = layout_table_with_native_text(&tree, 200.0).unwrap();
    let laid_out = &layout.cells[0];

    assert!(inline_widths.min_width >= 47.0);
    assert!(laid_out.bounds.width >= inline_widths.min_width);
    assert!(laid_out.bounds.height >= 31.0);
    assert!(matches!(
        laid_out.node.children[0].kind,
        StyledNodeKind::SceneLeaf(_)
    ));
    assert!(laid_out.node.children[0].scene.is_some());
}

#[test]
fn collapsed_rowspan_edge_resolves_each_neighbour_segment() {
    let tree = parse_fragment("<table style='border-collapse:collapse'><tr><td rowspan='2' style='border-right:1px solid black'>a</td><td style='border-left:3px dotted black'>b</td></tr><tr><td style='border-left:2px double black'>c</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 200.0, &(measure, height)).unwrap();
    let spanning = &layout.cells[0];
    assert_eq!(spanning.border_segments[1].len(), 2);
    assert_eq!(spanning.border_segments[1][0].start, 0);
    assert_eq!(
        spanning.border_segments[1][0].border.style,
        BorderStyle::Dotted
    );
    assert_eq!(spanning.border_segments[1][1].start, 1);
    assert_eq!(
        spanning.border_segments[1][1].border.style,
        BorderStyle::Double
    );
}

#[test]
fn collapsed_colspan_edge_resolves_each_neighbour_segment() {
    let tree = parse_fragment("<table style='border-collapse:collapse'><tr><td colspan='2' style='border-bottom:1px solid black'>a</td></tr><tr><td style='border-top:2px dashed black'>b</td><td style='border-top:3px dotted black'>c</td></tr></table>").unwrap();
    let layout = layout_table(&tree, 200.0, &(measure, height)).unwrap();
    let spanning = &layout.cells[0];
    assert_eq!(spanning.border_segments[2].len(), 2);
    assert_eq!(
        spanning.border_segments[2][0].border.style,
        BorderStyle::Dashed
    );
    assert_eq!(
        spanning.border_segments[2][1].border.style,
        BorderStyle::Dotted
    );
}
