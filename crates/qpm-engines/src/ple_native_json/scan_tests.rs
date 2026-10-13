use super::*;
use crate::ple_native_json::source::{
    Blank, Choice, MatchMode, MatchingChoice, MatchingPrompt, OrderingItem,
};

fn document(prompt: &str, response: Response) -> SourceDocument {
    SourceDocument {
        format: "pleQuestionJson".to_owned(),
        prompt: prompt.to_owned(),
        response,
        external_resources: Vec::new(),
    }
}

fn single_choice(text: &str) -> Response {
    Response::SingleChoice {
        choices: vec![Choice {
            id: "choice-1".to_owned(),
            text: text.to_owned(),
        }],
        correct_choice: "choice-1".to_owned(),
    }
}

#[test]
fn script_in_a_display_choice_reports_item_and_conversion_advice() {
    let source = document("<p>Question</p>", single_choice("<ScRiPt>draw()</ScRiPt>"));
    let error = scan_document(&source, 17).expect_err("script is lossy");
    let message = error.to_string();
    assert!(message.contains("item 17"), "{message}");
    assert!(message.contains("<script>"), "{message}");
    assert!(message.contains("--html-to-image"), "{message}");
}

#[test]
fn non_https_absolute_urls_in_scanned_attributes_fail() {
    for html in [
        "<img src='http://example.test/figure.png'>",
        "<a href='ftp://example.test/data'>data</a>",
        "<link href='javascript:alert(1)'>",
        "<a href='javascript&#58;alert(1)'>encoded scheme</a>",
        "<a href='data:text/html,hello'>data</a>",
        "<img src='//example.test/protocol-relative.png'>",
    ] {
        let source = document(html, single_choice("Choice"));
        let error = scan_document(&source, 23).expect_err("non-HTTPS URL is lossy");
        let message = error.to_string();
        assert!(message.contains("item 23"), "{message}");
        assert!(message.contains("non-HTTPS URL"), "{message}");
    }
}

#[test]
fn one_url_used_as_different_resource_kinds_is_rejected() {
    let source = document(
        "<a href='https://example.test/shared'>source</a>",
        single_choice("<img src='https://example.test/shared'>"),
    );
    let error = scan_document(&source, 31).expect_err("one URL cannot declare two PLE kinds");
    let message = error.to_string();
    assert!(message.contains("item 31"), "{message}");
    assert!(message.contains("https://example.test/shared"), "{message}");
    assert!(
        message.contains("Link") && message.contains("Image"),
        "{message}"
    );
}

#[test]
fn inventory_decodes_html_attributes_and_deduplicates_by_url() {
    let source = document(
        "<img src='https://example.test/figure?a=1&amp;b=2'><a href='https://example.test/guide'>guide</a>",
        single_choice(
            "<link href='https://example.test/theme.css'><img src='https://example.test/figure?a=1&amp;b=2'>",
        ),
    );
    let resources = scan_document(&source, 2).expect("valid display HTML");
    assert_eq!(resources.len(), 3);
    assert_eq!(resources[0].url, "https://example.test/figure?a=1&b=2");
    assert_eq!(resources[0].kind, ExternalResourceKind::Image);
    assert_eq!(resources[1].url, "https://example.test/guide");
    assert_eq!(resources[1].kind, ExternalResourceKind::Link);
    assert_eq!(resources[2].url, "https://example.test/theme.css");
    assert_eq!(resources[2].kind, ExternalResourceKind::Stylesheet);
}

#[test]
fn every_display_collection_is_scanned_but_answer_strings_are_not() {
    let https = "https://example.test/";
    let sources = [
        document(
            "Prompt",
            Response::MultipleAnswer {
                choices: vec![Choice {
                    id: "choice-1".to_owned(),
                    text: format!("<a href='{https}ma'>MA</a>"),
                }],
                correct_choices: vec!["choice-1".to_owned()],
            },
        ),
        document(
            "Prompt",
            Response::MultiFillIn {
                blanks: vec![Blank {
                    id: "blank-1".to_owned(),
                    label: format!("<a href='{https}blank'>blank</a>"),
                    answers: vec!["<script>answer</script>".to_owned()],
                    match_mode: MatchMode::Normalized,
                    max_length: 16_384,
                }],
            },
        ),
        document(
            "Prompt",
            Response::Matching {
                prompts: vec![MatchingPrompt {
                    id: "prompt-1".to_owned(),
                    text: format!("<a href='{https}match-prompt'>prompt</a>"),
                }],
                choices: vec![MatchingChoice {
                    id: "choice-1".to_owned(),
                    text: format!("<a href='{https}match-choice'>choice</a>"),
                }],
                matches: Vec::new(),
            },
        ),
        document(
            "Prompt",
            Response::Ordering {
                items: vec![OrderingItem {
                    id: "item-1".to_owned(),
                    text: format!("<a href='{https}order'>order</a>"),
                }],
                correct_order: vec!["item-1".to_owned()],
            },
        ),
        document(
            "Prompt",
            Response::FillIn {
                answers: vec!["<script>answer</script>".to_owned()],
                match_mode: MatchMode::Normalized,
                max_length: 16_384,
            },
        ),
    ];
    let expected_counts = [1, 1, 2, 1, 0];
    for (source, expected_count) in sources.iter().zip(expected_counts) {
        assert_eq!(
            scan_document(source, 4).expect("display field").len(),
            expected_count
        );
    }
}

#[test]
fn data_image_is_reserved_for_package_policy() {
    let source = document(
        "<img src='data:image/png;base64,AA=='>",
        single_choice("<a href='relative/page'>local</a>"),
    );
    assert!(
        scan_document(&source, 5)
            .expect("scan defers data image")
            .is_empty()
    );
}
