use std::io::Write;

use zip::write::SimpleFileOptions;

use super::*;

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .expect("test ZIP member");
        writer.write_all(data).expect("test ZIP data");
    }
    writer.finish().expect("test ZIP finish").into_inner()
}

fn central_headers(bytes: &[u8]) -> Vec<usize> {
    bytes
        .windows(4)
        .enumerate()
        .filter_map(|(offset, signature)| (signature == b"PK\x01\x02").then_some(offset))
        .collect()
}

fn as_zip64(bytes: &[u8]) -> Vec<u8> {
    let end = bytes.len() - 22;
    let central_start = central_headers(bytes)[0] as u64;
    let central_size = end as u64 - central_start;
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap()) as u64;
    let mut zip64 = bytes[..end].to_vec();
    zip64.extend(b"PK\x06\x06");
    zip64.extend(44_u64.to_le_bytes());
    zip64.extend([45, 0, 45, 0]);
    zip64.extend(0_u32.to_le_bytes());
    zip64.extend(0_u32.to_le_bytes());
    zip64.extend(count.to_le_bytes());
    zip64.extend(count.to_le_bytes());
    zip64.extend(central_size.to_le_bytes());
    zip64.extend(central_start.to_le_bytes());
    zip64.extend(b"PK\x06\x07");
    zip64.extend(0_u32.to_le_bytes());
    zip64.extend((end as u64).to_le_bytes());
    zip64.extend(1_u32.to_le_bytes());
    let mut end_record = bytes[end..].to_vec();
    end_record[8..12].fill(255);
    end_record[12..20].fill(255);
    zip64.extend(end_record);
    zip64
}

fn append_zip64_candidate(bytes: &mut Vec<u8>, record_size: u64, count: u64) {
    let start = bytes.len();
    bytes.resize(start + 56, 0);
    bytes[start..start + 4].copy_from_slice(b"PK\x06\x06");
    bytes[start + 4..start + 12].copy_from_slice(&record_size.to_le_bytes());
    bytes[start + 32..start + 40].copy_from_slice(&count.to_le_bytes());
}

fn append_zip64_fallback(bytes: &mut Vec<u8>, declared_offset: u64, disks: u32) {
    bytes.extend(b"PK\x06\x07");
    bytes.extend(0_u32.to_le_bytes());
    bytes.extend(declared_offset.to_le_bytes());
    bytes.extend(disks.to_le_bytes());
    let mut end_record = vec![0; 22];
    end_record[..4].copy_from_slice(b"PK\x05\x06");
    end_record[8..20].fill(255);
    bytes.extend(end_record);
}

fn assert_rejected(bytes: &[u8], code: &str) {
    let violation = read_zip_entries(bytes).expect_err("unsafe archive accepted");
    assert_eq!(violation.code, code, "{violation:?}");
    assert_eq!(violation.provenance, Provenance::SafeInputHandling);
}

#[test]
fn every_input_boundary_rejects_unsafe_posix_names() {
    for name in [
        "",
        "/root",
        "../escape",
        "a/../escape",
        "a\\b",
        "C:/a",
        "a:b",
        "a\0b",
        "a\nb",
        "a\u{85}b",
        "a//b",
        "./a",
        "a/./b",
        "a/",
    ] {
        let entries = BTreeMap::from([(name.to_owned(), Vec::new())]);
        let findings = crate::check_entries(&entries);
        assert_eq!(findings[0].code, "unsafe-entry-path", "accepted {name:?}");
        // ZIP writers treat a trailing slash as an explicit directory marker.
        if name != "a/" {
            assert_rejected(&archive(&[(name, b"")]), "unsafe-entry-path");
        }
    }
    assert_rejected(&archive(&[("../directory/", b"")]), "unsafe-entry-path");
    let bytes = archive(&[("safe/", b""), ("safe/file", b"data")]);
    assert_eq!(
        read_zip_entries(&bytes).expect("safe ZIP")["safe/file"],
        b"data"
    );
}

#[test]
fn duplicate_and_symbolic_link_metadata_are_rejected() {
    let mut bytes = archive(&[("first", b"a"), ("other", b"b")]);
    let headers = central_headers(&bytes);
    bytes[headers[1] + 46..headers[1] + 51].copy_from_slice(b"first");
    assert_rejected(&bytes, "duplicate-entry");

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink("link", "target", SimpleFileOptions::default())
        .expect("test symbolic link");
    let bytes = writer.finish().expect("test ZIP finish").into_inner();
    assert_rejected(&bytes, "symlink-entry");
}

#[test]
fn counts_include_directories_and_original_duplicate_records() {
    let bytes = archive(&[("directory/", b"")]);
    let end = bytes.len() - 22;
    let header = central_headers(&bytes)[0];
    let record = &bytes[header..end];
    let mut repeated = bytes[..header].to_vec();
    for _ in 0..=MAX_ENTRY_COUNT {
        repeated.extend(record);
    }
    let repeated_end = repeated.len();
    repeated.extend(&bytes[end..]);
    let count = (MAX_ENTRY_COUNT as u16 + 1).to_le_bytes();
    repeated[repeated_end + 8..repeated_end + 10].copy_from_slice(&count);
    repeated[repeated_end + 10..repeated_end + 12].copy_from_slice(&count);
    repeated[repeated_end + 12..repeated_end + 16]
        .copy_from_slice(&((record.len() * (MAX_ENTRY_COUNT + 1)) as u32).to_le_bytes());
    // A malicious original count is rejected before zip-rs allocates metadata,
    // even though zip-rs would collapse these directory records to one entry.
    assert_rejected(&repeated, "package-size-limit");

    let entries = (0..=MAX_ENTRY_COUNT)
        .map(|index| (format!("{index}"), Vec::new()))
        .collect();
    assert_eq!(crate::check_entries(&entries)[0].code, "package-size-limit");
}

#[test]
fn fallback_directory_cannot_bypass_metadata_bounds() {
    let names: Vec<_> = (0..=MAX_ENTRY_COUNT)
        .map(|index| format!("earlier-{index}"))
        .collect();
    let entries: Vec<_> = names
        .iter()
        .map(|name| (name.as_str(), b"".as_slice()))
        .collect();
    let earlier = archive(&entries);
    let later = archive(&[("later", b"")]);
    let header = central_headers(&later)[0];
    let end = later.len() - 22;
    for mut bytes in [earlier.clone(), as_zip64(&earlier)] {
        // Fallback searches the whole input, beyond the ordinary EOCD window.
        bytes.resize(bytes.len() + 70_000, 0);
        let directory_start = bytes.len();
        bytes.extend(&later[header..end]);
        // An invalid final directory can trigger fallback in ZIP decoders.
        // Bound the earlier directory regardless of which one the decoder selects.
        bytes[directory_start + 10..directory_start + 12].copy_from_slice(&99_u16.to_le_bytes());
        let mut end_record = later[end..].to_vec();
        end_record[16..20].copy_from_slice(&(directory_start as u32).to_le_bytes());
        bytes.extend(end_record);
        assert_rejected(&bytes, "package-size-limit");
    }
}

#[test]
fn non_directory_end_signatures_in_payloads_are_supported() {
    let mut data = vec![0; 22];
    data[..4].copy_from_slice(b"PK\x05\x06");
    data[10..12].copy_from_slice(&(MAX_ENTRY_COUNT as u16 + 1).to_le_bytes());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "file",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(&data).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    assert_eq!(
        read_zip_entries(&bytes).expect("payload signature ZIP")["file"],
        data
    );
}

#[test]
fn stacked_fallback_candidates_preserve_valid_prefixed_archives() {
    for zip64 in [false, true] {
        let mut bytes = Vec::new();
        for _ in 0..2_500 {
            if zip64 {
                append_zip64_candidate(&mut bytes, 44, 0);
                append_zip64_fallback(&mut bytes, 0, 1);
            } else {
                let start = bytes.len();
                bytes.resize(start + 22, 0);
                bytes[start..start + 4].copy_from_slice(b"PK\x05\x06");
                let count = (MAX_ENTRY_COUNT as u16 + 1).to_le_bytes();
                bytes[start + 8..start + 10].copy_from_slice(&count);
                bytes[start + 10..start + 12].copy_from_slice(&count);
            }
        }
        bytes.extend(archive(&[]));
        assert!(
            read_zip_entries(&bytes)
                .expect("stacked prefix ZIP")
                .is_empty()
        );
    }
}

#[test]
fn fallback_zip64_candidate_ranges_preserve_allocation_bounds() {
    for (record_size, declared_offset, disks, rejected) in [
        (39, 0, 1, false),
        (40, 0, 1, true),
        (44, 0, 1, true),
        (45, 0, 1, false),
        (u64::MAX, 0, 1, false),
        (44, 1, 1, false),
        (44, 56, 1, false),
        (44, 0, 2, false),
    ] {
        let mut bytes = Vec::new();
        append_zip64_candidate(&mut bytes, record_size, MAX_ENTRY_COUNT as u64 + 1);
        // Preserve zip-rs's malformed 40-byte case: the candidate's fixed
        // header extends into the locator even though its declared sector fits.
        if record_size == 40 {
            bytes.truncate(52);
        }
        append_zip64_fallback(&mut bytes, declared_offset, disks);
        bytes.extend(archive(&[]));
        if rejected {
            assert_rejected(&bytes, "package-size-limit");
        } else {
            assert!(
                read_zip_entries(&bytes)
                    .expect("irrelevant ZIP64 candidate")
                    .is_empty()
            );
        }
    }

    // The first locator excludes the dangerous candidate; a later locator
    // includes it. A prior query must not discard it from the index.
    let mut bytes = Vec::new();
    append_zip64_candidate(&mut bytes, 44, MAX_ENTRY_COUNT as u64 + 1);
    append_zip64_fallback(&mut bytes, 1, 1);
    append_zip64_fallback(&mut bytes, 0, 1);
    bytes.extend(archive(&[]));
    assert_rejected(&bytes, "package-size-limit");

    // Candidate starts and ends have different orders. The first candidate
    // only fits the second locator, while the second is harmless and fits both.
    let mut bytes = Vec::new();
    append_zip64_candidate(&mut bytes, 142, MAX_ENTRY_COUNT as u64 + 1);
    append_zip64_candidate(&mut bytes, 44, 0);
    append_zip64_fallback(&mut bytes, 0, 1);
    append_zip64_fallback(&mut bytes, 0, 1);
    bytes.extend(archive(&[]));
    assert_rejected(&bytes, "package-size-limit");

    // An ineligible earlier candidate must not hide a later candidate that
    // already fits the first locator.
    let mut bytes = Vec::new();
    append_zip64_candidate(&mut bytes, 142, MAX_ENTRY_COUNT as u64 + 1);
    append_zip64_candidate(&mut bytes, 44, MAX_ENTRY_COUNT as u64 + 1);
    append_zip64_fallback(&mut bytes, 0, 1);
    bytes.extend(archive(&[]));
    assert_rejected(&bytes, "package-size-limit");
}

#[test]
fn sizes_are_enforced_for_maps_metadata_and_actual_decompression() {
    let entries = BTreeMap::from([("large".to_owned(), vec![0; MAX_ENTRY_BYTES as usize + 1])]);
    assert_eq!(crate::check_entries(&entries)[0].code, "package-size-limit");

    let mut bytes = archive(&[("large", b"a")]);
    let header = central_headers(&bytes)[0];
    bytes[header + 24..header + 28].copy_from_slice(&(MAX_ENTRY_BYTES as u32 + 1).to_le_bytes());
    assert_rejected(&bytes, "package-size-limit");

    let mut bytes = archive(&[("large", &entries["large"])]);
    let header = central_headers(&bytes)[0];
    // The true CRC and compressed payload stay valid; only the claimed size lies.
    bytes[header + 24..header + 28].copy_from_slice(&1_u32.to_le_bytes());
    assert_rejected(&bytes, "package-size-limit");

    let mut bytes = archive(&[("small", b"actual")]);
    let header = central_headers(&bytes)[0];
    bytes[header + 24..header + 28].copy_from_slice(&1_u32.to_le_bytes());
    assert_rejected(&bytes, "package-input");
}

#[test]
fn aggregate_bounds_are_checked_before_decompression() {
    let pairs: Vec<_> = (0..9)
        .map(|index| (format!("entry-{index}"), b"a".as_slice()))
        .collect();
    let borrowed: Vec<_> = pairs
        .iter()
        .map(|(name, data)| (name.as_str(), *data))
        .collect();
    let mut bytes = archive(&borrowed);
    for header in central_headers(&bytes) {
        bytes[header + 24..header + 28].copy_from_slice(&(MAX_ENTRY_BYTES as u32).to_le_bytes());
    }
    assert_rejected(&bytes, "package-size-limit");
}

#[test]
fn malformed_archives_are_input_violations_without_panics() {
    let bytes = archive(&[("file", b"contents")]);
    for prefix in 0..bytes.len() {
        assert_rejected(&bytes[..prefix], "package-input");
    }
    let mut bad_crc = bytes.clone();
    let header = central_headers(&bad_crc)[0];
    bad_crc[header + 16..header + 20].copy_from_slice(&0_u32.to_le_bytes());
    assert_rejected(&bad_crc, "package-input");
}

#[test]
fn zip64_packages_and_prefixed_packages_are_supported() {
    let bytes = archive(&[("file", b"contents")]);
    let zip64 = as_zip64(&bytes);
    for archive in [bytes, zip64] {
        assert_eq!(
            read_zip_entries(&archive).expect("valid ZIP")["file"],
            b"contents"
        );
        let mut prefixed = b"archive prefix".to_vec();
        prefixed.extend(&archive);
        assert_eq!(
            read_zip_entries(&prefixed).expect("valid prefixed ZIP")["file"],
            b"contents"
        );
    }
}
