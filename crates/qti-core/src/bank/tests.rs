//! Existing bank contract tests, kept with their owning module.

use super::{AddOutcome, AssetCollectionAction, BankError, ItemBank};
use crate::item::{Item, ItemBody};
use crate::media::{AssetSource, MediaError, MemoryAssets};

fn mc(question: &str, answer: &str) -> Item {
    Item::new(
        question.to_owned(),
        ItemBody::Mc {
            choices: vec!["one".to_owned(), "two".to_owned()],
            answer: answer.to_owned(),
        },
    )
    .expect("test item is valid")
}

#[test]
fn presentation_rewrite_preserves_merge_numbers_and_rejects_source_changes() {
    let mut left = ItemBank::default();
    left.add_item(mc("first", "one")).expect("first");
    left.add_item(mc("shared", "one")).expect("shared");
    let mut right = ItemBank::default();
    right.add_item(mc("shared", "one")).expect("replacement");
    let bank = left.merge(&right).expect("merge");
    let rewritten = bank
        .with_rewritten_items(|item| Ok::<_, BankError>(item.clone().with_item_number(99)))
        .expect("presentation rewrite");
    assert_eq!(
        rewritten
            .iter_ordered()
            .map(|item| (*item.crc(), item.common().item_number))
            .collect::<Vec<_>>(),
        bank.iter_ordered()
            .map(|item| (*item.crc(), item.common().item_number))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        rewritten.get(1).expect("replacement").common().item_number,
        1
    );
    let changed = bank.with_rewritten_items(|_| Ok::<_, BankError>(mc("changed", "one")));
    assert!(matches!(changed, Err(BankError::RewriteIdentity { .. })));
}

#[test]
fn adds_in_order_and_reports_duplicate_without_replacing() {
    let item = mc("Question", "one");
    let crc = *item.crc();
    let mut bank = ItemBank::default();

    assert_eq!(
        bank.add_item(item).expect("add succeeds"),
        AddOutcome::Added { crc }
    );
    assert_eq!(
        bank.add_item(mc("Question", "two"))
            .expect("duplicate is valid"),
        AddOutcome::Duplicate { crc }
    );
    assert_eq!(bank.len(), 1);
    assert_eq!(bank.get(0).expect("first item").common().item_number, 1);
    assert!(
        matches!(bank.get(0).expect("first item").body(), ItemBody::Mc { answer, .. } if answer == "one")
    );
}

#[test]
fn merge_replaces_from_right_but_keeps_left_order() {
    let mut left = ItemBank::default();
    left.add_item(mc("First", "one")).expect("add succeeds");
    left.add_item(mc("Shared", "one")).expect("add succeeds");
    let mut right = ItemBank::default();
    right
        .add_item(mc("Shared", "two").with_item_number(99))
        .expect("add succeeds");
    right.add_item(mc("Last", "one")).expect("add succeeds");

    let merged = left.merge(&right).expect("merge succeeds");
    let questions = merged
        .iter_ordered()
        .map(|item| item.common().question_text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(questions, ["First", "Shared", "Last"]);
    assert!(
        matches!(merged.get(1).expect("shared item").body(), ItemBody::Mc { answer, .. } if answer == "two")
    );
    assert_eq!(merged.get(1).expect("shared item").common().item_number, 1);
}

#[test]
fn rejects_mixed_items_when_not_allowed() {
    let mut bank = ItemBank::default();
    bank.add_item(mc("Question", "one")).expect("add succeeds");
    let fib = Item::new(
        "Other question".to_owned(),
        ItemBody::Fib {
            answers: vec!["answer".to_owned()],
        },
    )
    .expect("test item is valid");

    assert!(matches!(
        bank.add_item(fib),
        Err(BankError::MixedItemKinds { .. })
    ));
}

#[test]
fn slice_trim_sort_and_renumber_preserve_a_safe_read_api() {
    let mut bank = ItemBank::default();
    for question in ["Third", "First", "Second"] {
        bank.add_item(mc(question, "one")).expect("add succeeds");
    }
    let slice = bank.slice(1..3);
    assert_eq!(slice.len(), 2);
    assert_eq!(slice.get(0).expect("slice first").common().item_number, 1);

    bank.trim_to(2);
    assert_eq!(bank.len(), 2);
    bank.sort_by_crc();
    let crcs = bank
        .iter_ordered()
        .map(|item| item.crc())
        .collect::<Vec<_>>();
    assert!(crcs.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(bank.get(1).expect("second item").common().item_number, 2);
}

#[test]
fn collect_assets_deduplicates_by_source_and_assigns_collision_safe_names() {
    let mut source = MemoryAssets::new();
    source
        .insert("a/figure.png", b"first".to_vec())
        .expect("first image");
    source
        .insert("b/figure.png", b"second".to_vec())
        .expect("second image");
    let mut bank = ItemBank::default();
    let item = mc(
        "Diagram <img src='a/figure.png'/><img src='b/figure.png'/><img src='a/figure.png'/>",
        "one",
    );
    let crc = *item.crc();
    bank.add_item(item).expect("add item");

    let collected = bank.collect_assets(&source).expect("collect media");
    assert_eq!(collected.assets().len(), 2);
    assert_eq!(collected.assets()[0].src, "a/figure.png");
    assert_eq!(
        collected.assets()[0].output_name.as_deref(),
        Some("figure.png")
    );
    assert_eq!(collected.assets()[1].src, "b/figure.png");
    assert_eq!(
        collected.assets()[1].output_name.as_deref(),
        Some("figure(1).png")
    );
    let dependencies = collected.dependencies_for(&crc).expect("item dependencies");
    assert_eq!(dependencies.len(), 2);
    assert_eq!(dependencies[0].src, "a/figure.png");
    assert_eq!(dependencies[1].src, "b/figure.png");
}

#[test]
fn missing_assets_report_item_source_and_operation() {
    let mut bank = ItemBank::default();
    let item = mc("Diagram <img src='figures/missing.png'/>", "one");
    let crc = *item.crc();
    bank.add_item(item).expect("add item");
    match bank.collect_assets(&MemoryAssets::new()) {
        Err(BankError::CollectAsset {
            item_crc,
            src,
            action,
            source,
        }) => {
            assert_eq!(item_crc, crc);
            assert_eq!(src.as_deref(), Some("figures/missing.png"));
            assert_eq!(action, AssetCollectionAction::ResolveAsset);
            assert!(matches!(source, MediaError::MissingAsset { .. }));
        }
        result => panic!("expected contextual collection error, received {result:?}"),
    }
    let inspected = bank.inspect_assets().expect("reference-only inspection");
    assert_eq!(
        inspected.assets()[0].output_name.as_deref(),
        Some("missing.png")
    );
    assert!(inspected.assets()[0].data_bytes.is_none());
}

#[test]
fn snapshots_each_source_once_and_preserves_dependency_document_order() {
    use std::borrow::Cow;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountingSource(AtomicUsize);
    impl AssetSource for CountingSource {
        fn read(&self, _: &str) -> Result<Cow<'_, [u8]>, MediaError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Cow::Borrowed(b"image"))
        }
    }
    let source = CountingSource(AtomicUsize::new(0));
    let mut bank = ItemBank::default();
    let first = mc(
        "First <img src='z/figure.png'/><img src='a/figure.png'/><img src='z/figure.png'/>",
        "one",
    );
    let crc = *first.crc();
    bank.add_item(first).expect("first item");
    bank.add_item(mc("Second <img src='z/figure.png'/>", "one"))
        .expect("second item");
    assert_eq!(
        source.0.load(Ordering::SeqCst),
        0,
        "building a bank is lazy"
    );
    let inspected = bank.inspect_assets().expect("inspect");
    assert_eq!(
        source.0.load(Ordering::SeqCst),
        0,
        "inspection does not read"
    );
    let collected = bank.collect_assets(&source).expect("collect");
    assert_eq!(source.0.load(Ordering::SeqCst), 2);
    assert_eq!(
        collected.assets()[0].output_name,
        inspected.assets()[0].output_name
    );
    let dependencies = collected.dependencies_for(&crc).expect("dependencies");
    assert_eq!(
        dependencies
            .iter()
            .map(|asset| asset.src.as_str())
            .collect::<Vec<_>>(),
        ["z/figure.png", "a/figure.png"]
    );
    assert_eq!(
        dependencies[0].output_name.as_deref(),
        Some("figure(1).png")
    );
    assert_eq!(
        dependencies[0].read_bytes().expect("owned snapshot"),
        b"image"
    );
}

#[test]
fn shared_image_fanout_retains_one_immutable_payload() {
    use std::borrow::Cow;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSource {
        bytes: Vec<u8>,
        reads: AtomicUsize,
    }
    impl AssetSource for CountingSource {
        fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
            assert_eq!(src, "shared.png");
            self.reads.fetch_add(1, Ordering::Relaxed);
            Ok(Cow::Borrowed(&self.bytes))
        }
    }
    let source = CountingSource {
        bytes: (0..1024 * 1024).map(|index| (index % 251) as u8).collect(),
        reads: AtomicUsize::new(0),
    };
    let mut bank = ItemBank::default();
    for index in 0..160 {
        bank.add_item(mc(
            &format!("Question {index} <img src='shared.png'/>"),
            "one",
        ))
        .expect("unique item");
    }

    let collected = bank.collect_assets(&source).expect("collect");
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    assert_eq!(collected.assets().len(), 1);
    let payload = collected.assets()[0].data_bytes.as_ref().expect("snapshot");
    assert_eq!(payload.as_ref(), source.bytes);
    assert_eq!(collected.iter_item_dependencies().len(), 160);
    for (_, dependencies) in collected.iter_item_dependencies() {
        assert_eq!(dependencies.len(), 1);
        assert!(Arc::ptr_eq(
            payload,
            dependencies[0]
                .data_bytes
                .as_ref()
                .expect("dependency payload")
        ));
    }
}

#[test]
fn source_read_failure_is_propagated_without_retry_or_fallback() {
    use std::borrow::Cow;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FailingSource(AtomicUsize);
    impl AssetSource for FailingSource {
        fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(MediaError::AssetRead {
                src: src.to_owned(),
                message: "provider denied the image".into(),
            })
        }
    }
    let source = FailingSource(AtomicUsize::new(0));
    let mut bank = ItemBank::default();
    bank.add_item(mc("<img src='shared.png'/>", "one"))
        .expect("item");
    assert!(matches!(
        bank.collect_assets(&source),
        Err(BankError::CollectAsset {
            source: MediaError::AssetRead { src, message },
            ..
        }) if src == "shared.png" && message == "provider denied the image"
    ));
    assert_eq!(source.0.load(Ordering::Relaxed), 1);
}
