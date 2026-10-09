use std::fs;

use qti_core::EntryMap;

use super::write_entries;

fn export(count: usize, images: &[(&str, &[u8])]) -> EntryMap {
    let mut entries = EntryMap::new();
    for number in 1..=count {
        entries.insert(
            format!("item_{number:05}.json"),
            format!("{{\"item\":{number}}}").into_bytes(),
        );
    }
    for (name, bytes) in images {
        entries.insert(format!("media/{name}"), bytes.to_vec());
    }
    entries
}

#[test]
fn staged_media_names_must_represent_distinct_files_before_publish() {
    let temporary = tempfile::tempdir().expect("directory");
    let probe = temporary.path().join("case_probe");
    fs::create_dir(&probe).expect("probe");
    fs::write(probe.join("A.png"), b"first").expect("first");
    fs::write(probe.join("a.png"), b"second").expect("second");
    let distinct = fs::read_dir(&probe).expect("probe entries").count() == 2;
    let entries = export(2, &[("A.png", b"first"), ("a.png", b"second")]);
    let fresh = temporary.path().join("fresh");
    let result = write_entries(&entries, &fresh);
    if distinct {
        result.expect("distinct names publish");
        assert_eq!(
            fs::read(fresh.join("media/A.png")).expect("first"),
            b"first"
        );
        assert_eq!(
            fs::read(fresh.join("media/a.png")).expect("second"),
            b"second"
        );
    } else {
        assert!(
            result
                .expect_err("aliased names rejected")
                .to_string()
                .contains("filesystem may alias output names")
        );
        assert!(!fresh.exists());
    }
    let destination = temporary.path().join("existing");
    write_entries(&export(1, &[("cell.png", b"original")]), &destination).expect("first publish");
    let original = fs::read(destination.join("item_00001.json")).expect("original");
    let manifest = fs::read(destination.join(".qpm-ple-native-json")).expect("manifest");
    let result = write_entries(&entries, &destination);
    if distinct {
        result.expect("distinct names replace");
    } else {
        assert!(result.is_err());
        assert_eq!(
            fs::read(destination.join("item_00001.json")).expect("preserved"),
            original
        );
        assert_eq!(
            fs::read(destination.join(".qpm-ple-native-json")).expect("manifest preserved"),
            manifest
        );
        assert_eq!(
            fs::read(destination.join("media/cell.png")).expect("image preserved"),
            b"original"
        );
        assert!(!destination.join("item_00002.json").exists());
    }
}

#[test]
fn writes_exact_entries_and_replaces_owned_output_on_rerun() {
    let temporary = tempfile::tempdir().expect("directory");
    let destination = temporary.path().join("ple");
    let first = export(2, &[("cell.png", b"image")]);
    write_entries(&first, &destination).expect("first write");
    for (name, bytes) in &first {
        assert_eq!(
            &fs::read(destination.join(name)).expect("persisted entry"),
            bytes
        );
    }
    assert!(destination.join(".qpm-ple-native-json").is_file());
    write_entries(&export(1, &[("cell.png", b"changed")]), &destination).expect("replacement");
    assert!(!destination.join("item_00002.json").exists());
    assert_eq!(
        fs::read(destination.join("media/cell.png")).expect("changed media"),
        b"changed"
    );
}

#[test]
fn refuses_independently_authored_output_without_manifest() {
    let temporary = tempfile::tempdir().expect("directory");
    let destination = temporary.path().join("ple");
    fs::create_dir(&destination).expect("destination");
    fs::write(destination.join("item_00001.json"), b"{\"item\":1}").expect("authored JSON");
    assert!(write_entries(&export(1, &[]), &destination).is_err());
    assert_eq!(
        fs::read(destination.join("item_00001.json")).expect("preserved"),
        b"{\"item\":1}"
    );
    assert_eq!(fs::read_dir(destination).expect("entries").count(), 1);
}

#[test]
fn refuses_modified_prior_json_media_and_manifest_without_changing_them() {
    for name in ["item_00001.json", "media/cell.png", ".qpm-ple-native-json"] {
        let temporary = tempfile::tempdir().expect("directory");
        let destination = temporary.path().join("ple");
        let entries = export(1, &[("cell.png", b"original")]);
        write_entries(&entries, &destination).expect("publish");
        let changed = destination.join(name);
        fs::write(&changed, b"manual edit").expect("manual edit");
        assert!(write_entries(&entries, &destination).is_err());
        assert_eq!(fs::read(changed).expect("preserved"), b"manual edit");
    }
}

#[test]
fn refuses_foreign_nested_and_extra_media_without_changing_them() {
    for name in ["note.txt", "media/foreign.png", "media/unrelated/note.txt"] {
        let temporary = tempfile::tempdir().expect("directory");
        let destination = temporary.path().join("ple");
        let entries = export(1, &[("cell.png", b"original")]);
        write_entries(&entries, &destination).expect("publish");
        let foreign = destination.join(name);
        fs::create_dir_all(foreign.parent().expect("parent")).expect("foreign parent");
        fs::write(&foreign, b"do not delete").expect("foreign file");
        assert!(write_entries(&entries, &destination).is_err());
        assert_eq!(fs::read(foreign).expect("preserved"), b"do not delete");
        assert_eq!(
            fs::read(destination.join("media/cell.png")).expect("original"),
            b"original"
        );
    }
}

#[test]
fn rejects_unsafe_entries_before_publishing_anything() {
    let temporary = tempfile::tempdir().expect("directory");
    let destination = temporary.path().join("ple");
    for name in [
        "../outside.json",
        "item_00000.json",
        "media/a/b.png",
        ".qpm-ple-native-json",
    ] {
        let mut entries = export(1, &[]);
        entries.insert(name.into(), vec![1]);
        assert!(write_entries(&entries, &destination).is_err());
        assert!(!destination.exists());
    }
}

#[test]
fn refuses_missing_prior_media() {
    let temporary = tempfile::tempdir().expect("directory");
    let destination = temporary.path().join("ple");
    let entries = export(1, &[("cell.png", b"original")]);
    write_entries(&entries, &destination).expect("publish");
    fs::remove_file(destination.join("media/cell.png")).expect("remove");
    assert!(write_entries(&entries, &destination).is_err());
    assert_eq!(
        fs::read(destination.join("item_00001.json")).expect("preserved"),
        b"{\"item\":1}"
    );
}

#[cfg(unix)]
#[test]
fn refuses_destination_and_media_symlinks() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().expect("directory");
    let destination = temporary.path().join("ple");
    let outside = temporary.path().join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("note.txt"), b"untouched").expect("outside file");
    let entries = export(1, &[("cell.png", b"original")]);
    symlink(&outside, &destination).expect("destination link");
    assert!(write_entries(&entries, &destination).is_err());
    fs::remove_file(&destination).expect("remove link");
    write_entries(&entries, &destination).expect("publish");
    fs::remove_file(destination.join("media/cell.png")).expect("remove media");
    symlink(outside.join("note.txt"), destination.join("media/cell.png")).expect("media link");
    assert!(write_entries(&entries, &destination).is_err());
    assert_eq!(
        fs::read(outside.join("note.txt")).expect("outside preserved"),
        b"untouched"
    );
}
