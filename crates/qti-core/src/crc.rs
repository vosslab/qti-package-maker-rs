//! Python-compatible CRC16-XMODEM item identities.

use std::fmt;
use std::sync::LazyLock;

use crc::{CRC_16_XMODEM, Crc};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::item::ItemBody;

const XMODEM: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

/// A failure while constructing a CRC from authored item content.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CrcError {
    /// Python hashes ASCII bytes and rejects text that cannot be encoded as ASCII.
    #[error("cannot encode string to ASCII: {input:?}")]
    NonAscii { input: String },
}

/// The two CRC16 values that identify an item in the Python implementation.
///
/// The packed representation keeps the value `Copy`, while its fields remain private and its
/// display/serialization form stays the stable `question_secondary` lowercase hexadecimal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemCrc(u32);

impl ItemCrc {
    /// Construct an item identity from already-normalized question and secondary text.
    pub fn new(question: &str, secondary: &str) -> Result<Self, CrcError> {
        let question_crc = crc_value(question)?;
        let secondary_crc = crc_value(secondary)?;
        Ok(Self(
            (u32::from(question_crc) << 16) | u32::from(secondary_crc),
        ))
    }

    /// Return the question component of this item identity.
    pub fn question_crc(self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Return the item-body component of this item identity.
    pub fn secondary_crc(self) -> u16 {
        self.0 as u16
    }
}

impl fmt::Display for ItemCrc {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04x}_{:04x}",
            self.question_crc(),
            self.secondary_crc()
        )
    }
}

impl Serialize for ItemCrc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ItemCrc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let (question, secondary) = encoded
            .split_once('_')
            .ok_or_else(|| serde::de::Error::custom("item CRC must contain one underscore"))?;
        if question.len() != 4 || secondary.len() != 4 || encoded.matches('_').count() != 1 {
            return Err(serde::de::Error::custom(
                "item CRC must contain two four-digit hexadecimal fields",
            ));
        }
        let question = u16::from_str_radix(question, 16)
            .map_err(|_| serde::de::Error::custom("invalid question CRC hexadecimal field"))?;
        let secondary = u16::from_str_radix(secondary, 16)
            .map_err(|_| serde::de::Error::custom("invalid secondary CRC hexadecimal field"))?;
        Ok(Self((u32::from(question) << 16) | u32::from(secondary)))
    }
}

/// Return the lowercase CRC16-XMODEM checksum of ASCII text, matching Python's `crcmod` call.
pub fn get_crc16_from_string(value: &str) -> Result<String, CrcError> {
    Ok(format!("{:04x}", crc_value(value)?))
}

fn crc_value(value: &str) -> Result<u16, CrcError> {
    if !value.is_ascii() {
        return Err(CrcError::NonAscii {
            input: value.to_owned(),
        });
    }
    Ok(XMODEM.checksum(value.as_bytes()))
}

/// Construct the raw per-item-type string whose CRC forms the secondary identity component.
///
/// The input body is deliberately unnormalized: Python computes this CRC before removing choice
/// prefixes from the stored values.
pub fn secondary_string(body: &ItemBody) -> String {
    match body {
        ItemBody::Mc { choices, .. } | ItemBody::Ma { choices, .. } => choices.join("|"),
        ItemBody::Match { prompts, choices } => prompts
            .iter()
            .chain(choices)
            .cloned()
            .collect::<Vec<_>>()
            .join("|"),
        ItemBody::Num {
            answer, tolerance, ..
        } => format!(
            "{}_{}",
            python_scientific(*answer),
            python_scientific(*tolerance)
        ),
        ItemBody::Fib { answers } | ItemBody::Order { answers } => answers.join("|"),
        ItemBody::MultiFib { answers } => answers
            .iter()
            .map(|(key, values)| format!("{key}:{}", python_list_repr(values)))
            .collect::<Vec<_>>()
            .join("|"),
    }
}

fn python_scientific(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf"
        } else {
            "inf"
        }
        .to_owned();
    }
    let rendered = format!("{value:.2e}");
    let (mantissa, exponent) = rendered
        .split_once('e')
        .expect("Rust scientific float formatting includes an exponent");
    let exponent = exponent
        .parse::<i32>()
        .expect("Rust scientific float exponent is an integer");
    format!("{mantissa}e{exponent:+03}")
}

fn python_list_repr(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| python_string_repr(value))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

fn python_string_repr(value: &str) -> String {
    // Python repr escapes Unicode separators and non-printable characters before ASCII CRC
    // encoding. Printable authored Unicode continues to fail that existing encoding boundary.
    static NON_PRINTABLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[\p{C}\p{Z}]$").expect("Unicode character category expression is valid")
    });
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut result = String::new();
    result.push(quote);
    for character in value.chars() {
        match character {
            '\\' => result.push_str("\\\\"),
            character if character == quote => {
                result.push('\\');
                result.push(character);
            }
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{08}' => result.push_str("\\x08"),
            '\u{0c}' => result.push_str("\\x0c"),
            character
                if character != ' '
                    && NON_PRINTABLE.is_match(character.encode_utf8(&mut [0; 4])) =>
            {
                use std::fmt::Write;
                let code = character as u32;
                match code {
                    0..=0xff => write!(&mut result, "\\x{code:02x}"),
                    0x100..=0xffff => write!(&mut result, "\\u{code:04x}"),
                    _ => write!(&mut result, "\\U{code:08x}"),
                }
                .expect("writing to a string cannot fail");
            }
            character => result.push(character),
        }
    }
    result.push(quote);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn crc_xmodem_matches_python_known_values() {
        assert_eq!(get_crc16_from_string(""), Ok("0000".into()));
        assert_eq!(get_crc16_from_string("123456789"), Ok("31c3".into()));
        assert_eq!(get_crc16_from_string("Question?"), Ok("3891".into()));
        assert_eq!(get_crc16_from_string("A. one|B. two"), Ok("574a".into()));
        assert_eq!(
            get_crc16_from_string("é"),
            Err(CrcError::NonAscii { input: "é".into() })
        );
    }

    #[test]
    fn secondary_strings_match_each_python_item_shape() {
        assert_eq!(
            secondary_string(&ItemBody::Mc {
                choices: vec!["A. yes".into(), "B. no".into()],
                answer: "A. yes".into()
            }),
            "A. yes|B. no"
        );
        assert_eq!(
            secondary_string(&ItemBody::Ma {
                choices: vec!["A".into(), "B".into()],
                answers: vec!["A".into()],
                min_answers_required: 1,
                allow_all_correct: true
            }),
            "A|B"
        );
        assert_eq!(
            secondary_string(&ItemBody::Match {
                prompts: vec!["p1".into()],
                choices: vec!["c1".into(), "c2".into()]
            }),
            "p1|c1|c2"
        );
        assert_eq!(
            secondary_string(&ItemBody::Num {
                answer: 1.0,
                tolerance: 0.25,
                tolerance_message: true
            }),
            "1.00e+00_2.50e-01"
        );
        assert_eq!(
            secondary_string(&ItemBody::Fib {
                answers: vec!["one".into(), "two".into()]
            }),
            "one|two"
        );
        let answers = BTreeMap::from([
            ("b".into(), vec!["z".into()]),
            ("a".into(), vec!["x".into(), "y".into()]),
        ]);
        assert_eq!(
            secondary_string(&ItemBody::MultiFib { answers }),
            "a:['x', 'y']|b:['z']"
        );
        let answers = BTreeMap::from([("path".into(), vec!["Zed's\\path".into()])]);
        assert_eq!(
            secondary_string(&ItemBody::MultiFib { answers }),
            "path:[\"Zed's\\\\path\"]"
        );
        assert_eq!(
            secondary_string(&ItemBody::Order {
                answers: vec!["first".into(), "second".into()]
            }),
            "first|second"
        );
    }

    #[test]
    fn item_crc_is_copyable_displayable_and_string_serializable() {
        let item_crc = ItemCrc::new("Question?", "A. one|B. two").unwrap();
        assert_eq!(item_crc.to_string(), "3891_574a");
        let copied = item_crc;
        assert_eq!(copied, item_crc);
        assert_eq!(serde_json::to_string(&item_crc).unwrap(), "\"3891_574a\"");
        assert_eq!(
            serde_json::from_str::<ItemCrc>("\"3891_574a\"").unwrap(),
            item_crc
        );
    }

    #[test]
    fn scientific_formatting_matches_python_edge_values() {
        assert_eq!(python_scientific(-12.34), "-1.23e+01");
        assert_eq!(python_scientific(-0.0), "-0.00e+00");
        assert_eq!(python_scientific(1e20), "1.00e+20");
        assert_eq!(python_scientific(2.675), "2.67e+00");
        assert_eq!(python_scientific(9.995), "9.99e+00");
    }
}
