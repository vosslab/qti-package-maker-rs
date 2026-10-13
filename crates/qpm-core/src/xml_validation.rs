//! Python/lxml XML well-formedness for the validation-only copy of authored HTML.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;

// Default libxml2 limits used by Python's lxml parser (without XML_PARSE_HUGE).
const XML_MAX_NAME_LENGTH: usize = 50_000;
const XML_MAX_TEXT_LENGTH: usize = 10_000_000;

// XML 1.0 (fifth edition) NCName: Name without the namespace separator colon.
static NCNAME: LazyLock<Regex> = LazyLock::new(|| {
    let start = concat!(
        "A-Z_a-z\\x{c0}-\\x{d6}\\x{d8}-\\x{f6}\\x{f8}-\\x{2ff}",
        "\\x{370}-\\x{37d}\\x{37f}-\\x{1fff}\\x{200c}-\\x{200d}",
        "\\x{2070}-\\x{218f}\\x{2c00}-\\x{2fef}\\x{3001}-\\x{d7ff}",
        "\\x{f900}-\\x{fdcf}\\x{fdf0}-\\x{fffd}\\x{10000}-\\x{effff}"
    );
    Regex::new(&format!(
        "^[{start}][{start}0-9.\\-\\x{{b7}}\\x{{300}}-\\x{{36f}}\\x{{203f}}-\\x{{2040}}]*$"
    ))
    .expect("XML NCName expression is valid")
});

// libxml2 checks namespace names as URI references (including relative references).
// Its URI parser accepts bracketed host literals without validating the enclosed IP address.
static URI_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    let atom = "(?:[a-zA-Z0-9._~!$&'()*+,;=\\-]|%[a-fA-F0-9]{2})";
    let pchar = format!("(?:{atom}|[:@])");
    let authority =
        format!("(?:(?:{atom}|:)*@)?(?:\\[[a-zA-Z0-9:._~!$&'()*+,;=%\\-]*\\]|{atom}*)(?::[0-9]+)?");
    let absolute = format!("/(?:{pchar}+(?:/{pchar}*)*)?");
    let path = format!("(?:{absolute}|{pchar}+(?:/{pchar}*)*|)");
    let relative = format!("(?:{absolute}|(?:{atom}|@)+(?:/{pchar}*)*|)");
    let network = format!("//{authority}(?:/{pchar}*)*");
    Regex::new(&format!(
        "^(?:[a-zA-Z][a-zA-Z0-9+.\\-]*:(?:{network}|{path})|{network}|{relative})(?:\\?(?:{pchar}|[/?])*)?(?:#(?:{pchar}|[/?\\[\\]])*)?$"
    )).expect("URI reference expression is valid")
});

pub(crate) fn validate_fragment(cleaned: &str) -> Result<(), String> {
    let wrapped = format!("<root><cleaned>{cleaned}</cleaned></root>");
    // The tree parser intentionally omits some lexical and lxml-specific checks. Retain raw
    // names for QName syntax and duplicate declarations before inspecting resolved attributes.
    let mut reader = Reader::from_str(&wrapped);
    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(event) | Event::Empty(event) => {
                check_qname(event.name().as_ref())?;
                for attribute in event.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|error| error.to_string())?;
                    check_qname(attribute.key.as_ref())?;
                    check_text_length(attribute.value.len())?;
                    let name = attribute.key.as_ref();
                    if name == "xmlns" || name.starts_with("xmlns:") {
                        let uri = attribute.value.as_ref();
                        if name == "xmlns:xmlns" || (name != "xmlns" && uri.is_empty()) {
                            return Err("invalid namespace declaration".to_owned());
                        }
                        if !URI_REFERENCE.is_match(uri) {
                            return Err("namespace name is not a valid URI reference".to_owned());
                        }
                    }
                }
            }
            Event::PI(event) if event.target().eq_ignore_ascii_case("xml") => {
                return Err("xml is a reserved processing instruction target".to_owned());
            }
            Event::PI(event) => check_text_length(event.content().len())?,
            Event::Text(event) | Event::Comment(event) => check_text_length(event.len())?,
            Event::CData(event) => check_text_length(event.len())?,
            Event::Eof => break,
            _ => {}
        }
    }

    // ASVS 1.5.1: default DTD rejection and no entity resolver keep this a pure parse.
    // roxmltree checks structure, XML characters, namespace bindings and expanded attributes.
    let document = roxmltree::Document::parse(&wrapped).map_err(|error| error.to_string())?;
    let mut ids = BTreeSet::new();
    for node in document.descendants().filter(|node| node.is_element()) {
        // Python/lxml's default limit includes the two wrapper elements.
        if node
            .ancestors()
            .filter(|parent| parent.is_element())
            .count()
            > 256
        {
            return Err("Excessive depth in document: 256".to_owned());
        }
        if let Some(value) = node.attribute((roxmltree::NS_XML_URI, "id")) {
            if !NCNAME.is_match(value.trim_matches([' ', '\t', '\r', '\n'])) {
                return Err("xml:id must be an NCName".to_owned());
            }
            // lxml checks trimmed syntax, but keeps original whitespace for identity.
            if !ids.insert(value) {
                return Err("xml:id must be unique".to_owned());
            }
        }
    }
    Ok(())
}

fn check_qname(name: &str) -> Result<(), String> {
    if name.split(':').any(|part| part.len() > XML_MAX_NAME_LENGTH) {
        return Err("XML name is too long".to_owned());
    }
    let mut parts = name.split(':');
    if !parts.next().is_some_and(|part| NCNAME.is_match(part))
        || parts.next().is_some_and(|part| !NCNAME.is_match(part))
        || parts.next().is_some()
    {
        return Err("element and attribute names must be XML QNames".to_owned());
    }
    Ok(())
}

fn check_text_length(length: usize) -> Result<(), String> {
    if length > XML_MAX_TEXT_LENGTH {
        return Err("XML text or attribute value is too long".to_owned());
    }
    Ok(())
}
