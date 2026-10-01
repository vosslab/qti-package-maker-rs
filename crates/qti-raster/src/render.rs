//! End-to-end native table rendering from an allowlisted fragment to PNG bytes.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use scraper::{ElementRef, Html, Node};

use crate::{
    Border, CellMeasurer, ComputedStyle, CssLength, DisplayCommand, ElementKind, IntrinsicSize,
    LayoutBox, RasterConfig, RasterError, StyledNode, StyledNodeKind, StyledTree, layout_inline,
    layout_table, paint_display_list_with_metrics,
};

/// Measured native-render work for one fragment.
///
/// Every span is local to a single render call. The spans do not overlap: `select` resolves the
/// supported route and parses a general fragment, `layout` builds the display list, and the
/// raster backend separately reports `paint` and `encode`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderMetrics {
    /// Route selection and allowlisted general-fragment parsing.
    pub select: Duration,
    /// Display-list construction, including physical table layout.
    pub layout: Duration,
    /// Raster allocation and display-command painting.
    pub paint: Duration,
    /// In-memory PNG encoding after painting completes.
    pub encode: Duration,
}

/// Encoded table raster plus the local work measurements that produced it.
#[derive(Clone, Debug)]
pub struct RenderedPng {
    /// Fully encoded PNG bytes.
    pub bytes: Vec<u8>,
    /// Actual non-overlapping renderer work spans.
    pub metrics: RenderMetrics,
}

/// Renders one outer table using the native parser, layout, text, image, and paint pipeline.
///
/// Layout is performed in physical pixels after scaling the parsed CSS tree. The input fragment is
/// parsed before any resource is decoded, and only parser-approved embedded images are measured
/// or painted. No JavaScript, network resource, host font, or browser process is involved.
pub fn render_table_png(html: &str, config: &RasterConfig) -> Result<Vec<u8>, RasterError> {
    Ok(render_table_png_with_metrics(html, config)?.bytes)
}

/// Renders one outer table and returns its PNG with local, non-overlapping work measurements.
///
/// These measurements deliberately exclude cache lookup, source loading, and output-file writes:
/// those operations belong to the conversion owner, which aggregates this per-fragment result.
pub fn render_table_png_with_metrics(
    html: &str,
    config: &RasterConfig,
) -> Result<RenderedPng, RasterError> {
    if !config.available_width_css_px.is_finite()
        || config.available_width_css_px <= 0.0
        || config.device_scale_factor == 0
    {
        return Err(RasterError::InvalidGeometry {
            stage: "configured viewport",
            css_width: config.available_width_css_px,
            css_height: 0.0,
            device_scale_factor: config.device_scale_factor,
        });
    }

    let select_started = Instant::now();
    if is_boxplot_fragment(html) {
        let select = select_started.elapsed();
        let layout_started = Instant::now();
        let display_list = crate::parse_boxplot_fragment(html, config)?;
        let (width, height) = canvas_dimensions(display_list.bounds, config.device_scale_factor)?;
        let layout = layout_started.elapsed();
        let painted =
            paint_display_list_with_metrics(&display_list, width, height).map_err(|error| {
                RasterError::Paint {
                    reason: error.to_string(),
                }
            });
        return painted.map(|painted| RenderedPng {
            bytes: painted.bytes,
            metrics: RenderMetrics {
                select,
                layout,
                paint: painted.paint_duration,
                encode: painted.encode_duration,
            },
        });
    }
    if is_restriction_digest_fragment(html) {
        let select = select_started.elapsed();
        let layout_started = Instant::now();
        let display_list = crate::parse_restriction_digest_fragment(html, config)?;
        let (width, height) = canvas_dimensions(display_list.bounds, config.device_scale_factor)?;
        let layout = layout_started.elapsed();
        let painted =
            paint_display_list_with_metrics(&display_list, width, height).map_err(|error| {
                RasterError::Paint {
                    reason: error.to_string(),
                }
            });
        return painted.map(|painted| RenderedPng {
            bytes: painted.bytes,
            metrics: RenderMetrics {
                select,
                layout,
                paint: painted.paint_duration,
                encode: painted.encode_duration,
            },
        });
    }

    let tree = crate::parse_fragment_with_config(html, config)?;
    let select = select_started.elapsed();
    let layout_started = Instant::now();
    let scale = config.device_scale_factor as f32;
    let tree = scale_tree(&tree, scale);
    let images = ImageMetrics::collect(&tree)?;
    let measurer = RenderMeasurer { images: &images };
    let layout = layout_table(&tree, config.available_width_css_px * scale, &measurer)?;
    let display_list = build_display_list(&tree, &layout, &measurer)?;
    let (width, height) = canvas_dimensions(display_list.bounds, config.device_scale_factor)?;
    let layout = layout_started.elapsed();
    let painted =
        paint_display_list_with_metrics(&display_list, width, height).map_err(|error| {
            RasterError::Paint {
                reason: error.to_string(),
            }
        })?;
    Ok(RenderedPng {
        bytes: painted.bytes,
        metrics: RenderMetrics {
            select,
            layout,
            paint: painted.paint_duration,
            encode: painted.encode_duration,
        },
    })
}

fn is_boxplot_fragment(html: &str) -> bool {
    let document = Html::parse_fragment(html);
    document.root_element().children().any(|node| {
        let Node::Element(_) = node.value() else {
            return false;
        };
        let element = ElementRef::wrap(node).expect("element node wraps");
        element.value().name() == "table"
            && element.value().attr("class").is_some_and(|classes| {
                classes
                    .split_ascii_whitespace()
                    .any(|class| class == "boxplot")
            })
    })
}

/// Restriction-digest scenes are an explicit pair of harvested ARIA-labelled figures.  Keeping
/// recognition this narrow prevents positioned ordinary tables from entering the scene parser.
fn is_restriction_digest_fragment(html: &str) -> bool {
    const LABELS: [&str; 2] = [
        "Linear restriction-digest DNA map.",
        "Circular restriction-digest DNA map.",
    ];

    let document = Html::parse_fragment(html);
    document.root_element().children().any(|node| {
        let Node::Element(_) = node.value() else {
            return false;
        };
        let element = ElementRef::wrap(node).expect("element node wraps");
        element.value().name() == "table"
            && element.value().attr("role") == Some("img")
            && element
                .value()
                .attr("aria-label")
                .is_some_and(|label| LABELS.contains(&label))
    })
}

fn canvas_dimensions(
    bounds: LayoutBox,
    device_scale_factor: u32,
) -> Result<(u32, u32), RasterError> {
    let scale = device_scale_factor as f32;
    let css_width = bounds.width / scale;
    let css_height = bounds.height / scale;
    if !css_width.is_finite() || !css_height.is_finite() || css_width <= 0.0 || css_height <= 0.0 {
        return Err(RasterError::InvalidGeometry {
            stage: "resolved table",
            css_width,
            css_height,
            device_scale_factor,
        });
    }
    let width = bounds.width.ceil();
    let height = bounds.height.ceil();
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return Err(RasterError::InvalidGeometry {
            stage: "PNG canvas",
            css_width,
            css_height,
            device_scale_factor,
        });
    }
    Ok((width as u32, height as u32))
}

#[allow(clippy::collapsible_if)]
#[path = "render_assembly.rs"]
mod assembly;
use assembly::{build_display_list, collect_direct_images, without_nested_tables};

struct ImageMetrics {
    dimensions: BTreeMap<String, (f32, f32)>,
}

impl ImageMetrics {
    fn collect(tree: &StyledTree) -> Result<Self, RasterError> {
        let mut dimensions = BTreeMap::new();
        collect_image_dimensions(&tree.root, &mut dimensions)?;
        Ok(Self { dimensions })
    }
}

fn collect_image_dimensions(
    node: &StyledNode,
    dimensions: &mut BTreeMap<String, (f32, f32)>,
) -> Result<(), RasterError> {
    if !node.style.visible {
        return Ok(());
    }
    if matches!(node.kind, StyledNodeKind::Element(ElementKind::Image)) {
        let source = node.attributes.get("src").expect("parser requires img src");
        let size = crate::paint::data_image_dimensions(source).map_err(|error| {
            RasterError::EmbeddedImage {
                reason: error.to_string(),
            }
        })?;
        dimensions.insert(source.clone(), size);
    }
    for child in &node.children {
        collect_image_dimensions(child, dimensions)?;
    }
    Ok(())
}

struct RenderMeasurer<'a> {
    images: &'a ImageMetrics,
}

impl RenderMeasurer<'_> {
    fn image_dimensions(&self, source: &str) -> Result<(f32, f32), RasterError> {
        self.images
            .dimensions
            .get(source)
            .copied()
            .ok_or_else(|| RasterError::EmbeddedImage {
                reason: "parser-approved image was not measured".to_owned(),
            })
    }

    fn image_sizes(&self, cell: &StyledNode, content_width: f32) -> (f32, f32) {
        let mut images = Vec::new();
        collect_direct_images(cell, &mut images);
        images
            .into_iter()
            .fold((0.0, 0.0), |(max_width, total_height), image| {
                let source = image
                    .attributes
                    .get("src")
                    .expect("parser requires img src");
                let (natural_width, natural_height) = self
                    .images
                    .dimensions
                    .get(source)
                    .copied()
                    .expect("images are validated before table layout");
                let width = natural_width.min(content_width.max(0.0));
                let height = if natural_width > content_width && natural_width > 0.0 {
                    natural_height * content_width / natural_width
                } else {
                    natural_height
                };
                (max_width.max(width), total_height + height.max(0.0))
            })
    }
}

impl CellMeasurer for RenderMeasurer<'_> {
    fn intrinsic_size(&self, cell: &StyledNode) -> IntrinsicSize {
        let text = crate::measure_inline(&without_nested_tables(cell));
        let (image_width, image_height) = self.image_sizes(cell, f32::MAX);
        IntrinsicSize {
            min_width: text.min_width.max(image_width),
            max_width: text.max_width.max(image_width),
            height: image_height,
        }
    }

    fn layout_height(&self, cell: &StyledNode, content_width: f32) -> f32 {
        let text = layout_inline(&without_nested_tables(cell), content_width).height;
        let (_, images) = self.image_sizes(cell, content_width);
        text + images
    }
}

fn scale_tree(tree: &StyledTree, scale: f32) -> StyledTree {
    StyledTree {
        root: scale_node(&tree.root, scale),
    }
}

#[allow(clippy::collapsible_if)]
fn scale_node(node: &StyledNode, scale: f32) -> StyledNode {
    let mut attributes = node.attributes.clone();
    for name in ["cellpadding", "cellspacing"] {
        if let Some(value) = attributes.get_mut(name) {
            if let Ok(number) = value.parse::<f32>() {
                *value = (number * scale).to_string();
            }
        }
    }
    StyledNode {
        kind: node.kind.clone(),
        style: scale_style(&node.style, scale),
        attributes,
        children: node
            .children
            .iter()
            .map(|child| scale_node(child, scale))
            .collect(),
        scene: node
            .scene
            .as_ref()
            .map(|scene| scale_scene_leaf(scene, scale)),
    }
}

fn scale_scene_leaf(scene: &crate::SceneLeaf, scale: f32) -> crate::SceneLeaf {
    crate::SceneLeaf {
        intrinsic_width: scene.intrinsic_width * scale,
        intrinsic_height: scene.intrinsic_height * scale,
        display_list: crate::DisplayList {
            bounds: scale_box(scene.display_list.bounds, scale),
            commands: scene
                .display_list
                .commands
                .iter()
                .cloned()
                .map(|command| scale_display_command(command, scale))
                .collect(),
        },
        text_groups: scene
            .text_groups
            .iter()
            .map(|group| crate::SceneText {
                node: scale_node(&group.node, scale),
                anchor: group.anchor,
                x_css: group.x_css * scale,
                y_css: group.y_css * scale,
            })
            .collect(),
        ..scene.clone()
    }
}

fn scale_display_command(command: DisplayCommand, scale: f32) -> DisplayCommand {
    match command {
        DisplayCommand::Fill { bounds, color } => DisplayCommand::Fill {
            bounds: scale_box(bounds, scale),
            color,
        },
        DisplayCommand::Border { bounds, style } => DisplayCommand::Border {
            bounds: scale_box(bounds, scale),
            style: scale_style(&style, scale),
        },
        DisplayCommand::Text {
            bounds,
            text,
            style,
        } => DisplayCommand::Text {
            bounds: scale_box(bounds, scale),
            text,
            style: scale_style(&style, scale),
        },
        DisplayCommand::PushClip { bounds, radius } => DisplayCommand::PushClip {
            bounds: scale_box(bounds, scale),
            radius: radius.map(|value| value * scale),
        },
        DisplayCommand::PopClip => DisplayCommand::PopClip,
        DisplayCommand::Image { bounds, data_url } => DisplayCommand::Image {
            bounds: scale_box(bounds, scale),
            data_url,
        },
        DisplayCommand::StrokeLine {
            from,
            to,
            width,
            color,
        } => DisplayCommand::StrokeLine {
            from: (from.0 * scale, from.1 * scale),
            to: (to.0 * scale, to.1 * scale),
            width: width * scale,
            color,
        },
        // Scene text is transported through `text_groups`, so a scene grammar must not put a
        // cached glyph run in its static display list.
        DisplayCommand::GlyphRun { run } => DisplayCommand::GlyphRun { run },
    }
}

fn scale_box(bounds: LayoutBox, scale: f32) -> LayoutBox {
    LayoutBox {
        x: bounds.x * scale,
        y: bounds.y * scale,
        width: bounds.width * scale,
        height: bounds.height * scale,
    }
}

fn scale_style(style: &ComputedStyle, scale: f32) -> ComputedStyle {
    let mut result = style.clone();
    result.font_size = scale_length(result.font_size, scale);
    result.line_height = result.line_height.map(|value| scale_length(value, scale));
    result.letter_spacing = scale_length(result.letter_spacing, scale);
    result.width = result.width.map(|value| scale_length(value, scale));
    result.min_width = result.min_width.map(|value| scale_length(value, scale));
    result.height = result.height.map(|value| scale_length(value, scale));
    result.max_width = result.max_width.map(|value| scale_length(value, scale));
    result.border_radius = result.border_radius.map(|value| scale_length(value, scale));
    result.padding = result.padding.map(|value| scale_length(value, scale));
    result.margin = result.margin.map(|value| scale_length(value, scale));
    result.border_spacing = scale_length(result.border_spacing, scale);
    result.relative_top = result.relative_top.map(|value| scale_length(value, scale));
    result.box_shadow = result.box_shadow.map(|shadow| crate::BoxShadow {
        x: scale_length(shadow.x, scale),
        y: scale_length(shadow.y, scale),
        blur: scale_length(shadow.blur, scale),
        spread: scale_length(shadow.spread, scale),
        ..shadow
    });
    result.border = Border {
        top: scale_border_side(result.border.top, scale),
        right: scale_border_side(result.border.right, scale),
        bottom: scale_border_side(result.border.bottom, scale),
        left: scale_border_side(result.border.left, scale),
    };
    result
}

fn scale_border_side(side: crate::BorderSide, scale: f32) -> crate::BorderSide {
    crate::BorderSide {
        width: scale_length(side.width, scale),
        ..side
    }
}

fn scale_length(length: CssLength, scale: f32) -> CssLength {
    match length {
        CssLength::Px(value) => CssLength::Px(value * scale),
        CssLength::Pt(value) => CssLength::Pt(value * scale),
        // Relative units resolve only after the inherited font size has reached the physical
        // scale. Scaling them here would multiply a unitless line-height twice.
        CssLength::Em(value) => CssLength::Em(value),
        CssLength::Percent(value) => CssLength::Percent(value),
        CssLength::Zero => CssLength::Zero,
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
