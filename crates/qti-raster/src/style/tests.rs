use super::*;

#[test]
fn box_values_apply_to_each_border_side() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    apply_inline(
            &mut style,
            "border: 1px solid black; border-width: 2px 3px; border-style: dashed dotted; border-color: red blue",
            "<td>",
        )
        .unwrap();
    assert_eq!(style.border.top.width, CssLength::Px(2.0));
    assert_eq!(style.border.right.width, CssLength::Px(3.0));
    assert_eq!(style.border.bottom.style, BorderStyle::Dashed);
    assert_eq!(style.border.left.style, BorderStyle::Dotted);
    assert_eq!(style.border.top.color, Color(255, 0, 0, 255));
    assert_eq!(style.border.right.color, Color(0, 0, 255, 255));
}

#[test]
fn border_shorthand_accepts_css_component_order() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    apply_inline(&mut style, "border: solid white 0px", "<td>").unwrap();
    assert_eq!(style.border.top.width, CssLength::Px(0.0));
    assert_eq!(style.border.top.style, BorderStyle::Solid);
    assert_eq!(style.border.top.color, Color(255, 255, 255, 255));
}

#[test]
fn box_values_do_not_inherit_from_parent() {
    let mut parent = ComputedStyle::ua_defaults("table", None);
    parent.background = Some(Color(1, 2, 3, 255));
    parent.width = Some(CssLength::Px(300.0));
    parent.border.top.width = CssLength::Px(4.0);
    let child = ComputedStyle::ua_defaults("td", Some(&parent));
    assert_eq!(child.color, parent.color);
    assert_eq!(child.background, None);
    assert_eq!(child.width, None);
    assert_eq!(child.border.top.width, CssLength::Zero);
}

#[test]
fn explicit_zero_padding_is_distinct_from_an_omitted_declaration() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    assert_eq!(style.padding_declared, [false; 4]);
    apply_inline(&mut style, "padding: 0", "<td>").unwrap();
    assert_eq!(style.padding, [CssLength::Zero; 4]);
    assert_eq!(style.padding_declared, [true; 4]);
}

#[test]
fn rgb_and_rgba_require_bounded_numeric_channels() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    apply_inline(
        &mut style,
        "color:rgb(12, 34, 56); background-color:rgba(7,8,9,0.5)",
        "<td>",
    )
    .unwrap();
    assert_eq!(style.color, Color(12, 34, 56, 255));
    assert_eq!(style.background, Some(Color(7, 8, 9, 128)));
    for declaration in [
        "color:rgb(256,0,0)",
        "color:rgb(-1,0,0)",
        "color:rgba(1,2,3,1.1)",
        "color:rgba(1,2,3,-0.1)",
        "color:rgb(1,2)",
    ] {
        assert!(apply_inline(&mut style, declaration, "<td>").is_err());
    }
}

#[test]
fn compatibility_noops_are_limited_to_the_ruling_receipts() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    style.color = Color(1, 2, 3, 255);
    apply_inline(
            &mut style,
            "color:ff0303; color:ff9000; color:b9e710; color:1c7d72; color:6d1685; vert-align:middle; spacing:20px",
            "<td>",
        )
        .unwrap();
    assert_eq!(style.color, Color(1, 2, 3, 255));
    assert!(apply_inline(&mut style, "colro:red", "<td>").is_err());
    assert!(apply_inline(&mut style, "color:bad", "<td>").is_err());
    assert!(apply_inline(&mut style, "spacing:12px", "<td>").is_err());
}

#[test]
fn unitless_line_height_is_font_relative() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    apply_inline(&mut style, "line-height:1.2", "<td>").unwrap();
    assert_eq!(style.line_height, Some(CssLength::Em(1.2)));
}

#[test]
fn visibility_is_inherited_but_can_be_reenabled() {
    let mut parent = ComputedStyle::ua_defaults("span", None);
    apply_inline(&mut parent, "visibility:hidden", "<span>").unwrap();
    let mut child = ComputedStyle::ua_defaults("span", Some(&parent));
    assert!(!child.visible);
    apply_inline(&mut child, "visibility:visible", "<span>").unwrap();
    assert!(child.visible);
}

#[test]
fn parses_only_the_published_spacing_shadow_and_relative_grammar() {
    let mut style = ComputedStyle::ua_defaults("table", None);
    apply_inline(
            &mut style,
            "display:inline-table; position:relative; top:-.2em; letter-spacing:2px; box-shadow:0 0 6px 1px #6495ed",
            "<table>",
        )
        .unwrap();
    assert_eq!(style.letter_spacing, CssLength::Px(2.0));
    assert_eq!(style.relative_top, Some(CssLength::Em(-0.2)));
    assert_eq!(
        style.box_shadow,
        Some(BoxShadow {
            x: ZERO,
            y: ZERO,
            blur: CssLength::Px(6.0),
            spread: CssLength::Px(1.0),
            color: Color(100, 149, 237, 255),
        })
    );
    assert!(validate_general_position(&style, "table", "<table>").is_ok());
    for declaration in [
        "letter-spacing:-1px",
        "letter-spacing:1em",
        "box-shadow:inset 0 0 1px 0 black",
        "box-shadow:0 0 1px 0 black, 0 0 1px 0 red",
        "top:2px",
    ] {
        assert!(
            apply_inline(
                &mut ComputedStyle::ua_defaults("td", None),
                declaration,
                "<td>"
            )
            .is_err()
        );
    }
}

#[test]
fn box_shadow_defaults_an_omitted_spread_to_zero() {
    let mut style = ComputedStyle::ua_defaults("div", None);
    apply_inline(&mut style, "box-shadow:0 0 2px #99dbfb", "<div style>").unwrap();
    assert_eq!(
        style.box_shadow,
        Some(BoxShadow {
            x: ZERO,
            y: ZERO,
            blur: CssLength::Px(2.0),
            spread: ZERO,
            color: Color(153, 219, 251, 255),
        })
    );
}

#[test]
fn legacy_bgcolor_accepts_a_six_digit_value_without_hash() {
    let mut style = ComputedStyle::ua_defaults("td", None);
    apply_presentational(&mut style, "bgcolor", "874e18", "<td>").unwrap();
    assert_eq!(style.background, Some(Color(135, 78, 24, 255)));
}

#[test]
fn records_only_the_bound_hidden_border_auto_margin_and_bracket_scale_extensions() {
    let mut table = ComputedStyle::ua_defaults("table", None);
    apply_inline(&mut table, "border-style:hidden; margin:0 auto", "<table>").unwrap();
    assert_eq!(table.border.top.style, BorderStyle::Hidden);
    assert!(table.table_auto_horizontal_margins);
    assert!(validate_general_position(&table, "table", "<table>").is_ok());

    let mut bracket = ComputedStyle::ua_defaults("span", None);
    apply_inline(
        &mut bracket,
        "font-size:xx-large;transform:scale(1.35);display:inline-block",
        "<span>",
    )
    .unwrap();
    assert_eq!(bracket.inline_scale_x, Some(1.35));
    assert!(validate_general_position(&bracket, "span", "<span>").is_ok());

    let mut invalid = ComputedStyle::ua_defaults("span", None);
    assert!(apply_inline(&mut invalid, "transform:scale(1.4)", "<span>").is_err());
    apply_inline(&mut invalid, "transform:scale(1.35)", "<span>").unwrap();
    assert!(validate_general_position(&invalid, "span", "<span>").is_err());
}
