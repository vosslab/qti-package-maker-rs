//! Blackboard question presentation and grading XML for supported item kinds.

use qti_core::{ItemBody, ItemRenderView};

use crate::EngineError;

use super::html::{smart, xml};

fn tag(name: &str, value: &str) -> String {
    format!("<{name}>{}</{name}>", xml(value))
}

fn item_id(item: &ItemRenderView) -> String {
    format!("_{}_1", item.crc())
}

fn format_number(value: f64) -> String {
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

fn skeleton(
    item: &ItemRenderView,
    question_type: &str,
    response: String,
    processing: String,
) -> String {
    skeleton_with_metadata(item, question_type, response, processing, "")
}

fn skeleton_with_metadata(
    item: &ItemRenderView,
    question_type: &str,
    response: String,
    processing: String,
    grading_metadata: &str,
) -> String {
    format!(
        "<item title=\"{}\" maxattempts=\"0\"><itemmetadata>{}<bbmd_questiontype>{}</bbmd_questiontype>{grading_metadata}</itemmetadata><presentation><flow class=\"Block\"><flow class=\"QUESTION_BLOCK\">{}</flow><flow class=\"RESPONSE_BLOCK\">{response}</flow></flow></presentation>{processing}</item>",
        xml(&item.crc().to_string()),
        tag("bbmd_asi_object_id", &item_id(item)),
        xml(question_type),
        smart(&item.common().question_text)
    )
}

pub(super) fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let rendered = match item.body() {
        ItemBody::Mc { choices, answer } => choice_item(
            item,
            "Multiple Choice",
            choices,
            std::slice::from_ref(answer),
            false,
            None,
        ),
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => choice_item(
            item,
            "Multiple Answer",
            choices,
            answers,
            true,
            Some((*min_answers_required, *allow_all_correct)),
        ),
        ItemBody::Fib { answers } => fib_item(item, answers),
        ItemBody::MultiFib { answers } => multi_fib_item(item, answers),
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => numeric_item(item, *answer, *tolerance, *tolerance_message),
        ItemBody::Match { prompts, choices } => match_item(item, prompts, choices),
        ItemBody::Order { .. } => return Ok(None),
    };
    Ok(Some(rendered))
}

fn processing(max_score: usize, branches: String) -> String {
    format!(
        "<resprocessing scoremodel=\"SumOfScores\"><outcomes><decvar varname=\"SCORE\" vartype=\"Decimal\" defaultval=\"0\" minvalue=\"0\" maxvalue=\"{max_score}.00000\"/></outcomes>{branches}</resprocessing>"
    )
}

fn score_branch(condition: String, score: &str, feedback: &str, title: Option<&str>) -> String {
    let title = title.map_or_else(String::new, |title| format!(" title=\"{title}\""));
    format!(
        "<respcondition{title}><conditionvar>{condition}</conditionvar><setvar variablename=\"SCORE\" action=\"Set\">{score}</setvar>{feedback}</respcondition>"
    )
}

fn feedback(identifier: &str) -> String {
    format!("<displayfeedback linkrefid=\"{identifier}\" feedbacktype=\"Response\"/>")
}

fn render_fib(fibtype: &str) -> String {
    format!(
        "<render_fib charset=\"us-ascii\" columns=\"0\" encoding=\"UTF_8\" fibtype=\"{fibtype}\" maxchars=\"0\" maxnumber=\"0\" minnumber=\"0\" prompt=\"Box\" rows=\"0\"/>"
    )
}

fn incorrect_branch() -> String {
    score_branch(
        "<other/>".to_owned(),
        "0",
        &feedback("incorrect"),
        Some("incorrect"),
    )
}

fn item_feedback() -> String {
    "<itemfeedback ident=\"correct\" view=\"All\"><flow_mat class=\"Block\"/></itemfeedback><itemfeedback ident=\"incorrect\" view=\"All\"><flow_mat class=\"Block\"/></itemfeedback>".to_owned()
}

fn choice_item(
    item: &ItemRenderView,
    typ: &str,
    choices: &[String],
    answers: &[String],
    multiple: bool,
    grading_options: Option<(i64, bool)>,
) -> String {
    let label_ids = choices
        .iter()
        .enumerate()
        .map(|(index, _)| format!("{}_label_{index}", item.crc()))
        .collect::<Vec<_>>();
    let labels = choices
        .iter()
        .zip(&label_ids)
        .map(|(choice, id)| {
            format!(
                "<response_label ident=\"{id}\" shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\">{}</response_label>",
                smart(choice)
            )
        })
        .collect::<String>();
    let correct_ids = answers
        .iter()
        .filter_map(|answer| choices.iter().position(|choice| choice == answer))
        .map(|index| &label_ids[index])
        .collect::<Vec<_>>();
    let response = format!(
        "<response_lid ident=\"response\" rcardinality=\"{}\" rtiming=\"No\"><render_choice shuffle=\"No\" minnumber=\"0\" maxnumber=\"0\">{labels}</render_choice></response_lid>",
        if multiple { "Multiple" } else { "Single" }
    );
    let correct_condition = if multiple {
        let all = label_ids
            .iter()
            .map(|id| {
                let equal = format!("<varequal respident=\"response\" case=\"No\">{id}</varequal>");
                if correct_ids.contains(&id) {
                    equal
                } else {
                    format!("<not>{equal}</not>")
                }
            })
            .collect::<String>();
        format!("<and>{all}</and>")
    } else {
        correct_ids
            .iter()
            .map(|id| format!("<varequal respident=\"response\" case=\"No\">{id}</varequal>"))
            .collect()
    };
    let mut branches = score_branch(
        correct_condition,
        "SCORE.max",
        &feedback("correct"),
        Some("correct"),
    );
    branches.push_str(&incorrect_branch());
    for id in &label_ids {
        // Blackboard's frozen exports put dangling displayfeedback links on
        // single-choice penalties.  The source has no corresponding authored
        // feedback body, so retain the predicate and zero-score penalty but
        // omit the invalid reference.
        branches.push_str(&score_branch(
            format!("<varequal respident=\"{id}\" case=\"No\"></varequal>"),
            "0",
            "",
            None,
        ));
    }
    let grading_metadata = grading_options.map_or_else(String::new, |(minimum, allow_all)| {
        format!(
            "<bbmd_qti_package_maker_ma_min_answers_required>{minimum}</bbmd_qti_package_maker_ma_min_answers_required><bbmd_qti_package_maker_ma_allow_all_correct>{allow_all}</bbmd_qti_package_maker_ma_allow_all_correct>"
        )
    });
    skeleton_with_metadata(
        item,
        typ,
        response,
        format!("{}{}", processing(1, branches), item_feedback()),
        &grading_metadata,
    )
}

fn fib_item(item: &ItemRenderView, answers: &[String]) -> String {
    let mut branches = String::new();
    let mut answer_feedback = String::new();
    for (index, answer) in answers.iter().enumerate() {
        let title = format!("{}_fib_answer_{index}", item.crc());
        branches.push_str(&format!(
            "<respcondition title=\"{title}\"><conditionvar><varequal respident=\"response\" case=\"No\">{}</varequal></conditionvar>{}{}</respcondition>",
            xml(answer), feedback("correct"), feedback(&title),
        ));
        answer_feedback.push_str(&format!("<itemfeedback ident=\"{title}\" view=\"All\"><solution view=\"All\" feedbackstyle=\"Complete\"><solutionmaterial><flow_mat class=\"Block\"/></solutionmaterial></solution></itemfeedback>"));
    }
    branches.push_str(&incorrect_branch());
    skeleton(
        item,
        "Fill in the Blank",
        format!(
            "<response_str ident=\"response\" rcardinality=\"Single\" rtiming=\"No\">{}</response_str>",
            render_fib("String")
        ),
        format!(
            "{}{}{}",
            processing(1, branches),
            item_feedback(),
            answer_feedback
        ),
    )
}

fn multi_fib_item(
    item: &ItemRenderView,
    answers: &std::collections::BTreeMap<String, Vec<String>>,
) -> String {
    let response = answers.keys().map(|key| format!("<response_str ident=\"{}\" rcardinality=\"Single\" rtiming=\"No\">{}</response_str>", xml(key), render_fib("String"))).collect::<String>();
    let groups = answers
        .iter()
        .map(|(key, values)| {
            format!(
                "<or>{}</or>",
                values
                    .iter()
                    .map(|value| format!(
                        "<varequal respident=\"{}\" case=\"No\">{}</varequal>",
                        xml(key),
                        xml(value)
                    ))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    let branches = format!(
        "{}{}",
        score_branch(
            format!("<and>{groups}</and>"),
            "SCORE.max",
            &feedback("correct"),
            Some("correct")
        ),
        incorrect_branch()
    );
    skeleton(
        item,
        "Fill in the Blank Plus",
        response,
        format!("{}{}", processing(answers.len(), branches), item_feedback()),
    )
}

fn numeric_item(
    item: &ItemRenderView,
    answer: f64,
    tolerance: f64,
    tolerance_message: bool,
) -> String {
    let correct_title = format!("{}_num_correct_0", item.crc());
    let lower_bound = format_number(answer - tolerance);
    let upper_bound = format_number(answer + tolerance);
    let answer = format_number(answer);
    let tolerance = format_number(tolerance);
    let condition = format!(
        "<vargte respident=\"response\">{lower_bound}</vargte><varlte respident=\"response\">{upper_bound}</varlte><varequal respident=\"response\" case=\"No\">{answer}</varequal>",
    );
    let branches = format!(
        "<respcondition title=\"{correct_title}\"><conditionvar>{condition}</conditionvar>{}</respcondition>{}",
        feedback("correct"),
        incorrect_branch()
    );
    let grading_metadata = format!(
        "<bbmd_qti_package_maker_num_tolerance>{tolerance}</bbmd_qti_package_maker_num_tolerance><bbmd_qti_package_maker_num_tolerance_message>{tolerance_message}</bbmd_qti_package_maker_num_tolerance_message>"
    );
    skeleton_with_metadata(
        item,
        "Numeric",
        format!(
            "<response_num ident=\"response\" rcardinality=\"Single\" rtiming=\"No\">{}</response_num>",
            render_fib("Decimal")
        ),
        format!("{}{}", processing(1, branches), item_feedback()),
        &grading_metadata,
    )
}

fn match_item(item: &ItemRenderView, prompts: &[String], choices: &[String]) -> String {
    let response = prompts.iter().enumerate().map(|(prompt_index, prompt)| {
        let labels = choices.iter().enumerate().map(|(choice_index, _)| format!("<response_label ident=\"{}_match_{prompt_index}_{choice_index}\" shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\"/>", item.crc())).collect::<String>();
        format!("<flow class=\"Block\"><response_lid ident=\"{}_match_prompt_{prompt_index}\" rcardinality=\"Single\" rtiming=\"No\"><render_choice shuffle=\"Yes\" minnumber=\"0\" maxnumber=\"0\"><flow_label class=\"Block\">{labels}</flow_label></render_choice></response_lid>{}</flow>", item.crc(), smart(prompt))
    }).collect::<String>();
    let right_choices = choices
        .iter()
        .map(|choice| format!("<flow class=\"Block\">{}</flow>", smart(choice)))
        .collect::<String>();
    let branches = prompts.iter().enumerate().map(|(index, _)| format!("<respcondition><conditionvar><varequal respident=\"{}_match_prompt_{index}\" case=\"No\">{}_match_{index}_{index}</varequal></conditionvar>{}</respcondition>", item.crc(), item.crc(), feedback("correct"))).collect::<String>() + &incorrect_branch();
    skeleton(
        item,
        "Matching",
        format!("{response}<flow class=\"RIGHT_MATCH_BLOCK\">{right_choices}</flow>"),
        format!("{}{}", processing(1, branches), item_feedback()),
    )
}

#[cfg(test)]
mod tests {
    use qti_core::{Item, ItemBody};

    use super::{format_number, numeric_item};

    #[test]
    fn numeric_grading_uses_frozen_six_decimal_xml_values() {
        assert_eq!(format_number(0.284_179_92), "0.28418");
        assert_eq!(format_number(5.734_747_26), "5.734747");
        assert_eq!(format_number(4.0), "4");

        let item = Item::new(
            "Precise numeric question".to_owned(),
            ItemBody::Num {
                answer: 0.284_179_92,
                tolerance: 0.0028,
                tolerance_message: true,
            },
        )
        .expect("valid numeric fixture");
        let xml = numeric_item(&item.render_view(), 0.284_179_92, 0.0028, true);
        assert!(xml.contains("<vargte respident=\"response\">0.28138</vargte>"));
        assert!(xml.contains("<varlte respident=\"response\">0.28698</varlte>"));
        assert!(xml.contains("<varequal respident=\"response\" case=\"No\">0.28418</varequal>"));
        assert!(xml.contains(
            "<bbmd_qti_package_maker_num_tolerance>0.0028</bbmd_qti_package_maker_num_tolerance>"
        ));
        assert!(!xml.contains("0.28417992"));
    }
}

#[cfg(test)]
mod grading_tests {
    use std::collections::BTreeSet;

    use qti_core::{Item, ItemBody};
    use regex::Regex;

    fn assert_displayfeedback_targets(xml: &str) {
        let references = Regex::new(r#"<displayfeedback linkrefid=\"([^\"]+)\""#)
            .expect("static reference regex is valid")
            .captures_iter(xml)
            .map(|capture| capture[1].to_owned())
            .collect::<BTreeSet<_>>();
        let targets = Regex::new(r#"<itemfeedback ident=\"([^\"]+)\""#)
            .expect("static target regex is valid")
            .captures_iter(xml)
            .map(|capture| capture[1].to_owned())
            .collect::<BTreeSet<_>>();

        assert!(references.is_subset(&targets));
    }

    use super::{render_fib, render_item};

    #[test]
    fn emits_blackboard_score_actions_and_feedback_for_every_supported_kind() {
        let items = [
            Item::new(
                "MC question".to_owned(),
                ItemBody::Mc {
                    choices: vec!["a".to_owned(), "b".to_owned()],
                    answer: "a".to_owned(),
                },
            ),
            Item::new(
                "MA question".to_owned(),
                ItemBody::Ma {
                    choices: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
                    answers: vec!["a".to_owned()],
                    min_answers_required: 1,
                    allow_all_correct: true,
                },
            ),
            Item::new(
                "FIB question".to_owned(),
                ItemBody::Fib {
                    answers: vec!["a".to_owned(), "b".to_owned()],
                },
            ),
            Item::new(
                "NUM question".to_owned(),
                ItemBody::Num {
                    answer: 4.0,
                    tolerance: 0.01,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "MATCH question".to_owned(),
                ItemBody::Match {
                    prompts: vec!["a".to_owned(), "b".to_owned()],
                    choices: vec!["one".to_owned(), "two".to_owned()],
                },
            ),
            Item::new(
                "MULTI [left] [right]".to_owned(),
                ItemBody::MultiFib {
                    answers: [
                        ("left".to_owned(), vec!["L".to_owned()]),
                        ("right".to_owned(), vec!["R".to_owned()]),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
        ];
        for item in items {
            let item = item.expect("valid fixture");
            let xml = render_item(&item.render_view())
                .expect("rendering succeeds")
                .expect("supported kind renders");
            assert!(
                xml.contains(
                    "<resprocessing scoremodel=\"SumOfScores\"><outcomes><decvar varname=\"SCORE\""
                ),
                "{:?} omits SCORE outcome: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains("<setvar variablename=\"SCORE\" action=\"Set\">0</setvar>"),
                "{:?} omits zero-score fallback: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains(
                    "<displayfeedback linkrefid=\"incorrect\" feedbacktype=\"Response\"/>"
                ),
                "{:?} omits incorrect feedback: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains("<itemfeedback ident=\"correct\" view=\"All\">")
                    && xml.contains("<itemfeedback ident=\"incorrect\" view=\"All\">"),
                "{:?} omits item feedback blocks: {xml}",
                item.kind(),
            );
            assert_displayfeedback_targets(&xml);
        }
    }

    #[test]
    fn emits_frozen_response_presentation_attributes() {
        assert_eq!(
            render_fib("Decimal"),
            "<render_fib charset=\"us-ascii\" columns=\"0\" encoding=\"UTF_8\" fibtype=\"Decimal\" maxchars=\"0\" maxnumber=\"0\" minnumber=\"0\" prompt=\"Box\" rows=\"0\"/>"
        );
        let item = Item::new(
            "Choice attributes".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid MC fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MC renders");
        assert!(xml.contains("<response_label ident=\""));
        assert!(xml.contains("shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\""));
    }

    #[test]
    fn multiple_answer_uses_all_choice_predicates_and_choice_ordered_penalties() {
        let item = Item::new(
            "MA question".to_owned(),
            ItemBody::Ma {
                choices: vec![
                    "correct".to_owned(),
                    "incorrect".to_owned(),
                    "also incorrect".to_owned(),
                ],
                answers: vec!["correct".to_owned()],
                min_answers_required: 1,
                allow_all_correct: true,
            },
        )
        .expect("valid MA fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MA renders");
        let first = format!("{}_label_0", item.crc());
        let second = format!("{}_label_1", item.crc());
        let third = format!("{}_label_2", item.crc());
        assert!(xml.contains(&format!(
            "<and><varequal respident=\"response\" case=\"No\">{first}</varequal><not><varequal respident=\"response\" case=\"No\">{second}</varequal></not><not><varequal respident=\"response\" case=\"No\">{third}</varequal></not></and>"
        )));
        let first_penalty = format!(
            "<varequal respident=\"{first}\" case=\"No\"></varequal></conditionvar><setvar"
        );
        let second_penalty = format!(
            "<varequal respident=\"{second}\" case=\"No\"></varequal></conditionvar><setvar"
        );
        assert!(xml.find(&first_penalty) < xml.find(&second_penalty));
    }

    #[test]
    fn emits_only_displayfeedback_references_with_itemfeedback_targets() {
        let item = Item::new(
            "MC question".to_owned(),
            ItemBody::Mc {
                choices: vec!["correct".to_owned(), "incorrect".to_owned()],
                answer: "correct".to_owned(),
            },
        )
        .expect("valid MC fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MC renders");
        assert_displayfeedback_targets(&xml);
        assert!(!xml.contains(&format!(
            "<displayfeedback linkrefid=\"{}_label_0\"",
            item.crc()
        )));
        assert!(!xml.contains(&format!(
            "<displayfeedback linkrefid=\"{}_label_1\"",
            item.crc()
        )));
    }
}
