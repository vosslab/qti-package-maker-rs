use std::collections::BTreeMap;

use super::*;
use crate::parse_fragment;
use crate::subset::{SceneLeafKind, SceneText};

fn cell(html: &str) -> StyledNode {
    parse_fragment(html).expect("valid table").root.children[0].children[0].children[0].clone()
}

#[test]
fn final_width_wraps_but_nowrap_and_pre_do_not() {
    let normal = cell("<table><tr><td>alpha beta gamma</td></tr></table>");
    let nowrap =
        cell("<table><tr><td style='white-space:nowrap'>alpha beta gamma</td></tr></table>");
    let pre = cell("<table><tr><td style='white-space:pre'>alpha beta\ngamma</td></tr></table>");
    assert!(layout_inline(&normal, 40.0).lines.len() > 1);
    assert_eq!(layout_inline(&nowrap, 40.0).lines.len(), 1);
    assert_eq!(layout_inline(&pre, 40.0).lines.len(), 2);
}

#[test]
fn intrinsic_widths_include_unbreakable_words_and_are_ordered() {
    let node = cell(
        "<table><tr><td>small pneumonoultramicroscopicsilicovolcanoconiosis</td></tr></table>",
    );
    let widths = measure_inline(&node);
    assert!(widths.min_width > 100.0);
    assert!(widths.max_width >= widths.min_width);
}

#[test]
fn formatted_sub_sup_entities_and_zero_line_height_are_supported() {
    let node = cell(
        "<table><tr><td><b>H</b><sub>2</sub>O<sup>+</sup>&nbsp;<i style='font-weight:650'>ion</i></td><td style='font-size:0;line-height:0;height:21px'>x</td></tr></table>",
    );
    let formatted = layout_inline(&node, 200.0);
    assert!(formatted.runs.len() >= 4);
    assert!(formatted.runs.iter().any(|run| run.face.weight == 650));
    let normal_baseline = formatted.runs[0].baseline_y;
    assert!(
        formatted
            .runs
            .iter()
            .any(|run| run.baseline_y < normal_baseline)
    );
    let zero = cell("<table><tr><td style='font-size:0;line-height:0'>x</td></tr></table>");
    assert_eq!(measure_inline(&zero).max_width, 0.0);
    assert_eq!(layout_inline(&zero, 100.0).height, 0.0);
}

#[test]
fn right_and_center_alignment_change_run_origin() {
    let right = cell("<table><tr><td style='text-align:right'>X</td></tr></table>");
    let center = cell("<table><tr><td style='text-align:center'>X</td></tr></table>");
    let right_x = layout_inline(&right, 100.0).runs[0].bounds.x;
    let center_x = layout_inline(&center, 100.0).runs[0].bounds.x;
    assert!(right_x > center_x);
    assert!(center_x > 0.0);
}

#[test]
fn paragraph_margins_and_cell_vertical_alignment_are_geometry() {
    let paragraph =
        cell("<table><tr><td><p>one</p><div style='margin-top:4px'>two</div></td></tr></table>");
    let mut layout = layout_inline(&paragraph, 100.0);
    let unplaced_height = layout.height;
    assert!(
        unplaced_height > 40.0,
        "paragraph UA margins remain visible"
    );
    place_inline(
        &mut layout,
        LayoutBox {
            x: 7.0,
            y: 11.0,
            width: 100.0,
            height: unplaced_height + 20.0,
        },
        VerticalAlign::Bottom,
    );
    assert!(layout.lines[0].bounds.x >= 7.0);
    assert!(layout.lines[0].bounds.y >= 31.0);
    assert!(layout.runs.iter().all(|run| run.bounds.x >= 7.0));
}

#[test]
fn hidden_text_retains_geometry_without_glyph_runs_and_spacing_changes_measurement() {
    let visible = cell("<table><tr><td>AB</td></tr></table>");
    let hidden = cell("<table><tr><td><span style='visibility:hidden'>AB</span></td></tr></table>");
    let spaced = cell("<table><tr><td style='letter-spacing:4px'>AB</td></tr></table>");
    let visible_layout = layout_inline(&visible, 100.0);
    let hidden_layout = layout_inline(&hidden, 100.0);
    assert_eq!(hidden_layout.runs.len(), 0);
    assert_eq!(hidden_layout.width, visible_layout.width);
    assert_eq!(hidden_layout.height, visible_layout.height);
    assert!(measure_inline(&spaced).max_width > measure_inline(&visible).max_width);
}

#[test]
fn observed_scaled_inline_bracket_expands_advance_and_marks_its_paint_run() {
    let ordinary = cell(
        "<table><tr><td>A<span style='font-size:xx-large;display:inline-block'>\u{27ee}</span>B</td></tr></table>",
    );
    let scaled = cell(
        "<table><tr><td>A<span style='font-size:xx-large;transform:scale(1.35);display:inline-block'>\u{27ee}</span>B</td></tr></table>",
    );

    let ordinary_width = measure_inline(&ordinary).max_width;
    let scaled_width = measure_inline(&scaled).max_width;
    let layout = layout_inline(&scaled, 300.0);
    let bracket = layout
        .runs
        .iter()
        .find(|run| (run.scale_x - 1.35).abs() < f32::EPSILON)
        .expect("scaled bracket has an independently paintable run");

    assert!(scaled_width > ordinary_width);
    assert!(bracket.bounds.width > 0.0);
    assert!(layout.width >= scaled_width);
    assert!(parse_fragment(
            "<table><tr><td><span style='font-size:xx-large;transform:scale(1.4);display:inline-block'>\u{27ee}</span></td></tr></table>"
        )
        .is_err());
}

#[test]
fn scene_leaf_is_an_inline_object_between_text_without_dropping_its_payload() {
    let prototype = cell("<table><tr><td>prototype</td></tr></table>");
    let style = prototype.style.clone();
    let scene_text = StyledNode {
        kind: StyledNodeKind::Text("scene label".to_owned()),
        style: style.clone(),
        attributes: BTreeMap::new(),
        children: Vec::new(),
        scene: None,
    };
    let leaf = StyledNode {
        kind: StyledNodeKind::SceneLeaf(SceneLeafKind::PedigreeGlyph),
        style: style.clone(),
        attributes: BTreeMap::new(),
        children: Vec::new(),
        scene: Some(SceneLeaf {
            kind: SceneLeafKind::PedigreeGlyph,
            intrinsic_width: 40.0,
            intrinsic_height: 24.0,
            display_list: crate::DisplayList::default(),
            text_groups: vec![SceneText {
                node: scene_text,
                anchor: crate::subset::SceneAnchor::Center,
                x_css: 0.0,
                y_css: 0.0,
            }],
        }),
    };
    let node = StyledNode {
        kind: StyledNodeKind::Element(ElementKind::Div),
        style,
        attributes: BTreeMap::new(),
        children: vec![
            StyledNode {
                kind: StyledNodeKind::Text("before ".to_owned()),
                style: prototype.style.clone(),
                attributes: BTreeMap::new(),
                children: Vec::new(),
                scene: None,
            },
            leaf,
            StyledNode {
                kind: StyledNodeKind::Text(" after".to_owned()),
                style: prototype.style.clone(),
                attributes: BTreeMap::new(),
                children: Vec::new(),
                scene: None,
            },
        ],
        scene: None,
    };
    let layout = layout_inline(&node, 300.0);
    assert_eq!(layout.scene_objects.len(), 1);
    assert_eq!(layout.scene_objects[0].bounds.width, 40.0);
    assert_eq!(layout.scene_objects[0].scene.text_groups.len(), 1);
    let scene = &layout.scene_objects[0].bounds;
    assert!(
        layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .any(|glyph| (glyph.x as f32) < scene.x)
    );
    assert!(
        layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .any(|glyph| (glyph.x as f32) > scene.x + scene.width)
    );
    assert!(layout.width >= 40.0);
}

#[test]
fn comment_adjacent_leading_whitespace_does_not_shift_a_scene_leaf() {
    let node = cell(
        "<table><tr><td align='center'>\n<!-- source indentation -->\n<span style='display:inline-block;position:relative;width:65px;height:65px;line-height:65px;text-align:center;font-weight:bold;font-size:39px;border:2px solid #000;box-sizing:border-box;overflow:hidden;background-color:#ffffff;color:#000000'><span style='position:relative;z-index:1'>&#160;</span></span></td></tr></table>",
    );
    let layout = layout_inline(&node, 65.0);

    assert_eq!(layout.scene_objects.len(), 1);
    assert_eq!(layout.scene_objects[0].bounds.x, 0.0);
}

#[test]
fn finite_decorated_boxes_retain_final_geometry_and_inline_children() {
    let node = cell(
        "<table><tr><td><div style='height:7px;background-color:#99dbfb;border-radius:4px;box-shadow:0 0 2px #99dbfb'></div><span style='display:inline-block;width:38px;height:38px;border-radius:50%;background:#5e6312;border:2px solid #333'>I</span></td></tr></table>",
    );
    let mut layout = layout_inline(&node, 120.0);

    assert_eq!(layout.decorated_empty_blocks.len(), 1);
    assert_eq!(layout.decorated_empty_blocks[0].bounds.width, 120.0);
    assert_eq!(layout.decorated_empty_blocks[0].bounds.height, 7.0);
    assert_eq!(layout.decorated_inline_boxes.len(), 1);
    assert_eq!(layout.decorated_inline_boxes[0].bounds.width, 38.0);
    assert_eq!(layout.decorated_inline_boxes[0].bounds.height, 38.0);
    assert_eq!(layout.decorated_inline_boxes[0].children.len(), 1);

    place_inline(
        &mut layout,
        LayoutBox {
            x: 3.0,
            y: 5.0,
            width: 120.0,
            height: 100.0,
        },
        VerticalAlign::Top,
    );
    assert_eq!(layout.decorated_empty_blocks[0].bounds.x, 3.0);
    assert_eq!(layout.decorated_inline_boxes[0].bounds.x, 3.0);
}
