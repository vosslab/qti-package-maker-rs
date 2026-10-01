//! Blackboard Original pool-export ZIP reader and writer.
//!
//! The format keeps question XML in a pool `.dat` resource and carries item
//! images through Blackboard's `csfiles` side channel.  This module keeps the
//! public engine boundary small; the two directions live in separate files so
//! their deliberately asymmetric media rules stay reviewable.

mod read;
mod write;

use std::path::Path;

use qti_core::ItemKind;
use qti_core::media::MediaPolicy;

use crate::{EngineError, EngineOptions, ReadOutcome, Reader, WriteOutcome, Writer};

pub(crate) const NAME: &str = "blackboard_export_zip";
const KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
];

/// Creates the Blackboard pool-export writer.
pub fn boxed_writer(options: EngineOptions) -> Box<dyn Writer> {
    Box::new(BlackboardWriter {
        html_to_image: options.html_to_image,
    })
}

/// Creates the Blackboard pool-export reader.
pub fn boxed_reader(_: EngineOptions) -> Box<dyn Reader> {
    Box::new(BlackboardReader)
}

struct BlackboardWriter {
    // M19 owns the common conversion pass.  Retaining this option here makes
    // the writer factory contract explicit while it is wired through the CLI.
    html_to_image: bool,
}

impl Writer for BlackboardWriter {
    fn name(&self) -> &'static str {
        NAME
    }
    fn media_policy(&self) -> MediaPolicy {
        MediaPolicy::Package
    }
    fn supported_kinds(&self) -> &'static [ItemKind] {
        KINDS
    }
    fn save_package(
        &self,
        bank: &qti_core::ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError> {
        if self.html_to_image {
            // Conversion must happen once above all writers.  This value is
            // intentionally consumed here so an engine-only caller cannot
            // silently claim conversion occurred.
            return Err(EngineError::InvalidFormat {
                engine: NAME,
                format: "html-to-image",
                message: "HTML conversion must be performed by the shared conversion pass"
                    .to_owned(),
            });
        }
        write::save_package(bank, output)
    }
}

struct BlackboardReader;

impl Reader for BlackboardReader {
    fn name(&self) -> &'static str {
        NAME
    }
    fn read_items(&self, input: &Path, allow_mixed: bool) -> Result<ReadOutcome, EngineError> {
        read::read_package(input, allow_mixed)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::path::Path;

    use super::{BlackboardReader, BlackboardWriter};
    use crate::{Reader, Writer};
    use qti_core::{ArchiveEntry, ArchiveMap, FieldId, Item, ItemBank, ItemBody, MediaRef};
    use sha2::{Digest, Sha256};

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

    fn write_pool_without_private_grading_metadata(source: &Path, destination: &Path) {
        let source = std::fs::File::open(source).expect("source package");
        let mut source = zip::ZipArchive::new(source).expect("source ZIP");
        let mut entries = ArchiveMap::new();
        for index in 0..source.len() {
            let mut entry = source.by_index(index).expect("source ZIP entry");
            let name = entry.name().to_owned();
            if name.ends_with('/') {
                continue;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("source ZIP bytes");
            if name == "res00002.dat" {
                let pool_xml = String::from_utf8(bytes).expect("writer pool XML");
                bytes = without_private_grading_metadata(pool_xml).into_bytes();
            }
            entries.insert(name, ArchiveEntry::Bytes(bytes));
        }
        qti_core::build_zip(destination, &entries, ["res00001/"]).expect("metadata-free package");
    }

    #[test]
    fn native_package_round_trips_supported_kinds_and_all_grading_options() {
        let original = mixed_bank();
        let directory = tempfile::tempdir().expect("temporary directory");
        let output = directory.path().join("pool.zip");
        BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&original, Some(&output))
        .expect("write Blackboard pool");
        let violations = qti_integrity::check_package(&output);
        assert!(violations.is_empty(), "{violations:#?}");
        let recovered = BlackboardReader
            .read_items(&output, true)
            .expect("read Blackboard pool");
        assert!(recovered.warnings.is_empty(), "{:?}", recovered.warnings);
        assert_eq!(recovered.bank.len(), 6);
        assert_eq!(
            recovered
                .bank
                .iter_ordered()
                .map(|item| item.kind())
                .collect::<Vec<_>>(),
            original
                .iter_ordered()
                .map(|item| item.kind())
                .collect::<Vec<_>>()
        );
        let original_fingerprints = original
            .iter_ordered()
            .map(|item| qti_core::ItemFingerprint::new(item, Vec::new()).expect("fingerprint"))
            .collect::<Vec<_>>();
        let recovered_fingerprints = recovered
            .bank
            .iter_ordered()
            .map(|item| qti_core::ItemFingerprint::new(item, Vec::new()).expect("fingerprint"))
            .collect::<Vec<_>>();
        assert_eq!(recovered_fingerprints, original_fingerprints);
    }

    #[test]
    fn absent_private_grading_metadata_uses_frozen_blackboard_defaults() {
        let original = mixed_bank();
        let directory = tempfile::tempdir().expect("temporary directory");
        let authored = directory.path().join("authored-options.zip");
        BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&original, Some(&authored))
        .expect("write Blackboard pool");

        let present = BlackboardReader
            .read_items(&authored, true)
            .expect("read metadata-bearing pool");
        let present_ma = present
            .bank
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
        let present_num = present
            .bank
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
        assert_eq!(
            present_ma,
            (0, false),
            "private MA fields preserve authorship"
        );
        assert_eq!(
            present_num,
            (0.01, true),
            "private NUM fields preserve authorship"
        );

        let without_metadata = directory.path().join("frozen-default-options.zip");
        write_pool_without_private_grading_metadata(&authored, &without_metadata);
        let absent = BlackboardReader
            .read_items(&without_metadata, true)
            .expect("read metadata-free pool");
        let absent_ma = absent
            .bank
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
        let absent_num = absent
            .bank
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
        assert_eq!(absent_ma, (1, true), "frozen MA reader defaults");
        assert_eq!(
            absent_num,
            ((4.01_f64 - 3.99) / 2.0, true),
            "frozen NUM reader derives tolerance from exported bounds"
        );
    }

    #[test]
    fn committed_blackboard_fixture_recovers_media_and_skips_unsupported_hotspot() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/bb_export_slice.zip");
        let outcome = BlackboardReader
            .read_items(&fixture, true)
            .expect("read real Blackboard fixture");
        assert_eq!(outcome.bank.len(), 1);
        assert_eq!(outcome.warnings.len(), 1);
        assert!(
            outcome
                .bank
                .media_base_dir()
                .expect("recovered media base")
                .join("image-1.jpg")
                .is_file()
        );
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
    }

    #[test]
    fn packages_and_recovers_local_images_with_a_bank_owned_lifetime() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let image = directory.path().join("diagram.png");
        std::fs::write(
            &image,
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248,
                207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68,
                174, 66, 96, 130,
            ],
        )
        .expect("image bytes");
        let mut bank =
            ItemBank::with_media_base_dir(true, qti_core::MediaBaseDir::external(directory.path()));
        bank.add_item(
            Item::new(
                "Image <img src=\"diagram.png\" alt=\"diagram\" />".to_owned(),
                ItemBody::Mc {
                    choices: vec!["first".to_owned(), "second".to_owned()],
                    answer: "first".to_owned(),
                },
            )
            .expect("valid image item"),
        )
        .expect("add item");
        assert!(
            bank.iter_ordered()
                .next()
                .expect("source item")
                .common()
                .question_text
                .contains("<img")
        );
        let output = directory.path().join("image-pool.zip");
        BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("write image pool");
        let violations = qti_integrity::check_package(&output);
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "{violations:#?}"
        );
        let recovered = BlackboardReader
            .read_items(&output, true)
            .expect("read image pool");
        let base = recovered
            .bank
            .media_base_dir()
            .expect("owned extracted media");
        assert_eq!(
            std::fs::read(base.join("diagram.png")).expect("recovered image"),
            std::fs::read(&image).expect("original image")
        );
        assert!(
            recovered
                .bank
                .iter_ordered()
                .next()
                .expect("item")
                .common()
                .question_text
                .contains("diagram.png")
        );
        assert_eq!(
            recovered
                .bank
                .iter_ordered()
                .map(|item| qti_core::ItemFingerprint::new(
                    item,
                    stem_media_fingerprint(
                        &std::fs::read(base.join("diagram.png")).expect("recovered image")
                    ),
                )
                .expect("recovered fingerprint"))
                .collect::<Vec<_>>(),
            bank.iter_ordered()
                .map(|item| qti_core::ItemFingerprint::new(
                    item,
                    stem_media_fingerprint(&std::fs::read(&image).expect("original image")),
                )
                .expect("original fingerprint"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn csfiles_lom_sidecars_match_frozen_namespace_and_collision_safe_names() {
        use std::io::Read;

        let png = [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248, 207,
            192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
            96, 130,
        ];
        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir_all(directory.path().join("images/a")).expect("first image directory");
        std::fs::create_dir_all(directory.path().join("images/b")).expect("second image directory");
        std::fs::write(directory.path().join("images/a/shared.png"), png).expect("first image");
        std::fs::write(directory.path().join("images/b/shared.png"), png).expect("second image");
        let mut bank =
            ItemBank::with_media_base_dir(true, qti_core::MediaBaseDir::external(directory.path()));
        bank.add_item(
            Item::new(
                "Repeated images <img src=\"images/a/shared.png\" alt=\"first\" /> <img src=\"images/b/shared.png\" alt=\"second\" />".to_owned(),
                ItemBody::Mc {
                    choices: vec!["first".to_owned(), "second".to_owned()],
                    answer: "first".to_owned(),
                },
            )
            .expect("valid image item"),
        )
        .expect("add image item");
        let output = directory.path().join("repeated-images.zip");
        BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("write Blackboard pool");
        let violations = qti_integrity::check_package(&output);
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "{violations:#?}"
        );

        let file = std::fs::File::open(&output).expect("package output");
        let mut archive = zip::ZipArchive::new(file).expect("ZIP package");
        let mut first = String::new();
        archive
            .by_name("csfiles/home_dir/__xid-1_1.png.xml")
            .expect("first LOM sidecar")
            .read_to_string(&mut first)
            .expect("first LOM XML");
        let mut second = String::new();
        archive
            .by_name("csfiles/home_dir/__xid-2_1.png.xml")
            .expect("second LOM sidecar")
            .read_to_string(&mut second)
            .expect("second LOM XML");
        let namespace = "http://www.imsglobal.org/xsd/imsmd_rootv1p2p1";
        assert!(first.contains(&format!("xmlns=\"{namespace}\"")));
        assert!(first.contains("xsi:schemaLocation=\"http://www.imsglobal.org/xsd/imsmd_rootv1p2p1 imsmd_rootv1p2p1.xsd\""));
        assert!(first.contains("1_1#/courses/qti_package_maker/shared.png"));
        assert!(second.contains("2_1#/courses/qti_package_maker/shared(1).png"));

        let recovered = BlackboardReader
            .read_items(&output, true)
            .expect("read repeated-image pool");
        let question = &recovered
            .bank
            .iter_ordered()
            .next()
            .expect("recovered item")
            .common()
            .question_text;
        assert!(question.contains("shared.png"));
        assert!(question.contains("shared(1).png"));
    }

    #[test]
    fn cs_resource_links_assign_each_image_to_its_first_referencing_item() {
        use std::io::Read;

        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::write(directory.path().join("first.png"), b"first image").expect("first image");
        std::fs::write(directory.path().join("second.png"), b"second image").expect("second image");
        let mut bank =
            ItemBank::with_media_base_dir(true, qti_core::MediaBaseDir::external(directory.path()));
        let first = Item::new(
            "First <img src=\"first.png\" alt=\"first\" />".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid first item");
        let second = Item::new(
            "Second <img src=\"second.png\" alt=\"second\" />".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "two".to_owned(),
            },
        )
        .expect("valid second item");
        let first_parent = format!("_{}_1", first.crc());
        let second_parent = format!("_{}_1", second.crc());
        bank.add_item(first).expect("add first item");
        bank.add_item(second).expect("add second item");
        let output = directory.path().join("two-images.zip");
        BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("write Blackboard pool");

        let file = std::fs::File::open(output).expect("package output");
        let mut archive = zip::ZipArchive::new(file).expect("ZIP package");
        let mut links = String::new();
        archive
            .by_name("res00005.dat")
            .expect("CSResourceLinks file")
            .read_to_string(&mut links)
            .expect("CSResourceLinks text");
        assert_eq!(
            links
                .matches(&format!("<parentId>{first_parent}</parentId>"))
                .count(),
            1
        );
        assert_eq!(
            links
                .matches(&format!("<parentId>{second_parent}</parentId>"))
                .count(),
            1
        );
    }

    #[test]
    fn order_only_bank_writes_an_empty_pool_with_an_order_diagnostic() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let output = directory.path().join("order-only.zip");
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "ORDER item".to_owned(),
                ItemBody::Order {
                    answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                },
            )
            .expect("valid ORDER"),
        )
        .expect("add ORDER");
        let outcome = BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("ORDER-only Blackboard package");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert!(output.is_file());
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].src, "ORDER");
        assert!(outcome.warnings[0].reason.contains("unsupported ORDER"));
    }

    #[test]
    fn order_media_is_not_emitted_without_a_rendered_pool_object() {
        use std::io::Read;

        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::write(directory.path().join("glycolysis.png"), b"glycolysis image")
            .expect("source image");
        let output = directory.path().join("order-only-media.zip");
        let mut bank = ItemBank::with_media_base_dir(
            false,
            qti_core::MediaBaseDir::external(directory.path()),
        );
        bank.add_item(
            Item::new(
                "Put these molecules in order <img src=\"glycolysis.png\" alt=\"glycolysis\" />"
                    .to_owned(),
                ItemBody::Order {
                    answers: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                },
            )
            .expect("valid ORDER"),
        )
        .expect("add ORDER");

        let outcome = BlackboardWriter {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("ORDER-only Blackboard package");
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].src, "ORDER");
        let violations = qti_integrity::check_package(&output);
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "{violations:#?}"
        );

        let file = std::fs::File::open(output).expect("package output");
        let mut archive = zip::ZipArchive::new(file).expect("ZIP package");
        let mut links = String::new();
        archive
            .by_name("res00005.dat")
            .expect("CSResourceLinks file")
            .read_to_string(&mut links)
            .expect("CSResourceLinks text");
        assert!(!links.contains("<cms_resource_link>"));
        assert!(
            (0..archive.len()).all(|index| {
                !archive
                    .by_index(index)
                    .expect("archive entry")
                    .name()
                    .starts_with("csfiles/")
            }),
            "skipped ORDER media must not become an export artifact"
        );
    }

    #[test]
    fn rejects_zip_slip_before_any_reader_processing() {
        use std::io::Write;

        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("unsafe.zip");
        let output = std::fs::File::create(&archive_path).expect("archive output");
        let mut archive = zip::ZipWriter::new(output);
        archive
            .start_file("../outside.xml", zip::write::SimpleFileOptions::default())
            .expect("unsafe entry");
        archive
            .write_all(b"not written outside the archive")
            .expect("entry bytes");
        archive.finish().expect("finish archive");
        let error = BlackboardReader
            .read_items(&archive_path, true)
            .expect_err("ZIP slip must be fatal");
        assert!(error.to_string().contains("unsafe archive entry"));
        assert!(!directory.path().join("outside.xml").exists());
    }

    #[test]
    fn rejects_a_pool_file_attribute_in_an_untrusted_namespace() {
        use std::io::Write;

        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("wrong-namespace.zip");
        let output = std::fs::File::create(&archive_path).expect("archive output");
        let mut archive = zip::ZipWriter::new(output);
        archive
            .start_file("imsmanifest.xml", zip::write::SimpleFileOptions::default())
            .expect("manifest entry");
        archive
            .write_all(
                br#"<manifest xmlns:fake="https://example.invalid/blackboard"><resources><resource type="assessment/x-bb-qti-pool" fake:file="res00002.dat" /></resources></manifest>"#,
            )
            .expect("manifest bytes");
        archive.finish().expect("finish archive");

        let error = BlackboardReader
            .read_items(&archive_path, true)
            .expect_err("foreign file attribute is not Blackboard bb:file");
        assert!(error.to_string().contains("lacks bb:file"));
    }

    #[test]
    fn rejects_a_cs_resource_links_file_in_an_untrusted_namespace() {
        use std::io::Write;

        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("wrong-link-namespace.zip");
        let output = std::fs::File::create(&archive_path).expect("archive output");
        let mut archive = zip::ZipWriter::new(output);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("imsmanifest.xml", options)
            .expect("manifest entry");
        archive
            .write_all(
                br#"<manifest xmlns:bb="http://www.blackboard.com/content-packaging/" xmlns:fake="https://example.invalid/blackboard"><resources><resource type="assessment/x-bb-qti-pool" bb:file="res00002.dat" /><resource type="course/x-bb-csresourcelinks" fake:file="res00005.dat" /></resources></manifest>"#,
            )
            .expect("manifest bytes");
        archive
            .start_file("res00002.dat", options)
            .expect("pool entry");
        archive
            .write_all(b"<questestinterop />")
            .expect("pool bytes");
        archive.finish().expect("finish archive");

        let error = BlackboardReader
            .read_items(&archive_path, true)
            .expect_err("foreign CSResourceLinks file is not Blackboard bb:file");
        assert!(
            error
                .to_string()
                .contains("CSResourceLinks resource lacks bb:file")
        );
    }

    #[test]
    fn accepts_a_non_bb_prefix_when_it_resolves_to_the_blackboard_namespace() {
        use std::io::Write;

        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("alternate-prefix.zip");
        let output = std::fs::File::create(&archive_path).expect("archive output");
        let mut archive = zip::ZipWriter::new(output);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("imsmanifest.xml", options)
            .expect("manifest entry");
        archive
            .write_all(
                br#"<manifest xmlns:content="http://www.blackboard.com/content-packaging/"><resources><resource type="assessment/x-bb-qti-pool" content:file="res00002.dat" /></resources></manifest>"#,
            )
            .expect("manifest bytes");
        archive
            .start_file("res00002.dat", options)
            .expect("pool entry");
        archive
            .write_all(b"<questestinterop />")
            .expect("pool bytes");
        archive.finish().expect("finish archive");

        let outcome = BlackboardReader
            .read_items(&archive_path, true)
            .expect("namespace URI, not its prefix, determines bb:file");
        assert_eq!(outcome.bank.len(), 0);
    }

    #[test]
    fn absent_xml_base_uses_the_pool_dat_stem_for_hotspot_media() {
        use std::io::Write;

        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("minimal-hotspot.zip");
        let output = std::fs::File::create(&archive_path).expect("archive output");
        let mut archive = zip::ZipWriter::new(output);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("imsmanifest.xml", options)
            .expect("manifest entry");
        archive
            .write_all(
                br#"<manifest xmlns:bb="http://www.blackboard.com/content-packaging/"><resources><resource type="assessment/x-bb-qti-pool" bb:file="res00002.dat" /></resources></manifest>"#,
            )
            .expect("manifest bytes");
        archive
            .start_file("res00002.dat", options)
            .expect("pool entry");
        archive
            .write_all(b"<questestinterop><item><presentation><matapplication uri=\"picture.png\" label=\"recovered.png\" /></presentation></item></questestinterop>")
            .expect("pool bytes");
        archive
            .start_file("res00002/picture.png", options)
            .expect("media entry");
        archive.write_all(b"hotspot bytes").expect("media bytes");
        archive.finish().expect("finish archive");

        let outcome = BlackboardReader
            .read_items(&archive_path, true)
            .expect("read package without xml:base");
        assert_eq!(outcome.bank.len(), 0);
        assert!(
            outcome
                .bank
                .media_base_dir()
                .expect("owned hotspot media")
                .join("recovered.png")
                .is_file()
        );
    }
}
