//! XML-safe preservation of Blackboard-authored HTML fragments.

use lol_html::{RewriteStrSettings, element, rewrite_str};
use markup5ever::data::NAMED_ENTITIES;

use super::xml_attr;

pub(super) fn plain_text(value: &str) -> String {
    html_entities_to_xml(&value.replace('<', "&lt;").replace('>', "&gt;"))
}

pub(super) fn fragment(value: &str) -> String {
    let normalized = rewrite_str(
        value,
        RewriteStrSettings::new().append_element_content_handler(element!("*", |element| {
            let attributes = element
                .attributes()
                .iter()
                .map(|attribute| (attribute.name(), attribute.value()))
                .collect::<Vec<_>>();
            for (name, value) in attributes {
                element.set_attribute(&name, &value)?;
            }
            Ok(())
        })),
    )
    .expect("validated item HTML must be serializable");
    xml_safe_fragment(&normalized)
}

fn xml_safe_fragment(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remainder = value;
    while let Some((start, tag)) = raw_text_start(remainder) {
        let open_end = remainder[start..]
            .find('>')
            .expect("HTML parser returned an unclosed raw-text tag")
            + start;
        let close = format!("</{tag}>");
        let content_start = open_end + 1;
        let close_start = remainder[content_start..]
            .find(&close)
            .expect("HTML parser returned an unclosed raw-text element")
            + content_start;
        output.push_str(&html_entities_to_xml(&remainder[..content_start]));
        output.push_str("<![CDATA[");
        output
            .push_str(&remainder[content_start..close_start].replace("]]>", r#"]]]]><![CDATA[>"#));
        output.push_str("]]>");
        remainder = &remainder[close_start..];
    }
    output.push_str(&html_entities_to_xml(remainder));
    output
}

fn raw_text_start(value: &str) -> Option<(usize, &str)> {
    let lowercase = value.to_ascii_lowercase();
    ["script", "style"]
        .into_iter()
        .filter_map(|tag| lowercase.find(&format!("<{tag}")).map(|index| (index, tag)))
        .min_by_key(|(index, _)| *index)
}

fn html_entities_to_xml(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut offset = 0;
    while offset < value.len() {
        let entity = value[offset..]
            .starts_with('&')
            .then(|| html_entity_replacement(&value[offset..]))
            .flatten();
        if let Some((end, replacement)) = entity {
            output.push_str(&replacement);
            offset += end;
            continue;
        }
        let character = value[offset..]
            .chars()
            .next()
            .expect("offset is within a UTF-8 string");
        if character == '&' {
            output.push_str("&amp;");
        } else {
            output.push(character);
        }
        offset += character.len_utf8();
    }
    output
}

fn html_entity_replacement(value: &str) -> Option<(usize, String)> {
    let end = value.find(';')?;
    let entity = value.get(1..=end)?;
    if entity.len() < 2
        || !entity[..entity.len() - 1]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'#')
    {
        return None;
    }
    if matches!(entity, "amp;" | "lt;" | "gt;" | "quot;" | "apos;") {
        return Some((end + 1, format!("&{entity}")));
    }
    let decoded = if let Some(number) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(number.trim_end_matches(';'), 16)
            .ok()
            .and_then(char::from_u32)
            .filter(|character| is_xml_character(*character))
            .map(|character| character.to_string())
    } else if let Some(number) = entity.strip_prefix('#') {
        number
            .trim_end_matches(';')
            .parse()
            .ok()
            .and_then(char::from_u32)
            .filter(|character| is_xml_character(*character))
            .map(|character| character.to_string())
    } else {
        NAMED_ENTITIES.get(entity).and_then(|(first, second)| {
            let first = char::from_u32(*first).filter(|character| is_xml_character(*character))?;
            let second = if *second == 0 {
                None
            } else {
                Some(char::from_u32(*second).filter(|character| is_xml_character(*character))?)
            };
            let mut result = first.to_string();
            if let Some(second) = second {
                result.push(second);
            }
            Some(result)
        })
    };
    let replacement = decoded.map_or_else(|| format!("&amp;{entity}"), |value| xml_attr(&value));
    Some((end + 1, replacement))
}

fn is_xml_character(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\r')
        || ('\u{20}'..='\u{D7FF}').contains(&character)
        || ('\u{E000}'..='\u{FFFD}').contains(&character)
        || ('\u{10000}'..='\u{10FFFF}').contains(&character)
}
