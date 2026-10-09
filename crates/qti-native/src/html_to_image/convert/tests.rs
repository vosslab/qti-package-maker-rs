use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tempfile::tempdir;
use thiserror::Error;

use super::{
    ConversionError, FragmentRenderer, RenderCache, RenderMetrics, RenderedPng, convert_bank,
    convert_bank_with_metrics,
};
use qti_core::{FieldId, Item, ItemBank, ItemBody, ItemKind};
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
            bytes: test_png(),
            width: 120.0,
            height: 80.0,
            metrics: Default::default(),
        })
    }

    fn render_table(&self, _html: &str) -> Result<RenderedPng, Self::Error> {
        self.table_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(TestRenderError)
        } else {
            Ok(RenderedPng {
                bytes: test_png(),
                width: 120.0,
                height: 80.0,
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

fn source_bank(_root: &std::path::Path, question: &str) -> ItemBank {
    let mut bank = ItemBank::new(false);
    bank.add_item(mc(question)).expect("item added");
    bank
}

#[test]
fn fill_in_grading_literals_are_never_planned_read_or_rewritten() {
    let table = "<table><tr><td>literal</td></tr></table>";
    let literal = "<img src=\"missing.png\" />";
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            table.into(),
            ItemBody::Fib {
                answers: vec![table.into(), literal.into()],
            },
        )
        .expect("FIB"),
    )
    .expect("add FIB");
    bank.add_item(
        Item::new(
            format!("{table} [cell]"),
            ItemBody::MultiFib {
                answers: std::collections::BTreeMap::from([("cell".into(), vec![literal.into()])]),
            },
        )
        .expect("MULTI_FIB"),
    )
    .expect("add MULTI_FIB");
    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Fib, ItemKind::MultiFib],
        &qti_core::media::MemoryAssets::new(),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("grading literals require no asset payload");
    for (source, rewritten) in bank.iter_ordered().zip(output.iter_ordered()) {
        assert_eq!(source.body(), rewritten.body());
        assert!(rewritten.common().question_text.contains("<img"));
    }
    assert_eq!(assets.entries().len(), 2);
}

#[test]
fn selected_kinds_leave_unsupported_items_and_source_numbers_unchanged() {
    let mut left = ItemBank::new(true);
    left.add_item(
        Item::new(
            "<table><tr><td>skipped</td></tr></table><img src=\"missing.png\" />".into(),
            ItemBody::Order {
                answers: vec!["first".into(), "second".into(), "third".into()],
            },
        )
        .expect("ORDER"),
    )
    .expect("add ORDER");
    left.add_item(mc("<table><tr><td>selected</td></tr></table>"))
        .expect("MC");
    let mut right = ItemBank::new(true);
    right
        .add_item(left.get(1).expect("MC").clone())
        .expect("replacement numbered 1");
    let bank = left.merge(&right).expect("source numbers retained");
    let renderer = TestRenderer::default();
    let (output, _) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &qti_core::media::MemoryAssets::new(),
        &renderer,
        &RenderCache::new(),
    )
    .expect("unsupported missing image is ignored");
    assert_eq!(output.get(0), bank.get(0));
    assert_eq!(output.get(1).expect("MC").common().item_number, 1);
    assert_eq!(renderer.table_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn multiple_answer_keys_and_order_display_sequence_stay_synchronized() {
    let table = "<table><tr><td>display</td></tr></table>";
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            "Choose".into(),
            ItemBody::Ma {
                choices: vec![table.into(), "second".into(), "third".into()],
                answers: vec![table.into(), "second".into()],
                min_answers_required: 1,
                allow_all_correct: false,
            },
        )
        .expect("MA"),
    )
    .expect("add MA");
    bank.add_item(
        Item::new(
            "Order".into(),
            ItemBody::Order {
                answers: vec!["first".into(), table.into(), "third".into()],
            },
        )
        .expect("ORDER"),
    )
    .expect("add ORDER");
    let (output, _) = convert_bank(
        &bank,
        &[ItemKind::Ma, ItemKind::Order],
        &qti_core::media::MemoryAssets::new(),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("display conversion");
    let ItemBody::Ma {
        choices,
        answers,
        min_answers_required,
        allow_all_correct,
    } = output.get(0).expect("MA").body()
    else {
        panic!("MA")
    };
    assert!(choices[0].contains("_table_1.png"));
    assert_eq!(answers, &choices[..2]);
    assert_eq!((*min_answers_required, *allow_all_correct), (1, false));
    let ItemBody::Order { answers } = output.get(1).expect("ORDER").body() else {
        panic!("ORDER")
    };
    assert_eq!(answers[0], "first");
    assert!(answers[1].contains("_table_1.png"));
    assert_eq!(answers[2], "third");
}

#[test]
fn only_images_inside_rendered_tables_are_read_and_inlined() {
    use qti_core::media::{AssetSource, MediaError};
    use std::borrow::Cow;
    struct TableSource(AtomicUsize);
    impl AssetSource for TableSource {
        fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
            assert_eq!(
                src, "inside.png",
                "unrendered images must use the caller fallback"
            );
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Cow::Borrowed(b"inside bytes"))
        }
    }
    struct ImageRenderer;
    impl FragmentRenderer for ImageRenderer {
        type Error = TestRenderError;
        fn cache_discriminator(&self) -> String {
            "table-image-test".into()
        }
        fn render_canvas(&self, _: &CanvasSource) -> Result<RenderedPng, Self::Error> {
            panic!("no canvas")
        }
        fn render_table(&self, html: &str) -> Result<RenderedPng, Self::Error> {
            assert!(html.contains("data:image/png;base64,aW5zaWRlIGJ5dGVz"));
            assert!(!html.contains("inside.png"));
            Ok(RenderedPng {
                bytes: test_png(),
                width: 120.0,
                height: 80.0,
                metrics: Default::default(),
            })
        }
    }
    let source = TableSource(AtomicUsize::new(0));
    let mut bank = ItemBank::default();
    bank.add_item(mc(
        "<img src=\"outside.png\" /><table><tr><td><img src=\"inside.png\" /></td></tr></table>",
    ))
    .expect("MC");
    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &source,
        &ImageRenderer,
        &RenderCache::new(),
    )
    .expect("render only needed image");
    assert_eq!(source.0.load(Ordering::SeqCst), 1);
    assert_eq!(assets.entries().len(), 1);
    assert!(
        output
            .get(0)
            .expect("MC")
            .common()
            .question_text
            .contains("outside.png")
    );
}

#[test]
fn table_images_bind_browser_decoded_source_and_preserve_remote_and_inline_urls() {
    let mut assets = qti_core::media::MemoryAssets::new();
    assets
        .insert("a&b.png", b"viewer bytes".to_vec())
        .expect("viewer source");
    assets
        .insert("a&amp;b.png", b"wrong literal bytes".to_vec())
        .expect("literal source");
    let html = "<table><tr><td><img src=\"a&amp;b.png\" /><img src=\"https://example.org/a.png\" /><img src=\"data:image/png;base64,AQ==\" /></td></tr></table>";
    let prepared = super::inline_table_images(html, &assets).expect("browser source resolution");
    assert!(prepared.contains("data:image/png;base64,dmlld2VyIGJ5dGVz"));
    assert!(!prepared.contains("d3JvbmcgbGl0ZXJhbCBieXRlcw=="));
    assert!(prepared.contains("https://example.org/a.png"));
    assert!(prepared.contains("data:image/png;base64,AQ=="));
}

#[test]
fn conversion_retains_relative_media_for_the_original_provider() {
    let source = tempdir().expect("source directory");
    fs::write(source.path().join("existing.png"), [1, 2, 3]).expect("source image");
    let bank = source_bank(
        source.path(),
        "<img src=\"existing.png\" /><table><tr><td>table</td></tr></table>",
    );

    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("conversion succeeds");
    assert_eq!(
        fs::read(source.path().join("existing.png")).expect("source untouched"),
        [1, 2, 3]
    );
    assert!(!source.path().join("__qti_generated").exists());
    assert!(assets.get("existing.png").is_none());
    let directory = crate::DirectoryAssets::new(source.path()).expect("source root");
    let overlay = qti_engines::AssetOverlay {
        memory: &assets,
        fallback: &directory,
    };
    assert!(
        output
            .collect_assets(&overlay)
            .expect("all derived sources resolve")
            .assets()
            .iter()
            .any(|asset| asset.src.contains("_table_1.png"))
    );

    assert!(
        assets
            .entries()
            .keys()
            .any(|name| name.contains("_table_1.png"))
    );
}

#[test]
fn conversion_preserves_authorized_absolute_media_for_original_provider() {
    let source = tempdir().expect("source directory");
    let absolute = source.path().join("original.png");
    fs::write(&absolute, [9, 8, 7]).expect("absolute image");
    let bank = source_bank(
        source.path(),
        &format!(
            "<img src=\"{}\" /><table><tr><td>x</td></tr></table>",
            absolute.display()
        ),
    );

    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("conversion succeeds");
    let question = &output
        .get(0)
        .expect("converted item")
        .common()
        .question_text;
    assert!(question.contains(absolute.to_string_lossy().as_ref()));
    assert!(!question.contains("__qti_input/"));
    assert_eq!(
        fs::read(&absolute).expect("external source untouched"),
        [9, 8, 7]
    );
    let directory = crate::DirectoryAssets::new(source.path()).expect("source root");
    let overlay = qti_engines::AssetOverlay {
        memory: &assets,
        fallback: &directory,
    };
    output
        .collect_assets(&overlay)
        .expect("rewritten source resolves");
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

    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
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
        .collect_assets(&qti_engines::AssetOverlay {
            memory: &assets,
            fallback: &crate::DirectoryAssets::new(source.path()).expect("source root"),
        })
        .expect("both source and generated images resolve");
}

#[test]
fn no_selected_fragment_returns_shared_source_bank() {
    let source = tempdir().expect("source directory");
    let bank = source_bank(source.path(), "plain question");
    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("conversion succeeds");

    assert!(assets.entries().is_empty());
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
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
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
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
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
    let mut bank = ItemBank::new(false);
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
    let (output, _assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &renderer,
        &RenderCache::new(),
    )
    .expect("conversion succeeds");

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
    let mut bank = ItemBank::new(false);
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

    let (output, assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &TestRenderer::default(),
        &RenderCache::new(),
    )
    .expect("conversion succeeds");

    assert!(!assets.entries().is_empty());
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
    let mut bank = ItemBank::new(false);
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

    let (_converted, _assets, metrics) = convert_bank_with_metrics(
        &bank,
        &[ItemKind::Mc],
        &crate::DirectoryAssets::new(source.path()).expect("source root"),
        &renderer,
        &RenderCache::new(),
    )
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

fn test_png() -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=").expect("valid PNG")
}

#[test]
fn native_adapter_matches_portable_identity_grading_media_and_css_sizing() {
    let mut bank = ItemBank::new(false);
    for stem in [
        "<table><tr><td>x</td></tr></table>",
        "<table><tbody><tr><td>x</td></tr></tbody></table>",
    ] {
        bank.add_item(mc(stem)).expect("distinct source items");
    }
    let source = qti_core::media::MemoryAssets::new();
    let renderer = TestRenderer::default();
    let plan = qti_render::plan_bank(&bank, &[ItemKind::Mc], &source).expect("plan");
    let completions = plan
        .jobs
        .iter()
        .map(|job| {
            let rendered = renderer
                .render_table(job.html.as_deref().expect("table"))
                .expect("render");
            qti_render::RenderCompletion {
                id: job.id.clone(),
                png: rendered.bytes,
                width: rendered.width,
                height: rendered.height,
            }
        })
        .collect::<Vec<_>>();
    let (portable, portable_assets) =
        qti_render::finish_bank(&bank, &plan, &completions).expect("portable finish");
    let (native, native_assets) = convert_bank(
        &bank,
        &[ItemKind::Mc],
        &source,
        &renderer,
        &RenderCache::new(),
    )
    .expect("native finish");
    assert_eq!(native.len(), 2);
    assert_eq!(native_assets, portable_assets);
    for ((original, native), portable) in bank
        .iter_ordered()
        .zip(native.iter_ordered())
        .zip(portable.iter_ordered())
    {
        assert_eq!(original.crc(), native.crc());
        assert_eq!(native.common(), portable.common());
        assert_eq!(native.body(), portable.body());
        assert_eq!(native.body(), original.body());
    }
}
