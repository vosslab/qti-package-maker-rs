use std::collections::BTreeMap;

use qti_core::media::MemoryAssets;
use qti_core::{Item, ItemBank, ItemBody};
use serde_json::Value;

use super::export_bank;

#[test]
fn exports_seven_kinds_as_compact_documents_in_bank_order() {
    let mut bank = ItemBank::new(true);
    let items = [
        (
            "Choose one",
            ItemBody::Mc {
                choices: vec!["A".into(), "B".into()],
                answer: "B".into(),
            },
            "singleChoice",
        ),
        (
            "Choose several",
            ItemBody::Ma {
                choices: vec!["A".into(), "B".into(), "C".into()],
                answers: vec!["A".into(), "C".into()],
                min_answers_required: 2,
                allow_all_correct: false,
            },
            "multipleAnswer",
        ),
        (
            "Name it",
            ItemBody::Fib {
                answers: vec!["DNA".into()],
            },
            "fillIn",
        ),
        (
            "Name [alpha] and [beta]",
            ItemBody::MultiFib {
                answers: BTreeMap::from([
                    ("alpha".into(), vec!["A".into()]),
                    ("beta".into(), vec!["B".into()]),
                ]),
            },
            "multiFillIn",
        ),
        (
            "Compute it",
            ItemBody::Num {
                answer: 2.0,
                tolerance: 0.1,
                tolerance_message: false,
            },
            "numeric",
        ),
        (
            "Match pairs",
            ItemBody::Match {
                prompts: vec!["P".into(), "Q".into()],
                choices: vec!["A".into(), "B".into()],
            },
            "matching",
        ),
        (
            "Put in order",
            ItemBody::Order {
                answers: vec!["First".into(), "Second".into(), "Third".into()],
            },
            "ordering",
        ),
    ];
    let mut expected = Vec::new();
    for (prompt, body, kind) in items {
        let item = Item::new(prompt.into(), body).expect("valid item");
        expected.push((kind, *item.crc()));
        bank.add_item(item).expect("add item");
    }

    let result = export_bank(&bank, &MemoryAssets::new()).expect("export bank");
    assert_eq!(result.questions.len(), expected.len());
    assert!(result.warnings.is_empty());
    for (index, (question, (kind, crc))) in result.questions.iter().zip(expected).enumerate() {
        assert_eq!(question.item_number, index + 1);
        assert_eq!(question.crc, crc);
        assert!(question.files.is_empty());
        let value: Value = serde_json::from_str(&question.source_json).expect("valid JSON");
        assert_eq!(value["format"], "pleQuestionJson");
        assert_eq!(value["response"]["kind"], kind);
        assert!(!question.source_json.contains("\n"));
    }
}

#[test]
fn empty_bank_returns_no_questions_or_warnings() {
    let result = export_bank(&ItemBank::new(true), &MemoryAssets::new()).expect("empty export");
    assert!(result.questions.is_empty());
    assert!(result.warnings.is_empty());
}

#[test]
fn export_rejects_lossy_display_html_with_item_number() {
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            "<script>draw()</script>".into(),
            ItemBody::Fib {
                answers: vec!["DNA".into()],
            },
        )
        .expect("valid source item"),
    )
    .expect("add item");
    let message = export_bank(&bank, &MemoryAssets::new())
        .expect_err("lossy HTML")
        .to_string();
    assert!(message.contains("item 1"), "{message}");
    assert!(message.contains("<script>"), "{message}");
}
