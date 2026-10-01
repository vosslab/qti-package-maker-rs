//! Raster painting and PNG encoding for the supported table display list.
//!
//! The painter receives resolved geometry only.  It does not parse HTML, fetch resources, or
//! decide line breaks; those boundaries belong to the subset, table-layout, and inline-layout
//! modules respectively.

use std::{
    io,
    time::{Duration, Instant},
};

use base64::Engine;
use cosmic_text::{Color as CosmicColor, SwashCache};
use thiserror::Error;
use tiny_skia::{
    FillRule, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, StrokeDash,
    Transform,
};

use crate::{
    BorderSide, BorderStyle, Color, ComputedStyle, CssLength, DisplayCommand, DisplayList,
    LayoutBox, TextRun,
};

const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const KAPPA: f32 = 0.552_284_8;

/// A paint or output encoding failure.
#[derive(Debug, Error)]
pub enum PaintError {
    /// The requested canvas cannot be represented safely by the raster backend.
    #[error("invalid raster canvas dimensions {width} by {height}")]
    InvalidCanvasDimensions { width: u32, height: u32 },
    /// The requested canvas would allocate too many pixels.
    #[error("raster canvas has {actual_pixels} pixels; maximum is {maximum_pixels}")]
    CanvasTooLarge {
        actual_pixels: u64,
        maximum_pixels: u64,
    },
    /// A data URL did not use one of the supported image forms.
    #[error("unsupported embedded image data URL")]
    UnsupportedDataImage,
    /// A base64 payload was malformed.
    #[error("embedded image base64 payload is malformed")]
    InvalidBase64(#[source] base64::DecodeError),
    /// A percent-encoded SVG data URL has an incomplete or non-hex escape.
    #[error("embedded SVG percent escape is malformed")]
    InvalidPercentEncoding,
    /// Percent decoding yielded bytes that are not valid UTF-8 SVG markup.
    #[error("embedded SVG data is not UTF-8")]
    InvalidSvgUtf8(#[source] std::str::Utf8Error),
    /// PNG image decoding failed.
    #[error("embedded PNG could not be decoded")]
    PngDecode(#[source] png::DecodingError),
    /// SVG image parsing or rendering failed.
    #[error("embedded SVG could not be decoded: {0}")]
    SvgDecode(String),
    /// PNG encoding failed.
    #[error("PNG encoding failed")]
    PngEncode(#[source] png::EncodingError),
    /// Writing the in-memory PNG buffer failed.
    #[error("PNG output buffer write failed")]
    PngWrite(#[source] io::Error),
}

/// PNG bytes and the two painter stages measured by the native raster backend.
#[derive(Debug)]
pub struct PaintedPng {
    /// Fully encoded in-memory PNG bytes.
    pub bytes: Vec<u8>,
    /// Time spent creating the pixmap and painting resolved display commands.
    pub paint_duration: Duration,
    /// Time spent encoding the completed pixmap as PNG.
    pub encode_duration: Duration,
}

/// Paints the display list into a white, table-bounded PNG.
///
/// `width` and `height` are output pixels.  The caller supplies the table's border box through
/// the display-list bounds; no command may paint outside that box, even when its own geometry
/// extends beyond it.
pub fn paint_display_list(
    display_list: &DisplayList,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, PaintError> {
    Ok(paint_display_list_with_metrics(display_list, width, height)?.bytes)
}

/// Paints a resolved display list and returns separate paint and PNG-encoding durations.
pub fn paint_display_list_with_metrics(
    display_list: &DisplayList,
    width: u32,
    height: u32,
) -> Result<PaintedPng, PaintError> {
    validate_canvas(width, height)?;
    let paint_started = Instant::now();
    let mut painter = Painter::new(width, height)?;
    painter.fill_white();
    painter.push_clip(display_list.bounds, None)?;

    for command in &display_list.commands {
        painter.paint_command(command)?;
    }
    let paint_duration = paint_started.elapsed();
    let encode_started = Instant::now();
    let bytes = painter.encode_png()?;
    Ok(PaintedPng {
        bytes,
        paint_duration,
        encode_duration: encode_started.elapsed(),
    })
}

fn validate_canvas(width: u32, height: u32) -> Result<(), PaintError> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 {
        return Err(PaintError::InvalidCanvasDimensions { width, height });
    }
    if pixels > MAX_PIXELS {
        return Err(PaintError::CanvasTooLarge {
            actual_pixels: pixels,
            maximum_pixels: MAX_PIXELS,
        });
    }
    Ok(())
}

struct Painter {
    pixmap: Pixmap,
    clips: Vec<Mask>,
    table_clip_depth: usize,
    glyph_cache: SwashCache,
}

impl Painter {
    fn new(width: u32, height: u32) -> Result<Self, PaintError> {
        let pixmap = Pixmap::new(width, height)
            .ok_or(PaintError::InvalidCanvasDimensions { width, height })?;
        Ok(Self {
            pixmap,
            clips: Vec::new(),
            table_clip_depth: 0,
            glyph_cache: SwashCache::new(),
        })
    }

    fn fill_white(&mut self) {
        self.pixmap.fill(tiny_skia::Color::WHITE);
    }

    fn paint_command(&mut self, command: &DisplayCommand) -> Result<(), PaintError> {
        match command {
            DisplayCommand::Fill { bounds, color } => self.fill(*bounds, *color, None),
            DisplayCommand::Border { bounds, style } => self.border(*bounds, style),
            // WP-R3 replaces this compatibility command with positioned shaped glyph runs.  It
            // remains intentionally inert here: drawing Unicode as unshaped boxes would make a
            // plausible but incorrect renderer.  Glyph runs are painted through the R3 command.
            DisplayCommand::Text {
                bounds,
                text,
                style,
            } => self.text(*bounds, text, style),
            DisplayCommand::PushClip { bounds, radius } => self.push_clip(*bounds, *radius),
            DisplayCommand::PopClip => {
                if self.clips.len() > self.table_clip_depth {
                    self.clips.pop();
                }
                Ok(())
            }
            DisplayCommand::Image { bounds, data_url } => self.image(*bounds, data_url),
            DisplayCommand::StrokeLine {
                from,
                to,
                width,
                color,
            } => self.stroke_line(*from, *to, *width, *color),
            DisplayCommand::GlyphRun { run } => self.glyph_run(run),
        }
    }

    fn clip(&self) -> Option<&Mask> {
        self.clips.last()
    }

    fn push_clip(&mut self, bounds: LayoutBox, radius: Option<f32>) -> Result<(), PaintError> {
        let radius = radius.unwrap_or(0.0);
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height()).ok_or(
            PaintError::InvalidCanvasDimensions {
                width: self.pixmap.width(),
                height: self.pixmap.height(),
            },
        )?;
        let path = rounded_rect_path(bounds, radius);
        mask.fill_path(&path, FillRule::Winding, true, Transform::identity());

        if let Some(parent) = self.clips.last() {
            for (destination, source) in mask.data_mut().iter_mut().zip(parent.data()) {
                *destination = u8::try_from((u16::from(*destination) * u16::from(*source)) / 255)
                    .expect("mask multiplication is bounded by 255");
            }
        }
        self.clips.push(mask);
        if self.table_clip_depth == 0 {
            self.table_clip_depth = self.clips.len();
        }
        Ok(())
    }

    fn glyph_run(&mut self, run: &TextRun) -> Result<(), PaintError> {
        let width = self.pixmap.width() as i32;
        let height = self.pixmap.height() as i32;
        let clip = self.clip().map(|mask| mask.data().to_vec());
        let pixels = self.pixmap.data_mut();
        crate::fonts::with_font_system(|font_system| {
            let base = CosmicColor::rgba(run.color.0, run.color.1, run.color.2, run.color.3);
            for glyph in &run.glyphs {
                self.glyph_cache.with_pixels(
                    font_system,
                    glyph.cache_key,
                    base,
                    |offset_x, offset_y, color| {
                        let y = glyph.y + offset_y;
                        if y < 0 || y >= height {
                            return;
                        }
                        let [red, green, blue, alpha] = color.as_rgba();
                        let start = (offset_x as f32 * run.scale_x).floor() as i32;
                        let end = ((offset_x + 1) as f32 * run.scale_x).ceil() as i32;
                        for x in glyph.x + start..glyph.x + end.max(start + 1) {
                            if x < 0 || x >= width {
                                continue;
                            }
                            let index = (y as usize * width as usize + x as usize) * 4;
                            let mut pixel_alpha = alpha;
                            if let Some(clip) = &clip {
                                pixel_alpha = ((u16::from(pixel_alpha)
                                    * u16::from(clip[y as usize * width as usize + x as usize]))
                                    / 255) as u8;
                            }
                            blend_over(
                                &mut pixels[index..index + 4],
                                red,
                                green,
                                blue,
                                pixel_alpha,
                            );
                        }
                    },
                );
            }
        });
        Ok(())
    }

    fn text(
        &mut self,
        bounds: LayoutBox,
        text: &str,
        style: &ComputedStyle,
    ) -> Result<(), PaintError> {
        let node = crate::StyledNode {
            kind: crate::StyledNodeKind::Text(text.to_owned()),
            style: style.clone(),
            attributes: std::collections::BTreeMap::new(),
            children: Vec::new(),
            scene: None,
        };
        let mut layout = crate::layout_inline(&node, bounds.width.max(0.0));
        crate::place_inline(&mut layout, bounds, crate::VerticalAlign::Top);
        for run in &layout.runs {
            self.glyph_run(run)?;
        }
        Ok(())
    }

    fn fill(
        &mut self,
        bounds: LayoutBox,
        color: Color,
        radius: Option<CssLength>,
    ) -> Result<(), PaintError> {
        let radius = radius
            .map(|value| resolve_radius(value, bounds))
            .unwrap_or(0.0);
        let path = rounded_rect_path(bounds, radius);
        let clip = self.clips.last().cloned();
        self.pixmap.fill_path(
            &path,
            &paint(color),
            FillRule::Winding,
            Transform::identity(),
            clip.as_ref(),
        );
        Ok(())
    }

    fn border(&mut self, bounds: LayoutBox, style: &ComputedStyle) -> Result<(), PaintError> {
        if let Some(shadow) = style.box_shadow {
            self.shadow(bounds, shadow, style.border_radius)?;
        }
        if let Some(background) = style.background {
            self.fill(bounds, background, style.border_radius)?;
        }
        let top = resolve_width(style.border.top.width, bounds.width);
        let right = resolve_width(style.border.right.width, bounds.width);
        let bottom = resolve_width(style.border.bottom.width, bounds.width);
        let left = resolve_width(style.border.left.width, bounds.width);
        if let Some(radius) = style
            .border_radius
            .map(|value| resolve_radius(value, bounds))
            && top > 0.0
            && (top - right).abs() < f32::EPSILON
            && (top - bottom).abs() < f32::EPSILON
            && (top - left).abs() < f32::EPSILON
            && style.border.top == style.border.right
            && style.border.top == style.border.bottom
            && style.border.top == style.border.left
            && matches!(style.border.top.style, BorderStyle::Solid)
        {
            let path = rounded_rect_path(bounds, radius);
            return self.paint_single_stroke(&path, top, style.border.top.color, None);
        }
        self.paint_side(
            bounds.x,
            bounds.y,
            bounds.x + bounds.width,
            bounds.y,
            top,
            &style.border.top,
        )?;
        self.paint_side(
            bounds.x + bounds.width,
            bounds.y,
            bounds.x + bounds.width,
            bounds.y + bounds.height,
            right,
            &style.border.right,
        )?;
        self.paint_side(
            bounds.x + bounds.width,
            bounds.y + bounds.height,
            bounds.x,
            bounds.y + bounds.height,
            bottom,
            &style.border.bottom,
        )?;
        self.paint_side(
            bounds.x,
            bounds.y + bounds.height,
            bounds.x,
            bounds.y,
            left,
            &style.border.left,
        )?;
        Ok(())
    }

    fn shadow(
        &mut self,
        bounds: LayoutBox,
        shadow: crate::BoxShadow,
        radius: Option<CssLength>,
    ) -> Result<(), PaintError> {
        let spread = resolve_width(shadow.spread, bounds.width);
        let shifted = LayoutBox {
            x: bounds.x + resolve_width(shadow.x, bounds.width) - spread,
            y: bounds.y + resolve_width(shadow.y, bounds.height) - spread,
            width: (bounds.width + 2.0 * spread).max(0.0),
            height: (bounds.height + 2.0 * spread).max(0.0),
        };
        let radius = radius
            .map(|value| resolve_radius(value, bounds) + spread)
            .unwrap_or(spread);
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height()).ok_or(
            PaintError::InvalidCanvasDimensions {
                width: self.pixmap.width(),
                height: self.pixmap.height(),
            },
        )?;
        mask.fill_path(
            &rounded_rect_path(shifted, radius),
            FillRule::Winding,
            true,
            Transform::identity(),
        );
        blur_mask(
            &mut mask,
            resolve_width(shadow.blur, bounds.width).ceil() as usize,
        );
        if let Some(clip) = self.clip() {
            for (alpha, clip_alpha) in mask.data_mut().iter_mut().zip(clip.data()) {
                *alpha = ((u16::from(*alpha) * u16::from(*clip_alpha)) / 255) as u8;
            }
        }
        self.pixmap.fill_path(
            &PathBuilder::from_rect(rect(LayoutBox {
                x: 0.0,
                y: 0.0,
                width: self.pixmap.width() as f32,
                height: self.pixmap.height() as f32,
            })),
            &paint(shadow.color),
            FillRule::Winding,
            Transform::identity(),
            Some(&mask),
        );
        Ok(())
    }

    fn paint_side(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        side: &BorderSide,
    ) -> Result<(), PaintError> {
        if width <= 0.0 || matches!(side.style, BorderStyle::None | BorderStyle::Hidden) {
            return Ok(());
        }
        let mut path = PathBuilder::new();
        path.move_to(x1, y1);
        path.line_to(x2, y2);
        let path = path.finish().expect("a two-point border path is valid");
        let mut stroke = Stroke {
            width,
            ..Stroke::default()
        };
        match side.style {
            BorderStyle::Dashed => {
                stroke.dash = StrokeDash::new(vec![3.0 * width, 2.0 * width], 0.0)
            }
            BorderStyle::Dotted => {
                stroke.dash = StrokeDash::new(vec![width, width], 0.0);
                stroke.line_cap = tiny_skia::LineCap::Round;
            }
            BorderStyle::Double => {
                let line_width = width / 3.0;
                self.paint_single_stroke(&path, line_width, side.color, None)?;
                let offset = width / 3.0;
                let (dx, dy) = if (x1 - x2).abs() < f32::EPSILON {
                    (offset, 0.0)
                } else {
                    (0.0, offset)
                };
                let mut parallel = PathBuilder::new();
                parallel.move_to(x1 + dx, y1 + dy);
                parallel.line_to(x2 + dx, y2 + dy);
                let parallel = parallel.finish().expect("a two-point border path is valid");
                return self.paint_single_stroke(&parallel, line_width, side.color, None);
            }
            BorderStyle::None | BorderStyle::Hidden | BorderStyle::Solid => {}
        }
        let mut paint = paint(side.color);
        paint.anti_alias = true;
        let clip = self.clips.last().cloned();
        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), clip.as_ref());
        Ok(())
    }

    fn stroke_line(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        color: Color,
    ) -> Result<(), PaintError> {
        if !width.is_finite()
            || width <= 0.0
            || !from.0.is_finite()
            || !from.1.is_finite()
            || !to.0.is_finite()
            || !to.1.is_finite()
        {
            return Ok(());
        }
        let mut builder = PathBuilder::new();
        builder.move_to(from.0, from.1);
        builder.line_to(to.0, to.1);
        let path = builder.finish().expect("a finite two-point line is valid");
        self.paint_single_stroke(&path, width, color, None)
    }

    fn paint_single_stroke(
        &mut self,
        path: &Path,
        width: f32,
        color: Color,
        dash: Option<StrokeDash>,
    ) -> Result<(), PaintError> {
        let stroke = Stroke {
            width,
            dash,
            ..Stroke::default()
        };
        let clip = self.clips.last().cloned();
        self.pixmap.stroke_path(
            path,
            &paint(color),
            &stroke,
            Transform::identity(),
            clip.as_ref(),
        );
        Ok(())
    }

    fn image(&mut self, bounds: LayoutBox, data_url: &str) -> Result<(), PaintError> {
        let image = decode_data_image(data_url)?;
        let source_width = image.width();
        let source_height = image.height();
        if source_width == 0 || source_height == 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return Ok(());
        }
        let transform = Transform::from_scale(
            bounds.width / source_width as f32,
            bounds.height / source_height as f32,
        )
        .post_translate(bounds.x, bounds.y);
        let pixmap_paint = PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..PixmapPaint::default()
        };
        let clip = self.clips.last().cloned();
        self.pixmap.draw_pixmap(
            0,
            0,
            image.as_ref(),
            &pixmap_paint,
            transform,
            clip.as_ref(),
        );
        Ok(())
    }

    fn encode_png(&self) -> Result<Vec<u8>, PaintError> {
        let mut output = Vec::new();
        {
            let mut encoder =
                png::Encoder::new(&mut output, self.pixmap.width(), self.pixmap.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            let mut writer = encoder.write_header().map_err(PaintError::PngEncode)?;
            writer
                .write_image_data(self.pixmap.data())
                .map_err(PaintError::PngEncode)?;
        }
        Ok(output)
    }
}

fn paint(color: Color) -> Paint<'static> {
    let mut output = Paint::default();
    output.set_color_rgba8(color.0, color.1, color.2, color.3);
    output.anti_alias = true;
    output
}

/// Composites a straight-alpha Cosmic Text sample over tiny-skia's premultiplied RGBA pixel.
fn blend_over(destination: &mut [u8], red: u8, green: u8, blue: u8, alpha: u8) {
    debug_assert_eq!(destination.len(), 4);
    let source_alpha = u16::from(alpha);
    let destination_alpha = u16::from(destination[3]);
    let inverse = 255 - source_alpha;
    for (index, source) in [red, green, blue].into_iter().enumerate() {
        destination[index] = ((u16::from(source) * source_alpha
            + u16::from(destination[index]) * inverse)
            / 255) as u8;
    }
    destination[3] = (source_alpha + (destination_alpha * inverse) / 255) as u8;
}

fn resolve_width(length: CssLength, available: f32) -> f32 {
    match length {
        CssLength::Px(value) => value,
        CssLength::Pt(value) => value * (96.0 / 72.0),
        CssLength::Em(value) => value * 16.0,
        CssLength::Percent(value) => available * value / 100.0,
        CssLength::Zero => 0.0,
    }
    .max(0.0)
}

fn resolve_radius(length: CssLength, bounds: LayoutBox) -> f32 {
    resolve_width(length, bounds.width.min(bounds.height))
        .min(bounds.width.min(bounds.height) / 2.0)
}

/// Applies a separable box blur to a bounded 8-bit paint mask.
///
/// The source grammar admits only one shadow; keeping the blur here prevents a general filter
/// language and makes cost linear in the raster size rather than in blur radius times pixels.
fn blur_mask(mask: &mut Mask, radius: usize) {
    if radius == 0 {
        return;
    }
    let width = mask.width() as usize;
    let height = mask.height() as usize;
    let radius = radius.min(width.max(height));
    let mut horizontal = vec![0_u8; width * height];
    for y in 0..height {
        let row = &mask.data()[y * width..(y + 1) * width];
        let mut prefix = vec![0_u32; width + 1];
        for x in 0..width {
            prefix[x + 1] = prefix[x] + u32::from(row[x]);
        }
        for x in 0..width {
            let start = x.saturating_sub(radius);
            let end = x.saturating_add(radius).min(width - 1) + 1;
            horizontal[y * width + x] =
                ((prefix[end] - prefix[start]) / (end - start) as u32) as u8;
        }
    }
    for x in 0..width {
        let mut prefix = vec![0_u32; height + 1];
        for y in 0..height {
            prefix[y + 1] = prefix[y] + u32::from(horizontal[y * width + x]);
        }
        for y in 0..height {
            let start = y.saturating_sub(radius);
            let end = y.saturating_add(radius).min(height - 1) + 1;
            mask.data_mut()[y * width + x] =
                ((prefix[end] - prefix[start]) / (end - start) as u32) as u8;
        }
    }
}

fn rounded_rect_path(bounds: LayoutBox, radius: f32) -> Path {
    let radius = radius.max(0.0).min(bounds.width.min(bounds.height) / 2.0);
    if radius <= f32::EPSILON {
        return PathBuilder::from_rect(rect(bounds));
    }
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    let curve = radius * KAPPA;
    let mut builder = PathBuilder::new();
    builder.move_to(bounds.x + radius, bounds.y);
    builder.line_to(right - radius, bounds.y);
    builder.cubic_to(
        right - radius + curve,
        bounds.y,
        right,
        bounds.y + radius - curve,
        right,
        bounds.y + radius,
    );
    builder.line_to(right, bottom - radius);
    builder.cubic_to(
        right,
        bottom - radius + curve,
        right - radius + curve,
        bottom,
        right - radius,
        bottom,
    );
    builder.line_to(bounds.x + radius, bottom);
    builder.cubic_to(
        bounds.x + radius - curve,
        bottom,
        bounds.x,
        bottom - radius + curve,
        bounds.x,
        bottom - radius,
    );
    builder.line_to(bounds.x, bounds.y + radius);
    builder.cubic_to(
        bounds.x,
        bounds.y + radius - curve,
        bounds.x + radius - curve,
        bounds.y,
        bounds.x + radius,
        bounds.y,
    );
    builder.close();
    builder.finish().expect("a rounded rectangle path is valid")
}

fn rect(bounds: LayoutBox) -> Rect {
    Rect::from_xywh(
        bounds.x,
        bounds.y,
        bounds.width.max(0.001),
        bounds.height.max(0.001),
    )
    .expect("finite, positive layout boxes reach the painter")
}

fn decode_data_image(data_url: &str) -> Result<Pixmap, PaintError> {
    if let Some(encoded) = data_url.strip_prefix("data:image/png;base64,") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(PaintError::InvalidBase64)?;
        validate_png_dimensions(&bytes)?;
        return Pixmap::decode_png(&bytes).map_err(PaintError::PngDecode);
    }
    let svg = if let Some(encoded) = data_url.strip_prefix("data:image/svg+xml;base64,") {
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(PaintError::InvalidBase64)?
    } else if let Some(encoded) = data_url.strip_prefix("data:image/svg+xml,") {
        percent_decode(encoded)?
    } else {
        return Err(PaintError::UnsupportedDataImage);
    };
    render_svg(&svg)
}

/// Returns the natural pixel dimensions of a parser-approved embedded image.
pub(crate) fn data_image_dimensions(data_url: &str) -> Result<(f32, f32), PaintError> {
    let image = decode_data_image(data_url)?;
    Ok((image.width() as f32, image.height() as f32))
}

fn percent_decode(input: &str) -> Result<Vec<u8>, PaintError> {
    let mut output = Vec::with_capacity(input.len());
    let mut bytes = input.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes.next();
            let low = bytes.next();
            let (Some(high), Some(low)) = (high.and_then(hex), low.and_then(hex)) else {
                return Err(PaintError::InvalidPercentEncoding);
            };
            output.push((high << 4) | low);
        } else {
            output.push(byte);
        }
    }
    Ok(output)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn render_svg(bytes: &[u8]) -> Result<Pixmap, PaintError> {
    std::str::from_utf8(bytes).map_err(PaintError::InvalidSvgUtf8)?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|error| PaintError::SvgDecode(error.to_string()))?;
    let size = tree.size().to_int_size();
    if u64::from(size.width()) * u64::from(size.height()) > MAX_PIXELS {
        return Err(PaintError::CanvasTooLarge {
            actual_pixels: u64::from(size.width()) * u64::from(size.height()),
            maximum_pixels: MAX_PIXELS,
        });
    }
    let mut pixmap =
        Pixmap::new(size.width(), size.height()).ok_or(PaintError::InvalidCanvasDimensions {
            width: size.width(),
            height: size.height(),
        })?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    Ok(pixmap)
}

fn validate_png_dimensions(bytes: &[u8]) -> Result<(), PaintError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let reader = decoder.read_info().map_err(PaintError::PngDecode)?;
    let info = reader.info();
    let pixels = u64::from(info.width) * u64::from(info.height);
    if pixels > MAX_PIXELS {
        return Err(PaintError::CanvasTooLarge {
            actual_pixels: pixels,
            maximum_pixels: MAX_PIXELS,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "paint_tests.rs"]
mod paint_tests;
