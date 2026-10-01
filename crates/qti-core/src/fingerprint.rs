//! Structural item fingerprints for round-trip parity checks.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use lol_html::{RewriteStrSettings, element, rewrite_str};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::item::{Item, ItemBody, ItemKind};

/// The position of a media reference in an item's HTML-bearing fields.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum FieldId {
    /// The item question stem.
    Stem,
    /// A choice, indexed within its choice list.
    Choice(usize),
    /// An answer, indexed within its answer list.
    Answer(usize),
    /// A matching prompt, indexed within its prompt list.
    Prompt(usize),
    /// A MULTI_FIB answer value.
    MultiFibAnswer { key: String, answer: usize },
}

/// A resolved media dependency.  It deliberately stores no path or filename.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct MediaRef {
    /// SHA-256 bytes of the resolved asset.
    pub content_hash: [u8; 32],
    /// The HTML field containing the reference.
    pub field: FieldId,
    /// The reference's document-order position within `field`.
    pub ordinal: usize,
}

/// A mismatch between HTML image locations and their resolved media dependencies.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FingerprintError {
    /// More than one resolved dependency claims the same image position.
    #[error("duplicate media reference at {field:?} ordinal {ordinal}")]
    DuplicateMediaRef { field: FieldId, ordinal: usize },
    /// HTML contains an image for which the resolver supplied no content hash.
    #[error("missing media reference at {field:?} ordinal {ordinal}")]
    MissingMediaRef { field: FieldId, ordinal: usize },
    /// The resolver supplied a reference which does not correspond to an HTML image.
    #[error("unexpected media reference at {field:?} ordinal {ordinal}")]
    UnexpectedMediaRef { field: FieldId, ordinal: usize },
    /// The HTML rewriter could not safely inspect an HTML-bearing field.
    #[error("cannot normalize media in {field:?}: {reason}")]
    MalformedHtml { field: FieldId, reason: String },
}

/// Complete semantic content used to compare assessment items after a round trip.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemFingerprint {
    question_text: String,
    kind: ItemKind,
    body: FingerprintBody,
    media: Vec<MediaRef>,
}

#[derive(Clone, Debug, PartialEq)]
enum FingerprintBody {
    Mc {
        choices: Vec<String>,
        answer: String,
    },
    Ma {
        choices: Vec<String>,
        answers: Vec<String>,
        min_answers_required: usize,
        allow_all_correct: bool,
    },
    Match {
        prompts: Vec<String>,
        choices: Vec<String>,
    },
    Num {
        answer: f64,
        tolerance: f64,
        tolerance_message: bool,
    },
    Fib {
        answers: Vec<String>,
    },
    MultiFib {
        answers: BTreeMap<String, Vec<String>>,
    },
    Order {
        answers: Vec<String>,
    },
}

impl ItemFingerprint {
    /// Captures all item semantics plus resolved media placement.
    pub fn new(item: &Item, media: Vec<MediaRef>) -> Result<Self, FingerprintError> {
        validate_media_refs(&media)?;
        let mut used = Vec::new();
        let question_text = normalize_media_html(
            &item.common().question_text,
            FieldId::Stem,
            &media,
            &mut used,
        )?;
        let body = fingerprint_body(item.body(), &media, &mut used)?;
        if let Some(reference) = media.iter().find(|reference| {
            !used
                .iter()
                .any(|used_reference| *used_reference == **reference)
        }) {
            return Err(FingerprintError::UnexpectedMediaRef {
                field: reference.field.clone(),
                ordinal: reference.ordinal,
            });
        }
        Ok(Self {
            question_text,
            kind: item.kind(),
            body,
            media,
        })
    }

    /// Returns the media references in their supplied order.
    #[must_use]
    pub fn media(&self) -> &[MediaRef] {
        &self.media
    }
}

// `Item::new` rejects NaN, so `f64` equality is reflexive for every value that can enter a
// fingerprint.  Normalize signed zero in hashing to preserve the `Eq`/`Hash` contract.
impl Eq for ItemFingerprint {}

impl Hash for ItemFingerprint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.question_text.hash(state);
        self.kind.hash(state);
        hash_body(&self.body, state);
        self.media.hash(state);
    }
}

fn hash_body<H: Hasher>(body: &FingerprintBody, state: &mut H) {
    std::mem::discriminant(body).hash(state);
    match body {
        FingerprintBody::Mc { choices, answer } => {
            choices.hash(state);
            answer.hash(state);
        }
        FingerprintBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => {
            choices.hash(state);
            answers.hash(state);
            min_answers_required.hash(state);
            allow_all_correct.hash(state);
        }
        FingerprintBody::Match { prompts, choices } => {
            prompts.hash(state);
            choices.hash(state);
        }
        FingerprintBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            hash_finite_float(*answer, state);
            hash_finite_float(*tolerance, state);
            tolerance_message.hash(state);
        }
        FingerprintBody::Fib { answers } | FingerprintBody::Order { answers } => {
            answers.hash(state)
        }
        FingerprintBody::MultiFib { answers } => {
            for (key, values) in answers {
                key.hash(state);
                values.hash(state);
            }
        }
    }
}

fn hash_finite_float<H: Hasher>(value: f64, state: &mut H) {
    let bits = if value == 0.0 { 0 } else { value.to_bits() };
    bits.hash(state);
}

fn fingerprint_body(
    body: &ItemBody,
    media: &[MediaRef],
    used: &mut Vec<MediaRef>,
) -> Result<FingerprintBody, FingerprintError> {
    let mut normalize_list = |values: &[String], field: fn(usize) -> FieldId| {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| normalize_media_html(value, field(index), media, used))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(match body {
        ItemBody::Mc { choices, answer } => FingerprintBody::Mc {
            choices: normalize_list(choices, FieldId::Choice)?,
            answer: normalize_media_html(answer, FieldId::Answer(0), media, used)?,
        },
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => FingerprintBody::Ma {
            choices: normalize_list(choices, FieldId::Choice)?,
            answers: normalize_list(answers, FieldId::Answer)?,
            min_answers_required: *min_answers_required,
            allow_all_correct: *allow_all_correct,
        },
        ItemBody::Match { prompts, choices } => FingerprintBody::Match {
            prompts: normalize_list(prompts, FieldId::Prompt)?,
            choices: normalize_list(choices, FieldId::Choice)?,
        },
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => FingerprintBody::Num {
            answer: *answer,
            tolerance: *tolerance,
            tolerance_message: *tolerance_message,
        },
        ItemBody::Fib { answers } => FingerprintBody::Fib {
            answers: normalize_list(answers, FieldId::Answer)?,
        },
        ItemBody::MultiFib { answers } => FingerprintBody::MultiFib {
            answers: answers
                .iter()
                .map(|(key, values)| {
                    let values = values
                        .iter()
                        .enumerate()
                        .map(|(index, value)| {
                            normalize_media_html(
                                value,
                                FieldId::MultiFibAnswer {
                                    key: key.clone(),
                                    answer: index,
                                },
                                media,
                                used,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok((key.clone(), values))
                })
                .collect::<Result<BTreeMap<_, _>, FingerprintError>>()?,
        },
        ItemBody::Order { answers } => FingerprintBody::Order {
            answers: normalize_list(answers, FieldId::Answer)?,
        },
    })
}

fn validate_media_refs(media: &[MediaRef]) -> Result<(), FingerprintError> {
    for (index, reference) in media.iter().enumerate() {
        if media[..index].iter().any(|previous| {
            previous.field == reference.field && previous.ordinal == reference.ordinal
        }) {
            return Err(FingerprintError::DuplicateMediaRef {
                field: reference.field.clone(),
                ordinal: reference.ordinal,
            });
        }
    }
    Ok(())
}

fn normalize_media_html(
    html: &str,
    field: FieldId,
    media: &[MediaRef],
    used: &mut Vec<MediaRef>,
) -> Result<String, FingerprintError> {
    let mut ordinal = 0;
    let mut missing = None;
    let rewritten = rewrite_str(
        html,
        RewriteStrSettings::new().append_element_content_handler(element!("img[src]", |element| {
            let source = element
                .get_attribute("src")
                .expect("img[src] selector guarantees the src attribute");
            let current_ordinal = ordinal;
            ordinal += 1;
            let reference = media
                .iter()
                .find(|reference| reference.field == field && reference.ordinal == current_ordinal);
            let Some(reference) = reference else {
                if !is_remote_source(&source) {
                    missing = Some(current_ordinal);
                }
                return Ok(());
            };
            let token = media_token(reference.content_hash);
            element.set_attribute("src", &token)?;
            used.push(reference.clone());
            Ok(())
        })),
    )
    .map_err(|error| FingerprintError::MalformedHtml {
        field: field.clone(),
        reason: error.to_string(),
    })?;
    if let Some(ordinal) = missing {
        return Err(FingerprintError::MissingMediaRef { field, ordinal });
    }
    Ok(rewritten)
}

/// Remote images remain literal because M2 does not fetch network resources.
/// Data URIs are resolver-owned assets, so they need a `MediaRef` too.
fn is_remote_source(source: &str) -> bool {
    let lower = source.trim_start().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
}

fn media_token(content_hash: [u8; 32]) -> String {
    use std::fmt::Write;

    let mut token = String::from("qti-content-hash:");
    for byte in content_hash {
        write!(&mut token, "{byte:02x}").expect("writing to a string cannot fail");
    }
    token
}

#[cfg(test)]
mod tests {
    use super::{FieldId, FingerprintError, ItemFingerprint, MediaRef};
    use crate::item::{Item, ItemBody};

    fn mc(answer: &str) -> Item {
        Item::new(
            "Which?".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: answer.to_owned(),
            },
        )
        .expect("valid MC item")
    }

    #[test]
    fn fingerprints_include_answer_omitted_from_mc_crc() {
        let one = mc("one");
        let two = mc("two");
        assert_eq!(one.crc(), two.crc());
        assert_ne!(
            ItemFingerprint::new(&one, vec![]).expect("first fingerprint should build"),
            ItemFingerprint::new(&two, vec![]).expect("second fingerprint should build")
        );
    }

    #[test]
    fn media_comparison_uses_content_and_placement() {
        let item = Item::new(
            "<img src=\"first.png\"/><img src=\"second.png\"/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid item with local images");
        let first = MediaRef {
            content_hash: [1; 32],
            field: FieldId::Stem,
            ordinal: 0,
        };
        let second = MediaRef {
            content_hash: [2; 32],
            field: FieldId::Stem,
            ordinal: 1,
        };
        assert_ne!(
            ItemFingerprint::new(&item, vec![first.clone(), second.clone()]),
            ItemFingerprint::new(&item, vec![second.clone(), first.clone()])
        );
        assert_ne!(
            ItemFingerprint::new(&item, vec![first.clone(), second.clone()]),
            ItemFingerprint::new(&item, vec![first.clone()])
        );
        assert!(
            ItemFingerprint::new(
                &item,
                vec![MediaRef {
                    content_hash: [1; 32],
                    field: FieldId::Choice(0),
                    ordinal: 0,
                }]
            )
            .is_err()
        );

        let renamed = Item::new(
            "<img src=\"renamed.png\"/><img src=\"again.png\"/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("renamed image paths are valid HTML");
        assert_eq!(
            ItemFingerprint::new(&item, vec![first.clone(), second.clone()])
                .expect("resolved local images should fingerprint"),
            ItemFingerprint::new(&renamed, vec![first, second])
                .expect("renamed resolved images should fingerprint")
        );
    }

    #[test]
    fn media_hashes_data_uri_at_the_same_ordinal_as_a_local_asset() {
        let local = Item::new(
            "<img src=\"local.png\"/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid local-image item");
        let embedded = Item::new(
            "<img src=\"data:image/png;base64,AAAA\"/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid embedded-image item");
        let reference = MediaRef {
            content_hash: [7; 32],
            field: FieldId::Stem,
            ordinal: 0,
        };

        assert_eq!(
            ItemFingerprint::new(&local, vec![reference.clone()])
                .expect("resolved local image should fingerprint"),
            ItemFingerprint::new(&embedded, vec![reference])
                .expect("resolved data URI should fingerprint")
        );
        assert_eq!(
            ItemFingerprint::new(&embedded, vec![]),
            Err(FingerprintError::MissingMediaRef {
                field: FieldId::Stem,
                ordinal: 0,
            })
        );
    }

    #[test]
    fn remote_images_consume_ordinals_before_resolved_assets() {
        let local = Item::new(
            "<img src=\"https://example.test/image.png\"/><img src=\"local.png\"/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid item");
        let embedded = Item::new(
            "<img src=\"https://example.test/image.png\"/><img src=\"data:image/png;base64,AAAA\"/>"
                .to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid item");
        let reference = MediaRef {
            content_hash: [9; 32],
            field: FieldId::Stem,
            ordinal: 1,
        };

        assert_eq!(
            ItemFingerprint::new(&local, vec![reference.clone()])
                .expect("local image at ordinal one should resolve"),
            ItemFingerprint::new(&embedded, vec![reference])
                .expect("data URI at ordinal one should resolve")
        );
    }
}
