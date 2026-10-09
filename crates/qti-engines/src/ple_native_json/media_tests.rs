use std::collections::BTreeMap;

use qti_core::media::MemoryAssets;
use qti_core::{Item, ItemBank, ItemBody};
use serde_json::Value;

use super::super::export_bank;

#[test]
fn shared_display_media_is_snapshotted_once_across_questions() {
    use qti_core::media::{AssetSource, MediaError};
    use std::borrow::Cow;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSource {
        reads: AtomicUsize,
        bytes: Vec<u8>,
    }
    impl AssetSource for CountingSource {
        fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
            assert_eq!(src, "shared.png");
            self.reads.fetch_add(1, Ordering::Relaxed);
            Ok(Cow::Borrowed(&self.bytes))
        }
    }
    let source = CountingSource {
        reads: AtomicUsize::new(0),
        bytes: (0..1024 * 1024).map(|index| (index % 251) as u8).collect(),
    };
    let mut bank = ItemBank::new(true);
    for index in 0..160 {
        bank.add_item(item(
            &format!("Question {index} <img src='shared.png'/>"),
            mc(),
        ))
        .expect("unique item");
    }
    let result = export_bank(&bank, &source).expect("export");
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    assert_eq!(result.questions.len(), 160);
    let payload = result.questions[0].files[0].bytes();
    assert_eq!(payload, source.bytes);
    for question in &result.questions {
        assert_eq!(question.files.len(), 1);
        assert_eq!(question.files[0].name(), "media/shared.png");
        assert_eq!(question.files[0].bytes().as_ptr(), payload.as_ptr());
        assert!(question.source_json.contains("media/shared.png"));
    }
}

fn item(prompt: &str, body: ItemBody) -> Item {
    Item::new(prompt.to_owned(), body).expect("valid item")
}

fn mc() -> ItemBody {
    ItemBody::Mc {
        choices: vec!["A".into(), "B".into()],
        answer: "A".into(),
    }
}

fn json(source: &str) -> Value {
    serde_json::from_str(source).expect("valid JSON")
}

#[test]
fn shared_file_is_named_bank_wide_but_attached_only_to_dependent_questions() {
    let mut assets = MemoryAssets::new();
    assets
        .insert("shared.png", b"shared image bytes".to_vec())
        .expect("image");
    assets
        .insert("other.png", b"other image bytes".to_vec())
        .expect("image");
    let mut bank = ItemBank::new(true);
    bank.add_item(item(
        "<p class='q'>First</p><img src='shared.png' alt='cell' style='width: 20px'/>",
        mc(),
    ))
    .expect("first");
    bank.add_item(item("<img src='shared.png'/>", mc()))
        .expect("second");
    bank.add_item(item("<img src='other.png'/>", mc()))
        .expect("third");

    let result = export_bank(&bank, &assets).expect("media export");
    assert!(result.warnings.is_empty());
    let first = &result.questions[0];
    let second = &result.questions[1];
    let third = &result.questions[2];
    assert_eq!(first.files, second.files);
    assert_eq!(first.files.len(), 1);
    assert_eq!(first.files[0].name(), "media/shared.png");
    assert_eq!(first.files[0].bytes(), b"shared image bytes");
    assert_eq!(third.files.len(), 1);
    assert_eq!(third.files[0].name(), "media/other.png");
    let prompt = json(&first.source_json)["prompt"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(prompt.contains("<p class='q'>First</p>"), "{prompt}");
    assert!(prompt.contains("src=\"media/shared.png\""), "{prompt}");
    assert!(prompt.contains("style='width: 20px'"), "{prompt}");
    assert!(
        json(&second.source_json)["prompt"]
            .as_str()
            .unwrap()
            .contains("media/shared.png")
    );
    assert!(
        json(&third.source_json)["prompt"]
            .as_str()
            .unwrap()
            .contains("media/other.png")
    );
}

#[test]
fn display_image_entity_decoding_must_resolve_to_supplied_file() {
    let mut assets = MemoryAssets::new();
    assets
        .insert("a&amp;b.png", b"literal entity filename".to_vec())
        .expect("image");
    let mut bank = ItemBank::new(true);
    bank.add_item(item("<img src='a&amp;b.png'/>", mc()))
        .expect("item");

    let error = export_bank(&bank, &assets).expect_err("decoded image path has no associated file");
    assert!(matches!(error, crate::EngineError::InvalidFormat { .. }));
    let message = error.to_string();
    assert!(message.contains("item 1"), "{message}");
    assert!(message.contains("media/a&b.png"), "{message}");
    assert!(message.contains("0 associated files"), "{message}");
}

#[test]
fn ordinary_local_image_and_encoded_external_url_remain_supported() {
    let mut assets = MemoryAssets::new();
    assets
        .insert("ordinary.png", b"local image".to_vec())
        .expect("image");
    let mut bank = ItemBank::new(true);
    bank.add_item(item(
        "<img src='ordinary.png'/><img src='https://example.org/image.png?a=1&amp;b=2'/>",
        mc(),
    ))
    .expect("item");

    let result = export_bank(&bank, &assets).expect("local and external media export");
    let question = &result.questions[0];
    assert_eq!(question.files.len(), 1);
    assert_eq!(question.files[0].name(), "media/ordinary.png");
    assert_eq!(question.files[0].bytes(), b"local image");
    let source = json(&question.source_json);
    assert!(
        source["prompt"]
            .as_str()
            .unwrap()
            .contains("media/ordinary.png")
    );
    assert_eq!(
        source["externalResources"][0]["url"],
        "https://example.org/image.png?a=1&b=2"
    );
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn display_choices_matching_and_ordering_receive_their_own_files() {
    let mut assets = MemoryAssets::new();
    assets.insert("a.png", b"a".to_vec()).expect("image");
    assets.insert("b.png", b"b".to_vec()).expect("image");
    let mut bank = ItemBank::new(true);
    bank.add_item(item(
        "Choose",
        ItemBody::Mc {
            choices: vec!["<img src='a.png'/>".into(), "other".into()],
            answer: "<img src='a.png'/>".into(),
        },
    ))
    .expect("choice");
    bank.add_item(item(
        "Match",
        ItemBody::Match {
            prompts: vec!["<img src='b.png'/>".into(), "P".into()],
            choices: vec!["C".into(), "<img src='a.png'/>".into()],
        },
    ))
    .expect("match");
    bank.add_item(item(
        "Order",
        ItemBody::Order {
            answers: vec!["<img src='b.png'/>".into(), "middle".into(), "last".into()],
        },
    ))
    .expect("order");

    let result = export_bank(&bank, &assets).expect("export");
    assert_eq!(result.questions[0].files.len(), 1);
    assert_eq!(result.questions[1].files.len(), 2);
    assert_eq!(result.questions[2].files.len(), 1);
    assert_eq!(
        json(&result.questions[0].source_json)["response"]["correctChoice"],
        "choice-1"
    );
    assert!(result.questions[1].source_json.contains("media/a.png"));
    assert!(result.questions[1].source_json.contains("media/b.png"));
    assert!(result.questions[2].source_json.contains("media/b.png"));
}

#[test]
fn literal_grading_answers_are_neither_collected_nor_rewritten() {
    let assets = MemoryAssets::new();
    let mut bank = ItemBank::new(true);
    let answer = "<img src=\"grading_only.png\"/>";
    bank.add_item(item(
        "Name the object",
        ItemBody::Fib {
            answers: vec![answer.into()],
        },
    ))
    .expect("fib");
    bank.add_item(item(
        "Name [object]",
        ItemBody::MultiFib {
            answers: BTreeMap::from([("object".into(), vec![answer.into()])]),
        },
    ))
    .expect("multi fib");

    let result = export_bank(&bank, &assets).expect("literal answers should need no media base");
    assert!(result.warnings.is_empty());
    for question in result.questions {
        assert!(question.files.is_empty());
        assert!(question.source_json.contains("grading_only.png"));
        let value = json(&question.source_json);
        let answers = if value["response"]["kind"] == "fillIn" {
            &value["response"]["answers"]
        } else {
            &value["response"]["blanks"][0]["answers"]
        };
        assert_eq!(answers[0], answer);
    }
}

#[test]
fn external_inventory_and_package_warning_are_per_question() {
    let assets = MemoryAssets::new();
    let mut bank = ItemBank::new(true);
    bank.add_item(item(
        "<img src='https://example.org/a.png'/><img src='https://example.org/a.png'/><a href='https://example.org/more'>More</a><link href='https://example.org/site.css'/>",
        mc(),
    ))
    .expect("first");
    bank.add_item(item("Second", mc())).expect("second");

    let result = export_bank(&bank, &assets).expect("external URL export");
    let resources = json(&result.questions[0].source_json)["externalResources"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(resources.len(), 3);
    assert_eq!(resources[0]["kind"], "image");
    assert_eq!(resources[1]["kind"], "link");
    assert_eq!(resources[2]["kind"], "stylesheet");
    assert!(result.questions[0].files.is_empty());
    assert!(result.questions[1].files.is_empty());
    assert!(
        json(&result.questions[1].source_json)
            .get("externalResources")
            .is_none()
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].src, "https://example.org/a.png");
}

#[test]
fn missing_local_file_keeps_typed_collection_error_and_data_uri_is_rejected() {
    let assets = MemoryAssets::new();
    let mut missing = ItemBank::new(true);
    missing
        .add_item(item("<img src='missing.png'/>", mc()))
        .expect("item");
    assert!(matches!(
        export_bank(&missing, &assets),
        Err(crate::EngineError::Bank(
            qti_core::BankError::CollectAsset { .. }
        ))
    ));

    let mut data = ItemBank::new(true);
    data.add_item(item("<img src='data:image/png;base64,YQ=='/>", mc()))
        .expect("item");
    let message = export_bank(&data, &assets)
        .expect_err("data image rejected")
        .to_string();
    assert!(
        message.contains("data URI image cannot be bundled"),
        "{message}"
    );
}
