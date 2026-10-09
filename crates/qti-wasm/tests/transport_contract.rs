//! Durable tests of the adapter's typed contract and shared-engine dispatch.

use qti_wasm::{
    Artifact, CheckPackageResult, ConversionInput, ConvertRequest, ConvertResult, DocumentOptions,
    NamedBytes, PackageInput, check_package_request, convert_request, format_inventory,
};

const DATE: &str = "2026-10-08";

fn request(format: &str) -> ConvertRequest {
    ConvertRequest {
        input_format: "bbq_text_upload".into(),
        output_format: format.into(),
        input: ConversionInput::File {
            name: "quiz.txt".into(),
            bytes: b"MC\tWhich base?\tA\tcorrect\tG\tincorrect\n".to_vec(),
            companions: Vec::new(),
        },
        allow_mixed: true,
        limit: None,
        output_name: None,
        document: DocumentOptions {
            title: Some("Genetics".into()),
            date: Some(DATE.into()),
        },
        shuffle_seed: 42,
    }
}

#[test]
fn every_registered_writer_dispatches_from_the_same_typed_request() {
    let inventory = format_inventory();
    assert_eq!(inventory.formats.len(), 11);
    assert_eq!(
        inventory
            .formats
            .iter()
            .filter(|format| format.can_read)
            .count(),
        4
    );
    for format in inventory.formats {
        let response = convert_request(request(&format.name), DATE);
        let ConvertResult::Success {
            artifact,
            item_count,
            ..
        } = response
        else {
            panic!("{}: {response:?}", format.name);
        };
        assert_eq!(item_count, 1, "{}", format.name);
        let artifact = artifact.expect("MC has an artifact in each format");
        match artifact {
            Artifact::File { primary, .. } => {
                assert_eq!(primary.name, format.default_output_name);
                assert!(!primary.bytes.is_empty());
            }
            Artifact::Directory { name, entries } => {
                assert_eq!(name, format.default_output_name);
                assert!(!entries.is_empty());
            }
        }
    }
}

#[test]
fn read_and_write_warnings_keep_their_order_and_provenance() {
    let mut input = request("human_readable");
    input.input = ConversionInput::File {
        name: "quiz.txt".into(),
        bytes: b"MC\t<img src='figure.svg'/> Which base?\tA\tcorrect\tG\tincorrect\nINVALID\tbad\n"
            .to_vec(),
        companions: Vec::new(),
    };
    let ConvertResult::Success { warnings, .. } = convert_request(input, DATE) else {
        panic!("valid conversion");
    };
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].stage, "read");
    assert_eq!(warnings[0].source.as_deref(), Some("quiz.txt"));
    assert_eq!(warnings[1].stage, "write", "{warnings:?}");
    assert_eq!(warnings[1].source.as_deref(), Some("figure.svg"));
    assert!(warnings[1].item.is_some());
}

#[test]
fn malformed_names_duplicate_entries_and_invalid_dates_are_stateless_errors() {
    let bad_inputs = [
        ConversionInput::File {
            name: "../quiz.txt".into(),
            bytes: Vec::new(),
            companions: Vec::new(),
        },
        ConversionInput::Entries {
            name: "package".into(),
            entries: vec![
                NamedBytes {
                    name: "a.txt".into(),
                    bytes: vec![1],
                },
                NamedBytes {
                    name: "a.txt".into(),
                    bytes: vec![2],
                },
            ],
        },
    ];
    for input in bad_inputs {
        let mut bad = request("bbq_text_upload");
        bad.input = input;
        assert!(matches!(
            convert_request(bad, DATE),
            ConvertResult::Error { .. }
        ));
        assert!(matches!(
            convert_request(request("bbq_text_upload"), DATE),
            ConvertResult::Success { .. }
        ));
    }
    let mut bad = request("exam_yaml");
    bad.document.date = Some("2026-02-29".into());
    assert!(matches!(
        convert_request(bad, DATE),
        ConvertResult::Error { .. }
    ));
    let mut leap = request("exam_yaml");
    leap.document.date = Some("2024-02-29".into());
    assert!(matches!(
        convert_request(leap, DATE),
        ConvertResult::Success { .. }
    ));
}

#[test]
fn independent_integrity_checks_generated_bytes_and_rejects_broken_zip_safely() {
    let ConvertResult::Success {
        artifact: Some(Artifact::File { primary, .. }),
        ..
    } = convert_request(request("canvas_qti_v1_2"), DATE)
    else {
        panic!("generated Canvas ZIP");
    };
    let CheckPackageResult::Success { report } = check_package_request(PackageInput::Zip {
        bytes: primary.bytes,
    }) else {
        panic!("integrity report");
    };
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report.entry_count > 0);
    let CheckPackageResult::Success { report } = check_package_request(PackageInput::Zip {
        bytes: vec![0, 1, 2],
    }) else {
        panic!("malformed ZIP yields integrity finding");
    };
    assert!(!report.errors.is_empty());
    assert_eq!(report.errors[0].provenance, "safeInputHandling");
}

#[test]
fn companions_allow_identical_duplicates_and_reject_conflicting_payloads() {
    let file = NamedBytes {
        name: "media/figure.png".into(),
        bytes: vec![1],
    };
    let mut same = request("human_readable");
    let ConversionInput::File { companions, .. } = &mut same.input else {
        panic!("file input");
    };
    *companions = vec![file.clone(), file];
    assert!(matches!(
        convert_request(same.clone(), DATE),
        ConvertResult::Success { .. }
    ));
    let ConversionInput::File { companions, .. } = &mut same.input else {
        panic!("file input");
    };
    companions[1].bytes = vec![2];
    let ConvertResult::Error { error, .. } = convert_request(same, DATE) else {
        panic!("conflicting source payloads");
    };
    assert_eq!(error.category, "media");
    assert_eq!(error.source.as_deref(), Some("media/figure.png"));
}

#[test]
fn serde_schema_rejects_wrong_types_unknown_fields_and_negative_limits() {
    for malformed in [
        r#"{"inputFormat":false,"outputFormat":"exam_yaml","input":{"kind":"file","name":"a","bytes":[]}}"#,
        r#"{"inputFormat":"bbq_text_upload","outputFormat":"exam_yaml","input":{"kind":"file","name":"a","bytes":[]},"limit":-1}"#,
        r#"{"inputFormat":"bbq_text_upload","outputFormat":"exam_yaml","input":{"kind":"file","name":"a","bytes":[]},"htmlRender":true}"#,
    ] {
        assert!(serde_json::from_str::<ConvertRequest>(malformed).is_err());
    }
}
