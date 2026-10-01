use std::collections::BTreeMap;

use crate::Severity;

use super::*;

fn manifest(body: &str) -> Vec<u8> {
    format!(
        "<manifest xmlns='http://www.imsglobal.org/xsd/imscp_v1p1' identifier='manifest'><resources>{body}</resources></manifest>"
    )
    .into_bytes()
}

fn entries(pairs: &[(&str, Vec<u8>)]) -> BTreeMap<String, Vec<u8>> {
    pairs
        .iter()
        .map(|(path, bytes)| ((*path).to_owned(), bytes.clone()))
        .collect()
}

fn has(violations: &[Violation], code: &str) -> bool {
    violations.iter().any(|violation| violation.code == code)
}

fn assert_error(violations: &[Violation], code: &str, path: &str, provenance: Provenance) {
    assert!(
        violations.iter().any(|violation| {
            violation.code == code
                && violation.path == path
                && violation.severity == Severity::Error
                && violation.provenance == provenance
        }),
        "missing {code} at {path}: {violations:?}"
    );
}

macro_rules! negative_case {
    ($name:ident, $items:expr, $code:literal, $path:literal, $provenance:expr) => {
        #[test]
        fn $name() {
            assert_error(&check_entries(&$items), $code, $path, $provenance);
        }
    };
}

macro_rules! advisory_case {
    ($name:ident, $items:expr, $code:literal, $path:literal) => {
        #[test]
        fn $name() {
            let findings = check_entries(&$items);
            assert!(
                findings.iter().any(|finding| finding.code == $code
                    && finding.path == $path
                    && finding.severity == Severity::Advisory
                    && finding.provenance == Provenance::PythonParityAdvisory),
                "missing {}: {findings:?}",
                $code
            );
        }
    };
}

negative_case!(
    negative_dangling_resource_href,
    entries(&[(
        "imsmanifest.xml",
        manifest("<resource identifier='item' href='gone.xml'/>")
    )]),
    "dangling-resource-href",
    "imsmanifest.xml",
    Provenance::FormatRequirement
);

negative_case!(negative_qti12_varequal, entries(&[("imsmanifest.xml", manifest("")), ("q.xml", b"<questestinterop xmlns='http://www.imsglobal.org/xsd/ims_qtiasiv1p2'><item><response_lid ident='r'><render_choice/><response_label ident='a'/></response_lid><varequal respident='r'>bad</varequal></item></questestinterop>".to_vec())]), "qti12-dangling-varequal", "q.xml", Provenance::FormatRequirement);
negative_case!(negative_qti21_response, entries(&[("imsmanifest.xml", manifest("")), ("q.xml", b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><outcomeDeclaration identifier='SCORE'/><responseDeclaration identifier='r'><correctResponse><value>bad</value></correctResponse></responseDeclaration><choiceInteraction responseIdentifier='r'><simpleChoice identifier='a'/></choiceInteraction></assessmentItem>".to_vec())]), "qti21-dangling-correct-response", "q.xml", Provenance::FormatRequirement);
negative_case!(
    negative_score,
    entries(&[
        ("imsmanifest.xml", manifest("")),
        (
            "q.xml",
            b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'/>".to_vec()
        )
    ]),
    "missing-score-outcome",
    "q.xml",
    Provenance::BlackboardImportFailure
);
advisory_case!(
    negative_single_pixel,
    entries(&[("imsmanifest.xml", manifest("")), ("tiny.png", png(1, 1))]),
    "invisible-raster",
    "tiny.png"
);
negative_case!(
    negative_media_trace,
    entries(&[
        (
            "imsmanifest.xml",
            manifest(
                "<resource identifier='item' href='item.xml'><file href='item.xml'/></resource>"
            )
        ),
        (
            "item.xml",
            b"<assessmentItem><img src='image.png'/></assessmentItem>".to_vec()
        ),
        ("image.png", png(8, 8))
    ]),
    "broken-media-trace",
    "item.xml",
    Provenance::BlackboardImportFailure
);
negative_case!(
    negative_xid,
    entries(&[
        (
            "imsmanifest.xml",
            b"<manifest xmlns:bb='http://www.blackboard.com/content-packaging/'><resource bb:file='pool.dat'/></manifest>".to_vec()
        ),
        ("pool.dat", b"<root>bbcswebdav/xid-77</root>".to_vec())
    ]),
    "orphaned-xid-token",
    "blackboard-export",
    Provenance::FormatRequirement
);
negative_case!(
    negative_parent,
    entries(&[
        (
            "imsmanifest.xml",
            b"<manifest xmlns:bb='http://www.blackboard.com/content-packaging/'><resource bb:file='links.dat'/></manifest>".to_vec()
        ),
        (
            "links.dat",
            b"<root><cms_resource_link><parentId>gone</parentId></cms_resource_link></root>"
                .to_vec()
        )
    ]),
    "orphaned-cs-parent",
    "blackboard-export",
    Provenance::FormatRequirement
);
negative_case!(
    negative_lom,
    entries(&[
        (
            "imsmanifest.xml",
            b"<manifest xmlns:bb='http://www.blackboard.com/content-packaging/'><resource bb:file='pool.dat'/></manifest>".to_vec()
        ),
        ("csfiles/__xid-77.png", png(8, 8))
    ]),
    "missing-lom-sidecar",
    "csfiles/__xid-77.png",
    Provenance::FormatRequirement
);
negative_case!(
    negative_dangling_file_href,
    entries(&[(
        "imsmanifest.xml",
        manifest("<resource identifier='item'><file href='gone.xml'/></resource>")
    )]),
    "dangling-file-href",
    "imsmanifest.xml",
    Provenance::FormatRequirement
);
negative_case!(
    negative_dangling_dependency,
    entries(&[(
        "imsmanifest.xml",
        manifest("<resource identifier='item'><dependency identifierref='gone'/></resource>")
    )]),
    "dangling-dependency",
    "imsmanifest.xml",
    Provenance::FormatRequirement
);
negative_case!(
    negative_unsafe_identifier,
    entries(&[(
        "imsmanifest.xml",
        manifest("<resource identifier='bad id'/>")
    )]),
    "unsafe-identifier",
    "imsmanifest.xml",
    Provenance::BlackboardImportFailure
);
negative_case!(
    negative_truncated_raster,
    entries(&[
        ("imsmanifest.xml", manifest("")),
        ("bad.png", b"bad".to_vec())
    ]),
    "unreadable-raster",
    "bad.png",
    Provenance::FormatRequirement
);
negative_case!(
    negative_dangling_image,
    entries(&[
        (
            "imsmanifest.xml",
            manifest(
                "<resource identifier='item' href='item.xml'><file href='item.xml'/></resource>"
            )
        ),
        (
            "item.xml",
            b"<assessmentItem><img src='gone.png'/></assessmentItem>".to_vec()
        )
    ]),
    "dangling-image-source",
    "item.xml",
    Provenance::FormatRequirement
);
negative_case!(
    canary_manifest_media,
    entries(&[
        (
            "imsmanifest.xml",
            manifest(
                "<resource identifier='item' href='item.xml'><file href='item.xml'/><dependency identifierref='gone'/></resource>"
            )
        ),
        (
            "item.xml",
            b"<assessmentItem><img src='gone.png'/></assessmentItem>".to_vec()
        )
    ]),
    "dangling-dependency",
    "imsmanifest.xml",
    Provenance::FormatRequirement
);

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut data = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    data.extend(width.to_be_bytes());
    data.extend(height.to_be_bytes());
    data
}

#[test]
fn valid_qti21_media_trace_passes_from_an_extracted_directory() {
    let root = std::env::temp_dir().join(format!(
        "qti-integrity-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("items")).expect("test package item directory");
    std::fs::create_dir_all(root.join("images")).expect("test package image directory");
    let package_manifest = manifest(
        "<resource identifier='item' href='items/item.xml'><file href='items/item.xml'/><dependency identifierref='media'/></resource><resource identifier='media'><file href='images/figure.png'/></resource>",
    );
    let item = b"<q:assessmentItem xmlns:q='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='item'><q:outcomeDeclaration identifier='SCORE'/><q:itemBody><img src='../images/figure.png'/></q:itemBody></q:assessmentItem>";
    std::fs::write(root.join("imsmanifest.xml"), package_manifest).expect("test manifest");
    std::fs::write(root.join("items/item.xml"), item).expect("test item");
    std::fs::write(root.join("images/figure.png"), png(8, 8)).expect("test image");
    let violations = crate::check_package(&root);
    std::fs::remove_dir_all(&root).expect("remove test package");
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
}

#[test]
fn public_api_accepts_a_zip_without_extracting_it() {
    use std::io::Write;

    let path = std::env::temp_dir().join(format!("qti-integrity-{}.zip", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::File::create(&path).expect("test ZIP");
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    archive
        .start_file("imsmanifest.xml", options)
        .expect("test manifest entry");
    archive
        .write_all(&manifest(""))
        .expect("test manifest bytes");
    archive.finish().expect("finish test ZIP");
    let violations = crate::check_package(&path);
    std::fs::remove_file(&path).expect("remove test ZIP");
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
}

#[test]
fn negative_manifest_and_answer_corpus_is_rejected() {
    let qti12 = b"<questestinterop xmlns='http://www.imsglobal.org/xsd/ims_qtiasiv1p2'><item ident='item'><response_lid ident='r'><render_choice/><response_label ident='a'/></response_lid><varequal respident='r'>gone</varequal></item></questestinterop>".to_vec();
    let qti21 = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='item'><responseDeclaration identifier='R'><correctResponse><value>gone</value></correctResponse></responseDeclaration><choiceInteraction responseIdentifier='R'><simpleChoice identifier='a'/></choiceInteraction></assessmentItem>".to_vec();
    let items = entries(&[
        (
            "imsmanifest.xml",
            manifest(
                "<resource identifier='item' href='missing.xml'><file href='ghost.xml'/><dependency identifierref='gone'/></resource><resource identifier='item' href='also-missing.xml'/>",
            ),
        ),
        ("qti12.xml", qti12),
        ("qti21.xml", qti21),
    ]);
    let violations = check_entries(&items);
    for code in [
        "dangling-resource-href",
        "dangling-file-href",
        "dangling-dependency",
        "duplicate-resource-identifier",
        "qti12-dangling-varequal",
        "qti21-dangling-correct-response",
        "missing-score-outcome",
    ] {
        assert!(has(&violations, code), "missing {code}: {violations:?}");
    }
}

#[test]
fn negative_media_trace_corpus_rejects_wrongly_linked_and_bad_media() {
    let item = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='item'><outcomeDeclaration identifier='SCORE'/><itemBody><img src='images/figure.png'/></itemBody></assessmentItem>".to_vec();
    let items = entries(&[
        (
            "imsmanifest.xml",
            manifest(
                "<resource identifier='item' href='item.xml'><file href='item.xml'/></resource><resource identifier='media'><file href='images/other.png'/></resource>",
            ),
        ),
        ("item.xml", item),
        ("images/figure.png", b"not an image".to_vec()),
        ("images/unlisted.png", png(8, 8)),
    ]);
    let violations = check_entries(&items);
    for code in ["broken-media-trace", "unreadable-raster"] {
        assert!(has(&violations, code), "missing {code}: {violations:?}");
    }
}

#[test]
fn malformed_xml_dtd_and_trailing_content_are_each_rejected() {
    for item in [
        b"<assessmentItem".to_vec(),
        b"<!DOCTYPE x><assessmentItem/>".to_vec(),
        b"<assessmentItem/> trailing".to_vec(),
    ] {
        let violations = check_entries(&entries(&[
            ("imsmanifest.xml", manifest("")),
            ("item.xml", item),
        ]));
        assert!(has(&violations, "invalid-xml"), "{violations:?}");
    }
}

#[test]
fn media_uri_decoding_base_and_nested_resource_boundaries_are_checked() {
    let item = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' xml:base='items/' identifier='item'><outcomeDeclaration identifier='SCORE'/><itemBody><img xml:base='../assets/' src='figure%20one.png?cache=1#x'/></itemBody></assessmentItem>".to_vec();
    let nested = "<resource identifier='item' href='items/item.xml'><file href='items/item.xml'/><dependency identifierref='media'/><resource identifier='hidden'><file href='assets/figure one.png'/></resource></resource><resource identifier='media'><file href='assets/figure one.png'/></resource>";
    let violations = check_entries(&entries(&[
        ("imsmanifest.xml", manifest(nested)),
        ("items/item.xml", item),
        ("assets/figure one.png", png(8, 8)),
    ]));
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
}

#[test]
fn nested_resource_file_does_not_create_a_media_trace() {
    let item = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='item'><outcomeDeclaration identifier='SCORE'/><itemBody><img src='images/figure.png'/></itemBody></assessmentItem>".to_vec();
    let resources = "<resource identifier='item' href='item.xml'><file href='item.xml'/><resource identifier='hidden'><file href='images/figure.png'/></resource></resource>";
    let violations = check_entries(&entries(&[
        ("imsmanifest.xml", manifest(resources)),
        ("item.xml", item),
        ("images/figure.png", png(8, 8)),
    ]));
    assert!(has(&violations, "broken-media-trace"), "{violations:?}");
}

#[test]
fn foreign_namespace_qti_children_do_not_satisfy_semantic_checks() {
    let item = b"<q:assessmentItem xmlns:q='http://www.imsglobal.org/xsd/imsqti_v2p1' xmlns:evil='urn:evil' identifier='item'><evil:outcomeDeclaration identifier='SCORE'/><evil:responseDeclaration identifier='R'><evil:correctResponse><evil:value>gone</evil:value></evil:correctResponse></evil:responseDeclaration><q:itemBody/></q:assessmentItem>".to_vec();
    let violations = check_entries(&entries(&[
        ("imsmanifest.xml", manifest("")),
        ("item.xml", item),
    ]));
    assert!(has(&violations, "missing-score-outcome"), "{violations:?}");
    assert!(
        !has(&violations, "qti21-dangling-correct-response"),
        "foreign declaration was incorrectly interpreted: {violations:?}"
    );
}

#[test]
fn qti21_choice_tokens_match_python_for_gap_order_match_and_literal_interactions() {
    let item = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='item'><outcomeDeclaration identifier='SCORE'/><responseDeclaration identifier='gap'><correctResponse><value>g t</value></correctResponse></responseDeclaration><responseDeclaration identifier='order'><correctResponse><value>a b</value></correctResponse></responseDeclaration><responseDeclaration identifier='match'><correctResponse><value>b a</value></correctResponse></responseDeclaration><responseDeclaration identifier='literal'><correctResponse><value>anything is literal</value></correctResponse></responseDeclaration><itemBody><gapMatchInteraction responseIdentifier='gap'><gap identifier='g'/><gapText identifier='t'/></gapMatchInteraction><orderInteraction responseIdentifier='order'><simpleChoice identifier='a'/><simpleChoice identifier='b'/></orderInteraction><matchInteraction responseIdentifier='match'><simpleAssociableChoice identifier='a'/><simpleAssociableChoice identifier='b'/></matchInteraction><textEntryInteraction responseIdentifier='literal'/><extendedTextInteraction responseIdentifier='literal'/><numericInteraction responseIdentifier='literal'/></itemBody></assessmentItem>".to_vec();
    let violations = check_entries(&entries(&[
        ("imsmanifest.xml", manifest("")),
        ("item.xml", item),
    ]));
    assert!(
        !has(&violations, "qti21-dangling-correct-response"),
        "Python parity uses membership, including the inverted directed pair: {violations:?}"
    );
}

#[test]
fn negative_image_identifier_and_blackboard_corpora_are_rejected() {
    let item = b"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='1bad'><itemBody/></assessmentItem>".to_vec();
    let bb_manifest = b"<manifest xmlns:bb='http://www.blackboard.com/content-packaging/' identifier='m'><resources><resource bb:file='missing.dat'/></resources></manifest>".to_vec();
    let pool =
        b"<root>bbcswebdav/xid-5_1<bbmd_asi_object_id>_1_1</bbmd_asi_object_id></root>".to_vec();
    let links = b"<root><cms_resource_link><resourceId>other</resourceId><parentId>_9_9</parentId></cms_resource_link></root>".to_vec();
    let image_items = entries(&[
        (
            "imsmanifest.xml",
            b"<manifest identifier='bad id'><resources/></manifest>".to_vec(),
        ),
        ("item.xml", item),
        ("tiny.png", png(1, 1)),
        ("truncated.jpg", b"\xff\xd8\xff".to_vec()),
    ]);
    let image_violations = check_entries(&image_items);
    for code in [
        "unsafe-identifier",
        "invisible-raster",
        "unreadable-raster",
        "missing-score-outcome",
    ] {
        assert!(
            has(&image_violations, code),
            "missing {code}: {image_violations:?}"
        );
    }
    let bb_violations = check_entries(&entries(&[
        ("imsmanifest.xml", bb_manifest),
        ("res00002.dat", pool),
        ("res00005.dat", links),
        ("csfiles/__xid-6_1.png", png(8, 8)),
    ]));
    for code in [
        "dangling-bb-file",
        "orphaned-xid-token",
        "orphaned-xid-resource-link",
        "orphaned-cs-parent",
        "missing-lom-sidecar",
    ] {
        assert!(
            has(&bb_violations, code),
            "missing {code}: {bb_violations:?}"
        );
    }
}
