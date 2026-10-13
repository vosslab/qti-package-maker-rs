//! Process-level contracts for user-visible conversion failures.

use std::process::Command;
use std::{fs, path::Path};

fn bbq_converter() -> &'static str {
    env!("CARGO_BIN_EXE_bbq-converter")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn missing_input_returns_a_fatal_diagnostic() {
    let output = Command::new(bbq_converter())
        .args(["-i", "bbq-missing-questions.txt", "-b"])
        .output()
        .expect("run converter");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("could not access"));
}

#[test]
fn unknown_engine_returns_usage_status_before_opening_input() {
    let output = Command::new(bbq_converter())
        .args(["-i", "bbq-not-opened-questions.txt", "-f", "not-a-format"])
        .output()
        .expect("run converter");
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown engine 'not-a-format'"));
}

#[test]
fn conversion_flag_rejects_an_explicit_non_package_output() {
    let output = Command::new(bbq_converter())
        .args([
            "-i",
            "bbq-not-opened-questions.txt",
            "-r",
            "--html-to-image",
            "-o",
            "human.html",
        ])
        .output()
        .expect("run converter");
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--html-to-image applies only to supported output formats"));
}

#[test]
fn conversion_flag_accepts_the_ple_directory_writer_with_explicit_output() {
    let output = Command::new(bbq_converter())
        .args([
            "-i",
            "bbq-not-opened-questions.txt",
            "-f",
            "ple_native_json",
            "--html-to-image",
            "-o",
            "ple-output",
        ])
        .output()
        .expect("run converter");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("could not access"));
}

#[test]
fn quiet_and_verbose_preserve_the_legacy_content_and_completion_receipt() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("bbq-parity-mc-only-questions.txt");
    fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("fixture");

    for flag in ["-q", "-v"] {
        let output = Command::new(bbq_converter())
            .args(["-i", path_text(&input), "-b", flag])
            .current_dir(directory.path())
            .output()
            .expect("run converter");
        assert!(output.status.success(), "{flag}: {}", stderr(&output));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.starts_with("Content Name: parity-mc-only\n"));
        assert!(stdout.ends_with("DONE, saved 1 of 1 output files\n"));
        if flag == "-q" {
            assert_eq!(
                stdout,
                "Content Name: parity-mc-only\nDONE, saved 1 of 1 output files\n"
            );
        } else {
            assert!(stdout.contains("Initialized Engine: bbq_text_upload (bbq_text_upload)"));
            assert!(stdout.contains("Successfully loaded 1 new assessment items"));
            assert!(stdout.contains("Saving package bbq_text_upload\n  with 1 assessment items."));
        }
    }
}

#[test]
fn all_formats_write_each_named_artifact_and_report_eleven_attempts() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("bbq-all-formats-questions.txt");
    fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("fixture");

    let output = Command::new(bbq_converter())
        .args(["-i", path_text(&input), "--all", "-q"])
        .current_dir(directory.path())
        .output()
        .expect("run converter");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Content Name: all-formats\nDONE, saved 11 of 11 output files\n"
    );
    for name in [
        "qti12-all-formats.zip",
        "qti21-all-formats.zip",
        "human-all-formats.html",
        "bbq-all-formats.txt",
        "selftest-all-formats.html",
        "aiken-all-formats.txt",
        "bez-all-formats.zip",
        "exam-all-formats.yaml",
        "okla-all-formats.txt",
        "text2qti-all-formats.txt",
    ] {
        assert!(directory.path().join(name).is_file(), "missing {name}");
    }
    let ple_directory = directory.path().join("ple-all-formats");
    assert!(ple_directory.is_dir(), "missing PLE directory");
    assert!(ple_directory.join("item_00001.json").is_file());
}

#[test]
fn ple_format_writes_to_an_explicit_directory() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("bbq-ple-questions.txt");
    fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("fixture");
    let destination = directory.path().join("native-questions");

    let output = Command::new(bbq_converter())
        .args([
            "-i",
            path_text(&input),
            "-f",
            "ple_native_json",
            "-o",
            path_text(&destination),
            "-q",
        ])
        .output()
        .expect("run converter");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Content Name: ple\nDONE, saved 1 of 1 output files\n"
    );
    assert!(destination.is_dir());
    assert!(destination.join("item_00001.json").is_file());
}

#[test]
fn ambiguous_engine_prefix_lists_candidates_before_opening_input() {
    let output = Command::new(bbq_converter())
        .args(["-i", "bbq-not-opened-questions.txt", "-f", "b"])
        .output()
        .expect("run converter");

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains(
        "ambiguous engine 'b'; candidates: blackboard_export_zip, blackboard_qti_v2_1, bbq_text_upload"
    ));
}

#[test]
fn writer_io_failure_is_fatal_and_names_the_selected_engine() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("bbq-writer-io-questions.txt");
    fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("fixture");
    let output_path = directory.path().join("existing-directory");
    fs::create_dir(&output_path).expect("output directory");

    let output = Command::new(bbq_converter())
        .args(["-i", path_text(&input), "-b", "-o", path_text(&output_path)])
        .output()
        .expect("run converter");

    assert_eq!(output.status.code(), Some(1));
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("engine 'bbq_text_upload' could not access"));
    assert!(diagnostic.contains("existing-directory"));
}

#[test]
fn three_packaging_formats_share_one_observable_html_conversion_pass() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("bbq-shared-conversion-questions.txt");
    fs::write(&input, "MC\tQuestion\ta\tCorrect\tb\tIncorrect\n").expect("fixture");

    let output = Command::new(bbq_converter())
        .args([
            "-i",
            path_text(&input),
            "-1",
            "-2",
            "-B",
            "--html-to-image",
            "-v",
        ])
        .current_dir(directory.path())
        .output()
        .expect("run converter");

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout
            .matches("Native HTML-to-image conversion passes: 1")
            .count(),
        1
    );
    assert!(stdout.ends_with("DONE, saved 3 of 3 output files\n"));
    for name in [
        "qti12-shared-conversion.zip",
        "qti21-shared-conversion.zip",
        "bez-shared-conversion.zip",
    ] {
        assert!(directory.path().join(name).is_file(), "missing {name}");
    }
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("UTF-8 temporary path")
}
