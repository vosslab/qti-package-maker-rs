//! Validated assessment-item domain types.

use std::collections::BTreeMap;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::crc::{ItemCrc, secondary_string};
use crate::strings::{remove_prefix_from_list, strip_crc_prefix, strip_prefix_from_string};
use crate::validate::{ValidationError, validate_item};

/// Fields shared by every assessment item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemCommon {
    /// The normalized, HTML-bearing question stem.
    pub question_text: String,
    /// Position assigned by the containing item bank.
    pub item_number: usize,
}

/// The seven assessment-item shapes supported by the Python implementation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemBody {
    Mc {
        choices: Vec<String>,
        answer: String,
    },
    Ma {
        choices: Vec<String>,
        answers: Vec<String>,
        min_answers_required: i64,
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

/// A concise identifier for an [`ItemBody`] variant.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemKind {
    Mc,
    Ma,
    Match,
    Num,
    Fib,
    MultiFib,
    Order,
}

impl ItemBody {
    /// Returns the kind represented by this body.
    #[must_use]
    pub const fn kind(&self) -> ItemKind {
        match self {
            Self::Mc { .. } => ItemKind::Mc,
            Self::Ma { .. } => ItemKind::Ma,
            Self::Match { .. } => ItemKind::Match,
            Self::Num { .. } => ItemKind::Num,
            Self::Fib { .. } => ItemKind::Fib,
            Self::MultiFib { .. } => ItemKind::MultiFib,
            Self::Order { .. } => ItemKind::Order,
        }
    }
}

/// An assessment item whose construction and deserialization both enforce validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    common: ItemCommon,
    /// The source stem whose contents define the Python-compatible CRC and serialized identity.
    raw_question_text: String,
    /// The source body whose pre-normalization contents define the Python-compatible CRC.
    raw_body: ItemBody,
    body: ItemBody,
    crc: ItemCrc,
}

/// An owned item presentation for a writer after its media URLs have been rewritten.
///
/// This view starts from an item's normalized display fields and preserves its original identity.
/// It cannot be serialized or converted back into an [`Item`], so writer-specific presentation
/// changes cannot bypass item validation or alter the bank's CRC identity.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemRenderView {
    common: ItemCommon,
    body: ItemBody,
    crc: ItemCrc,
}

impl ItemRenderView {
    /// Returns the common presentation fields, including the bank-owned item number.
    #[must_use]
    pub const fn common(&self) -> &ItemCommon {
        &self.common
    }

    /// Returns the item-specific presentation fields.
    #[must_use]
    pub const fn body(&self) -> &ItemBody {
        &self.body
    }

    /// Returns the source item's stable identity.
    #[must_use]
    pub const fn crc(&self) -> &ItemCrc {
        &self.crc
    }

    /// Returns the body's assessment-item kind.
    #[must_use]
    pub const fn kind(&self) -> ItemKind {
        self.body.kind()
    }

    /// Creates a presentation copy from validated, normalized item contents.
    pub(crate) fn from_item(item: &Item) -> Self {
        Self {
            common: item.common.clone(),
            body: item.body.clone(),
            crc: item.crc,
        }
    }

    /// Rewrites every HTML-bearing field while retaining the item shape and identity.
    pub(crate) fn map_html_fields<E>(
        &mut self,
        mut rewrite: impl FnMut(&str) -> Result<String, E>,
    ) -> Result<(), E> {
        map_item_html_fields(&mut self.common, &mut self.body, &mut rewrite)
    }
}

impl Item {
    /// Creates item number zero from a question and supporting body fields.
    ///
    /// The CRC preserves Python's sequence: it hashes unnormalized supporting fields, while the
    /// stored body removes display prefixes.  This keeps imported Python item identities stable.
    pub fn new(question: String, body: ItemBody) -> Result<Self, ValidationError> {
        Self::from_parts(question, 0, body)
    }

    fn from_parts(
        question: String,
        item_number: usize,
        raw_body: ItemBody,
    ) -> Result<Self, ValidationError> {
        // Python's constructors resolve answer indices against the original choices before
        // validating normalized fields. A prefix must not make an absent answer valid.
        match &raw_body {
            ItemBody::Mc { choices, answer } if !choices.contains(answer) => {
                return Err(ValidationError::AnswerAbsentFromChoices);
            }
            ItemBody::Ma {
                choices, answers, ..
            } if answers.iter().any(|a| !choices.contains(a)) => {
                return Err(ValidationError::AnswerAbsentFromChoices);
            }
            _ => {}
        }
        let question_text = strip_crc_prefix(&question);
        let secondary = secondary_string(&raw_body);
        let body = normalize_body(raw_body.clone());
        validate_item(&question_text, &body)?;
        let crc = ItemCrc::new(&question_text, &secondary).map_err(ValidationError::Crc)?;
        Ok(Self {
            common: ItemCommon {
                question_text: question_text.clone(),
                item_number,
            },
            raw_question_text: question_text,
            raw_body,
            body,
            crc,
        })
    }

    /// Returns the common question fields.
    #[must_use]
    pub const fn common(&self) -> &ItemCommon {
        &self.common
    }

    /// Returns the validated item-specific fields.
    #[must_use]
    pub const fn body(&self) -> &ItemBody {
        &self.body
    }

    /// Returns the stable Python-compatible item identity.
    #[must_use]
    pub const fn crc(&self) -> &ItemCrc {
        &self.crc
    }

    /// Returns the body's assessment-item kind.
    #[must_use]
    pub const fn kind(&self) -> ItemKind {
        self.body.kind()
    }

    /// Returns an owned presentation view for writer-specific media URL rewrites.
    ///
    /// The view begins with these normalized fields and keeps this item's CRC and item number.
    /// Its changes cannot modify this validated item or create a replacement [`Item`].
    #[must_use]
    pub fn render_view(&self) -> ItemRenderView {
        ItemRenderView::from_item(self)
    }

    /// Returns a validated presentation-derived item with the original identity retained.
    ///
    /// The supplied function is applied to every HTML-bearing field, so item kind, list shape,
    /// numeric grading values, and non-HTML options cannot be altered. The derived display fields
    /// are revalidated, while the source stem and source body remain the serialized CRC identity.
    /// Serialization therefore round-trips the source item, rather than treating a media rewrite
    /// as a new authored assessment identity.
    pub fn with_rewritten_html_fields<E>(
        &self,
        mut rewrite: impl FnMut(&str) -> Result<String, E>,
    ) -> Result<Self, E>
    where
        E: From<ValidationError>,
    {
        let mut derived = self.clone();
        map_item_html_fields(&mut derived.common, &mut derived.body, &mut rewrite)?;
        validate_item(&derived.common.question_text, &derived.body).map_err(E::from)?;
        Ok(derived)
    }

    /// Returns every HTML-bearing leaf, beginning with the question stem.
    pub fn field_strings(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.common.question_text.as_str()).chain(body_field_strings(&self.body))
    }

    /// Assigns a bank-owned position while preserving the item's validated contents.
    #[must_use]
    pub fn with_item_number(mut self, item_number: usize) -> Self {
        self.common.item_number = item_number;
        self
    }

    /// Updates the position assigned by the owning item bank.
    pub(crate) fn set_item_number(&mut self, item_number: usize) {
        self.common.item_number = item_number;
    }
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields = serializer.serialize_struct("Item", 3)?;
        let identity_common = ItemCommon {
            question_text: self.raw_question_text.clone(),
            item_number: self.common.item_number,
        };
        fields.serialize_field("common", &identity_common)?;
        fields.serialize_field("body", &self.raw_body)?;
        fields.serialize_field("crc", &self.crc)?;
        fields.end()
    }
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawItem {
            common: ItemCommon,
            body: ItemBody,
        }

        let raw = RawItem::deserialize(deserializer)?;
        Self::from_parts(raw.common.question_text, raw.common.item_number, raw.body)
            .map_err(serde::de::Error::custom)
    }
}

fn normalize_body(body: ItemBody) -> ItemBody {
    match body {
        ItemBody::Mc { choices, answer } => ItemBody::Mc {
            choices: remove_prefix_from_list(&choices),
            answer: strip_prefix_from_string(&answer),
        },
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => ItemBody::Ma {
            choices: remove_prefix_from_list(&choices),
            answers: remove_prefix_from_list(&answers),
            min_answers_required,
            allow_all_correct,
        },
        ItemBody::Match { prompts, choices } => ItemBody::Match {
            prompts: remove_prefix_from_list(&prompts),
            choices: remove_prefix_from_list(&choices),
        },
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        },
        ItemBody::Fib { answers } => ItemBody::Fib {
            answers: remove_prefix_from_list(&answers),
        },
        ItemBody::MultiFib { answers } => ItemBody::MultiFib { answers },
        ItemBody::Order { answers } => ItemBody::Order {
            answers: remove_prefix_from_list(&answers),
        },
    }
}

fn body_field_strings(body: &ItemBody) -> Box<dyn Iterator<Item = &str> + '_> {
    match body {
        ItemBody::Mc { choices, answer } => Box::new(
            choices
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(answer.as_str())),
        ),
        ItemBody::Ma {
            choices, answers, ..
        } => Box::new(
            choices
                .iter()
                .map(String::as_str)
                .chain(answers.iter().map(String::as_str)),
        ),
        ItemBody::Match { prompts, choices } => Box::new(
            prompts
                .iter()
                .map(String::as_str)
                .chain(choices.iter().map(String::as_str)),
        ),
        ItemBody::Num { .. } => Box::new(std::iter::empty()),
        ItemBody::Fib { answers } | ItemBody::Order { answers } => {
            Box::new(answers.iter().map(String::as_str))
        }
        ItemBody::MultiFib { answers } => Box::new(
            answers
                .values()
                .flat_map(|values| values.iter().map(String::as_str)),
        ),
    }
}

fn rewrite_html_list<E>(
    values: &mut [String],
    rewrite: &mut impl FnMut(&str) -> Result<String, E>,
) -> Result<(), E> {
    for value in values {
        *value = rewrite(value)?;
    }
    Ok(())
}

fn map_item_html_fields<E>(
    common: &mut ItemCommon,
    body: &mut ItemBody,
    rewrite: &mut impl FnMut(&str) -> Result<String, E>,
) -> Result<(), E> {
    common.question_text = rewrite(&common.question_text)?;
    match body {
        ItemBody::Mc { choices, answer } => {
            rewrite_html_list(choices, rewrite)?;
            *answer = rewrite(answer)?;
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            rewrite_html_list(choices, rewrite)?;
            rewrite_html_list(answers, rewrite)?;
        }
        ItemBody::Match { prompts, choices } => {
            rewrite_html_list(prompts, rewrite)?;
            rewrite_html_list(choices, rewrite)?;
        }
        ItemBody::Fib { answers } | ItemBody::Order { answers } => {
            rewrite_html_list(answers, rewrite)?;
        }
        ItemBody::MultiFib { answers } => {
            for values in answers.values_mut() {
                rewrite_html_list(values, rewrite)?;
            }
        }
        ItemBody::Num { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Item, ItemBody, ItemKind};
    use crate::crc::CrcError;
    use crate::validate::ValidationError;
    use std::collections::BTreeMap;

    #[test]
    fn every_supported_item_body_constructs() {
        let bodies = vec![
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
            ItemBody::Ma {
                choices: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                answers: vec!["one".to_owned()],
                min_answers_required: 1,
                allow_all_correct: false,
            },
            ItemBody::Match {
                prompts: vec!["one".to_owned(), "two".to_owned()],
                choices: vec!["first".to_owned(), "second".to_owned()],
            },
            ItemBody::Num {
                answer: 1.0,
                tolerance: 0.1,
                tolerance_message: true,
            },
            ItemBody::Fib {
                answers: vec!["one".to_owned()],
            },
            ItemBody::MultiFib {
                answers: BTreeMap::from([("blank".to_owned(), vec!["one".to_owned()])]),
            },
            ItemBody::Order {
                answers: vec!["first".to_owned(), "second".to_owned(), "third".to_owned()],
            },
        ];

        for body in bodies {
            let question = if matches!(body, ItemBody::MultiFib { .. }) {
                "Fill [blank] now"
            } else {
                "A valid question stem"
            };
            Item::new(question.to_owned(), body).expect("valid item body should construct");
        }
    }

    #[test]
    fn field_strings_yields_stem_and_every_html_leaf() {
        let item = Item::new(
            "Stem".to_owned(),
            ItemBody::Mc {
                choices: vec!["A. one".to_owned(), "B. two".to_owned()],
                answer: "A. one".to_owned(),
            },
        )
        .expect("valid MC item");

        assert_eq!(item.kind(), ItemKind::Mc);
        assert_eq!(
            item.field_strings().collect::<Vec<_>>(),
            ["Stem", "one", "two", "one"]
        );
    }

    #[test]
    fn deserialization_revalidates_contents() {
        let invalid = r#"{"common":{"question_text":"Stem","item_number":1},"body":{"MC":{"choices":["one","two"],"answer":"missing"}}}"#;
        assert!(serde_json::from_str::<Item>(invalid).is_err());
    }

    #[test]
    fn mc_and_ma_answers_must_belong_to_raw_choices_before_normalization() {
        let choices = vec!["A. one".into(), "B. two".into(), "C. three".into()];
        for answer in ["one", "A. one"] {
            for body in [
                ItemBody::Mc {
                    choices: choices.clone(),
                    answer: answer.into(),
                },
                ItemBody::Ma {
                    choices: choices.clone(),
                    answers: vec![answer.into()],
                    min_answers_required: 1,
                    allow_all_correct: true,
                },
            ] {
                assert_eq!(
                    Item::new("Question".into(), body).is_ok(),
                    answer == "A. one"
                );
            }
        }
    }

    #[test]
    fn typed_num_deserialization_rejects_boolean_answer_and_tolerance() {
        for body in [
            r#"{"NUM":{"answer":true,"tolerance":0.1,"tolerance_message":false}}"#,
            r#"{"NUM":{"answer":1.0,"tolerance":false,"tolerance_message":false}}"#,
        ] {
            assert!(serde_json::from_str::<ItemBody>(body).is_err());
            let item = format!(
                r#"{{"common":{{"question_text":"A valid stem","item_number":0}},"body":{body}}}"#
            );
            assert!(serde_json::from_str::<Item>(&item).is_err());
        }
    }

    #[test]
    fn construction_surfaces_non_ascii_crc_identity_input() {
        assert!(matches!(
            Item::new(
                "A valid stem".to_owned(),
                ItemBody::Fib {
                    answers: vec!["\u{00e9}".to_owned()]
                }
            ),
            Err(ValidationError::Crc(CrcError::NonAscii { .. }))
        ));
        // MULTIFIB hashes Python list repr, which escapes non-printable characters first.
        assert!(
            Item::new(
                "Fill [blank]".into(),
                ItemBody::MultiFib {
                    answers: BTreeMap::from([("blank".into(), vec!["x\u{85}y".into()])]),
                },
            )
            .is_ok()
        );
    }

    #[test]
    fn serialization_preserves_crc_from_prefixed_choice_identity() {
        let item = Item::new(
            "Stem".to_owned(),
            ItemBody::Mc {
                choices: vec!["A. one".to_owned(), "B. two".to_owned()],
                answer: "A. one".to_owned(),
            },
        )
        .expect("valid MC item");
        let round_trip: Item =
            serde_json::from_str(&serde_json::to_string(&item).expect("item should serialize"))
                .expect("serialized item should validate");

        assert_eq!(round_trip.crc(), item.crc());
        assert_eq!(round_trip.body(), item.body());
    }

    #[test]
    fn variants_share_stem_identity_but_keep_independent_question_crcs() {
        // Website completion uses both CRC components, never a stem or bank identifier.
        let variant = |answer| {
            Item::new(
                "Compute the concentration shown in your variant".to_owned(),
                ItemBody::Num {
                    answer,
                    tolerance: 0.125,
                    tolerance_message: true,
                },
            )
            .expect("valid numeric variant")
        };
        let first = variant(2.5);
        let second = variant(5.0);
        assert_eq!(first.crc().question_crc(), second.crc().question_crc());
        assert_ne!(first.crc(), second.crc());

        // Changing bank position or reconstructing an authored variant keeps its progress key.
        assert_eq!(first.crc(), variant(2.5).with_item_number(99).crc());
        let restored: Item = serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
        assert_eq!(first.crc(), restored.crc());
    }

    #[test]
    fn presentation_derivation_preserves_source_identity_through_serialization() {
        let source = Item::new(
            "<img src='stem.png'/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["A. <img src='choice.png'/>".to_owned(), "B. two".to_owned()],
                answer: "A. <img src='choice.png'/>".to_owned(),
            },
        )
        .expect("valid source item")
        .with_item_number(12);
        let derived = source
            .with_rewritten_html_fields(|html| {
                Ok::<_, ValidationError>(html.replace(".png", ".asset"))
            })
            .expect("presentation rewrite remains valid");

        assert_eq!(derived.crc(), source.crc());
        assert_eq!(derived.common().item_number, 12);
        assert_eq!(source.common().question_text, "<img src='stem.png'/>");
        assert_eq!(derived.common().question_text, "<img src='stem.asset'/>");
        assert_eq!(
            derived.body(),
            &ItemBody::Mc {
                choices: vec!["<img src='choice.asset'/>".to_owned(), "two".to_owned()],
                answer: "<img src='choice.asset'/>".to_owned(),
            }
        );

        let serialized = serde_json::to_string(&derived).expect("derived item serializes");
        assert!(serialized.contains("stem.png"));
        assert!(serialized.contains("A. <img src='choice.png'/>"));
        assert!(!serialized.contains(".asset"));
        let restored: Item = serde_json::from_str(&serialized).expect("source identity restores");
        assert_eq!(restored.crc(), source.crc());
        assert_eq!(restored.common(), source.common());
        assert_eq!(restored.body(), source.body());
    }

    #[test]
    fn presentation_derivation_revalidates_grading_relationships() {
        let item = Item::new(
            "Question".to_owned(),
            ItemBody::Mc {
                choices: vec!["first".to_owned(), "second".to_owned()],
                answer: "first".to_owned(),
            },
        )
        .expect("valid source item");
        let mut field_index = 0;
        let error = item
            .with_rewritten_html_fields(|html| {
                field_index += 1;
                Ok::<_, ValidationError>(if field_index == 2 {
                    "rewritten first choice".to_owned()
                } else {
                    html.to_owned()
                })
            })
            .expect_err("rewriting a choice without its answer is invalid");

        assert!(matches!(error, ValidationError::AnswerAbsentFromChoices));
    }

    #[test]
    fn render_view_rewrites_presentation_without_changing_item_identity() {
        let item = Item::new(
            "<img src='source.png'/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["<img src='choice.png'/>".to_owned(), "two".to_owned()],
                answer: "<img src='choice.png'/>".to_owned(),
            },
        )
        .expect("valid item")
        .with_item_number(12);
        let mut view = item.render_view();

        view.map_html_fields(|html| Ok::<_, ()>(html.replace(".png", ".asset")))
            .expect("rewriting cannot fail");

        assert_eq!(view.common().item_number, 12);
        assert_eq!(view.crc(), item.crc());
        assert_eq!(view.kind(), item.kind());
        assert_eq!(item.common().question_text, "<img src='source.png'/>");
        assert_eq!(view.common().question_text, "<img src='source.asset'/>");
        assert_eq!(
            view.body(),
            &ItemBody::Mc {
                choices: vec!["<img src='choice.asset'/>".to_owned(), "two".to_owned()],
                answer: "<img src='choice.asset'/>".to_owned(),
            }
        );
    }
}
