//! Serde types for the subset of PLE Native JSON emitted by QPM.

use serde::Serialize;

/// One emitted question document.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceDocument {
    pub(crate) format: String,
    pub(crate) prompt: String,
    pub(crate) response: Response,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) external_resources: Vec<ExternalResource>,
}

/// A response representation supported by the Native JSON handoff.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Response {
    SingleChoice {
        choices: Vec<Choice>,
        correct_choice: String,
    },
    MultipleAnswer {
        choices: Vec<Choice>,
        correct_choices: Vec<String>,
    },
    FillIn {
        answers: Vec<String>,
        match_mode: MatchMode,
        max_length: u32,
    },
    MultiFillIn {
        blanks: Vec<Blank>,
    },
    Numeric {
        answer: f64,
        tolerance: Tolerance,
    },
    Matching {
        prompts: Vec<MatchingPrompt>,
        choices: Vec<MatchingChoice>,
        matches: Vec<Match>,
    },
    Ordering {
        items: Vec<OrderingItem>,
        correct_order: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Choice {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Blank {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) answers: Vec<String>,
    pub(crate) match_mode: MatchMode,
    pub(crate) max_length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchingPrompt {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchingChoice {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Match {
    pub(crate) prompt: String,
    pub(crate) choice: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrderingItem {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MatchMode {
    Normalized,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Tolerance {
    Absolute { epsilon: f64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalResource {
    pub(crate) url: String,
    pub(crate) kind: ExternalResourceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ExternalResourceKind {
    Link,
    Image,
    Stylesheet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn single_choice_serializes_as_the_handoff_example() {
        let document = SourceDocument {
            format: "pleQuestionJson".to_owned(),
            prompt: "<p><strong>Which structure is shown?</strong></p><img src=\"assets/image_001.png\" alt=\"A simple cell diagram\" style=\"max-width: 20rem\">".to_owned(),
            response: Response::SingleChoice {
                choices: vec![
                    Choice { id: "cell".to_owned(), text: "<em>Cell</em>".to_owned() },
                    Choice { id: "tissue".to_owned(), text: "Tissue".to_owned() },
                ],
                correct_choice: "cell".to_owned(),
            },
            external_resources: Vec::new(),
        };

        let actual = serde_json::to_value(document).expect("document should serialize");
        assert_eq!(
            actual,
            json!({
                "format": "pleQuestionJson",
                "prompt": "<p><strong>Which structure is shown?</strong></p><img src=\"assets/image_001.png\" alt=\"A simple cell diagram\" style=\"max-width: 20rem\">",
                "response": {
                    "kind": "singleChoice",
                    "choices": [
                        {"id": "cell", "text": "<em>Cell</em>"},
                        {"id": "tissue", "text": "Tissue"}
                    ],
                    "correctChoice": "cell"
                }
            })
        );
    }
}
