//! String normalization and plain-text rendering shared by QTI writers.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

static CRC_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:\d{1,3}\.\s*)?(?:<p>)?[a-f0-9_]{4,16}(?:</p>)?\s*")
        .expect("CRC prefix regular expression is valid")
});
static PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)^(?P<leading>(?:\s*<[^/!][^>]*>\s*)*)(?P<prefix>[A-Za-z0-9][):.])\s*")
        .expect("choice prefix regular expression is valid")
});
static NO_BORDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)border\s*:\s*(0|0px|none|0\s)").expect("no-border regular expression is valid")
});

/// Errors returned by bounded string conversion helpers.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StringError {
    /// The requested alphabet position has no corresponding letter.
    #[error("invalid alphabet index {value}; expected 1 through 26")]
    InvalidAlphabetIndex { value: usize },
    /// Python's `num2words` refuses negative ordinal inputs.
    #[error("cannot treat negative number {value} as ordinal")]
    NegativeOrdinal { value: i64 },
}

/// Remove a leading Python-style item CRC, including its optional item number.
pub fn strip_crc_prefix(question_text: &str) -> String {
    CRC_PREFIX_RE.replace(question_text, "").into_owned()
}

/// Return whether every choice begins with the same supported choice-prefix shape.
pub fn has_prefix(choices: &[String]) -> bool {
    choices.iter().all(|choice| prefix_span(choice).is_some())
}

/// Remove one supported letter or number prefix while retaining leading nested HTML tags.
pub fn strip_prefix_from_string(choice: &str) -> String {
    remove_prefix(choice)
}

fn remove_prefix(value: &str) -> String {
    prefix_span(value).map_or_else(
        || value.to_owned(),
        |(leading_end, prefix_end)| format!("{}{}", &value[..leading_end], &value[prefix_end..]),
    )
}

fn prefix_span(value: &str) -> Option<(usize, usize)> {
    let captures = PREFIX_RE.captures(value)?;
    let prefix = captures
        .name("prefix")
        .expect("prefix capture is present")
        .as_str();
    let after_prefix = captures
        .name("prefix")
        .expect("prefix capture is present")
        .end();
    if prefix.ends_with('.')
        && value[after_prefix..]
            .chars()
            .next()
            .is_some_and(char::is_numeric)
    {
        return None;
    }
    let leading_end = captures
        .name("leading")
        .expect("leading tag capture is present")
        .end();
    let prefix_end = captures
        .get(0)
        .expect("whole prefix capture is present")
        .end();
    Some((leading_end, prefix_end))
}

/// Remove choice prefixes only when every supplied choice has one.
pub fn remove_prefix_from_list(choices: &[String]) -> Vec<String> {
    if !has_prefix(choices) {
        return choices.to_vec();
    }
    choices
        .iter()
        .map(|choice| strip_prefix_from_string(choice))
        .collect()
}

/// Convert a one-based position to an uppercase English letter.
pub fn number_to_letter(value: usize) -> Result<char, StringError> {
    alphabet_letter(value, b'A')
}

/// Convert a one-based position to a lowercase English letter.
pub fn number_to_lowercase(value: usize) -> Result<char, StringError> {
    alphabet_letter(value, b'a')
}

/// Spell an integer using the English (`en_US`) wording used by the Python writer.
pub fn number_to_cardinal(value: i64) -> String {
    if value < 0 {
        return format!("minus {}", cardinal_positive(value.unsigned_abs()));
    }
    cardinal_positive(value as u64)
}

/// Spell an integer's English ordinal (`first`, `twenty-first`, and so on).
pub fn number_to_ordinal(value: i64) -> Result<String, StringError> {
    if value < 0 {
        return Err(StringError::NegativeOrdinal { value });
    }
    let cardinal = cardinal_positive(value as u64);
    let split_at = cardinal.rfind([' ', '-']).map_or(0, |index| index + 1);
    let (head, tail) = cardinal.split_at(split_at);
    let ordinal_tail = match tail {
        "zero" => "zeroth".to_owned(),
        "one" => "first".to_owned(),
        "two" => "second".to_owned(),
        "three" => "third".to_owned(),
        "four" => "fourth".to_owned(),
        "five" => "fifth".to_owned(),
        "six" => "sixth".to_owned(),
        "seven" => "seventh".to_owned(),
        "eight" => "eighth".to_owned(),
        "nine" => "ninth".to_owned(),
        "ten" => "tenth".to_owned(),
        "eleven" => "eleventh".to_owned(),
        "twelve" => "twelfth".to_owned(),
        "thirteen" => "thirteenth".to_owned(),
        "fourteen" => "fourteenth".to_owned(),
        "fifteen" => "fifteenth".to_owned(),
        "sixteen" => "sixteenth".to_owned(),
        "seventeen" => "seventeenth".to_owned(),
        "eighteen" => "eighteenth".to_owned(),
        "nineteen" => "nineteenth".to_owned(),
        "twenty" | "thirty" | "forty" | "fifty" | "sixty" | "seventy" | "eighty" | "ninety" => {
            format!(
                "{}ieth",
                tail.strip_suffix('y').expect("tens words end in y")
            )
        }
        "hundred" | "thousand" | "million" | "billion" | "trillion" => format!("{tail}th"),
        _ => format!("{tail}th"),
    };
    if head.is_empty() {
        Ok(ordinal_tail)
    } else {
        Ok(format!("{head}{ordinal_tail}"))
    }
}

fn cardinal_positive(value: u64) -> String {
    const SMALL: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 8] = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    if value < 20 {
        return SMALL[value as usize].to_owned();
    }
    if value < 100 {
        let tens = TENS[(value / 10 - 2) as usize];
        return if value.is_multiple_of(10) {
            tens.to_owned()
        } else {
            format!("{tens}-{}", SMALL[(value % 10) as usize])
        };
    }
    if value < 1_000 {
        let prefix = format!("{} hundred", SMALL[(value / 100) as usize]);
        return if value.is_multiple_of(100) {
            prefix
        } else {
            format!("{prefix} and {}", cardinal_positive(value % 100))
        };
    }
    for (scale, label) in [
        (1_000_000_000_000_u64, "trillion"),
        (1_000_000_000, "billion"),
        (1_000_000, "million"),
        (1_000, "thousand"),
    ] {
        if value >= scale {
            let prefix = format!("{} {label}", cardinal_positive(value / scale));
            return if value.is_multiple_of(scale) {
                prefix
            } else {
                format!("{prefix}, {}", cardinal_positive(value % scale))
            };
        }
    }
    unreachable!("positive cardinal values below one thousand return above")
}

fn alphabet_letter(value: usize, first: u8) -> Result<char, StringError> {
    if !(1..=26).contains(&value) {
        return Err(StringError::InvalidAlphabetIndex { value });
    }
    Ok(char::from(first + (value as u8 - 1)))
}

/// Convert a positive integer to its conventional Roman-numeral representation.
pub fn number_to_roman(mut value: u32) -> String {
    const NUMERALS: &[(u32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    for &(amount, numeral) in NUMERALS {
        while value >= amount {
            result.push_str(numeral);
            value -= amount;
        }
    }
    result
}

/// Convert `<sub>` and `<sup>` digit-like content to Unicode super/subscripts.
pub fn convert_sub_sup(question: &str) -> String {
    let sub = replace_script_tags(
        question,
        "sub",
        "\u{2080}\u{2081}\u{2082}\u{2083}\u{2084}\u{2085}\u{2086}\u{2087}\u{2088}\u{2089}\u{208a}\u{208b}\u{208c}\u{208d}\u{208e}",
    );
    replace_script_tags(
        &sub,
        "sup",
        "\u{2070}\u{00b9}\u{00b2}\u{00b3}\u{2074}\u{2075}\u{2076}\u{2077}\u{2078}\u{2079}\u{207a}\u{207b}\u{207c}\u{207d}\u{207e}",
    )
}

fn replace_script_tags(question: &str, tag: &str, replacements: &str) -> String {
    let expression = format!(r"<{tag}>(.*?)</{tag}>");
    let pattern = Regex::new(&expression).expect("script tag regular expression is valid");
    pattern
        .replace_all(question, |captures: &regex::Captures<'_>| {
            translate_script(&captures[1], replacements)
        })
        .into_owned()
}

fn translate_script(text: &str, replacements: &str) -> String {
    const INPUT: &str = "0123456789+-=()";
    text.chars()
        .map(|character| {
            INPUT
                .chars()
                .position(|candidate| candidate == character)
                .and_then(|index| replacements.chars().nth(index))
                .unwrap_or(character)
        })
        .collect()
}

/// Make authored HTML readable in text-only QTI output.
pub fn make_question_pretty(question: &str) -> String {
    let mut tables = Vec::new();
    let mut pretty = question.to_owned();
    while let Some((start, end)) = innermost_table_range(&pretty) {
        let token = format!("__QTI_TABLE_{}__", tables.len());
        tables.push(html_table_to_text(&pretty[start..end]));
        pretty.replace_range(start..end, &format!("\n\n{token}\n"));
    }
    pretty = pretty.replace("&nbsp;", " ").replace("&NBSP;", " ");
    pretty = Regex::new(r"(?i)<h[0-9]>")
        .expect("heading regular expression is valid")
        .replace_all(&pretty, "<p>")
        .into_owned();
    for (pattern, replacement) in [
        (r"(?i)<br/>", "\n"),
        (r"(?i)<li>", "\n* "),
        (r"(?i)<span [^>]*>", " "),
        (r"(?i)</?strong>", " "),
        (r"(?i)</?[bi]>", " "),
        (r"(?i)</span>", ""),
        (r"(?i)<hr/>", ""),
        (r"(?i)</p>\s*<p>", "\n"),
        (r"(?i)<p>\s*</p>", "\n"),
        (r"(?i)\n</p>", ""),
        (r"(?i)\n<p>", "\n"),
        (r"(?i)</?[^>]+>", ""),
        (r"\n{3,}", "\n\n"),
        (r" {2,}", " "),
    ] {
        pretty = Regex::new(pattern)
            .expect("question formatting regular expression is valid")
            .replace_all(&pretty, replacement)
            .into_owned();
    }
    pretty = convert_sub_sup(&decode_html_entities(&pretty));
    for (index, table) in tables.into_iter().enumerate() {
        pretty = pretty.replace(&format!("__QTI_TABLE_{index}__"), &format!("\0\n{table}\n"));
    }
    pretty.trim().to_owned()
}

fn innermost_table_range(html: &str) -> Option<(usize, usize)> {
    let closing = find_ascii_case_insensitive(html, "</table>")?;
    let opening = rfind_ascii_case_insensitive(&html[..closing], "<table")?;
    html[opening..].find('>')?;
    Some((opening, closing + "</table>".len()))
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn rfind_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .rposition(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn html_table_to_text(table_html: &str) -> String {
    let document = Html::parse_fragment(table_html);
    let table_selector = Selector::parse("table").expect("table selector is valid");
    let row_selector = Selector::parse("tr").expect("row selector is valid");
    let table = match document.select(&table_selector).next() {
        Some(table) => table,
        None => return "[TABLE]".to_owned(),
    };
    let rows: Vec<_> = table.select(&row_selector).collect();
    if rows.is_empty() {
        return "[TABLE]".to_owned();
    }
    let header = rows.iter().copied().find(|row| {
        row.children()
            .filter_map(ElementRef::wrap)
            .any(|cell| cell.value().name() == "th")
    });
    let mut headers = Vec::new();
    let mut data = Vec::new();
    for row in rows {
        let cells: Vec<String> = row_cells(row).map(element_text).collect();
        if cells.is_empty() {
            continue;
        }
        if header.is_some_and(|candidate| candidate.id() == row.id()) {
            headers = cells;
        } else {
            data.push(cells);
        }
    }
    if data.is_empty() && !headers.is_empty() {
        return "[TABLE]".to_owned();
    }
    let width = data
        .iter()
        .map(Vec::len)
        .chain(std::iter::once(headers.len()))
        .max()
        .unwrap_or_default();
    if width == 0 {
        return "[TABLE]".to_owned();
    }
    if !headers.is_empty() {
        headers.resize(width, String::new());
    }
    for row in &mut data {
        row.resize(width, String::new());
    }
    let rows = if headers.is_empty() {
        data.clone()
    } else {
        [vec![headers.clone()], data.clone()].concat()
    };
    let numeric_columns = (0..width)
        .map(|column| !data.is_empty() && data.iter().all(|row| is_number(&row[column])))
        .collect::<Vec<_>>();
    let content_widths = (0..width)
        .map(|column| {
            rows.iter()
                .map(|row| display_width(&row[column]))
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let column_widths = content_widths
        .iter()
        .enumerate()
        .map(|(column, content_width)| {
            if numeric_columns[column] {
                content_width + 2
            } else {
                (*content_width).max(2)
            }
        })
        .collect::<Vec<_>>();
    let fancy_widths = content_widths
        .iter()
        .map(|width| width + usize::from(!headers.is_empty()) * 2)
        .collect::<Vec<_>>();
    match table_format(table) {
        TableFormat::Plain => format_plain(&headers, &data, &column_widths, &numeric_columns),
        TableFormat::FancyGrid => {
            format_fancy(&headers, &data, &fancy_widths, &numeric_columns, true)
        }
        TableFormat::FancyOutline => {
            format_fancy(&headers, &data, &fancy_widths, &numeric_columns, false)
        }
    }
}

fn row_cells(row: ElementRef<'_>) -> impl Iterator<Item = ElementRef<'_>> {
    row.children()
        .filter_map(ElementRef::wrap)
        .filter(|cell| matches!(cell.value().name(), "th" | "td"))
}

fn element_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<String>()
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy)]
enum TableFormat {
    Plain,
    FancyGrid,
    FancyOutline,
}

fn table_format(table: ElementRef<'_>) -> TableFormat {
    let cell_selector = Selector::parse("td, th").expect("cell selector is valid");
    if table
        .select(&cell_selector)
        .any(|cell| has_visible_border(cell.value().attr("style").unwrap_or("")))
    {
        return TableFormat::FancyGrid;
    }
    if table
        .value()
        .attr("border")
        .is_some_and(|border| border.trim() == "0")
    {
        return TableFormat::Plain;
    }
    let style = table.value().attr("style").unwrap_or("");
    let lower = style.to_ascii_lowercase();
    let collapse = lower.contains("border-collapse");
    let visible = has_visible_border(&lower);
    if visible {
        return TableFormat::FancyOutline;
    }
    if collapse && NO_BORDER_RE.is_match(&lower) {
        return TableFormat::Plain;
    }
    if table.value().attr("border").is_some() {
        return TableFormat::FancyOutline;
    }
    TableFormat::FancyGrid
}

fn has_visible_border(style: &str) -> bool {
    if !style.to_ascii_lowercase().contains("border") {
        return false;
    }
    let without_zero = NO_BORDER_RE.replace_all(style, "");
    without_zero.to_ascii_lowercase().contains("border")
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

fn is_number(value: &str) -> bool {
    value.replace(',', "").parse::<f64>().is_ok()
}

fn aligned(value: &str, width: usize, numeric: bool) -> String {
    let padding = width.saturating_sub(display_width(value));
    if numeric {
        format!("{}{}", " ".repeat(padding), value)
    } else {
        format!("{value}{}", " ".repeat(padding))
    }
}

fn format_fancy(
    headers: &[String],
    data: &[Vec<String>],
    widths: &[usize],
    numeric_columns: &[bool],
    grid: bool,
) -> String {
    let border = |left: char, middle: char, right: char, fill: char| -> String {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|width| fill.to_string().repeat(width + 2))
                .collect::<Vec<_>>()
                .join(&middle.to_string())
        )
    };
    let row = |cells: &[String]| -> String {
        format!(
            "\u{2502} {} \u{2502}",
            cells
                .iter()
                .zip(widths)
                .zip(numeric_columns)
                .map(|((cell, width), numeric)| aligned(cell, *width, *numeric))
                .collect::<Vec<_>>()
                .join(" \u{2502} ")
        )
    };
    let mut lines = vec![border('\u{2552}', '\u{2564}', '\u{2555}', '\u{2550}')];
    if !headers.is_empty() {
        lines.push(row(headers));
        lines.push(border('\u{255e}', '\u{256a}', '\u{2561}', '\u{2550}'));
    }
    for (index, cells) in data.iter().enumerate() {
        lines.push(row(cells));
        if grid && index + 1 < data.len() {
            lines.push(border('\u{251c}', '\u{253c}', '\u{2524}', '\u{2500}'));
        }
    }
    lines.push(border('\u{2558}', '\u{2567}', '\u{255b}', '\u{2550}'));
    lines.join("\n")
}

fn format_plain(
    headers: &[String],
    data: &[Vec<String>],
    widths: &[usize],
    numeric_columns: &[bool],
) -> String {
    let row = |cells: &[String]| -> String {
        cells
            .iter()
            .zip(widths)
            .zip(numeric_columns)
            .map(|((cell, width), numeric)| aligned(cell, *width, *numeric))
            .collect::<Vec<_>>()
            .join(if numeric_columns.iter().any(|numeric| *numeric) {
                "  "
            } else {
                "   "
            })
    };
    let mut lines = Vec::new();
    if !headers.is_empty() {
        lines.push(row(headers));
    }
    lines.extend(data.iter().map(|cells| row(cells)));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_crc_and_choice_prefixes_like_python() {
        assert_eq!(
            strip_crc_prefix("34. <p>6902_b5b6</p> Question"),
            "Question"
        );
        assert_eq!(
            strip_prefix_from_string("<p>A. Choice</p>"),
            "<p>Choice</p>"
        );
        assert_eq!(strip_prefix_from_string("1.5 decimal"), "1.5 decimal");
        assert_eq!(
            strip_prefix_from_string(
                "<table><tbody><tr><td><div>A. box plot</div></td></tr></tbody></table>"
            ),
            "<table><tbody><tr><td><div>box plot</div></td></tr></tbody></table>"
        );
        assert_eq!(
            remove_prefix_from_list(&[
                "<table><tr><td><div>A. first</div></td></tr></table>".into(),
                "<table><tr><td><div>B. second</div></td></tr></table>".into(),
            ]),
            [
                "<table><tr><td><div>first</div></td></tr></table>",
                "<table><tr><td><div>second</div></td></tr></table>",
            ]
        );
        assert_eq!(
            remove_prefix_from_list(&["A. one".into(), "B. two".into()]),
            ["one", "two"]
        );
    }

    #[test]
    fn converts_number_and_script_utilities() {
        assert_eq!(number_to_letter(1), Ok('A'));
        assert_eq!(number_to_lowercase(26), Ok('z'));
        assert_eq!(
            number_to_letter(27),
            Err(StringError::InvalidAlphabetIndex { value: 27 })
        );
        assert_eq!(number_to_roman(1994), "MCMXCIV");
        assert_eq!(number_to_cardinal(101), "one hundred and one");
        assert_eq!(number_to_ordinal(20), Ok("twentieth".into()));
        assert_eq!(number_to_ordinal(21), Ok("twenty-first".into()));
        assert_eq!(
            number_to_ordinal(-1),
            Err(StringError::NegativeOrdinal { value: -1 })
        );
        assert_eq!(
            convert_sub_sup("H<sub>2</sub>O<sup>+</sup>"),
            "H\u{2082}O\u{207a}"
        );
    }

    #[test]
    fn formats_python_table_shapes() {
        assert_eq!(
            html_table_to_text(
                "<table border=\"0\"><tr><th>A</th><th>BB</th></tr><tr><td>1</td><td>22</td></tr></table>"
            ),
            "  A    BB\n  1    22"
        );
        assert_eq!(
            html_table_to_text(
                "<table><tr><th>A</th><th>BB</th></tr><tr><td>1</td><td>22</td></tr></table>"
            ),
            "\u{2552}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2564}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2555}\n\u{2502}   A \u{2502}   BB \u{2502}\n\u{255e}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{256a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2561}\n\u{2502}   1 \u{2502}   22 \u{2502}\n\u{2558}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2567}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255b}"
        );
        assert_eq!(
            html_table_to_text(
                "<table><tr><th>A</th><th>BB</th></tr><tr><td>x</td><td>yy</td></tr></table>"
            ),
            "\u{2552}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2564}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2555}\n\u{2502} A   \u{2502} BB   \u{2502}\n\u{255e}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{256a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2561}\n\u{2502} x   \u{2502} yy   \u{2502}\n\u{2558}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2567}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255b}"
        );
        assert_eq!(
            html_table_to_text(
                "<table style=\"border-collapse: collapse; border: 0\"><tr><th>A</th><th>B</th></tr><tr><td>x</td><td>y</td></tr></table>"
            ),
            "\u{2552}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2564}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2555}\n\u{2502} A   \u{2502} B   \u{2502}\n\u{255e}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{256a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2561}\n\u{2502} x   \u{2502} y   \u{2502}\n\u{2558}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2567}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255b}"
        );
    }

    #[test]
    fn preserves_authored_numeric_spelling_in_inline_tables() {
        let rendered = html_table_to_text(
            "<table style=\"border-collapse: collapse\"><tr><td>&half;(2,729)</td></tr><tr><td>4,400</td></tr><tr><td>2.80</td></tr></table>",
        );

        assert!(rendered.contains("\u{00bd}(2,729)"));
        assert!(rendered.contains("4,400"));
        assert!(rendered.contains("2.80"));
    }
}
