//! One-time Playwright input builder for the HTML self-test writer.
//!
//! Kept ignored because the following browser assertion belongs to the temporary
//! local proof lane, not to a headless Cargo requirement.

use std::collections::BTreeMap;
use std::fs;

use qti_core::{Item, ItemBank, ItemBody};
use qti_engines::{EngineOptions, html_selftest};

#[test]
#[ignore = "run only with the local Playwright proof lane"]
fn writes_one_standalone_page_for_every_item_kind() {
    let destination = std::env::temp_dir().join("qti-selftest-browser-proof");
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).expect("proof directory");
    for (name, question, body) in cases() {
        let mut bank = ItemBank::new(true);
        bank.add_item(Item::new(question.into(), body).expect("valid item"))
            .expect("bank item");
        html_selftest::boxed_writer(EngineOptions::default())
            .save_package(&bank, Some(&destination.join(format!("{name}.html"))))
            .expect("standalone self-test page");
    }
}

fn cases() -> Vec<(&'static str, &'static str, ItemBody)> {
    vec![
        (
            "mc",
            "Which is correct?",
            ItemBody::Mc {
                choices: vec!["one".into(), "two".into(), "three".into()],
                answer: "two".into(),
            },
        ),
        (
            "ma",
            "Select both.",
            ItemBody::Ma {
                choices: vec!["one".into(), "two".into(), "three".into()],
                answers: vec!["one".into(), "three".into()],
                min_answers_required: 2,
                allow_all_correct: false,
            },
        ),
        (
            "match",
            "Match the bases.",
            ItemBody::Match {
                prompts: vec!["A".into(), "C".into()],
                choices: vec!["T".into(), "G".into()],
            },
        ),
        (
            "num",
            "Calculate.",
            ItemBody::Num {
                answer: 3.5,
                tolerance: 0.1,
                tolerance_message: true,
            },
        ),
        (
            "fib",
            "Type yes.",
            ItemBody::Fib {
                answers: vec!["yes".into()],
            },
        ),
        (
            "multi_fib",
            "Type [color].",
            ItemBody::MultiFib {
                answers: BTreeMap::from([("color".into(), vec!["blue".into()])]),
            },
        ),
        (
            "order",
            "Order them.",
            ItemBody::Order {
                answers: vec!["first".into(), "second".into(), "third".into()],
            },
        ),
    ]
}
