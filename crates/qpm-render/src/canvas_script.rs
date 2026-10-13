//! Parser for the deliberately small RDKit drawing-script grammar.
//!
//! ASVS V1.3.2 and V2.2.1: input is positively validated and never executed.

use crate::{CanvasSource, MAX_CANVAS_DIMENSION};
use regex::Regex;
use thiserror::Error;

const MAX_SMILES_BYTES: usize = 4096;

/// A static RDKit canvas-script grammar violation.
#[derive(Debug, Error, PartialEq)]
pub enum CanvasScriptError {
    #[error("RDKit canvas is missing width or height")]
    MissingDimensions,
    #[error("RDKit canvas width and height must be ASCII integers")]
    InvalidDimensions,
    #[error(
        "RDKit canvas dimensions must be positive and within the supported limit ({width}x{height})"
    )]
    DimensionOutOfRange { width: u32, height: u32 },
    #[error("RDKit script must call get_mol(smiles) exactly once")]
    GetMolCount,
    #[error("RDKit script must contain one supported canvas draw call")]
    DrawCallCount,
    #[error("RDKit script must define one static SMILES string")]
    SmilesCount,
    #[error("RDKit canvas SMILES must not be empty")]
    EmptySmiles,
    #[error("RDKit canvas SMILES exceeds the supported length")]
    SmilesTooLong,
    #[error("RDKit drawing options must use a static empty mdetails object")]
    InvalidDetailsInitializer,
    #[error("RDKit drawing options must use static string keys")]
    DynamicDetailsKey,
    #[error("RDKit drawing options must not assign a key more than once")]
    DuplicateDetailsKey,
    #[error("unsupported RDKit drawing option: {0}")]
    UnsupportedDetailsKey(String),
    #[error("RDKit canvas legend must be a static string")]
    InvalidLegend,
    #[error("RDKit explicitMethyl must be a boolean")]
    InvalidExplicitMethyl,
    #[error("RDKit atom highlights must be a static integer list")]
    InvalidAtomHighlights,
    #[error("RDKit bond highlights must be a static list or peptide bonds")]
    InvalidBondHighlights,
    #[error("RDKit highlight colour must be an RGB list")]
    InvalidHighlightColour,
    #[error("RDKit highlight colour components must be between 0 and 1")]
    HighlightColourOutOfRange,
}

/// Parses a static script paired with a canvas's declared dimensions.
pub fn parse_canvas_script(
    script: &str,
    width_text: Option<&str>,
    height_text: Option<&str>,
) -> Result<CanvasSource, CanvasScriptError> {
    let (width, height) = parse_dimensions(width_text, height_text)?;
    if matches(&get_mol_re(), script).count() != 1 {
        return Err(CanvasScriptError::GetMolCount);
    }
    if matches(&draw_call_re(), script).count() != 1 {
        return Err(CanvasScriptError::DrawCallCount);
    }
    let smiles = captures(&smiles_re(), script);
    if smiles.len() != 1 {
        return Err(CanvasScriptError::SmilesCount);
    }
    let smiles = smiles[0][1].clone();
    if smiles.is_empty() {
        return Err(CanvasScriptError::EmptySmiles);
    }
    if smiles.len() > MAX_SMILES_BYTES {
        return Err(CanvasScriptError::SmilesTooLong);
    }

    let initializers = captures(&details_initializer_re(), script)
        .into_iter()
        .filter(|initializer| !initializer[1].trim_start().starts_with('='))
        .collect::<Vec<_>>();
    if initializers.len() != 1 || initializers[0][1].trim() != "{}" {
        return Err(CanvasScriptError::InvalidDetailsInitializer);
    }
    let keys = captures(&details_assignment_re(), script)
        .into_iter()
        .map(|capture| capture[1].clone())
        .collect::<Vec<_>>();
    if keys.len() != matches(&details_member_assignment_re(), script).count() {
        return Err(CanvasScriptError::DynamicDetailsKey);
    }
    let mut unique_keys = keys.clone();
    unique_keys.sort_unstable();
    unique_keys.dedup();
    if unique_keys.len() != keys.len() {
        return Err(CanvasScriptError::DuplicateDetailsKey);
    }
    for key in &keys {
        if !matches!(
            key.as_str(),
            "legend" | "explicitMethyl" | "atoms" | "bonds" | "highlightColour"
        ) {
            return Err(CanvasScriptError::UnsupportedDetailsKey(key.clone()));
        }
    }

    let legend = option_string(
        script,
        &keys,
        "legend",
        &legend_re(),
        CanvasScriptError::InvalidLegend,
    )?;
    let explicit_methyl = option_boolean(script, &keys)?;
    let highlight_atoms = option_integer_list(
        script,
        &keys,
        "atoms",
        &atoms_re(),
        CanvasScriptError::InvalidAtomHighlights,
    )?;
    let (highlight_bonds, highlight_peptide_bonds) = option_bonds(script, &keys)?;
    let highlight_colour = option_colour(script, &keys)?;

    Ok(CanvasSource {
        smiles,
        legend,
        explicit_methyl,
        width,
        height,
        highlight_atoms,
        highlight_bonds,
        highlight_colour,
        highlight_peptide_bonds,
    })
}

fn parse_dimensions(
    width: Option<&str>,
    height: Option<&str>,
) -> Result<(u32, u32), CanvasScriptError> {
    let (Some(width), Some(height)) = (width, height) else {
        return Err(CanvasScriptError::MissingDimensions);
    };
    if !is_ascii_digits(width) || !is_ascii_digits(height) || width.len() > 4 || height.len() > 4 {
        return Err(CanvasScriptError::InvalidDimensions);
    }
    let width = width.parse().expect("four ASCII digits fit u32");
    let height = height.parse().expect("four ASCII digits fit u32");
    if width == 0 || height == 0 || width > MAX_CANVAS_DIMENSION || height > MAX_CANVAS_DIMENSION {
        return Err(CanvasScriptError::DimensionOutOfRange { width, height });
    }
    Ok((width, height))
}

fn is_ascii_digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn option_string(
    script: &str,
    keys: &[String],
    name: &str,
    regex: &Regex,
    error: CanvasScriptError,
) -> Result<Option<String>, CanvasScriptError> {
    let values = captures(regex, script);
    if keys.iter().any(|key| key == name) && values.len() != 1 {
        return Err(error);
    }
    Ok(values.first().map(|capture| capture[1].clone()))
}

fn option_boolean(script: &str, keys: &[String]) -> Result<bool, CanvasScriptError> {
    let values = captures(&explicit_methyl_re(), script);
    if keys.iter().any(|key| key == "explicitMethyl") && values.len() != 1 {
        return Err(CanvasScriptError::InvalidExplicitMethyl);
    }
    Ok(values.first().is_some_and(|capture| capture[1] == "true"))
}

fn option_integer_list(
    script: &str,
    keys: &[String],
    name: &str,
    regex: &Regex,
    error: CanvasScriptError,
) -> Result<Vec<i32>, CanvasScriptError> {
    let values = captures(regex, script);
    if keys.iter().any(|key| key == name) && values.len() != 1 {
        return Err(error);
    }
    values.first().map_or(Ok(Vec::new()), |capture| {
        parse_integer_list(&capture[1]).map_err(|()| error)
    })
}

fn option_bonds(script: &str, keys: &[String]) -> Result<(Vec<i32>, bool), CanvasScriptError> {
    let values = captures(&bonds_re(), script);
    if keys.iter().any(|key| key == "bonds") && values.len() != 1 {
        return Err(CanvasScriptError::InvalidBondHighlights);
    }
    let Some(value) = values.first() else {
        return Ok((Vec::new(), false));
    };
    let value = value[1].trim();
    if value == "getPeptideBonds(mol)" {
        return Ok((Vec::new(), true));
    }
    Ok((
        parse_integer_list(value).map_err(|()| CanvasScriptError::InvalidBondHighlights)?,
        false,
    ))
}

fn option_colour(script: &str, keys: &[String]) -> Result<Option<[f64; 3]>, CanvasScriptError> {
    let values = captures(&colour_re(), script);
    if keys.iter().any(|key| key == "highlightColour") && values.len() != 1 {
        return Err(CanvasScriptError::InvalidHighlightColour);
    }
    let Some(value) = values.first() else {
        return Ok(None);
    };
    let parsed = [
        value[1]
            .parse()
            .map_err(|_| CanvasScriptError::InvalidHighlightColour)?,
        value[2]
            .parse()
            .map_err(|_| CanvasScriptError::InvalidHighlightColour)?,
        value[3]
            .parse()
            .map_err(|_| CanvasScriptError::InvalidHighlightColour)?,
    ];
    if parsed
        .iter()
        .any(|component: &f64| !(0.0..=1.0).contains(component))
    {
        return Err(CanvasScriptError::HighlightColourOutOfRange);
    }
    Ok(Some(parsed))
}

fn parse_integer_list(value: &str) -> Result<Vec<i32>, ()> {
    if !integer_list_re().is_match(value) {
        return Err(());
    }
    let contents = value[1..value.len() - 1].trim();
    if contents.is_empty() {
        return Ok(Vec::new());
    }
    contents
        .split(',')
        .map(|item| item.trim().parse().map_err(|_| ()))
        .collect()
}

fn captures(regex: &Regex, input: &str) -> Vec<Vec<String>> {
    regex
        .captures_iter(input)
        .map(|capture| {
            capture
                .iter()
                .map(|item| item.map_or("", |entry| entry.as_str()).to_owned())
                .collect()
        })
        .collect()
}
fn matches<'a>(regex: &Regex, input: &'a str) -> impl Iterator<Item = regex::Match<'a>> {
    regex.find_iter(input)
}

fn smiles_re() -> Regex {
    Regex::new(r#"\bsmiles\s*=\s*\"([^\"]*)\"\s*;"#).expect("valid regex")
}
fn legend_re() -> Regex {
    Regex::new(r#"mdetails\s*\[\s*\"legend\"\s*\]\s*=\s*\"([^\"\\\r\n]*)\"\s*;"#)
        .expect("valid regex")
}
fn explicit_methyl_re() -> Regex {
    Regex::new(r#"mdetails\s*\[\s*\"explicitMethyl\"\s*\]\s*=\s*(true|false)\b"#)
        .expect("valid regex")
}
fn atoms_re() -> Regex {
    Regex::new(r#"mdetails\s*\[\s*\"atoms\"\s*\]\s*=\s*(\[[^]]*\])"#).expect("valid regex")
}
fn bonds_re() -> Regex {
    Regex::new(r#"mdetails\s*\[\s*\"bonds\"\s*\]\s*=\s*([^;]+)"#).expect("valid regex")
}
fn colour_re() -> Regex {
    Regex::new(r#"mdetails\s*\[\s*\"highlightColour\"\s*\]\s*=\s*\[\s*([0-9]+(?:\.[0-9]+)?|\.[0-9]+)\s*,\s*([0-9]+(?:\.[0-9]+)?|\.[0-9]+)\s*,\s*([0-9]+(?:\.[0-9]+)?|\.[0-9]+)\s*\]"#).expect("valid regex")
}
fn details_initializer_re() -> Regex {
    Regex::new(r"\bmdetails\s*=\s*([^;]+)").expect("valid regex")
}
fn details_assignment_re() -> Regex {
    Regex::new(r#"\bmdetails\s*\[\s*\"([^\"]+)\"\s*\]\s*="#).expect("valid regex")
}
fn details_member_assignment_re() -> Regex {
    Regex::new(r"\bmdetails\s*(?:\[[^]]+\]|\.\s*[A-Za-z_$][\w$]*)\s*=").expect("valid regex")
}
fn get_mol_re() -> Regex {
    Regex::new(r"get_mol\s*\(\s*smiles\s*\)").expect("valid regex")
}
fn draw_call_re() -> Regex {
    Regex::new(r"\.draw_to_canvas(?:_with_highlights)?\s*\(").expect("valid regex")
}
fn integer_list_re() -> Regex {
    Regex::new(r"^\[\s*(?:[0-9]+\s*(?:,\s*[0-9]+\s*)*)?\]$").expect("valid regex")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> &'static str {
        r#"let smiles="CC(=O)NCC(=O)O";let mol=RDKitModule.get_mol(smiles);let mdetails={};mdetails["bonds"]=getPeptideBonds(mol);mdetails["atoms"]=[0, 2];mdetails["highlightColour"]=[0,1,.5];mdetails["legend"]="peptide";mdetails["explicitMethyl"]=true;mol.draw_to_canvas_with_highlights(canvas,JSON.stringify(mdetails));"#
    }
    #[test]
    fn parses_every_supported_option() {
        let source =
            parse_canvas_script(valid(), Some("480"), Some("512")).expect("valid static grammar");
        assert_eq!(source.legend.as_deref(), Some("peptide"));
        assert_eq!(source.highlight_atoms, [0, 2]);
        assert!(source.highlight_peptide_bonds);
        assert_eq!(source.highlight_colour, Some([0.0, 1.0, 0.5]));
    }
    #[test]
    fn rejects_dynamic_options_without_execution() {
        let script = valid().replace("[0, 2]", "getAtoms(mol)");
        assert_eq!(
            parse_canvas_script(&script, Some("120"), Some("80")),
            Err(CanvasScriptError::InvalidAtomHighlights)
        );
    }
    #[test]
    fn rejects_limits_and_duplicate_keys() {
        let duplicate = valid().replace(
            "mdetails[\"legend\"]=\"peptide\";",
            "mdetails[\"legend\"]=\"a\";mdetails[\"legend\"]=\"b\";",
        );
        assert_eq!(
            parse_canvas_script(&duplicate, Some("120"), Some("80")),
            Err(CanvasScriptError::DuplicateDetailsKey)
        );
        assert_eq!(
            parse_canvas_script(valid(), Some("4097"), Some("80")),
            Err(CanvasScriptError::DimensionOutOfRange {
                width: 4097,
                height: 80
            })
        );
    }

    #[test]
    fn accepts_the_pinned_python_comment_form() {
        let script = r#"let/* */smiles="CCO";let/* */mol=RDKitModule.get_mol(smiles);let/* */mdetails={};mdetails["legend"]="ethanol";mdetails["explicitMethyl"]=true;mol.draw_to_canvas_with_highlights(canvas,JSON.stringify(mdetails));"#;
        let source = parse_canvas_script(script, Some("320"), Some("240")).expect("Python form");
        assert_eq!(source.legend.as_deref(), Some("ethanol"));
        assert!(source.explicit_methyl);
    }

    #[test]
    fn extracts_receiver_agnostic_python_draw_spelling_without_execution() {
        let script = valid().replace(
            "mol.draw_to_canvas_with_highlights(canvas,JSON.stringify(mdetails))",
            "attacker.draw_to_canvas(evil())",
        );
        let source = parse_canvas_script(&script, Some("120"), Some("80"))
            .expect("pinned extractor grammar accepts the spelling");
        assert_eq!(source.smiles, "CC(=O)NCC(=O)O");
    }

    #[test]
    fn rejects_each_static_option_violation() {
        let cases = [
            (
                valid().replace("let mdetails={};", "let mdetails={\"atoms\":[0]};"),
                CanvasScriptError::InvalidDetailsInitializer,
            ),
            (
                valid().replace("mdetails[\"atoms\"]", "mdetails.atoms"),
                CanvasScriptError::DynamicDetailsKey,
            ),
            (
                valid().replace("[0, 2]", "[-1]"),
                CanvasScriptError::InvalidAtomHighlights,
            ),
            (
                valid().replace("getPeptideBonds(mol)", "getBonds(mol)"),
                CanvasScriptError::InvalidBondHighlights,
            ),
            (
                valid().replace("[0,1,.5]", "[1.1,0,0]"),
                CanvasScriptError::HighlightColourOutOfRange,
            ),
            (
                valid().replace("\"peptide\"", "dynamicLegend"),
                CanvasScriptError::InvalidLegend,
            ),
            (
                valid().replace("=true", "=1"),
                CanvasScriptError::InvalidExplicitMethyl,
            ),
        ];
        for (script, expected) in cases {
            assert_eq!(
                parse_canvas_script(&script, Some("120"), Some("80")),
                Err(expected)
            );
        }
    }

    #[test]
    fn rejects_each_required_script_and_input_limit() {
        let cases = [
            (
                valid().replace("get_mol(smiles)", "get_mol(smiles);mol.get_mol(smiles)"),
                Some("120"),
                Some("80"),
                CanvasScriptError::GetMolCount,
            ),
            (
                valid().replace(
                    "draw_to_canvas_with_highlights",
                    "draw_to_canvas_with_highlights();mol.draw_to_canvas",
                ),
                Some("120"),
                Some("80"),
                CanvasScriptError::DrawCallCount,
            ),
            (
                valid().replace("smiles=\"CC(=O)NCC(=O)O\"", "smiles=\"\""),
                Some("120"),
                Some("80"),
                CanvasScriptError::EmptySmiles,
            ),
            (
                valid().replace("smiles=\"CC(=O)NCC(=O)O\"", "smiles=dynamicSmiles"),
                Some("120"),
                Some("80"),
                CanvasScriptError::SmilesCount,
            ),
            (
                valid().to_owned(),
                Some("0"),
                Some("80"),
                CanvasScriptError::DimensionOutOfRange {
                    width: 0,
                    height: 80,
                },
            ),
            (
                valid().to_owned(),
                Some("12x"),
                Some("80"),
                CanvasScriptError::InvalidDimensions,
            ),
        ];
        for (script, width, height, expected) in cases {
            assert_eq!(parse_canvas_script(&script, width, height), Err(expected));
        }
        let long = format!("let smiles=\"{}\";", "C".repeat(4097));
        let long = valid().replacen("let smiles=\"CC(=O)NCC(=O)O\";", &long, 1);
        assert_eq!(
            parse_canvas_script(&long, Some("120"), Some("80")),
            Err(CanvasScriptError::SmilesTooLong)
        );
    }
}
