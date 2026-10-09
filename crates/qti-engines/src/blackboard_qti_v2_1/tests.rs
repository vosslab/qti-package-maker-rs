use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use qti_core::media::MediaAction;
use qti_core::{Item, ItemBank, ItemBody};

use super::fragment::{fragment, plain_text};
use super::item_xml::multi_fib_item;
use super::writer::BlackboardQti21Writer;
use crate::{DocumentMetadata, WriteArtifact, WriteContext, WriteOutcome, Writer};
use qti_core::media::MemoryAssets;

fn context(name: &str) -> WriteContext {
    WriteContext::new(
        name.to_owned(),
        DocumentMetadata {
            title: "QTI 2.1".to_owned(),
            date: "2026-09-30".to_owned(),
        },
        0,
    )
    .expect("context")
}

fn primary(outcome: &WriteOutcome) -> &qti_core::NamedFile {
    match outcome.artifact.as_ref().expect("package artifact") {
        WriteArtifact::File {
            primary,
            companions,
        } => {
            assert!(companions.is_empty());
            primary
        }
        WriteArtifact::Directory { .. } => panic!("ZIP must be a file"),
    }
}

fn seven_item_bank() -> ItemBank {
    let mut bank = ItemBank::new(true);
    let bodies = [
        ItemBody::Mc {
            choices: vec!["a".into(), "b".into()],
            answer: "b".into(),
        },
        ItemBody::Ma {
            choices: vec!["a".into(), "b".into(), "c".into()],
            answers: vec!["a".into(), "c".into()],
            min_answers_required: 1,
            allow_all_correct: false,
        },
        ItemBody::Match {
            prompts: vec!["one".into(), "two".into()],
            choices: vec!["one".into(), "two".into()],
        },
        ItemBody::Num {
            answer: 4.0,
            tolerance: 0.1,
            tolerance_message: false,
        },
        ItemBody::Fib {
            answers: vec!["answer".into()],
        },
        ItemBody::MultiFib {
            answers: BTreeMap::from([
                ("one".into(), vec!["one".into()]),
                ("two".into(), vec!["two".into()]),
            ]),
        },
        ItemBody::Order {
            answers: vec!["first".into(), "second".into(), "third".into()],
        },
    ];
    for (index, body) in bodies.into_iter().enumerate() {
        let question = if index == 5 {
            "MULTI [one] [two]".into()
        } else {
            format!("Question number {index}")
        };
        bank.add_item(Item::new(question, body).expect("valid item"))
            .expect("add item");
    }
    bank
}

#[test]
fn plain_prompt_preserves_displayed_entities_and_literal_markup() {
    assert_eq!(
        plain_text("<strong>&Delta;G &minus; T</strong>"),
        "&lt;strong&gt;\u{0394}G \u{2212} T&lt;/strong&gt;"
    );
    assert_eq!(plain_text("A &amp; B &unknown;"), "A &amp; B &amp;unknown;");
}

#[test]
fn multi_fib_alternatives_use_single_correct_value_and_explicit_predicates() {
    let answers = BTreeMap::from([("order".into(), vec!["kfr".into(), "rfk".into()])]);
    let item = Item::new(
        "Gene order: [order]".into(),
        ItemBody::MultiFib {
            answers: answers.clone(),
        },
    )
    .expect("valid alternative answers");
    let xml = multi_fib_item(&item.render_view(), &answers);
    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut correct_response = false;
    let mut declared_values = 0;
    let mut alternatives = 0;
    loop {
        match reader.read_event().expect("well-formed QTI") {
            quick_xml::events::Event::Start(node) if node.name().as_ref() == "correctResponse" => {
                correct_response = true
            }
            quick_xml::events::Event::End(node) if node.name().as_ref() == "correctResponse" => {
                correct_response = false
            }
            quick_xml::events::Event::Start(node)
                if correct_response && node.name().as_ref() == "value" =>
            {
                declared_values += 1
            }
            quick_xml::events::Event::Start(node) if node.name().as_ref() == "match" => {
                alternatives += 1
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    assert_eq!(declared_values, 1, "single-cardinality declaration");
    assert_eq!(
        alternatives, 2,
        "both authored answers have grading predicates"
    );
    assert!(xml.contains(">kfr</baseValue>"));
    assert!(xml.contains(">rfk</baseValue>"));
}

fn emitted_multi_fib_score(xml: &str, correct: &[&str]) -> f64 {
    let correct = correct.iter().copied().collect::<BTreeSet<_>>();
    let mut score = 0.0;
    for condition in xml.split("<responseCondition>").skip(1) {
        let response = condition
            .split("<variable identifier=\"")
            .nth(1)
            .and_then(|tail| tail.split_once('\"'))
            .map(|(identifier, _)| identifier)
            .expect("response condition variable");
        if !correct.contains(response) {
            continue;
        }
        let action = condition
            .split("<setOutcomeValue identifier=\"SCORE\">")
            .nth(1)
            .expect("SCORE action");
        let value = action
            .split("<baseValue baseType=\"float\">")
            .nth(1)
            .and_then(|tail| tail.split_once("</baseValue>"))
            .map(|(value, _)| value.parse::<f64>().expect("float contribution"))
            .expect("float contribution");
        if action.starts_with("<sum><variable identifier=\"SCORE\"/>") {
            score += value;
        } else {
            score = value;
        }
    }
    score
}

fn png_bytes() -> Vec<u8> {
    [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248, 207, 192, 240, 31,
        0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
    .to_vec()
}

fn archive_text(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> String {
    let mut text = String::new();
    archive
        .by_name(name)
        .unwrap_or_else(|_| panic!("archive entry '{name}'"))
        .read_to_string(&mut text)
        .expect("read archive text");
    text
}

#[test]
fn writes_all_seven_types_with_local_media_and_no_integrity_errors() {
    let assets =
        MemoryAssets::from_entries(BTreeMap::from([("figure.png".to_owned(), png_bytes())]))
            .expect("memory assets");
    let bank = seven_item_bank();
    let first = bank.get(0).expect("first").clone();
    let mut image_bank = ItemBank::new(true);
    image_bank
        .add_item(
            Item::new(
                format!("{} <img src=\"figure.png\"/>", first.common().question_text),
                first.body().clone(),
            )
            .expect("image item"),
        )
        .expect("add image item");
    for item in bank.iter_ordered().skip(1) {
        image_bank.add_item(item.clone()).expect("add item");
    }
    let output = "package.zip";
    let outcome = BlackboardQti21Writer
        .write_package(&image_bank, &assets, &context(output))
        .expect("package");
    let mut archive = zip::ZipArchive::new(Cursor::new(primary(&outcome).bytes())).expect("zip");
    assert_eq!(archive.len(), 10);
    let first_item = archive_text(&mut archive, "qti21_items/item_00001.xml");
    assert!(first_item.contains("correctResponse><value>answer_2"));
    assert!(first_item.contains("src=\"../figure.png\""));
    let multiple_answer = archive_text(&mut archive, "qti21_items/item_00002.xml");
    assert!(multiple_answer.contains("cardinality=\"multiple\""));
    assert!(multiple_answer.contains("<value>answer_1</value><value>answer_3</value>"));
    let matching = archive_text(&mut archive, "qti21_items/item_00003.xml");
    // QTI directed-pair values are represented as whitespace-separated identifiers, as in
    // the frozen Python writer's `create_response_declaration_MATCH` helper.
    assert!(matching.contains("<value>prompt_001 choice_001</value>"));
    assert!(matching.contains("mapKey=\"prompt_002 choice_002\" mappedValue=\"1\""));
    let numeric = archive_text(&mut archive, "qti21_items/item_00004.xml");
    assert!(numeric.contains("baseType=\"float\""));
    assert!(numeric.contains("tolerance=\"0.1 0.1\""));
    let fib = archive_text(&mut archive, "qti21_items/item_00005.xml");
    assert!(fib.contains("mapKey=\"answer\" caseSensitive=\"false\" mappedValue=\"100.0\""));
    let multi_fib = archive_text(&mut archive, "qti21_items/item_00006.xml");
    assert!(multi_fib.contains("responseIdentifier=\"one\""));
    assert!(multi_fib.contains("responseIdentifier=\"two\""));
    assert!(multi_fib.contains(
        "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"><defaultValue><value>0</value></defaultValue></outcomeDeclaration>"
    ));
    let contribution = "<sum><variable identifier=\"SCORE\"/><baseValue baseType=\"float\">50.00</baseValue></sum>";
    let correct_blank_count = multi_fib.matches(contribution).count();
    assert_eq!(correct_blank_count, 2);
    assert_eq!(emitted_multi_fib_score(&multi_fib, &[]), 0.0);
    assert_eq!(emitted_multi_fib_score(&multi_fib, &["one"]), 50.0);
    assert_eq!(emitted_multi_fib_score(&multi_fib, &["two"]), 50.0);
    assert_eq!(emitted_multi_fib_score(&multi_fib, &["one", "two"]), 100.0);
    let overwrite_mutant = multi_fib
        .replace("<sum><variable identifier=\"SCORE\"/>", "")
        .replace("</sum>", "");
    assert_eq!(
        emitted_multi_fib_score(&overwrite_mutant, &["one", "two"]),
        50.0
    );
    let ordered = archive_text(&mut archive, "qti21_items/item_00007.xml");
    assert!(ordered.contains("cardinality=\"ordered\""));
    assert!(ordered.contains("<value>choice_001</value><value>choice_002</value>"));
    let entries = qti_integrity::check_package(primary(&outcome).bytes());
    assert!(
        entries
            .iter()
            .all(|violation| violation.severity != qti_integrity::Severity::Error),
        "integrity errors: {entries:#?}"
    );
}

#[test]
fn policies_reject_data_uri_before_output() {
    let bank = seven_item_bank();
    let item = bank.get(0).expect("item");
    let mut media_bank = ItemBank::new(true);
    media_bank
        .add_item(
            Item::new(
                format!(
                    "{} <img src=\"data:image/png;base64,AA==\"/>",
                    item.common().question_text
                ),
                item.body().clone(),
            )
            .expect("data item"),
        )
        .expect("add");
    let output = "must-not-exist.zip";
    let error = BlackboardQti21Writer
        .write_package(&media_bank, &MemoryAssets::default(), &context(output))
        .expect_err("data URI rejected");
    assert!(error.to_string().contains("data URI"));
}

#[test]
fn collision_safe_media_stays_with_its_item_and_remote_urls_are_preserved() {
    let assets = MemoryAssets::from_entries(BTreeMap::from([
        ("left/figure.png".to_owned(), png_bytes()),
        ("right/figure.png".to_owned(), png_bytes()),
    ]))
    .expect("memory assets");
    let mut bank = ItemBank::new(true);
    for question in [
        "First image <img src=\"left/figure.png\"/>",
        "Second image <img src=\"right/figure.png\"/>",
        "Remote image <img src=\"https://example.test/image.png\"/>",
    ] {
        bank.add_item(
            Item::new(
                question.into(),
                ItemBody::Mc {
                    choices: vec!["first".into(), "second".into()],
                    answer: "first".into(),
                },
            )
            .expect("item"),
        )
        .expect("add item");
    }
    let output = "collision.zip";
    let outcome = BlackboardQti21Writer
        .write_package(&bank, &assets, &context(output))
        .expect("package");
    assert_eq!(primary(&outcome).name(), output);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].item_crc,
        bank.get(2).expect("third item").crc().to_string()
    );
    assert_eq!(outcome.warnings[0].action, MediaAction::KeptVerbatim);
    assert_eq!(outcome.warnings[0].src, "https://example.test/image.png");
    let mut archive =
        zip::ZipArchive::new(Cursor::new(primary(&outcome).bytes())).expect("zip archive");
    assert!(archive.by_name("figure.png").is_ok());
    assert!(archive.by_name("figure(1).png").is_ok());
    for name in ["figure.png", "figure(1).png"] {
        let mut bytes = Vec::new();
        archive
            .by_name(name)
            .expect("collision image")
            .read_to_end(&mut bytes)
            .expect("image bytes");
        assert_eq!(bytes, png_bytes());
    }
    let mut first = String::new();
    archive
        .by_name("qti21_items/item_00001.xml")
        .expect("first XML")
        .read_to_string(&mut first)
        .expect("read first XML");
    let mut second = String::new();
    archive
        .by_name("qti21_items/item_00002.xml")
        .expect("second XML")
        .read_to_string(&mut second)
        .expect("read second XML");
    let mut third = String::new();
    archive
        .by_name("qti21_items/item_00003.xml")
        .expect("third XML")
        .read_to_string(&mut third)
        .expect("read third XML");
    assert!(first.contains("../figure.png"));
    assert!(second.contains("../figure(1).png"));
    assert!(third.contains("https://example.test/image.png"));
    assert!(
        qti_integrity::check_package(primary(&outcome).bytes())
            .iter()
            .all(|violation| violation.severity != qti_integrity::Severity::Error)
    );
}

#[test]
fn makes_html_named_entities_safe_for_qti_xml() {
    assert_eq!(
        fragment(
            "<p title=\"&Gamma;&#34;\">&alpha; &amp; &#946; &#38; &#60; &AMP; &notARealEntity;</p>",
        ),
        "<p title=\"\u{393}&quot;\">\u{3B1} &amp; \u{3B2} &amp; &lt; &amp; &amp;notARealEntity;</p>"
    );
    let raw_html = fragment(
        "<script>const terminator=\"]]>\";for(let i=0;i<items.length;i++){ready&&draw(i);}</script><td rowspan=2><a href=\"https://example.test/?a=1&b=2\">link</a></td>",
    );
    assert!(raw_html.contains("<![CDATA[const terminator=\"]]]]><![CDATA[>\";for(let i=0;i<items.length;i++){ready&&draw(i);}]]>"));
    assert!(raw_html.contains("rowspan=\"2\""));
    assert!(raw_html.contains("a=1&amp;b=2"));
    let xml = format!("<root>{raw_html}</root>");
    let mut reader = quick_xml::Reader::from_str(&xml);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("writer fragment must be XML: {error}"),
        }
    }
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            "<p title=\"&Gamma;&#34;\">&alpha; &amp; &#946; &#38; &#60; &AMP; &notARealEntity;</p>"
                .into(),
            ItemBody::Mc {
                choices: vec!["one".into(), "two".into()],
                answer: "one".into(),
            },
        )
        .expect("entity-bearing item"),
    )
    .expect("add item");
    let output = "entities.zip";
    let outcome = BlackboardQti21Writer
        .write_package(&bank, &MemoryAssets::default(), &context(output))
        .expect("package");
    let mut archive =
        zip::ZipArchive::new(Cursor::new(primary(&outcome).bytes())).expect("zip archive");
    let item = archive_text(&mut archive, "qti21_items/item_00001.xml");
    assert!(item.contains("title=\"\u{393}&quot;\""));
    assert!(item.contains("\u{3B1} &amp; \u{3B2} &amp; &lt; &amp; &amp;notARealEntity;"));
    assert!(
        qti_integrity::check_package(primary(&outcome).bytes())
            .iter()
            .all(|violation| violation.severity != qti_integrity::Severity::Error)
    );
}

#[test]
fn declares_every_supported_kind() {
    assert_eq!(
        BlackboardQti21Writer.supported_kinds(),
        crate::engine(super::NAME)
            .expect("registered engine")
            .supported_kinds
    );
}

#[test]
fn uses_explicit_output_name_and_manifest_date_with_existing_format_title() {
    let context = WriteContext::new(
        "custom/qti.payload".to_owned(),
        DocumentMetadata {
            title: "Caller document".to_owned(),
            date: "2040-01-02".to_owned(),
        },
        42,
    )
    .expect("context");
    let outcome = BlackboardQti21Writer
        .write_package(&seven_item_bank(), &MemoryAssets::default(), &context)
        .expect("package");
    assert_eq!(primary(&outcome).name(), "custom/qti.payload");
    let entries = qti_integrity::read_zip_entries(primary(&outcome).bytes()).expect("ZIP entries");
    let manifest = std::str::from_utf8(&entries["imsmanifest.xml"]).expect("manifest text");
    assert!(manifest.contains("2040-01-02"));
    assert!(manifest.contains("QTI 2.1"));
    assert!(!manifest.contains("Caller document"));
}
