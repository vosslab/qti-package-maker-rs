use std::fs;

use qti_core::{Item, ItemBank, ItemBody, MediaBaseDir};

use super::super::{boxed_writer, export_bank};
use crate::{EngineOptions, Writer};

fn bank_with_image(base: &std::path::Path, count: usize) -> ItemBank {
    let mut bank = ItemBank::with_media_base_dir(true, MediaBaseDir::external(base.to_owned()));
    for number in 0..count {
        bank.add_item(
            Item::new(
                format!("Question {number} <img src='cell.png'/>"),
                ItemBody::Mc {
                    choices: vec!["A".into(), "B".into()],
                    answer: "A".into(),
                },
            )
            .expect("item"),
        )
        .expect("add item");
    }
    bank
}

fn bank_with_distinct_case_media(base: &std::path::Path) -> ItemBank {
    let mut bank = ItemBank::with_media_base_dir(true, MediaBaseDir::external(base.to_owned()));
    for image in ["dir1/A.png", "dir2/a.png"] {
        bank.add_item(
            Item::new(
                format!("Identify <img src='{image}'/>"),
                ItemBody::Mc {
                    choices: vec!["A".into(), "B".into()],
                    answer: "A".into(),
                },
            )
            .expect("item"),
        )
        .expect("add item");
    }
    bank
}

#[test]
fn staged_media_names_must_represent_distinct_files_before_publish() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::create_dir(temporary.path().join("dir1")).expect("first media directory");
    fs::create_dir(temporary.path().join("dir2")).expect("second media directory");
    fs::write(temporary.path().join("dir1/A.png"), b"first image").expect("first image");
    fs::write(temporary.path().join("dir2/a.png"), b"second image").expect("second image");
    let bank = bank_with_distinct_case_media(temporary.path());
    let export = export_bank(&bank).expect("both source images resolve");
    assert_eq!(
        export.questions[0].files[0].path.to_str(),
        Some("media/A.png")
    );
    assert_eq!(
        export.questions[1].files[0].path.to_str(),
        Some("media/a.png")
    );

    // Measure the actual filesystem behavior instead of assuming an OS policy.
    let probe = temporary.path().join("case_probe");
    fs::create_dir(&probe).expect("probe directory");
    fs::write(probe.join("A.png"), b"first").expect("probe first name");
    fs::write(probe.join("a.png"), b"second").expect("probe second name");
    let names_are_distinct = fs::read_dir(&probe).expect("probe entries").count() == 2;

    let fresh = temporary.path().join("fresh_output");
    let fresh_result = writer().save_package(&bank, Some(&fresh));
    if names_are_distinct {
        fresh_result.expect("distinct names publish");
        assert_eq!(
            fs::read(fresh.join("media/A.png")).expect("first output"),
            b"first image"
        );
        assert_eq!(
            fs::read(fresh.join("media/a.png")).expect("second output"),
            b"second image"
        );
    } else {
        let error = fresh_result.expect_err("aliased names must fail before publish");
        assert!(
            error
                .to_string()
                .contains("filesystem may alias output names")
        );
        assert!(
            !fresh.exists(),
            "invalid first export must not be published"
        );
    }

    fs::write(temporary.path().join("cell.png"), b"original image").expect("original image");
    let destination = temporary.path().join("existing_output");
    writer()
        .save_package(&bank_with_image(temporary.path(), 1), Some(&destination))
        .expect("initial valid output");
    let original_json = fs::read(destination.join("item_00001.json")).expect("original JSON");
    let original_manifest =
        fs::read(destination.join(".qpm-ple-native-json")).expect("original manifest");
    let replacement = writer().save_package(&bank, Some(&destination));
    if names_are_distinct {
        replacement.expect("distinct names replace output");
        assert_eq!(
            fs::read(destination.join("media/A.png")).expect("first output"),
            b"first image"
        );
        assert_eq!(
            fs::read(destination.join("media/a.png")).expect("second output"),
            b"second image"
        );
    } else {
        let error = replacement.expect_err("aliased names must not replace valid output");
        assert!(
            error
                .to_string()
                .contains("filesystem may alias output names")
        );
        assert_eq!(
            fs::read(destination.join("item_00001.json")).expect("preserved JSON"),
            original_json
        );
        assert_eq!(
            fs::read(destination.join(".qpm-ple-native-json")).expect("preserved manifest"),
            original_manifest
        );
        assert_eq!(
            fs::read(destination.join("media/cell.png")).expect("preserved image"),
            b"original image"
        );
        assert!(!destination.join("item_00002.json").exists());
    }
}

fn writer() -> Box<dyn Writer> {
    boxed_writer(EngineOptions::default())
}

#[test]
fn writes_exact_export_and_replaces_owned_output_on_rerun() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let destination = temporary.path().join("ple");
    let first_bank = bank_with_image(temporary.path(), 2);
    let first = export_bank(&first_bank).expect("export");
    let engine = writer();
    assert_eq!(engine.media_policy(), qti_core::media::MediaPolicy::Package);
    assert_eq!(engine.supported_kinds().len(), 7);
    let outcome = engine
        .save_package(&first_bank, Some(&destination))
        .expect("first write");
    assert_eq!(outcome.path.as_deref(), Some(destination.as_path()));
    assert_eq!(outcome.warnings, first.warnings);
    for question in &first.questions {
        let name = format!("item_{:05}.json", question.item_number);
        assert_eq!(
            fs::read_to_string(destination.join(name)).expect("JSON"),
            question.source_json
        );
        for file in &question.files {
            assert_eq!(
                fs::read(destination.join(&file.path)).expect("media"),
                file.bytes
            );
        }
    }
    assert!(destination.join("item_00002.json").exists());
    assert!(destination.join(".qpm-ple-native-json").is_file());

    fs::write(temporary.path().join("cell.png"), b"changed image bytes").expect("changed image");
    let second_bank = bank_with_image(temporary.path(), 1);
    engine
        .save_package(&second_bank, Some(&destination))
        .expect("rerun");
    assert!(!destination.join("item_00002.json").exists());
    assert!(destination.join("item_00001.json").exists());
    assert_eq!(
        fs::read(destination.join("media/cell.png")).expect("media"),
        b"changed image bytes"
    );
}

#[test]
fn refuses_valid_independently_authored_json_without_manifest() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    fs::create_dir(&destination).expect("destination");
    let authored = export_bank(&bank).expect("valid export").questions[0]
        .source_json
        .clone();
    let item = destination.join("item_00001.json");
    fs::write(&item, &authored).expect("authored JSON");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(fs::read_to_string(&item).expect("preserved"), authored);
    assert_eq!(fs::read_dir(&destination).expect("read").count(), 1);
}

#[test]
fn refuses_modified_prior_json_and_manifest_without_changing_them() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    writer()
        .save_package(&bank, Some(&destination))
        .expect("first write");
    let item = destination.join("item_00001.json");
    let mut changed = fs::read(&item).expect("JSON");
    changed.extend_from_slice(b" \n");
    fs::write(&item, &changed).expect("manual edit");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(fs::read(&item).expect("preserved JSON"), changed);

    fs::write(
        &item,
        export_bank(&bank).expect("export").questions[0]
            .source_json
            .as_bytes(),
    )
    .expect("restore JSON");
    let manifest = destination.join(".qpm-ple-native-json");
    fs::write(&manifest, b"invalid manifest").expect("manual edit");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(&manifest).expect("preserved manifest"),
        b"invalid manifest"
    );
}

#[test]
fn refuses_modified_prior_media_without_changing_it() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    writer()
        .save_package(&bank, Some(&destination))
        .expect("first write");
    let media = destination.join("media/cell.png");
    fs::write(&media, b"manual change").expect("manual edit");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(fs::read(&media).expect("preserved media"), b"manual change");
}

#[test]
fn refuses_foreign_nested_and_same_named_invalid_json_without_changing_them() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    fs::create_dir(&destination).expect("destination");
    let foreign = destination.join("item_00001.json");
    fs::write(&foreign, b"{\"someone\":\"else\"}").expect("foreign JSON");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(&foreign).expect("foreign JSON"),
        b"{\"someone\":\"else\"}"
    );
    assert_eq!(fs::read_dir(&destination).expect("read").count(), 1);

    fs::remove_file(&foreign).expect("remove fixture");
    writer()
        .save_package(&bank, Some(&destination))
        .expect("empty directory accepted");
    let nested = destination.join("media/unrelated");
    fs::create_dir(&nested).expect("nested foreign directory");
    fs::write(nested.join("note.txt"), b"do not delete").expect("foreign note");
    let original = fs::read(destination.join("item_00001.json")).expect("original JSON");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(destination.join("item_00001.json")).expect("JSON"),
        original
    );
    assert_eq!(
        fs::read(nested.join("note.txt")).expect("note"),
        b"do not delete"
    );
}

#[test]
fn zero_render_creates_no_directory() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let destination = temporary.path().join("ple");
    let outcome = writer()
        .save_package(&ItemBank::new(true), Some(&destination))
        .expect("empty bank");
    assert!(outcome.path.is_none());
    assert!(!destination.exists());
}

#[test]
fn refuses_unreferenced_media_and_missing_referenced_media() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    writer()
        .save_package(&bank, Some(&destination))
        .expect("first write");
    let extra = destination.join("media/foreign.png");
    fs::write(&extra, b"foreign bytes").expect("extra media");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(fs::read(&extra).expect("foreign media"), b"foreign bytes");

    fs::remove_file(&extra).expect("remove fixture");
    fs::remove_file(destination.join("media/cell.png")).expect("remove referenced media");
    let original_json = fs::read(destination.join("item_00001.json")).expect("JSON");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(destination.join("item_00001.json")).expect("JSON"),
        original_json
    );
}

#[cfg(unix)]
#[test]
fn refuses_destination_and_media_symlinks() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().expect("temporary directory");
    fs::write(temporary.path().join("cell.png"), b"image bytes").expect("image");
    let bank = bank_with_image(temporary.path(), 1);
    let destination = temporary.path().join("ple");
    let outside = temporary.path().join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("note.txt"), b"untouched").expect("outside note");
    symlink(&outside, &destination).expect("link");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(outside.join("note.txt")).expect("outside note"),
        b"untouched"
    );

    fs::remove_file(&destination).expect("remove symlink");
    writer()
        .save_package(&bank, Some(&destination))
        .expect("write");
    let media = destination.join("media/cell.png");
    fs::remove_file(&media).expect("remove media");
    symlink(outside.join("note.txt"), &media).expect("media link");
    assert!(writer().save_package(&bank, Some(&destination)).is_err());
    assert_eq!(
        fs::read(outside.join("note.txt")).expect("outside note"),
        b"untouched"
    );
}
