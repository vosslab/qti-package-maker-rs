//! Demonstrable inline CSS errors. Unknown syntax is not evidence of invalidity.

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, Delimiter, ParseError, Parser, ParserInput,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, Token,
};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::stylesheet::ParserOptions;
use lightningcss::traits::Parse;
use lightningcss::values::color::CssColor;
use std::cell::RefCell;

use html5ever::tokenizer::{BufferQueue, TokenSink, TokenSinkResult, Tokenizer};

// Coverage matches Python assessment_items/inline_css.py. These sets select
// reliable checks; they are not a whitelist of permitted authoring features.
const COMMON: &str =
    "background background-color border border-top border-right border-bottom border-left
border-color border-top-color border-right-color border-bottom-color border-left-color
border-width border-top-width border-right-width border-bottom-width border-left-width
border-style border-top-style border-right-style border-bottom-style border-left-style
border-spacing border-radius box-shadow box-sizing
color display font font-family font-size font-style font-weight height width
min-height min-width max-height max-width left right top bottom line-height
margin margin-top margin-right margin-bottom margin-left padding padding-top
padding-right padding-bottom padding-left overflow overflow-x overflow-y position
text-align text-decoration text-transform vertical-align visibility white-space
letter-spacing word-spacing outline outline-color outline-style outline-width";
const COLORS: &str = "color background-color border-color border-top-color border-right-color
border-bottom-color border-left-color outline-color text-decoration-color
column-rule-color caret-color accent-color";
const COLOR_CONTEXTS: &str = "background border border-top border-right border-bottom border-left
outline box-shadow text-shadow text-decoration column-rule";
const COLOR_FUNCTIONS: &str = "rgb rgba hsl hsla hwb lab lch oklab oklch color";
const WIDE: &str = "inherit initial unset revert revert-layer";
const SYSTEM_COLORS: &str =
    "accentcolor accentcolortext activetext buttonborder buttonface buttontext canvas
canvastext field fieldtext graytext highlight highlighttext linktext mark marktext
selecteditem selecteditemtext visitedtext activeborder activecaption appworkspace
background buttonhighlight buttonshadow captiontext inactiveborder inactivecaption
inactivecaptiontext infobackground infotext menu menutext scrollbar threeddarkshadow
threedface threedhighlight threedlightshadow threedshadow window windowframe windowtext";
const LENGTH_UNITS: &str = "px cm mm q in pt pc em ex ch rem cap ic lh rlh rex rcap rch ric
vw vh vi vb vmin vmax svw svh svi svb svmin svmax lvw lvh lvi lvb lvmin lvmax
dvw dvh dvi dvb dvmin dvmax cqw cqh cqi cqb cqmin cqmax";

fn contains(set: &str, value: &str) -> bool {
    set.split_ascii_whitespace()
        .any(|candidate| candidate == value)
}

#[derive(Debug)]
enum Kind {
    Ident(String),
    Dimension(f32, String),
    Number(f32),
    Percentage(f32),
    Color,
    Other,
}

#[derive(Debug)]
struct Component {
    raw: String,
    kind: Kind,
    deferred: bool,
}

impl Component {
    fn ident(&self) -> Option<&str> {
        if let Kind::Ident(name) = &self.kind {
            Some(name)
        } else {
            None
        }
    }
}

fn components<'i>(input: &mut Parser<'i, '_>) -> Result<Vec<Component>, ParseError<'i, ()>> {
    let mut parts = Vec::new();
    while !input.is_exhausted() {
        let start = input.position();
        let token = input.next()?.clone();
        let mut deferred = false;
        let kind = match token {
            Token::Ident(name) => Kind::Ident(name.to_ascii_lowercase()),
            Token::Dimension { value, unit, .. } => {
                Kind::Dimension(value, unit.to_ascii_lowercase())
            }
            Token::Number { value, .. } => Kind::Number(value),
            Token::Percentage { unit_value, .. } => Kind::Percentage(unit_value),
            Token::Hash(_) | Token::IDHash(_) => Kind::Color,
            Token::Function(name) => {
                let nested = input.parse_nested_block(components)?;
                deferred = !contains(COLOR_FUNCTIONS, &name.to_ascii_lowercase())
                    || nested.iter().any(|part| part.deferred)
                    || nested.first().and_then(Component::ident) == Some("from")
                    || (name.eq_ignore_ascii_case("color")
                        && nested
                            .first()
                            .and_then(Component::ident)
                            .is_some_and(|name| name.starts_with("--")));
                Kind::Color
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                input.parse_nested_block(components)?;
                deferred = true;
                Kind::Other
            }
            Token::UnquotedUrl(_) => {
                deferred = true;
                Kind::Other
            }
            Token::BadUrl(_)
            | Token::BadString(_)
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket => {
                return Err(input.new_custom_error(()));
            }
            _ => Kind::Other,
        };
        parts.push(Component {
            raw: input.slice_from(start).trim().to_owned(),
            kind,
            deferred,
        });
    }
    Ok(parts)
}

struct Declaration {
    name: String,
    value: String,
    parts: Vec<Component>,
}

struct InlineParser;

impl<'i> DeclarationParser<'i> for InlineParser {
    type Declaration = Declaration;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _: &ParserState,
    ) -> Result<Declaration, ParseError<'i, ()>> {
        let (value, parts) = input.parse_until_before(Delimiter::Bang, |input| {
            let start = input.position();
            let parts = components(input)?;
            Ok((input.slice_from(start).trim().to_owned(), parts))
        })?;
        let _important = input.try_parse(cssparser::parse_important).is_ok();
        input.expect_exhausted()?;
        Ok(Declaration {
            name: name.to_ascii_lowercase(),
            value,
            parts,
        })
    }
}

impl<'i> AtRuleParser<'i> for InlineParser {
    type Prelude = ();
    type AtRule = Declaration;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for InlineParser {
    type Prelude = ();
    type QualifiedRule = Declaration;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Declaration, ()> for InlineParser {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        false
    }
}

fn color_valid(part: &Component) -> bool {
    part.deferred
        || part
            .ident()
            .is_some_and(|name| contains(SYSTEM_COLORS, name))
        || CssColor::parse_string(&part.raw).is_ok()
}

fn modern_keywords(name: &str) -> &'static str {
    match name {
        "display" => "flex inline-flex grid inline-grid contents flow-root ruby",
        "text-align" => "start end match-parent justify-all",
        "white-space" => "break-spaces preserve preserve-breaks preserve-spaces wrap collapse",
        "overflow" | "overflow-x" | "overflow-y" => "clip",
        "position" => "sticky",
        "font-size" => "xxx-large math",
        "width" | "height" | "max-width" | "max-height" => {
            "min-content max-content fit-content stretch contain"
        }
        "min-width" | "min-height" => "auto min-content max-content fit-content stretch contain",
        _ => "",
    }
}

fn nonnegative(name: &str) -> bool {
    contains(
        "width height min-width min-height max-width max-height font-size line-height border-spacing outline-width",
        name,
    ) || name.starts_with("padding")
        || name.starts_with("border")
}

fn declaration_error(decl: &Declaration) -> Option<&'static str> {
    let name = decl.name.as_str();
    let parts = &decl.parts;
    match name {
        "vert-align" => return Some("vert-align is not a CSS property; use vertical-align"),
        "spacing" => {
            return Some(
                "spacing is not a CSS property; choose padding, margin, or border-spacing",
            );
        }
        _ => {}
    }
    if name.starts_with("--") {
        return None;
    }
    if parts.is_empty() {
        return Some("a property value is required");
    }
    if parts.len() == 1 && parts[0].ident().is_some_and(|value| contains(WIDE, value)) {
        return None;
    }
    let color_context = contains(COLORS, name) || contains(COLOR_CONTEXTS, name);
    if color_context
        && parts
            .iter()
            .any(|part| matches!(part.kind, Kind::Color) && !color_valid(part))
    {
        return Some("expected a CSS color (hex colors require # and 3, 4, 6, or 8 hex digits)");
    }
    if parts.iter().any(|part| part.deferred) {
        return None;
    }
    if contains(COLORS, name) {
        if matches!(name, "caret-color" | "accent-color")
            && parts.len() == 1
            && parts[0].ident() == Some("auto")
        {
            return None;
        }
        if parts.len() > if name == "border-color" { 4 } else { 1 } {
            return Some("unexpected extra color components");
        }
        return if parts.iter().all(color_valid) {
            None
        } else {
            Some("expected a CSS color (hex colors require # and 3, 4, 6, or 8 hex digits)")
        };
    }
    if !contains(COMMON, name) {
        return None;
    }
    if parts.iter().any(|part| {
        part.ident()
            .is_some_and(|value| contains(modern_keywords(name), value))
    }) {
        return None;
    }
    // These expanded grammars are not shared by both validation libraries.
    if (contains("font-style text-decoration display white-space", name) && parts.len() > 1)
        || (name == "background" && parts.iter().any(|part| part.raw == "/"))
    {
        return None;
    }
    if name == "font-weight"
        && parts.len() == 1
        && let Kind::Number(value) = parts[0].kind
    {
        return if (1.0..=1000.0).contains(&value) {
            None
        } else {
            Some("numeric font-weight must be between 1 and 1000")
        };
    }
    let mut probe = Vec::new();
    for part in parts {
        match &part.kind {
            Kind::Number(value) if *value != 0.0 && contains(
                "width height min-width min-height max-width max-height left right top bottom font-size
                 padding padding-top padding-right padding-bottom padding-left
                 margin margin-top margin-right margin-bottom margin-left
                 border border-top border-right border-bottom border-left border-width
                 border-top-width border-right-width border-bottom-width border-left-width
                 border-spacing border-radius outline outline-width box-shadow letter-spacing word-spacing", name
            ) => return Some("non-zero lengths require a CSS unit"),
            Kind::Dimension(value, unit) => {
                if !contains(LENGTH_UNITS, unit) {
                    return Some("unknown length unit");
                }
                if *value < 0.0 && nonnegative(name) {
                    return Some("this property requires a non-negative length");
                }
                probe.push(format!("{value}px"));
            }
            Kind::Number(value) | Kind::Percentage(value) if *value < 0.0 && nonnegative(name) => {
                return Some("this property requires a non-negative value");
            }
            _ if color_context && color_valid(part) => probe.push("black".to_owned()),
            _ => probe.push(part.raw.clone()),
        }
    }
    let probe = probe.join(" ");
    // Lightning CSS leaves unsupported and custom properties unclassified.
    // An Unparsed value in this bounded, literal-only scope is demonstrably invalid.
    match Property::parse_string(PropertyId::from(name), &probe, ParserOptions::default()) {
        Ok(Property::Unparsed(_)) | Err(_) => {
            Some("invalid value for this CSS property; check units, keywords, and components")
        }
        _ => None,
    }
}

fn validate_style(style: &str) -> Result<(), String> {
    let mut input = ParserInput::new(style);
    let mut input = Parser::new(&mut input);
    let mut parser = InlineParser;
    for declaration in RuleBodyParser::new(&mut input, &mut parser) {
        let declaration = declaration.map_err(|_| {
            format!("invalid inline CSS declaration in {style:?}; expected property: value")
        })?;
        if let Some(reason) = declaration_error(&declaration) {
            return Err(format!(
                "invalid inline CSS {}: {:?}; {reason}",
                declaration.name, declaration.value
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_attributes(html: &str) -> Result<(), String> {
    // ASVS 2.2.1: inspect original attributes, not the entity-stripped XML copy.
    // The HTML parser decodes character references; no serialization reaches the caller.
    let input = BufferQueue::default();
    input.push_back(html.into());
    let tokenizer = Tokenizer::new(AttributeSink::default(), Default::default());
    let _result = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.error.into_inner().map_or(Ok(()), Err)
}

#[derive(Default)]
struct AttributeSink {
    error: RefCell<Option<String>>,
}

impl TokenSink for AttributeSink {
    type Handle = ();

    fn process_token(&self, token: html5ever::tokenizer::Token, _: u64) -> TokenSinkResult<()> {
        let html5ever::tokenizer::Token::TagToken(tag) = token else {
            return TokenSinkResult::Continue;
        };
        if tag.kind != html5ever::tokenizer::StartTag {
            return TokenSinkResult::Continue;
        }
        for attribute in &tag.attrs {
            if self.error.borrow().is_some() {
                break;
            }
            let name = attribute.name.local.as_ref();
            let value = attribute.value.as_ref();
            let result = match name {
                "style" => validate_style(value),
                "color" | "bgcolor" => {
                    let mut source = ParserInput::new(value);
                    let mut parser = Parser::new(&mut source);
                    components(&mut parser)
                        .map_err(|_| format!("invalid {name}={value:?}"))
                        .and_then(|parts| {
                            let declaration = Declaration {
                                name: "color".to_owned(),
                                value: value.to_owned(),
                                parts,
                            };
                            declaration_error(&declaration).map_or(Ok(()), |reason| {
                                Err(format!("invalid {name}={value:?}; {reason}"))
                            })
                        })
                }
                _ => continue,
            };
            if let Err(reason) = result {
                *self.error.borrow_mut() = Some(format!("<{}>: {reason}", tag.name));
            }
        }
        match tag.name.as_ref() {
            "script" => TokenSinkResult::RawData(html5ever::tokenizer::states::RawKind::ScriptData),
            "style" => TokenSinkResult::RawData(html5ever::tokenizer::states::RawKind::Rawtext),
            _ => TokenSinkResult::Continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Item, ItemBody, ValidationError, validate_html};

    #[test]
    fn rejects_confirmed_inline_css_errors() {
        for style in [
            "color: ff0303",
            "color: #gg0303",
            "color: #12345",
            "color: rgb(banana)",
            "color: hsl(banana)",
            "color: notacolor",
            "font-size: 12potatoes",
            "height: 25",
            "padding: -2px",
            "text-align: sideways",
            "vert-align: middle",
            "spacing: 20px",
            "border: 1px solid rgb(banana)",
            "text-shadow: 1px 1px rgb(banana)",
            "background: ff0303",
            "color: ff0303; color: red",
            "color: ff0303 !important; color: red",
            "color red",
            "color: red; broken",
        ] {
            let html = format!("<span style=\"{style}\">Text</span>");
            assert!(validate_html(&html).is_err(), "accepted {style:?}");
        }
    }

    #[test]
    fn preserves_valid_and_unclassified_content() {
        for style in [
            "color: #abc",
            "color: #abcd",
            "color: #aAbBcC",
            "color: #12345678",
            "color: rebeccapurple",
            "color: rgb(255, 3, 3)",
            "color: rgb(100% 0% 0% / .5)",
            "color: hsl(120deg 50% 50%)",
            "color: oklch(50% .2 120)",
            "color: currentColor",
            "color: CanvasText",
            "color: transparent",
            "border: 1px solid #abcd",
            "background: #12345678",
            "width: 50%; height: 0; margin-left: -2px; line-height: 1.5",
            "display: flex; text-align: start; font-weight: 450; width: 20cqw",
            "container-type: inline-size; aspect-ratio: 2 / 1",
            "color: var(--ink); --ink: #123; width: calc(100% - 2em)",
            "color: rgb(from red r g b); color: color-mix(in srgb, red, blue)",
            "future-property: future-value; color: future-color(1)",
            "color: red /* comment */ !important; COLOR: blue",
            "color: &#35;ff0303",
            "accent-color: auto",
            "background: center / cover red; text-decoration: underline solid red",
            "font-style: oblique 20deg; display: inline flow-root",
            "color: color(--my-profile 0 0 0)",
        ] {
            let html = format!("<span style=\"{style}\">Text</span>");
            assert!(
                validate_html(&html).is_ok(),
                "rejected {style:?}: {:?}",
                validate_html(&html)
            );
        }
    }

    #[test]
    fn reports_answer_and_legacy_attribute_context() {
        let result = Item::new(
            "Choose one.".to_owned(),
            ItemBody::Mc {
                choices: vec![
                    "<span style=\"color: ff0303\">Red</span>".to_owned(),
                    "Blue".to_owned(),
                ],
                answer: "Blue".to_owned(),
            },
        );
        assert!(matches!(
            result,
            Err(ValidationError::InvalidInlineCss {
                field: "choices",
                ..
            })
        ));
        let error =
            validate_html("<table><tr><td bgcolor=\"4d4d4d\">Cell</td></tr></table>").unwrap_err();
        assert!(error.contains("td") && error.contains("bgcolor") && error.contains("4d4d4d"));
        assert!(validate_html("<font color=\"#4d4d4d\">Text</font>").is_ok());
        assert!(validate_html("<tr><td style=\"color: ff0303\">Cell</td></tr>").is_err());
    }
}
