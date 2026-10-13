//! Escaping and Blackboard-compatible normalization of question HTML.

pub(super) fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn smart(html: &str) -> String {
    format!(
        "<flow class=\"FORMATTED_TEXT_BLOCK\"><material><mat_extension><mat_formattedtext type=\"SMART_TEXT\">{}</mat_formattedtext></mat_extension></material></flow>",
        xml(&sanitize_question_html(html))
    )
}

/// Removes invalid whitespace-only nodes directly inside Blackboard table
/// structure. Blackboard Ultra can fail on one directly inside a styled `<tr>`;
/// cell text, comments, and other authored markup remain byte-for-byte intact.
fn sanitize_question_html(html: &str) -> String {
    if !html.to_ascii_lowercase().contains("<table") {
        return html.to_owned();
    }
    let mut sanitized = String::with_capacity(html.len());
    let mut stack = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        if let Some(raw_tag) = raw_text_tag(&stack) {
            let closing_tag = format!("</{raw_tag}");
            let Some(offset) = find_ascii_case_insensitive(&html[cursor..], &closing_tag) else {
                sanitized.push_str(&html[cursor..]);
                break;
            };
            let closing = cursor + offset;
            sanitized.push_str(&html[cursor..closing]);
            cursor = closing;
        }
        let Some(offset) = html[cursor..].find('<') else {
            push_table_text(&mut sanitized, &html[cursor..], &stack);
            break;
        };
        let tag_start = cursor + offset;
        push_table_text(&mut sanitized, &html[cursor..tag_start], &stack);
        let Some(tag_end) = html_tag_end(&html[tag_start..]) else {
            sanitized.push_str(&html[tag_start..]);
            break;
        };
        let tag_end = tag_start + tag_end;
        let tag = &html[tag_start..=tag_end];
        sanitized.push_str(tag);
        apply_html_tag(tag, &mut stack);
        cursor = tag_end + 1;
    }
    sanitized
}

fn push_table_text(output: &mut String, text: &str, stack: &[String]) {
    let is_table_whitespace = stack
        .last()
        .is_some_and(|tag| matches!(tag.as_str(), "table" | "thead" | "tbody" | "tfoot" | "tr"));
    if !is_table_whitespace || !text.chars().all(char::is_whitespace) {
        output.push_str(text);
    }
}

fn raw_text_tag(stack: &[String]) -> Option<&str> {
    match stack.last().map(String::as_str) {
        Some("script") => Some("script"),
        Some("style") => Some("style"),
        _ => None,
    }
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn html_tag_end(fragment: &str) -> Option<usize> {
    if fragment.starts_with("<!--") {
        return fragment.find("-->").map(|offset| offset + 2);
    }
    let mut quote = None;
    for (offset, byte) in fragment.bytes().enumerate().skip(1) {
        match (quote, byte) {
            (Some(delimiter), value) if value == delimiter => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
    }
    None
}

fn apply_html_tag(tag: &str, stack: &mut Vec<String>) {
    let content = tag[1..tag.len() - 1].trim();
    if content.starts_with('!') || content.starts_with('?') || content.is_empty() {
        return;
    }
    let closing = content.starts_with('/');
    let name = content
        .trim_start_matches('/')
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | ':'))
        .collect::<String>()
        .to_ascii_lowercase();
    if name.is_empty() {
        return;
    }
    if closing {
        if let Some(index) = stack.iter().rposition(|open| open == &name) {
            stack.truncate(index);
        }
        return;
    }
    if !content.ends_with('/') && !is_void_html_tag(&name) {
        stack.push(name);
    }
}

fn is_void_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::sanitize_question_html;

    #[test]
    fn removes_only_invalid_table_structure_whitespace() {
        let input = "<table style=\"text-align:center\"><tr> <td> cell </td> <td><b>bold</b> text</td></tr> </table>";
        assert_eq!(
            sanitize_question_html(input),
            "<table style=\"text-align:center\"><tr><td> cell </td><td><b>bold</b> text</td></tr></table>"
        );
    }

    #[test]
    fn removes_comment_separated_table_whitespace_without_losing_authored_content() {
        let input = "<TABLE> <TR><!-- first cell --> <TD> cell text </TD> <!-- second cell --> <TD><b>bold</b> text</TD> </TR> </TABLE>";
        assert_eq!(
            sanitize_question_html(input),
            "<TABLE><TR><!-- first cell --><TD> cell text </TD><!-- second cell --><TD><b>bold</b> text</TD></TR></TABLE>"
        );
    }

    #[test]
    fn leaves_non_table_html_byte_for_byte_unchanged() {
        let input = "plain <b> text </b>";
        assert_eq!(sanitize_question_html(input), input);
    }
}
