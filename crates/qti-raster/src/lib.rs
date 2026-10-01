//! Native parsing and styling contracts for the supported QTI table subset.
//!
//! This crate deliberately separates the parsed and styled tree from layout and painting. The
//! later table, inline, and paint work packages consume the public contracts here.

mod boxplot_scene;
mod fonts;
mod inline_layout;
mod paint;
mod render;
mod restriction_digest_scene;
mod scene_leaf;
mod style;
mod subset;
mod table_layout;

pub use boxplot_scene::parse_boxplot_fragment;
pub use paint::{PaintError, PaintedPng, paint_display_list, paint_display_list_with_metrics};
pub use render::{RenderMetrics, RenderedPng, render_table_png, render_table_png_with_metrics};
pub use restriction_digest_scene::parse_restriction_digest_fragment;

pub use table_layout::{
    CellContentBox, CellMeasurer, CollapsedBorderSegment, IntrinsicSize, NativeCellMeasurer,
    TableCellLayout, TableLayout, layout_table, layout_table_with_native_text,
    layout_table_with_text_estimate,
};

pub use fonts::{ATKINSON_MONO_FAMILY, ATKINSON_MONO_OFL, ATKINSON_NEXT_FAMILY, ATKINSON_NEXT_OFL};
pub use inline_layout::{
    DecoratedEmptyBlock, DecoratedInlineBox, FontFaceKey, InlineLayout, InlineLine,
    IntrinsicWidths, PositionedGlyph, PositionedSceneObject, TextRun, layout_inline,
    measure_inline, place_inline,
};

pub use crate::style::{
    Border, BorderSide, BorderStyle, BoxShadow, CaptionSide, Color, ComputedStyle, CssLength,
    DisplayMode, FontFamily, FontStyle, FontWeight, TextAlign, VerticalAlign, WhiteSpace,
};
pub use crate::subset::{
    DisplayCommand, DisplayList, ElementKind, LayoutBox, RasterConfig, RasterError, SceneAnchor,
    SceneLeaf, SceneLeafKind, SceneText, StyledNode, StyledNodeKind, StyledTree,
    UnsupportedFeatureKind, UnsupportedTableFeature, parse_fragment, parse_fragment_with_config,
};
