//! Stable generated-image names and accessible alternative text.

use scraper::{Html, Selector};

use crate::CanvasSource;
use qti_core::ItemCrc;

/// The reserved directory leaf used for generated images when it is unoccupied.
pub(crate) const GENERATED_DIRECTORY: &str = "__qti_generated";

/// Returns Python-compatible generated-image leaf name using the original item CRC.
pub(crate) fn generated_leaf_name(crc: ItemCrc, family: &str, number: usize) -> String {
    format!("{crc}_{family}_{number}.png")
}

/// Chooses an unoccupied generated-image directory name deterministically.
pub(crate) fn generated_directory<'a>(
    occupied_sources: impl IntoIterator<Item = &'a str>,
) -> String {
    reserved_directory(GENERATED_DIRECTORY, occupied_sources)
}

/// Chooses an unoccupied directory for conversion-owned material.
pub(crate) fn reserved_directory<'a>(
    preferred: &str,
    occupied_sources: impl IntoIterator<Item = &'a str>,
) -> String {
    let occupied = occupied_sources.into_iter().collect::<Vec<_>>();
    let mut suffix = 0_usize;
    loop {
        let candidate = if suffix == 0 {
            preferred.to_owned()
        } else {
            format!("{preferred}_{suffix}")
        };
        let prefix = format!("{candidate}/");
        if occupied
            .iter()
            .all(|source| **source != candidate && !source.starts_with(&prefix))
        {
            return candidate;
        }
        suffix += 1;
    }
}

/// Collapses table-cell text to Python-compatible ASCII alternative text.
pub(crate) fn table_alt_text(table_html: &str) -> String {
    let document = Html::parse_fragment(table_html);
    let selector = Selector::parse("td, th").expect("static selector parses");
    let joined = document
        .select(&selector)
        .filter_map(|cell| {
            let text = cell
                .text()
                .flat_map(str::chars)
                .map(|character| {
                    if character.is_whitespace() {
                        ' '
                    } else {
                        character
                    }
                })
                .filter(char::is_ascii)
                .collect::<String>();
            (!text.is_empty()).then_some(text)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let collapsed = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        "table drawing".to_owned()
    } else {
        collapsed
    }
}

/// Builds the accessible canvas description retained by the Python converter.
pub(crate) fn canvas_alt_text(source: &CanvasSource) -> String {
    source.legend.as_ref().map_or_else(
        || format!("SMILES {}", source.smiles),
        |legend| format!("{legend} (SMILES {})", source.smiles),
    )
}

#[cfg(test)]
mod tests {
    use super::{canvas_alt_text, generated_directory, generated_leaf_name, table_alt_text};
    use crate::CanvasSource;
    use qti_core::ItemCrc;

    #[test]
    fn generated_names_keep_original_crc_and_reserve_authored_sources() {
        let crc = ItemCrc::new("question", "secondary").expect("valid crc");
        assert_eq!(
            generated_leaf_name(crc, "table", 2),
            format!("{crc}_table_2.png")
        );
        assert_eq!(
            generated_directory(["__qti_generated/diagram.png", "plain.png"]),
            "__qti_generated_1"
        );
    }

    #[test]
    fn alternative_text_is_ascii_and_matches_canvas_rules() {
        assert_eq!(
            table_alt_text("<table><tr><td>A&nbsp;B</td><th>C</th></tr></table>"),
            "A B C"
        );
        let source = CanvasSource {
            smiles: "CCO".to_owned(),
            legend: Some("ethanol".to_owned()),
            explicit_methyl: false,
            width: 100,
            height: 80,
            highlight_atoms: Vec::new(),
            highlight_bonds: Vec::new(),
            highlight_peptide_bonds: false,
            highlight_colour: None,
        };
        assert_eq!(canvas_alt_text(&source), "ethanol (SMILES CCO)");
    }
}
