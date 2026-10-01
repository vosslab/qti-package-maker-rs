use super::{
    is_restriction_digest_fragment, render_table_png, render_table_png_with_metrics, scale_length,
};
use crate::{CssLength, ElementKind, RasterConfig, StyledNode, StyledNodeKind, parse_fragment};

#[test]
fn renders_text_background_and_nested_table_without_a_browser() {
    let png = render_table_png(
            "<table style='background:#d0e0ff'><tr><td>outer<table><tr><td>inner</td></tr></table></td></tr></table>",
            &RasterConfig::default(),
        )
        .expect("native table render");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn metrics_render_preserves_the_png_contract() {
    let html = "<table><tr><td>measured</td></tr></table>";
    let config = RasterConfig::default();
    let measured = render_table_png_with_metrics(html, &config).expect("measured render");
    assert!(measured.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(
        render_table_png(html, &config).expect("byte-only render"),
        measured.bytes
    );
}

#[test]
fn physical_scaling_preserves_relative_lengths() {
    assert_eq!(scale_length(CssLength::Px(3.0), 2.0), CssLength::Px(6.0));
    assert_eq!(scale_length(CssLength::Pt(3.0), 2.0), CssLength::Pt(6.0));
    assert_eq!(scale_length(CssLength::Em(1.2), 2.0), CssLength::Em(1.2));
    assert_eq!(
        scale_length(CssLength::Percent(50.0), 2.0),
        CssLength::Percent(50.0)
    );
}

#[test]
fn nested_table_extraction_keeps_empty_decorated_blocks() {
    let tree = parse_fragment(
        "<table><tr><td><div style='height:7px;background:#99dbfb;border-radius:4px'></div></td></tr></table>",
    )
    .expect("supported decorated block");
    let filtered = super::assembly::without_nested_tables(&tree.root);
    assert!(contains_empty_div_with_background(&filtered));
}

fn contains_empty_div_with_background(node: &StyledNode) -> bool {
    (matches!(node.kind, StyledNodeKind::Element(ElementKind::Div))
        && node.children.is_empty()
        && node.style.background.is_some())
        || node.children.iter().any(contains_empty_div_with_background)
}

#[test]
fn renders_a_pedigree_leaf_inside_its_ordinary_table_cell() {
    let png = render_table_png(
            "<table><tr><td style='padding:0'><span style='display:inline-block;position:relative;width:65px;height:65px;line-height:65px;text-align:center;font-weight:bold;font-size:39px;border:2px solid #000;box-sizing:border-box;overflow:hidden;background-color:#fff;color:#000'><span style='position:relative;z-index:1'>&#160;</span></span></td><td>connector</td></tr></table>",
            &RasterConfig::default(),
        )
        .expect("ordinary grid and its scene leaf render together");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn rejects_invalid_output_geometry_with_context() {
    let config = RasterConfig {
        available_width_css_px: 0.0,
        ..RasterConfig::default()
    };
    assert!(render_table_png("<table><tr><td>x</td></tr></table>", &config).is_err());
}

#[test]
fn restriction_route_is_limited_to_the_two_harvested_aria_labels() {
    assert!(is_restriction_digest_fragment(
        "<table role=\"img\" aria-label=\"Linear restriction-digest DNA map.\"></table>"
    ));
    assert!(!is_restriction_digest_fragment(
        "<table role=\"img\" aria-label=\"Unrelated positioned figure.\"></table>"
    ));
}
