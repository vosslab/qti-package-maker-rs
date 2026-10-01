use super::*;
use crate::Border;

fn style() -> ComputedStyle {
    let mut style = ComputedStyle::ua_defaults("span", None);
    style.border = Border {
        top: BorderSide {
            width: CssLength::Px(2.0),
            style: BorderStyle::Dashed,
            color: Color(0, 0, 0, 255),
        },
        right: BorderSide {
            width: CssLength::Px(2.0),
            style: BorderStyle::Dotted,
            color: Color(0, 0, 0, 255),
        },
        bottom: BorderSide {
            width: CssLength::Px(3.0),
            style: BorderStyle::Double,
            color: Color(0, 0, 0, 255),
        },
        left: BorderSide {
            width: CssLength::Px(2.0),
            style: BorderStyle::Solid,
            color: Color(0, 0, 0, 255),
        },
    };
    style.border_radius = Some(CssLength::Percent(50.0));
    style.border_spacing = CssLength::Zero;
    style
}

#[test]
fn colored_background_is_encoded_as_png() {
    let bounds = LayoutBox {
        x: 2.0,
        y: 2.0,
        width: 12.0,
        height: 12.0,
    };
    let list = DisplayList {
        bounds: LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        },
        commands: vec![DisplayCommand::Fill {
            bounds,
            color: Color(240, 12, 23, 255),
        }],
    };
    let png = paint_display_list(&list, 16, 16).unwrap();
    let image = Pixmap::decode_png(&png).unwrap();
    let pixel = &image.data()[(8 * 16 + 8) * 4..(8 * 16 + 9) * 4];
    assert_eq!(pixel, &[240, 12, 23, 255]);
}

#[test]
fn invalid_canvas_is_rejected_before_allocation() {
    let list = DisplayList::default();
    assert!(matches!(
        paint_display_list(&list, 0, 1),
        Err(PaintError::InvalidCanvasDimensions { .. })
    ));
    assert!(matches!(
        paint_display_list(&list, 16_384, 16_384),
        Err(PaintError::CanvasTooLarge { .. })
    ));
}

#[test]
fn border_styles_and_percent_radius_paint() {
    let bounds = LayoutBox {
        x: 2.0,
        y: 2.0,
        width: 20.0,
        height: 20.0,
    };
    let list = DisplayList {
        bounds: LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 24.0,
            height: 24.0,
        },
        commands: vec![DisplayCommand::Border {
            bounds,
            style: style(),
        }],
    };
    assert!(
        paint_display_list(&list, 24, 24)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}

#[test]
fn data_url_png_is_decoded_without_network_access() {
    let source = Pixmap::new(1, 1).unwrap();
    let encoded = source.encode_png().unwrap();
    let url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(encoded)
    );
    assert_eq!(decode_data_image(&url).unwrap().width(), 1);
}

#[test]
fn svg_data_url_is_rendered_at_the_requested_bounds() {
    let list = DisplayList {
            bounds: LayoutBox {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
            },
            commands: vec![DisplayCommand::Image {
                bounds: LayoutBox {
                    x: 2.0,
                    y: 2.0,
                    width: 4.0,
                    height: 4.0,
                },
                data_url: "data:image/svg+xml,%3Csvg%20xmlns%3D%27http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%27%20width%3D%272%27%20height%3D%272%27%3E%3Crect%20width%3D%272%27%20height%3D%272%27%20fill%3D%27%2300ff00%27%2F%3E%3C%2Fsvg%3E".to_owned(),
            }],
        };
    let png = paint_display_list(&list, 8, 8).unwrap();
    let image = Pixmap::decode_png(&png).unwrap();
    let pixel = &image.data()[(3 * 8 + 3) * 4..(3 * 8 + 4) * 4];
    assert!(pixel[1] > 200 && pixel[0] < 30 && pixel[2] < 30);
}

#[test]
fn outer_table_clip_cannot_be_popped_by_a_display_command() {
    let list = DisplayList {
        bounds: LayoutBox {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        },
        commands: vec![
            DisplayCommand::PopClip,
            DisplayCommand::Fill {
                bounds: LayoutBox {
                    x: 0.0,
                    y: 0.0,
                    width: 8.0,
                    height: 8.0,
                },
                color: Color(240, 12, 23, 255),
            },
        ],
    };
    let png = paint_display_list(&list, 8, 8).unwrap();
    let image = Pixmap::decode_png(&png).unwrap();
    assert_eq!(&image.data()[0..4], &[255, 255, 255, 255]);
    assert_eq!(
        &image.data()[(3 * 8 + 3) * 4..(3 * 8 + 4) * 4],
        &[240, 12, 23, 255]
    );
}

#[test]
fn shaped_glyph_run_is_rasterized_with_its_bundled_font_cache_key() {
    let tree = crate::parse_fragment("<table><tr><td>text</td></tr></table>").unwrap();
    let cell = &tree.root.children[0].children[0];
    let layout = crate::layout_inline(cell, 64.0);
    let list = DisplayList {
        bounds: LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 64.0,
            height: 32.0,
        },
        commands: layout
            .runs
            .into_iter()
            .map(|run| DisplayCommand::GlyphRun { run })
            .collect(),
    };
    let png = paint_display_list(&list, 64, 32).unwrap();
    let image = Pixmap::decode_png(&png).unwrap();
    let (pixels, remainder) = image.data().as_chunks::<4>();
    assert!(remainder.is_empty());
    assert!(pixels.iter().any(|pixel| pixel[0] < 200));
}

#[test]
fn scene_lines_and_bounded_box_shadows_paint() {
    let mut shadowed = ComputedStyle::ua_defaults("span", None);
    shadowed.background = Some(Color(220, 234, 245, 255));
    shadowed.box_shadow = Some(crate::BoxShadow {
        x: CssLength::Px(2.0),
        y: CssLength::Px(2.0),
        blur: CssLength::Px(2.0),
        spread: CssLength::Zero,
        color: Color(0, 0, 0, 160),
    });
    let list = DisplayList {
        bounds: LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 24.0,
            height: 24.0,
        },
        commands: vec![
            DisplayCommand::Border {
                bounds: LayoutBox {
                    x: 6.0,
                    y: 6.0,
                    width: 8.0,
                    height: 8.0,
                },
                style: shadowed,
            },
            DisplayCommand::StrokeLine {
                from: (2.0, 20.0),
                to: (22.0, 20.0),
                width: 2.0,
                color: Color(183, 67, 0, 255),
            },
        ],
    };
    let image = Pixmap::decode_png(&paint_display_list(&list, 24, 24).unwrap()).unwrap();
    assert!(
        image.data()[(17 * 24 + 16) * 4] < 255,
        "shadow darkens its exterior"
    );
    assert!(image.data()[(20 * 24 + 12) * 4] > 120, "line has red paint");
}

#[test]
fn hidden_border_style_emits_no_stroke() {
    let mut hidden = ComputedStyle::ua_defaults("span", None);
    hidden.border.top = BorderSide {
        width: CssLength::Px(4.0),
        style: BorderStyle::Hidden,
        color: Color(240, 12, 23, 255),
    };
    let list = DisplayList {
        bounds: LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 12.0,
            height: 12.0,
        },
        commands: vec![DisplayCommand::Border {
            bounds: LayoutBox {
                x: 2.0,
                y: 2.0,
                width: 8.0,
                height: 8.0,
            },
            style: hidden,
        }],
    };
    let image = Pixmap::decode_png(&paint_display_list(&list, 12, 12).unwrap()).unwrap();
    assert_eq!(&image.data()[(2 * 12 + 6) * 4..(2 * 12 + 7) * 4], &[255; 4]);
}
