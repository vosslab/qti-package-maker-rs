use std::collections::BTreeMap;

use qti_core::{Item, ItemBody};

use super::map_item;
use crate::ple_native_json::source::{MatchMode, Response, Tolerance};

fn mapped(body: ItemBody, prompt: &str) -> crate::ple_native_json::source::SourceDocument {
    let item = Item::new(prompt.to_owned(), body).expect("valid source item");
    map_item(&item.render_view()).expect("source item maps")
}

#[test]
fn choice_answers_resolve_to_emitted_positional_ids() {
    let mc = mapped(
        ItemBody::Mc {
            choices: vec!["<em>A</em>".into(), "B".into(), "C".into()],
            answer: "B".into(),
        },
        "<p>Pick one.</p>",
    );
    assert_eq!(mc.prompt, "<p>Pick one.</p>");
    let Response::SingleChoice {
        choices,
        correct_choice,
    } = mc.response
    else {
        panic!("expected MC response");
    };
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["choice-1", "choice-2", "choice-3"]
    );
    assert_eq!(choices[0].text, "<em>A</em>");
    assert_eq!(correct_choice, "choice-2");
    assert!(choices.iter().any(|choice| choice.id == correct_choice));

    let ma = mapped(
        ItemBody::Ma {
            choices: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            answers: vec!["D".into(), "B".into(), "A".into()],
            min_answers_required: 2,
            allow_all_correct: false,
        },
        "<p>Pick several.</p>",
    );
    let Response::MultipleAnswer {
        choices,
        correct_choices,
    } = ma.response
    else {
        panic!("expected MA response");
    };
    assert_eq!(correct_choices, ["choice-1", "choice-2", "choice-4"]);
    assert!(
        correct_choices
            .iter()
            .all(|id| choices.iter().any(|choice| &choice.id == id))
    );
}

#[test]
fn fib_uses_normalized_mode_and_fixed_length() {
    let fib = mapped(
        ItemBody::Fib {
            answers: vec!["DNA".into(), "deoxyribonucleic acid".into()],
        },
        "<p>Name the molecule.</p>",
    );
    let Response::FillIn {
        answers,
        match_mode,
        max_length,
    } = fib.response
    else {
        panic!("expected FIB response");
    };
    assert_eq!(answers, ["DNA", "deoxyribonucleic acid"]);
    assert_eq!(match_mode, MatchMode::Normalized);
    assert_eq!(max_length, 16_384);
}

#[test]
fn multi_fib_uses_first_marker_appearance_and_escaped_labels() {
    let answers = BTreeMap::from([
        ("alpha".into(), vec!["A".into()]),
        ("zeta".into(), vec!["Z".into(), "Z".into(), "z".into()]),
    ]);
    let prompt = "<p>[zeta] pairs with [alpha], then [zeta] repeats.</p>";
    let multi = mapped(ItemBody::MultiFib { answers }, prompt);
    assert_eq!(multi.prompt, prompt);
    let Response::MultiFillIn { blanks } = multi.response else {
        panic!("expected MULTI_FIB response");
    };
    assert_eq!(blanks.len(), 2);
    assert_eq!(
        (&blanks[0].id, &blanks[0].label, &blanks[0].answers),
        (
            &"blank-1".to_owned(),
            &"[zeta]".to_owned(),
            &vec!["Z".to_owned(), "z".to_owned()]
        )
    );
    assert_eq!(
        (&blanks[1].id, &blanks[1].label, &blanks[1].answers),
        (
            &"blank-2".to_owned(),
            &"[alpha]".to_owned(),
            &vec!["A".to_owned()]
        )
    );
    assert!(
        blanks
            .iter()
            .all(|blank| blank.match_mode == MatchMode::Normalized && blank.max_length == 16_384)
    );
    assert_eq!(
        super::escape_html("[a&\"<>']"),
        "[a&amp;&quot;&lt;&gt;&#39;]"
    );
}

#[test]
fn numeric_preserves_absolute_tolerance_and_optional_student_note() {
    for note in [false, true] {
        let numeric = mapped(
            ItemBody::Num {
                answer: 3.5,
                tolerance: 0.25,
                tolerance_message: note,
            },
            "<p>Calculate it.</p>",
        );
        assert_eq!(
            numeric.prompt,
            if note {
                "<p>Calculate it.</p><p>Answer must be within &plusmn;0.25.</p>"
            } else {
                "<p>Calculate it.</p>"
            }
        );
        assert_eq!(
            numeric.response,
            Response::Numeric {
                answer: 3.5,
                tolerance: Tolerance::Absolute { epsilon: 0.25 },
            }
        );
    }
}

#[test]
fn matching_pairs_leave_extra_choices_as_distractors() {
    let matching = mapped(
        ItemBody::Match {
            prompts: vec!["A".into(), "B".into()],
            choices: vec!["1".into(), "2".into(), "3".into()],
        },
        "<p>Match.</p>",
    );
    let Response::Matching {
        prompts,
        choices,
        matches,
    } = matching.response
    else {
        panic!("expected MATCH response");
    };
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.id.as_str())
            .collect::<Vec<_>>(),
        ["prompt-1", "prompt-2"]
    );
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["choice-1", "choice-2", "choice-3"]
    );
    assert_eq!(
        matches
            .iter()
            .map(|pair| (pair.prompt.as_str(), pair.choice.as_str()))
            .collect::<Vec<_>>(),
        [("prompt-1", "choice-1"), ("prompt-2", "choice-2")]
    );
}

#[test]
fn ordering_key_is_a_permutation_of_emitted_ids() {
    let order = mapped(
        ItemBody::Order {
            answers: vec!["First".into(), "Second".into(), "Third".into()],
        },
        "<p>Order them.</p>",
    );
    let Response::Ordering {
        items,
        correct_order,
    } = order.response
    else {
        panic!("expected ORDER response");
    };
    assert_eq!(
        items
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        ["First", "Second", "Third"]
    );
    assert_eq!(correct_order, ["item-1", "item-2", "item-3"]);
    assert_eq!(
        items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        correct_order.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
