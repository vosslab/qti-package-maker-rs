//! Portable Blackboard reader and writer roundtrip behavior.

use super::super::{BlackboardReader, BlackboardWriter};
use crate::{
    DocumentMetadata, ReadInput, ReadOutcome, Reader, WriteArtifact, WriteContext, Writer,
};
use qti_core::{EntryMap, FieldId, Item, ItemBank, ItemBody, MediaRef, MemoryAssets};
use sha2::{Digest, Sha256};

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248, 207, 192, 240, 31, 0, 5,
    0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

// The same pixel with a different valid IDAT encoding: byte identity governs packaging.
const PNG_ALTERNATE: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 16, 73, 68, 65, 84, 120, 1, 1, 5, 0, 250, 255, 0, 255, 0, 0,
    255, 5, 0, 1, 255, 250, 92, 136, 209, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn package(bank: &ItemBank, assets: &MemoryAssets) -> crate::WriteOutcome {
    let context = WriteContext::new(
        "pool.zip".to_owned(),
        DocumentMetadata {
            title: "pool".to_owned(),
            date: "2026-10-08".to_owned(),
        },
        0,
    )
    .expect("valid context");
    BlackboardWriter
        .write_package(bank, assets, &context)
        .expect("write pool")
}

fn package_entries(bank: &ItemBank, assets: &MemoryAssets) -> EntryMap {
    let outcome = package(bank, assets);
    let Some(WriteArtifact::File {
        primary,
        companions,
    }) = outcome.artifact
    else {
        panic!("ZIP file artifact")
    };
    assert!(companions.is_empty());
    let entries = qti_integrity::read_zip_entries(primary.bytes()).expect("safe output ZIP");
    let violations = qti_integrity::check_entries(&entries);
    assert!(
        violations
            .iter()
            .all(|violation| { violation.severity != qti_integrity::Severity::Error })
    );
    entries
}

fn read(entries: &EntryMap) -> ReadOutcome {
    BlackboardReader
        .read_items(
            ReadInput::Archive {
                name: "pool.zip",
                entries,
            },
            true,
        )
        .expect("read pool")
}

fn empty_pool_manifest() -> Vec<u8> {
    br#"<manifest xmlns:bb="http://www.blackboard.com/content-packaging/"><resources><resource type="assessment/x-bb-qti-pool" bb:file="res00002.dat" /></resources></manifest>"#.to_vec()
}

fn pool_entries(pool: &[u8]) -> EntryMap {
    [
        ("imsmanifest.xml".to_owned(), empty_pool_manifest()),
        ("res00002.dat".to_owned(), pool.to_vec()),
    ]
    .into_iter()
    .collect()
}

fn image_bank(stem: &str) -> ItemBank {
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            stem.to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid image item"),
    )
    .expect("add image item");
    bank
}

fn stem_media_fingerprint(bytes: &[u8]) -> Vec<MediaRef> {
    let content_hash: [u8; 32] = Sha256::digest(bytes).into();
    vec![MediaRef {
        content_hash,
        field: FieldId::Stem,
        ordinal: 0,
    }]
}

fn mixed_bank() -> ItemBank {
    let mut bank = ItemBank::new(true);
    let items = [
        Item::new(
            "MC question".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        ),
        Item::new(
            "MA question".to_owned(),
            ItemBody::Ma {
                choices: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                answers: vec!["one".to_owned(), "two".to_owned()],
                min_answers_required: 0,
                allow_all_correct: false,
            },
        ),
        Item::new(
            "MATCH question".to_owned(),
            ItemBody::Match {
                prompts: vec!["one".to_owned(), "two".to_owned()],
                choices: vec!["first".to_owned(), "second".to_owned()],
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
            "FIB question".to_owned(),
            ItemBody::Fib {
                answers: vec!["answer".to_owned(), "other".to_owned()],
            },
        ),
        Item::new(
            "MULTI question [left] [right]".to_owned(),
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
        bank.add_item(item.expect("valid item"))
            .expect("mixed bank");
    }
    bank
}

fn without_private_grading_metadata(pool_xml: String) -> String {
    let mut pool_xml = pool_xml;
    for tag in [
        "bbmd_qti_package_maker_ma_min_answers_required",
        "bbmd_qti_package_maker_ma_allow_all_correct",
        "bbmd_qti_package_maker_num_tolerance",
        "bbmd_qti_package_maker_num_tolerance_message",
    ] {
        let opening = format!("<{tag}>");
        let closing = format!("</{tag}>");
        assert_eq!(
            pool_xml.matches(&opening).count(),
            1,
            "writer emits one {tag} field"
        );
        let start = pool_xml.find(&opening).expect("metadata start");
        let end = pool_xml[start..]
            .find(&closing)
            .map(|offset| start + offset + closing.len())
            .expect("metadata end");
        pool_xml.replace_range(start..end, "");
    }
    pool_xml
}

#[test]
fn native_package_round_trips_supported_kinds_and_all_grading_options() {
    let original = mixed_bank();
    let recovered = read(&package_entries(&original, &MemoryAssets::new()));
    assert!(recovered.warnings.is_empty(), "{:?}", recovered.warnings);
    assert_eq!(recovered.bank.len(), 6);
    let fingerprints = |bank: &ItemBank| {
        bank.iter_ordered()
            .map(|item| qti_core::ItemFingerprint::new(item, Vec::new()).expect("fingerprint"))
            .collect::<Vec<_>>()
    };
    assert_eq!(fingerprints(&recovered.bank), fingerprints(&original));
}

#[test]
fn absent_private_grading_metadata_uses_frozen_blackboard_defaults() {
    let original = mixed_bank();
    let mut entries = package_entries(&original, &MemoryAssets::new());
    let present = read(&entries);
    let grading = |bank: &ItemBank| {
        let ma = bank
            .iter_ordered()
            .find_map(|item| match item.body() {
                ItemBody::Ma {
                    min_answers_required,
                    allow_all_correct,
                    ..
                } => Some((*min_answers_required, *allow_all_correct)),
                _ => None,
            })
            .expect("MA item");
        let num = bank
            .iter_ordered()
            .find_map(|item| match item.body() {
                ItemBody::Num {
                    tolerance,
                    tolerance_message,
                    ..
                } => Some((*tolerance, *tolerance_message)),
                _ => None,
            })
            .expect("NUM item");
        (ma, num)
    };
    assert_eq!(grading(&present.bank), ((0, false), (0.01, true)));
    let xml =
        String::from_utf8(entries.remove("res00002.dat").expect("pool XML")).expect("UTF-8 XML");
    entries.insert(
        "res00002.dat".to_owned(),
        without_private_grading_metadata(xml).into_bytes(),
    );
    let absent = read(&entries);
    assert_eq!(
        grading(&absent.bank),
        ((1, true), ((4.01_f64 - 3.99) / 2.0, true))
    );
}

#[test]
fn committed_blackboard_fixture_recovers_media_and_skips_unsupported_hotspot() {
    let bytes = include_bytes!("../../../../../tests/fixtures/bb_export_slice.zip");
    let outcome = BlackboardReader
        .read_items(
            ReadInput::File {
                name: "fixture.zip",
                bytes,
            },
            true,
        )
        .expect("read real Blackboard fixture");
    assert_eq!(outcome.bank.len(), 1);
    assert_eq!(outcome.warnings.len(), 1);
    assert!(outcome.assets.entries().contains_key("image-1.jpg"));
    assert!(
        outcome
            .bank
            .iter_ordered()
            .next()
            .expect("MC item")
            .common()
            .question_text
            .contains("image-1.jpg")
    );
    let entries = qti_integrity::read_zip_entries(bytes).expect("fixture entries");
    let extracted = read(&entries);
    assert_eq!(extracted.assets.entries(), outcome.assets.entries());
    assert_eq!(extracted.warnings, outcome.warnings);
    assert_eq!(
        extracted
            .bank
            .iter_ordered()
            .map(|item| item.crc())
            .collect::<Vec<_>>(),
        outcome
            .bank
            .iter_ordered()
            .map(|item| item.crc())
            .collect::<Vec<_>>()
    );
}

#[test]
fn packages_and_recovers_local_images_as_owned_memory() {
    let bank = image_bank("Image <img src=\"diagram.png\" alt=\"diagram\" />");
    let mut assets = MemoryAssets::new();
    assets
        .insert("diagram.png".to_owned(), PNG.to_vec())
        .expect("source image");
    let recovered = read(&package_entries(&bank, &assets));
    assert_eq!(recovered.assets.entries()["diagram.png"], PNG);
    let item = recovered
        .bank
        .iter_ordered()
        .next()
        .expect("recovered item");
    assert!(item.common().question_text.contains("diagram.png"));
    assert_eq!(
        qti_core::ItemFingerprint::new(item, stem_media_fingerprint(PNG)).expect("fingerprint"),
        qti_core::ItemFingerprint::new(
            bank.iter_ordered().next().expect("original item"),
            stem_media_fingerprint(PNG)
        )
        .expect("fingerprint")
    );
}

#[test]
fn csfiles_lom_sidecars_match_frozen_namespace_and_collision_safe_names() {
    let bank = image_bank(
        "Repeated <img src=\"images/a/shared.png\" alt=\"first\" /> <img src=\"images/b/shared.png\" alt=\"second\" />",
    );
    let mut assets = MemoryAssets::new();
    for (src, bytes) in [
        ("images/a/shared.png", PNG),
        ("images/b/shared.png", PNG_ALTERNATE),
    ] {
        assets
            .insert(src.to_owned(), bytes.to_vec())
            .expect("source image");
    }
    let entries = package_entries(&bank, &assets);
    let first = String::from_utf8_lossy(&entries["csfiles/home_dir/__xid-1_1.png.xml"]);
    let second = String::from_utf8_lossy(&entries["csfiles/home_dir/__xid-2_1.png.xml"]);
    assert!(first.contains("xmlns=\"http://www.imsglobal.org/xsd/imsmd_rootv1p2p1\""));
    assert!(first.contains(
        "xsi:schemaLocation=\"http://www.imsglobal.org/xsd/imsmd_rootv1p2p1 imsmd_rootv1p2p1.xsd\""
    ));
    assert!(first.contains("1_1#/courses/qti_package_maker/shared.png"));
    assert!(second.contains("2_1#/courses/qti_package_maker/shared(1).png"));
    let recovered = read(&entries);
    let question = &recovered
        .bank
        .iter_ordered()
        .next()
        .expect("item")
        .common()
        .question_text;
    assert!(question.contains("shared.png"));
    assert!(question.contains("shared(1).png"));
    assert_eq!(recovered.assets.entries()["shared.png"], PNG);
    assert_eq!(recovered.assets.entries()["shared(1).png"], PNG_ALTERNATE);
    // Reader collision handling must also work for external packages whose LOM
    // names collide, independent of the writer's own naming convention.
    let mut collisions = entries;
    let sidecar = collisions
        .get_mut("csfiles/home_dir/__xid-2_1.png.xml")
        .expect("second sidecar");
    *sidecar = String::from_utf8(sidecar.clone())
        .expect("sidecar XML")
        .replace("shared(1).png", "shared.png")
        .into_bytes();
    let recovered = read(&collisions);
    assert_eq!(recovered.assets.entries()["shared.png"], PNG);
    assert_eq!(recovered.assets.entries()["shared_2.png"], PNG_ALTERNATE);
    assert!(
        recovered
            .bank
            .iter_ordered()
            .next()
            .expect("item")
            .common()
            .question_text
            .contains("shared_2.png")
    );
}

#[test]
fn identical_image_bytes_share_one_resource_owned_by_the_first_referring_item() {
    let mut bank = image_bank("First <img src=\"z_first.png\" alt=\"first\" />");
    let first_parent = format!("_{}_1", bank.iter_ordered().next().expect("first").crc());
    let second = Item::new(
        "Second <img src=\"a_second.png\" alt=\"second\" /> <img src=\"m_distinct.png\" />"
            .to_owned(),
        ItemBody::Mc {
            choices: vec!["one".to_owned(), "two".to_owned()],
            answer: "two".to_owned(),
        },
    )
    .expect("second item");
    let second_parent = format!("_{}_1", second.crc());
    bank.add_item(second).expect("add second");
    let mut assets = MemoryAssets::new();
    assets
        .insert("z_first.png".to_owned(), PNG.to_vec())
        .expect("first image");
    assets
        .insert("a_second.png".to_owned(), PNG.to_vec())
        .expect("second image");
    assets
        .insert("m_distinct.png".to_owned(), PNG_ALTERNATE.to_vec())
        .expect("distinct image");
    let entries = package_entries(&bank, &assets);
    let binaries = entries
        .iter()
        .filter(|(name, _)| name.starts_with("csfiles/") && name.ends_with(".png"))
        .collect::<Vec<_>>();
    assert_eq!(binaries.len(), 2);
    assert_eq!(
        binaries
            .iter()
            .filter(|(_, bytes)| bytes.as_slice() == PNG)
            .count(),
        1
    );
    assert_eq!(
        binaries
            .iter()
            .filter(|(_, bytes)| bytes.as_slice() == PNG_ALTERNATE)
            .count(),
        1
    );
    for (name, _) in &binaries {
        assert!(entries.contains_key(&format!("{name}.xml")));
    }
    let pool = String::from_utf8_lossy(&entries["res00002.dat"]);
    assert_eq!(pool.matches("<item ").count(), 2);
    assert_eq!(pool.matches("bbcswebdav/xid-1_1").count(), 2);
    assert_eq!(pool.matches("bbcswebdav/xid-2_1").count(), 1);
    let links = String::from_utf8_lossy(&entries["res00005.dat"]);
    assert!(links.contains(&format!(
        "<parentId>{first_parent}</parentId><resourceId>1_1</resourceId>"
    )));
    assert!(links.contains(&format!(
        "<parentId>{second_parent}</parentId><resourceId>2_1</resourceId>"
    )));
    assert_eq!(links.matches("<cms_resource_link>").count(), 2);
    let recovered = read(&entries);
    assert_eq!(recovered.bank.iter_ordered().count(), 2);
    for (original, restored) in bank.iter_ordered().zip(recovered.bank.iter_ordered()) {
        assert_eq!(original.body(), restored.body());
    }
    let restored = recovered.bank.iter_ordered().collect::<Vec<_>>();
    assert!(restored[0].common().question_text.contains("a_second.png"));
    assert!(restored[1].common().question_text.contains("a_second.png"));
    assert!(
        restored[1]
            .common()
            .question_text
            .contains("m_distinct.png")
    );
    assert_eq!(recovered.assets.entries()["a_second.png"], PNG);
    assert_eq!(recovered.assets.entries()["m_distinct.png"], PNG_ALTERNATE);
}

fn order_bank(stem: &str) -> ItemBank {
    let mut bank = ItemBank::new(false);
    bank.add_item(
        Item::new(
            stem.to_owned(),
            ItemBody::Order {
                answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            },
        )
        .expect("valid ORDER"),
    )
    .expect("add ORDER");
    bank
}

#[test]
fn order_only_bank_writes_an_empty_pool_with_an_order_diagnostic() {
    let outcome = package(&order_bank("ORDER item"), &MemoryAssets::new());
    assert!(outcome.artifact.is_some());
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.warnings[0].src, "ORDER");
    assert!(outcome.warnings[0].reason.contains("unsupported ORDER"));
}

#[test]
fn order_media_is_not_emitted_without_a_rendered_pool_object() {
    let bank = order_bank("Order <img src=\"glycolysis.png\" alt=\"glycolysis\" />");
    let mut assets = MemoryAssets::new();
    assets
        .insert("glycolysis.png".to_owned(), PNG.to_vec())
        .expect("source image");
    let outcome = package(&bank, &assets);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.warnings[0].src, "ORDER");
    let entries = package_entries(&bank, &assets);
    assert!(!String::from_utf8_lossy(&entries["res00005.dat"]).contains("<cms_resource_link>"));
    assert!(entries.keys().all(|name| !name.starts_with("csfiles/")));
}

#[test]
fn rejects_zip_slip_before_any_reader_processing() {
    use std::io::Write;
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    archive
        .start_file("../outside.xml", zip::write::SimpleFileOptions::default())
        .expect("entry");
    archive.write_all(b"unsafe entry").expect("entry bytes");
    let bytes = archive.finish().expect("ZIP").into_inner();
    assert!(
        BlackboardReader
            .read_items(
                ReadInput::File {
                    name: "unsafe.zip",
                    bytes: &bytes
                },
                true
            )
            .is_err()
    );
    let entries = [("../outside.xml".to_owned(), b"unsafe".to_vec())]
        .into_iter()
        .collect();
    assert!(
        BlackboardReader
            .read_items(
                ReadInput::Archive {
                    name: "unsafe",
                    entries: &entries
                },
                true
            )
            .is_err()
    );
}

#[test]
fn rejects_a_pool_file_attribute_in_an_untrusted_namespace() {
    let entries = [("imsmanifest.xml".to_owned(), br#"<manifest xmlns:fake="https://example.invalid/blackboard"><resources><resource type="assessment/x-bb-qti-pool" fake:file="res00002.dat" /></resources></manifest>"#.to_vec())].into_iter().collect();
    let error = BlackboardReader
        .read_items(
            ReadInput::Archive {
                name: "pool",
                entries: &entries,
            },
            true,
        )
        .expect_err("foreign file attribute");
    assert!(error.to_string().contains("lacks bb:file"));
}

#[test]
fn rejects_a_cs_resource_links_file_in_an_untrusted_namespace() {
    let mut entries = pool_entries(b"<questestinterop />");
    entries.insert("imsmanifest.xml".to_owned(), br#"<manifest xmlns:bb="http://www.blackboard.com/content-packaging/" xmlns:fake="https://example.invalid/blackboard"><resources><resource type="assessment/x-bb-qti-pool" bb:file="res00002.dat" /><resource type="course/x-bb-csresourcelinks" fake:file="res00005.dat" /></resources></manifest>"#.to_vec());
    let error = BlackboardReader
        .read_items(
            ReadInput::Archive {
                name: "pool",
                entries: &entries,
            },
            true,
        )
        .expect_err("foreign CSResourceLinks file");
    assert!(
        error
            .to_string()
            .contains("CSResourceLinks resource lacks bb:file")
    );
}

#[test]
fn accepts_a_non_bb_prefix_when_it_resolves_to_the_blackboard_namespace() {
    let mut entries = pool_entries(b"<questestinterop />");
    entries.insert("imsmanifest.xml".to_owned(), br#"<manifest xmlns:content="http://www.blackboard.com/content-packaging/"><resources><resource type="assessment/x-bb-qti-pool" content:file="res00002.dat" /></resources></manifest>"#.to_vec());
    assert_eq!(read(&entries).bank.len(), 0);
}

#[test]
fn absent_xml_base_uses_the_pool_dat_stem_for_hotspot_media() {
    let mut entries = pool_entries(b"<questestinterop><item><presentation><matapplication uri=\"picture.png\" label=\"recovered.png\" /></presentation></item></questestinterop>");
    entries.insert("res00002/picture.png".to_owned(), b"hotspot bytes".to_vec());
    let outcome = read(&entries);
    assert_eq!(outcome.bank.len(), 0);
    assert_eq!(outcome.assets.entries()["recovered.png"], b"hotspot bytes");
}

#[test]
fn missing_pool_is_a_typed_error_and_nested_root_remains_supported() {
    let entries = [("imsmanifest.xml".to_owned(), empty_pool_manifest())]
        .into_iter()
        .collect();
    let error = BlackboardReader
        .read_items(
            ReadInput::Archive {
                name: "missing",
                entries: &entries,
            },
            true,
        )
        .expect_err("missing pool");
    assert!(
        error
            .to_string()
            .contains("manifest pool file 'res00002.dat' is missing")
    );
    let entries = pool_entries(b"<questestinterop />")
        .into_iter()
        .map(|(name, bytes)| (format!("nested/{name}"), bytes))
        .collect();
    assert_eq!(read(&entries).bank.len(), 0);
}

#[test]
fn mixed_kind_rejections_preserve_source_warning_order() {
    let entries = package_entries(&mixed_bank(), &MemoryAssets::new());
    let recovered = BlackboardReader
        .read_items(
            ReadInput::Archive {
                name: "mixed",
                entries: &entries,
            },
            false,
        )
        .expect("mixed-kind items become reader warnings");
    assert_eq!(recovered.bank.len(), 1);
    assert_eq!(recovered.warnings.len(), 5);
    for (index, warning) in recovered.warnings.iter().enumerate() {
        assert_eq!(
            warning.location,
            crate::ReadLocation::PoolItem {
                resource: "res00002.dat".to_owned(),
                number: index + 2,
            }
        );
        assert!(warning.message.contains("mixing item types is not allowed"));
    }
}
