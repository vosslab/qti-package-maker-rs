use super::media::restore_markdown_images;
use super::parser::parse_block;
use super::{boxed_reader, boxed_writer};
use crate::{EngineError, ReadInput, ReadLocation};
use qti_core::media::resolve_item_media_refs;
use qti_core::{Item, ItemBank, ItemBody, ItemFingerprint, MemoryAssets};

fn every_supported_item() -> ItemBank {
    let mut bank = ItemBank::new(true);
    for item in [
        Item::new(
            "MC stem".to_owned(),
            ItemBody::Mc {
                choices: vec!["first".to_owned(), "second".to_owned()],
                answer: "second".to_owned(),
            },
        ),
        Item::new(
            "MA stem".to_owned(),
            ItemBody::Ma {
                choices: vec!["first".to_owned(), "second".to_owned(), "third".to_owned()],
                answers: vec!["first".to_owned(), "third".to_owned()],
                min_answers_required: 1,
                allow_all_correct: true,
            },
        ),
        Item::new(
            "NUM stem".to_owned(),
            ItemBody::Num {
                answer: 2.5,
                tolerance: 0.01,
                tolerance_message: true,
            },
        ),
        Item::new(
            "FIB stem".to_owned(),
            ItemBody::Fib {
                answers: vec!["alpha".to_owned(), "beta".to_owned()],
            },
        ),
    ] {
        bank.add_item(item.expect("valid item")).expect("add item");
    }
    bank
}

#[test]
fn every_python_supported_shape_round_trips_by_fingerprint_and_order() {
    let output = "questions.txt";
    let bank = every_supported_item();
    let outcome = boxed_writer()
        .write_package(&bank, &MemoryAssets::new(), &context(output))
        .expect("write");
    let restored = boxed_reader()
        .read_items(
            ReadInput::File {
                name: primary(&outcome).name(),
                bytes: primary(&outcome).bytes(),
            },
            true,
        )
        .expect("read");
    assert!(restored.warnings.is_empty());
    assert_eq!(restored.bank.len(), bank.len());
    for (source, round_trip) in bank.iter_ordered().zip(restored.bank.iter_ordered()) {
        assert_eq!(source.kind(), round_trip.kind());
        assert_eq!(
            ItemFingerprint::new(source, Vec::new()).expect("source fingerprint"),
            ItemFingerprint::new(round_trip, Vec::new()).expect("restored fingerprint")
        );
    }
}

#[test]
fn writer_uses_text2qti_markers_and_blank_block_separators() {
    let output = "questions.txt";
    let outcome = boxed_writer()
        .write_package(
            &every_supported_item(),
            &MemoryAssets::new(),
            &context(output),
        )
        .expect("write");
    assert_eq!(
        document(&outcome),
        "1. MC stem\nA) first\n*B) second\n\n2. MA stem\n[*] first\n[ ] second\n[*] third\n\n3. NUM stem\n= 2.5 +- 0.01\n\n4. FIB stem\n* alpha\n* beta\n"
    );
}

#[test]
fn writer_returns_typed_unsupported_errors() {
    let cases = [
        (
            "MATCH stem",
            ItemBody::Match {
                prompts: vec!["prompt one".to_owned(), "prompt two".to_owned()],
                choices: vec!["choice one".to_owned(), "choice two".to_owned()],
            },
        ),
        (
            "Fill [blank] now",
            ItemBody::MultiFib {
                answers: [("blank".to_owned(), vec!["answer".to_owned()])]
                    .into_iter()
                    .collect(),
            },
        ),
        (
            "ORDER stem",
            ItemBody::Order {
                answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            },
        ),
    ];
    for (question, body) in cases {
        let mut bank = ItemBank::new(true);
        let item = Item::new(question.to_owned(), body).expect("valid source item");
        bank.add_item(item).expect("add item");
        let error = boxed_writer()
            .write_package(&bank, &MemoryAssets::new(), &context("questions.txt"))
            .expect_err("unsupported");
        assert!(matches!(
            error,
            EngineError::UnsupportedItemKind {
                engine: super::NAME,
                ..
            }
        ));
    }
}

#[test]
fn reader_discards_feedback_and_retains_block_warning_locations() {
    let input = "questions.txt";
    let input_text = "1. First question\na) wrong\n... ignored feedback\n*b) right\n+ also ignored\n\n2. broken\n\n3. Last\n* answer\n";
    let outcome = boxed_reader()
        .read_items(
            ReadInput::File {
                name: input,
                bytes: input_text.as_bytes(),
            },
            true,
        )
        .expect("read");
    assert_eq!(outcome.bank.len(), 2);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].location,
        ReadLocation::Block { number: 2 }
    );
    assert_eq!(
        outcome.bank.get(0).expect("MC").common().question_text,
        "First question"
    );
}

#[test]
fn reader_uses_python_defaults_for_omitted_ma_grading_options() {
    let parsed = parse_block("1. Choose all correct answers\n[*] first\n[*] second\n[ ] third")
        .expect("MA block parses")
        .expect("MA block is recognized");

    assert!(matches!(
        parsed.body(),
        ItemBody::Ma {
            min_answers_required: 1,
            allow_all_correct: true,
            ..
        }
    ));
}

#[test]
fn local_remote_and_data_media_keep_the_contract_end_to_end() {
    let png = [137, 80, 78, 71, 13, 10, 26, 10];
    let mut assets = MemoryAssets::new();
    assets
        .insert("local.png".to_owned(), png.to_vec())
        .expect("local image");
    let mut bank = ItemBank::new(true);
    bank.add_item(Item::new("<img src=\"local.png\" alt=\"local figure\"/><img src=\"https://example.test/remote.png\" alt=\"remote\"/><img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"inline\"/>".to_owned(), ItemBody::Fib { answers: vec!["yes".to_owned()] }).expect("valid item")).expect("add");
    let output = "questions.txt";
    let outcome = boxed_writer()
        .write_package(&bank, &assets, &context(output))
        .expect("write");
    assert_eq!(primary(&outcome).name(), output);
    assert_eq!(
        outcome
            .warnings
            .iter()
            .map(|warning| warning.src.as_str())
            .collect::<Vec<_>>(),
        [
            "local.png",
            "https://example.test/remote.png",
            "data:image/png;base64,iVBORw0KGgo="
        ]
    );
    let written = document(&outcome);
    assert!(written.contains("![local figure](media/local.png)"));
    assert!(written.contains("![remote](https://example.test/remote.png)"));
    assert!(written.contains("![inline](data:image/png;base64,iVBORw0KGgo=)"));
    assert_eq!(companions(&outcome)[0].bytes(), png);
    let restored = boxed_reader()
        .read_items(
            ReadInput::File {
                name: primary(&outcome).name(),
                bytes: primary(&outcome).bytes(),
            },
            true,
        )
        .expect("read");
    let source = bank.get(0).expect("source");
    let round_trip = restored.bank.get(0).expect("round trip");
    let source_refs = resolve_item_media_refs(source, &assets).expect("source media refs");
    let mut restored_assets = MemoryAssets::new();
    for file in companions(&outcome) {
        restored_assets
            .insert(file.name().to_owned(), file.bytes().to_vec())
            .expect("companion asset");
    }
    let restored_refs =
        resolve_item_media_refs(round_trip, &restored_assets).expect("restored media refs");
    assert_eq!(
        ItemFingerprint::new(source, source_refs).expect("source fingerprint"),
        ItemFingerprint::new(round_trip, restored_refs).expect("restored fingerprint")
    );
}

#[test]
fn write_outcome_warnings_follow_rendered_item_order() {
    let mut bank = ItemBank::new(true);
    let first = Item::new(
        "<img src=\"https://example.test/first.png\"/> first".to_owned(),
        ItemBody::Fib {
            answers: vec!["first".to_owned()],
        },
    )
    .expect("first item");
    let first_crc = first.crc().to_string();
    bank.add_item(first).expect("add first");
    let second = Item::new(
        "<img src=\"https://example.test/second.png\"/> second".to_owned(),
        ItemBody::Fib {
            answers: vec!["second".to_owned()],
        },
    )
    .expect("second item");
    let second_crc = second.crc().to_string();
    bank.add_item(second).expect("add second");

    let outcome = boxed_writer()
        .write_package(&bank, &MemoryAssets::new(), &context("questions.txt"))
        .expect("write");
    assert_eq!(
        outcome
            .warnings
            .iter()
            .map(|warning| warning.item_crc.as_str())
            .collect::<Vec<_>>(),
        [first_crc.as_str(), second_crc.as_str()]
    );
}

#[test]
fn empty_bank_preserves_python_empty_output_behavior() {
    let output = "questions.txt";
    let outcome = boxed_writer()
        .write_package(&ItemBank::new(true), &MemoryAssets::new(), &context(output))
        .expect("empty bank");
    assert_eq!(primary(&outcome).name(), output);
    assert!(outcome.warnings.is_empty());
    assert_eq!(document(&outcome), "");
}

#[test]
fn markdown_restoration_encodes_attributes_before_creating_html() {
    let restored = restore_markdown_images("1. Stem ![x\" onerror=\"boom](image.png)\n* answer");
    assert!(restored.contains("alt=\"x&quot; onerror=&quot;boom\""));
    assert!(parse_block(&restored).expect("parse").is_some());
}

fn context(name: &str) -> crate::WriteContext {
    crate::WriteContext::new(
        name.to_owned(),
        crate::DocumentMetadata {
            title: "Exam".to_owned(),
            date: "2026-09-30".to_owned(),
        },
        0,
    )
    .expect("valid context")
}

fn primary(outcome: &crate::WriteOutcome) -> &qti_core::NamedFile {
    match outcome.artifact.as_ref().expect("file artifact") {
        crate::WriteArtifact::File { primary, .. } => primary,
        crate::WriteArtifact::Directory { .. } => panic!("expected file artifact"),
    }
}

fn document(outcome: &crate::WriteOutcome) -> &str {
    std::str::from_utf8(primary(outcome).bytes()).expect("UTF-8 document")
}

fn companions(outcome: &crate::WriteOutcome) -> &[qti_core::NamedFile] {
    match outcome.artifact.as_ref().expect("file artifact") {
        crate::WriteArtifact::File { companions, .. } => companions,
        crate::WriteArtifact::Directory { .. } => panic!("expected file artifact"),
    }
}

#[test]
fn missing_companion_payload_returns_its_authored_source() {
    let mut bank = ItemBank::new(false);
    bank.add_item(
        Item::new(
            "See <img src=\"missing.png\"/>".to_owned(),
            ItemBody::Fib {
                answers: vec!["yes".to_owned()],
            },
        )
        .expect("item"),
    )
    .expect("add");
    let error = boxed_writer()
        .write_package(&bank, &MemoryAssets::new(), &context("questions.txt"))
        .expect_err("payload is needed for companion");
    assert!(
        matches!(error, EngineError::Bank(qti_core::BankError::CollectAsset {
        src: Some(ref src), source: qti_core::media::MediaError::MissingAsset { .. }, ..
    }) if src == "missing.png")
    );
}
