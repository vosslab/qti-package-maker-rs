use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tempfile::tempdir;
use thiserror::Error;

use super::{
    ConversionError, FragmentRenderer, RenderCache, RenderMetrics, RenderedPng, convert_bank,
    convert_bank_with_metrics,
};
use qti_core::{FieldId, Item, ItemBank, ItemBody, MediaBaseDir};
use qti_molecule::CanvasSource;

#[derive(Debug, Error)]
#[error("test renderer deliberately failed")]
struct TestRenderError;

#[derive(Default)]
struct TestRenderer {
    table_calls: AtomicUsize,
    fail: bool,
}

impl FragmentRenderer for TestRenderer {
    type Error = TestRenderError;

    fn cache_discriminator(&self) -> String {
        "test-renderer-v1".to_owned()
    }

    fn render_canvas(&self, _source: &CanvasSource) -> Result<RenderedPng, Self::Error> {
        Ok(RenderedPng {
            bytes: vec![7],
            metrics: Default::default(),
        })
    }

    fn render_table(&self, _html: &str) -> Result<RenderedPng, Self::Error> {
        self.table_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(TestRenderError)
        } else {
            Ok(RenderedPng {
                bytes: vec![137, 80, 78, 71],
                metrics: Default::default(),
            })
        }
    }
}

fn mc(question: &str) -> Item {
    Item::new(
        question.to_owned(),
        ItemBody::Mc {
            choices: vec!["yes".to_owned(), "no".to_owned()],
            answer: "yes".to_owned(),
        },
    )
    .expect("valid test item")
}

fn source_bank(root: &std::path::Path, question: &str) -> ItemBank {
    let mut bank = ItemBank::with_media_base_dir(false, MediaBaseDir::external(root.to_owned()));
    bank.add_item(mc(question)).expect("item added");
    bank
}

#[test]
fn conversion_copies_relative_source_media_without_writing_source_root() {
    let source = tempdir().expect("source directory");
    fs::write(source.path().join("existing.png"), [1, 2, 3]).expect("source image");
    let bank = source_bank(
        source.path(),
        "<img src=\"existing.png\" /><table><tr><td>table</td></tr></table>",
    );

    let output = convert_bank(&bank, &TestRenderer::default(), &RenderCache::new())
        .expect("conversion succeeds");
    let output_root = output
        .media_base_dir()
        .expect("derived media root")
        .to_owned();
    assert_ne!(output_root, source.path());
    assert_eq!(
        fs::read(source.path().join("existing.png")).expect("source untouched"),
        [1, 2, 3]
    );
    assert!(!source.path().join("__qti_generated").exists());
    assert_eq!(
        fs::read(output_root.join("existing.png")).expect("copied source"),
        [1, 2, 3]
    );
    assert!(
        output
            .collect_assets()
            .expect("all derived sources resolve")
            .assets()
            .iter()
            .any(|asset| asset.src.contains("_table_1.png"))
    );

    drop(output);
    assert!(!output_root.exists(), "owned derived root is cleaned up");
}

#[test]
fn absolute_source_is_confined_under_conversion_input_directory() {
    let outside = tempdir().expect("outside directory");
    let absolute = outside.path().join("original.png");
    fs::write(&absolute, [9, 8, 7]).expect("absolute image");
    let source = tempdir().expect("source directory");
    let bank = source_bank(
        source.path(),
        &format!(
            "<img src=\"{}\" /><table><tr><td>x</td></tr></table>",
            absolute.display()
        ),
    );

    let output = convert_bank(&bank, &TestRenderer::default(), &RenderCache::new())
        .expect("conversion succeeds");
    let question = &output
        .get(0)
        .expect("converted item")
        .common()
        .question_text;
    assert!(!question.contains(absolute.to_string_lossy().as_ref()));
    assert!(question.contains("__qti_input/"));
    assert_eq!(
        fs::read(&absolute).expect("external source untouched"),
        [9, 8, 7]
    );
    output.collect_assets().expect("rewritten source resolves");
}

#[test]
fn authored_generated_directory_is_reserved_before_generated_pngs_are_named() {
    let source = tempdir().expect("source directory");
    fs::create_dir(source.path().join("__qti_generated")).expect("authored directory");
    fs::write(
        source.path().join("__qti_generated/authored.png"),
        [4, 3, 2],
    )
    .expect("authored image");
    let bank = source_bank(
        source.path(),
        "<img src=\"__qti_generated/authored.png\" /><table><tr><td>x</td></tr></table>",
    );

    let output = convert_bank(&bank, &TestRenderer::default(), &RenderCache::new())
        .expect("conversion succeeds");
    let question = &output
        .get(0)
        .expect("converted item")
        .common()
        .question_text;
    assert!(question.contains("__qti_generated_1/"));
    assert_eq!(
        fs::read(source.path().join("__qti_generated/authored.png"))
            .expect("authored source untouched"),
        [4, 3, 2]
    );
    output
        .collect_assets()
        .expect("both source and generated images resolve");
}

#[test]
fn no_selected_fragment_returns_shared_source_bank() {
    let source = tempdir().expect("source directory");
    let bank = source_bank(source.path(), "plain question");
    let output = convert_bank(&bank, &TestRenderer::default(), &RenderCache::new())
        .expect("conversion succeeds");

    assert_eq!(output.media_base_dir(), bank.media_base_dir());
    assert_eq!(output.get(0), bank.get(0));
}

#[test]
fn renderer_failure_preserves_source_and_identifies_fragment() {
    let source = tempdir().expect("source directory");
    fs::write(source.path().join("existing.png"), [5]).expect("source image");
    let bank = source_bank(
        source.path(),
        "<img src=\"existing.png\" /><table><tr><td>broken</td></tr></table>",
    );
    let original_crc = *bank.get(0).expect("item").crc();
    let error = convert_bank(
        &bank,
        &TestRenderer {
            fail: true,
            ..TestRenderer::default()
        },
        &RenderCache::new(),
    )
    .expect_err("renderer failure is returned");

    match error {
        ConversionError::Render {
            item_crc,
            field,
            snippet,
            source,
            ..
        } => {
            assert_eq!(item_crc, original_crc);
            assert_eq!(field, FieldId::Stem);
            assert!(snippet.contains("table"));
            assert!(source.downcast_ref::<TestRenderError>().is_some());
        }
        other => panic!("unexpected conversion error: {other:?}"),
    }
    assert_eq!(
        fs::read(source.path().join("existing.png")).expect("source unchanged"),
        [5]
    );
    assert!(!source.path().join("__qti_generated").exists());
}

#[test]
fn metrics_failure_keeps_typed_error_and_observed_renderer_attempt() {
    let source = tempdir().expect("source directory");
    let bank = source_bank(
        source.path(),
        "<table><tr><td>deliberately failing render</td></tr></table>",
    );
    let failure = convert_bank_with_metrics(
        &bank,
        &TestRenderer {
            fail: true,
            ..TestRenderer::default()
        },
        &RenderCache::new(),
    )
    .expect_err("metrics conversion returns failure context");

    assert_eq!(failure.metrics.requested_fragments, 1);
    assert_eq!(failure.metrics.cache_misses, 1);
    assert_eq!(failure.metrics.renderer_attempts, 1);
    assert_eq!(failure.metrics.renderer_successes, 0);
    assert_eq!(failure.metrics.renderer_failures, 1);
    assert!(matches!(*failure.error, ConversionError::Render { .. }));
}

#[test]
fn repeated_identical_field_reuses_one_render_and_preserves_item_numbering() {
    let source = tempdir().expect("source directory");
    let repeated = "<table><tr><td>same</td></tr></table>";
    let mut bank =
        ItemBank::with_media_base_dir(false, MediaBaseDir::external(source.path().to_owned()));
    bank.add_item(
        Item::new(
            repeated.to_owned(),
            ItemBody::Mc {
                choices: vec![repeated.to_owned(), "yes".to_owned()],
                answer: "yes".to_owned(),
            },
        )
        .expect("valid item"),
    )
    .expect("item added");
    let renderer = TestRenderer::default();
    let output = convert_bank(&bank, &renderer, &RenderCache::new()).expect("conversion succeeds");

    assert_eq!(renderer.table_calls.load(Ordering::SeqCst), 1);
    assert_eq!(output.get(0).expect("item").common().item_number, 1);
    let question = &output.get(0).expect("item").common().question_text;
    assert!(question.contains("_table_1.png"));
    let ItemBody::Mc { choices, .. } = output.get(0).expect("item").body() else {
        panic!("expected multiple choice item");
    };
    assert_eq!(question, &choices[0]);
}

#[test]
fn conversion_preserves_source_identity_order_and_bank_positions() {
    let source = tempdir().expect("source directory");
    let mut bank =
        ItemBank::with_media_base_dir(false, MediaBaseDir::external(source.path().to_owned()));
    for label in ["alpha", "beta"] {
        let field = format!("<table><tr><td>{label}</td></tr></table>");
        bank.add_item(
            Item::new(
                field.clone(),
                ItemBody::Mc {
                    choices: vec![field.clone(), "other".to_owned()],
                    answer: field,
                },
            )
            .expect("valid item"),
        )
        .expect("source item added");
    }
    let source_crcs = bank
        .iter_ordered()
        .map(|item| *item.crc())
        .collect::<Vec<_>>();

    let output = convert_bank(&bank, &TestRenderer::default(), &RenderCache::new())
        .expect("conversion succeeds");

    assert_eq!(
        output.len(),
        2,
        "distinct source identities remain distinct"
    );
    assert_eq!(
        output
            .iter_ordered()
            .map(|item| *item.crc())
            .collect::<Vec<_>>(),
        source_crcs,
        "conversion retains source CRCs in insertion order"
    );
    for (index, item) in output.iter_ordered().enumerate() {
        assert_eq!(item.common().item_number, index + 1);
        let display_fields = item.field_strings().collect::<Vec<_>>();
        assert_eq!(
            display_fields
                .iter()
                .filter(|field| field.contains("<img"))
                .count(),
            3,
            "every transformed display field has its generated image"
        );
        assert!(display_fields.iter().all(|field| !field.contains("<table")));
    }
}

#[test]
fn metrics_count_three_requested_fields_with_one_render_and_two_cache_hits() {
    let source = tempdir().expect("source directory");
    let table = "<table><tr><td>same</td></tr></table>";
    let mut bank =
        ItemBank::with_media_base_dir(false, MediaBaseDir::external(source.path().to_owned()));
    bank.add_item(
        Item::new(
            table.to_owned(),
            ItemBody::Mc {
                choices: vec![table.to_owned(), "no".to_owned()],
                answer: table.to_owned(),
            },
        )
        .expect("valid item"),
    )
    .expect("item added");
    let renderer = MetricsRenderer::default();

    let (_converted, metrics) = convert_bank_with_metrics(&bank, &renderer, &RenderCache::new())
        .expect("conversion succeeds");
    assert_eq!(metrics.requested_fragments, 3);
    assert_eq!(metrics.cache_misses, 1);
    assert_eq!(metrics.cache_hits, 2);
    assert_eq!(metrics.cache_waits, 0);
    assert_eq!(metrics.renderer_attempts, 1);
    assert_eq!(metrics.renderer_successes, 1);
    assert_eq!(metrics.renderer_failures, 0);
    assert_eq!(metrics.render.layout, Duration::from_millis(1));
    assert_eq!(metrics.render.paint, Duration::from_millis(2));
    assert_eq!(metrics.render.encode, Duration::from_millis(3));
}

#[derive(Default)]
struct MetricsRenderer(TestRenderer);

impl FragmentRenderer for MetricsRenderer {
    type Error = TestRenderError;

    fn cache_discriminator(&self) -> String {
        self.0.cache_discriminator()
    }

    fn render_canvas(&self, source: &CanvasSource) -> Result<RenderedPng, Self::Error> {
        self.0.render_canvas(source)
    }

    fn render_table(&self, html: &str) -> Result<RenderedPng, Self::Error> {
        self.0.render_table(html).map(|mut png| {
            png.metrics = RenderMetrics {
                layout: Duration::from_millis(1),
                paint: Duration::from_millis(2),
                encode: Duration::from_millis(3),
            };
            png
        })
    }
}
