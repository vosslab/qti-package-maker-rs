//! Native reader round-trip receipts for the M13 parity harness.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

use qti_core::{Item, ItemBank, ItemBody};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BLACKBOARD_PRIVATE_METADATA_TAGS: [&str; 4] = [
    "bbmd_qti_package_maker_ma_min_answers_required",
    "bbmd_qti_package_maker_ma_allow_all_correct",
    "bbmd_qti_package_maker_num_tolerance",
    "bbmd_qti_package_maker_num_tolerance_message",
];
const BLACKBOARD_NUMERIC_TOLERANCE_VALUE: &str = ">0.01</bbmd_qti_package_maker_num_tolerance>";
const BLACKBOARD_MALFORMED_NUMERIC_TOLERANCE_VALUE: &str =
    ">not-a-number</bbmd_qti_package_maker_num_tolerance>";

use super::{Divergence, display_error};

struct BlackboardMetadataReceipt<'a> {
    input: &'a Path,
    authored_fixture_sha256: String,
    ordinary_bbq_ma_defaults: &'a Value,
    present_path: &'a Path,
    absent_path: &'a Path,
    malformed_path: &'a Path,
    present_counts: &'a BTreeMap<String, usize>,
    absent_counts: &'a BTreeMap<String, usize>,
    malformed_counts: &'a BTreeMap<String, usize>,
    malformed_numeric_tolerance_replacements: usize,
    authored: &'a Value,
    present: &'a Value,
    pinned_absent: &'a Value,
    native_absent: &'a Value,
    malformed_projection: &'a Value,
    malformed_warnings: &'a Value,
}

/// Compares frozen writer-reader meaning with native writer-native-reader meaning.
pub(super) fn native_reader_roundtrips(
    repository: &Path,
    python_root: &Path,
    fixtures: &Path,
    temporary: &Path,
) -> Result<Vec<Divergence>, String> {
    let bbq_entry = qti_engines::ENGINES
        .iter()
        .find(|entry| entry.name == "bbq_text_upload")
        .expect("fixed registry has BBQ reader");
    let bbq_reader =
        (bbq_entry.make_reader.expect("BBQ reader"))(qti_engines::EngineOptions::default());
    let mut differences = Vec::new();
    for engine in [
        "bbq_text_upload",
        "text2qti",
        "okla_chrst_bqgen",
        "blackboard_export_zip",
    ] {
        let input = if engine == "text2qti" {
            fixtures.join("bbq-parity-text2qti-multiblock-delimiter-repair-questions.txt")
        } else {
            fixtures.join(format!("bbq-parity-{engine}-questions.txt"))
        };
        if engine == "text2qti"
            && sha256_file(&input)?
                != "24bb48081c507d3f98db25e288989206a4d70ec6ac4c05b5ddb644d81b4ce093"
        {
            return Err("text2qti multiblock repair fixture SHA-256 changed".to_owned());
        }
        let bbq_source = bbq_reader
            .read_items(&input, true)
            .map_err(|error| format!("native reader source load {engine}: {error}"))?
            .bank;
        let ordinary_bbq_ma_defaults = (engine == "blackboard_export_zip")
            .then(|| blackboard_ma_defaults(&bbq_source))
            .transpose()?;
        let source = if engine == "blackboard_export_zip" {
            blackboard_private_metadata_bank()?
        } else {
            bbq_source
        };
        let frozen_output = temporary.join(format!("frozen_reader_{engine}"));
        let mut frozen_writer = super::python_command(repository, python_root, "write")?;
        frozen_writer.args([
            "--input",
            &input.to_string_lossy(),
            "--output",
            &frozen_output.to_string_lossy(),
            "--engines",
            engine,
        ]);
        let frozen_result = frozen_writer.output().map_err(display_error)?;
        if !frozen_result.status.success() {
            return Err(format!(
                "frozen reader writer {engine} failed: {}",
                String::from_utf8_lossy(&frozen_result.stderr).trim()
            ));
        }
        let entry = qti_engines::ENGINES
            .iter()
            .find(|entry| entry.name == engine)
            .expect("fixed reader engine is in registry");
        let writer = (entry.make_writer.expect("reader engine writer"))(
            qti_engines::EngineOptions::default(),
        );
        let output = temporary.join(format!(
            "native_reader_{engine}.{}",
            output_extension(engine)
        ));
        let path = writer
            .save_package(&source, Some(&output))
            .map_err(|error| format!("native reader writer {engine}: {error}"))?
            .path
            .ok_or_else(|| format!("native reader writer {engine} produced no output"))?;
        let reader =
            (entry.make_reader.expect("reader factory"))(qti_engines::EngineOptions::default());
        let frozen_path = frozen_output.join(format!("{engine}.{}", output_extension(engine)));
        let mut frozen_reader = super::python_command(repository, python_root, "readback")?;
        frozen_reader.args([
            "--input",
            &frozen_path.to_string_lossy(),
            "--engines",
            engine,
        ]);
        let frozen_readback = frozen_reader.output().map_err(display_error)?;
        if !frozen_readback.status.success() {
            return Err(format!(
                "pinned reader reload {engine} failed: {}",
                String::from_utf8_lossy(&frozen_readback.stderr).trim()
            ));
        }
        let frozen_projection: Value =
            serde_json::from_slice(&frozen_readback.stdout).map_err(|error| {
                format!(
                    "pinned reader {engine} returned invalid JSON: {error}; output: {}",
                    String::from_utf8_lossy(&frozen_readback.stdout).trim()
                )
            })?;
        let restored = reader
            .read_items(&path, true)
            .map_err(|error| format!("native reader reload {engine}: {error}"))?
            .bank;
        let restored_projection = native_reader_projection(&restored);
        if engine == "text2qti" {
            let frozen_expected = text2qti_frozen_multiblock_receipt();
            let authored_projection = native_reader_projection(&source);
            let expected_native = text2qti_native_multiblock_receipt();
            write_text2qti_repair_receipt(
                temporary,
                &input,
                &frozen_projection,
                &authored_projection,
                &restored_projection,
            )?;
            if frozen_projection != frozen_expected {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "text2qti multiblock delimiter source receipt".to_owned(),
                    field: "pinned writer-reader malformed MA sequence".to_owned(),
                    python: format!("pinned writer-reader: {frozen_projection}"),
                    rust: format!("required frozen receipt: {frozen_expected}"),
                });
            }
            if restored_projection != expected_native || authored_projection != expected_native {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "text2qti multiblock delimiter repair".to_owned(),
                    field: "native writer-reader ordered four-item sequence".to_owned(),
                    python: format!("required four-item sequence: {expected_native}"),
                    rust: format!("native writer-reader: {restored_projection}"),
                });
            }
            continue;
        }
        if engine == "blackboard_export_zip" {
            let authored_projection = native_reader_projection(&source);
            let authored_fixture_sha256 = sha256_json(&authored_projection)?;
            let present_counts = blackboard_private_metadata_counts(&path)?;
            require_private_metadata_counts(&present_counts, 1, "present Blackboard ZIP")?;
            let absent_path = temporary.join("blackboard_private_metadata_absent.zip");
            rewrite_blackboard_metadata(&path, &absent_path, remove_private_metadata)?;
            let absent_counts = blackboard_private_metadata_counts(&absent_path)?;
            require_private_metadata_counts(&absent_counts, 0, "stripped Blackboard ZIP")?;
            let absent = reader
                .read_items(&absent_path, true)
                .map_err(|error| format!("Blackboard absent metadata reload: {error}"))?
                .bank;
            let absent_projection = native_reader_projection(&absent);
            let pinned_absent_projection =
                pinned_readback(repository, python_root, &absent_path, engine)?;
            let malformed_path = temporary.join("blackboard_private_metadata_malformed.zip");
            let present_tolerance_values =
                blackboard_metadata_occurrences(&path, BLACKBOARD_NUMERIC_TOLERANCE_VALUE)?;
            if present_tolerance_values != 1 {
                return Err(format!(
                    "present Blackboard ZIP expected one private NUM tolerance value, found {present_tolerance_values}"
                ));
            }
            rewrite_blackboard_metadata(&path, &malformed_path, malform_numeric_tolerance)?;
            let malformed_counts = blackboard_private_metadata_counts(&malformed_path)?;
            require_private_metadata_counts(&malformed_counts, 1, "malformed Blackboard ZIP")?;
            let malformed_tolerance_values = blackboard_metadata_occurrences(
                &malformed_path,
                BLACKBOARD_NUMERIC_TOLERANCE_VALUE,
            )?;
            let malformed_replacements = blackboard_metadata_occurrences(
                &malformed_path,
                BLACKBOARD_MALFORMED_NUMERIC_TOLERANCE_VALUE,
            )?;
            if malformed_tolerance_values != 0 || malformed_replacements != 1 {
                return Err(format!(
                    "malformed Blackboard ZIP expected one NUM tolerance replacement and no original value, found replacements={malformed_replacements}, originals={malformed_tolerance_values}"
                ));
            }
            let malformed = reader
                .read_items(&malformed_path, true)
                .map_err(|error| format!("Blackboard malformed metadata reload: {error}"))?;
            let malformed_projection = native_reader_projection(&malformed.bank);
            let malformed_warnings = Value::Array(
                malformed
                    .warnings
                    .iter()
                    .map(|warning| {
                        json!({
                            "location": format!("{:?}", warning.location),
                            "message": warning.message,
                        })
                    })
                    .collect(),
            );
            let expected_malformed_projection = Value::Array(
                authored_projection
                    .as_array()
                    .expect("authored Blackboard projection is an array")
                    .iter()
                    .filter(|item| item["kind"] != "NUM")
                    .cloned()
                    .collect(),
            );
            write_blackboard_metadata_receipt(
                temporary,
                BlackboardMetadataReceipt {
                    input: &input,
                    authored_fixture_sha256,
                    ordinary_bbq_ma_defaults: ordinary_bbq_ma_defaults
                        .as_ref()
                        .expect("Blackboard source defaults"),
                    present_path: &path,
                    absent_path: &absent_path,
                    malformed_path: &malformed_path,
                    present_counts: &present_counts,
                    absent_counts: &absent_counts,
                    malformed_counts: &malformed_counts,
                    malformed_numeric_tolerance_replacements: present_tolerance_values
                        - malformed_tolerance_values,
                    authored: &authored_projection,
                    present: &restored_projection,
                    pinned_absent: &pinned_absent_projection,
                    native_absent: &absent_projection,
                    malformed_projection: &malformed_projection,
                    malformed_warnings: &malformed_warnings,
                },
            )?;
            if !blackboard_projection_has_ma_options(&authored_projection, 0, false)
                || restored_projection != authored_projection
            {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "Blackboard private metadata present receipt".to_owned(),
                    field: "native writer-reader preserves authored MA and NUM fields".to_owned(),
                    python: format!("authored: {authored_projection}"),
                    rust: format!("native present metadata: {restored_projection}"),
                });
            }
            if absent_projection != pinned_absent_projection
                || !blackboard_projection_has_ma_options(&absent_projection, 1, true)
                || blackboard_projection_has_ma_options(&authored_projection, 1, true)
            {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "Blackboard private metadata absent receipt".to_owned(),
                    field: "stripped native projection exactly equals the pinned projection, with MA (1, true) distinct from authored MA (0, false)".to_owned(),
                    python: format!("pinned absent metadata: {pinned_absent_projection}"),
                    rust: format!("authored: {authored_projection}; native absent metadata: {absent_projection}"),
                });
            }
            if malformed_projection != expected_malformed_projection
                || malformed_warnings
                    != json!([{
                        "location": "PoolItem { resource: \"res00002.dat\", number: 2 }",
                        "message": "skipping malformed item: bbmd_qti_package_maker_num_tolerance metadata is not a finite float",
                    }])
            {
                differences.push(Divergence {
                    engine: engine.to_owned(),
                    item: "Blackboard private metadata malformed receipt".to_owned(),
                    field: "malformed private NUM tolerance skips only the NUM item and records its metadata parse warning".to_owned(),
                    python: "one MA item remains; NUM is skipped with its typed metadata warning".to_owned(),
                    rust: format!("items: {malformed_projection}; warnings: {malformed_warnings}"),
                });
            }
            continue;
        }
        if frozen_projection != restored_projection {
            differences.push(Divergence {
                engine: engine.to_owned(),
                item: "native reader round trip".to_owned(),
                field: "ordered item semantics".to_owned(),
                python: format!("pinned writer-reader: {frozen_projection}"),
                rust: format!("native writer-reader: {restored_projection}"),
            });
        }
    }
    Ok(differences)
}

fn blackboard_private_metadata_bank() -> Result<ItemBank, String> {
    let mut bank = ItemBank::new(true);
    bank.add_item(
        Item::new(
            "Typed Blackboard MA metadata.".to_owned(),
            ItemBody::Ma {
                choices: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                answers: vec!["one".to_owned(), "two".to_owned()],
                min_answers_required: 0,
                allow_all_correct: false,
            },
        )
        .map_err(|error| format!("typed Blackboard MA fixture: {error}"))?,
    )
    .map_err(|error| format!("typed Blackboard MA fixture: {error}"))?;
    bank.add_item(
        Item::new(
            "Typed Blackboard NUM metadata.".to_owned(),
            ItemBody::Num {
                answer: 4.0,
                tolerance: 0.01,
                tolerance_message: true,
            },
        )
        .map_err(|error| format!("typed Blackboard NUM fixture: {error}"))?,
    )
    .map_err(|error| format!("typed Blackboard NUM fixture: {error}"))?;
    Ok(bank)
}

fn blackboard_ma_defaults(bank: &ItemBank) -> Result<Value, String> {
    let (min_answers_required, allow_all_correct) = bank
        .iter_ordered()
        .find_map(|item| match item.body() {
            ItemBody::Ma {
                min_answers_required,
                allow_all_correct,
                ..
            } => Some((*min_answers_required, *allow_all_correct)),
            _ => None,
        })
        .ok_or_else(|| "ordinary BBQ fixture has no MA item".to_owned())?;
    if (min_answers_required, allow_all_correct) != (1, true) {
        return Err(format!(
            "ordinary BBQ MA defaults changed: expected (1, true), found ({min_answers_required}, {allow_all_correct})"
        ));
    }
    Ok(json!({
        "min_answers_required": min_answers_required,
        "allow_all_correct": allow_all_correct,
    }))
}

fn blackboard_projection_has_ma_options(
    projection: &Value,
    min_answers_required: usize,
    allow_all_correct: bool,
) -> bool {
    projection.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item["kind"] == "MA"
                && item["min_answers_required"] == min_answers_required
                && item["allow_all_correct"] == allow_all_correct
        })
    })
}

fn rewrite_blackboard_metadata(
    source: &Path,
    destination: &Path,
    transform: fn(String) -> Result<String, String>,
) -> Result<(), String> {
    let file = std::fs::File::open(source).map_err(display_error)?;
    let mut archive = zip::ZipArchive::new(file).map_err(display_error)?;
    let output = std::fs::File::create(destination).map_err(display_error)?;
    let mut writer = zip::ZipWriter::new(output);
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(display_error)?;
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).map_err(display_error)?;
        if name.ends_with(".dat") {
            let text = String::from_utf8(bytes).map_err(display_error)?;
            if BLACKBOARD_PRIVATE_METADATA_TAGS
                .iter()
                .any(|tag| text.contains(&format!("<{tag}>")))
            {
                bytes = transform(text)?.into_bytes();
            } else {
                bytes = text.into_bytes();
            }
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .map_err(display_error)?;
        writer.write_all(&bytes).map_err(display_error)?;
    }
    writer.finish().map_err(display_error)?;
    Ok(())
}

fn pinned_readback(
    repository: &Path,
    python_root: &Path,
    package: &Path,
    engine: &str,
) -> Result<Value, String> {
    let mut reader = super::python_command(repository, python_root, "readback")?;
    reader.args(["--input", &package.to_string_lossy(), "--engines", engine]);
    let output = reader.output().map_err(display_error)?;
    if !output.status.success() {
        return Err(format!(
            "pinned reader reload {engine} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(display_error)
}

fn blackboard_private_metadata_counts(path: &Path) -> Result<BTreeMap<String, usize>, String> {
    let file = std::fs::File::open(path).map_err(display_error)?;
    let mut archive = zip::ZipArchive::new(file).map_err(display_error)?;
    let mut counts = BLACKBOARD_PRIVATE_METADATA_TAGS
        .into_iter()
        .map(|tag| (tag.to_owned(), 0))
        .collect::<BTreeMap<_, _>>();
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(display_error)?;
        if !member.name().ends_with(".dat") {
            continue;
        }
        let mut text = String::new();
        member.read_to_string(&mut text).map_err(display_error)?;
        for tag in BLACKBOARD_PRIVATE_METADATA_TAGS {
            let opening = format!("<{tag}>");
            let count = counts
                .get_mut(tag)
                .ok_or_else(|| format!("private metadata counter missing for {tag}"))?;
            *count += text.matches(&opening).count();
        }
    }
    Ok(counts)
}

fn blackboard_metadata_occurrences(path: &Path, needle: &str) -> Result<usize, String> {
    let file = std::fs::File::open(path).map_err(display_error)?;
    let mut archive = zip::ZipArchive::new(file).map_err(display_error)?;
    let mut occurrences = 0;
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(display_error)?;
        if !member.name().ends_with(".dat") {
            continue;
        }
        let mut text = String::new();
        member.read_to_string(&mut text).map_err(display_error)?;
        occurrences += text.matches(needle).count();
    }
    Ok(occurrences)
}

fn require_private_metadata_counts(
    counts: &BTreeMap<String, usize>,
    expected: usize,
    description: &str,
) -> Result<(), String> {
    for tag in BLACKBOARD_PRIVATE_METADATA_TAGS {
        let count = counts
            .get(tag)
            .copied()
            .ok_or_else(|| format!("{description} omitted private metadata counter {tag}"))?;
        if count != expected {
            return Err(format!(
                "{description} expected {expected} <{tag}> elements, found {count}"
            ));
        }
    }
    Ok(())
}

fn remove_private_metadata(mut text: String) -> Result<String, String> {
    for tag in BLACKBOARD_PRIVATE_METADATA_TAGS {
        let opening = format!("<{tag}>");
        let closing = format!("</{tag}>");
        let occurrences = text.matches(&opening).count();
        if occurrences != 1 {
            return Err(format!(
                "private metadata <{tag}> must occur exactly once before removal, found {occurrences}"
            ));
        }
        let start = text
            .find(&opening)
            .ok_or_else(|| format!("private metadata <{tag}> disappeared before removal"))?;
        let end = text[start..]
            .find(&closing)
            .map(|offset| start + offset + closing.len())
            .ok_or_else(|| format!("private metadata <{tag}> has no closing element"))?;
        text.replace_range(start..end, "");
    }
    Ok(text)
}

fn malform_numeric_tolerance(text: String) -> Result<String, String> {
    let count = text.matches(BLACKBOARD_NUMERIC_TOLERANCE_VALUE).count();
    if count != 1 {
        return Err(format!(
            "expected exactly one private NUM tolerance value, found {count}"
        ));
    }
    Ok(text.replacen(
        BLACKBOARD_NUMERIC_TOLERANCE_VALUE,
        BLACKBOARD_MALFORMED_NUMERIC_TOLERANCE_VALUE,
        1,
    ))
}

fn write_blackboard_metadata_receipt(
    temporary: &Path,
    receipt: BlackboardMetadataReceipt<'_>,
) -> Result<(), String> {
    let receipt = json!({
        "contract": "blackboard_export_zip_private_metadata_roundtrip",
        "pinned_python_commit": super::PINNED_PYTHON_HEAD,
        "fixture_input_sha256": sha256_file(receipt.input)?,
        "authored_typed_fixture_projection_sha256": receipt.authored_fixture_sha256,
        "ordinary_bbq_source_ma_defaults": receipt.ordinary_bbq_ma_defaults,
        "present_zip_sha256": sha256_file(receipt.present_path)?,
        "absent_zip_sha256": sha256_file(receipt.absent_path)?,
        "malformed_zip_sha256": sha256_file(receipt.malformed_path)?,
        "present_private_metadata_counts": receipt.present_counts,
        "absent_private_metadata_counts": receipt.absent_counts,
        "malformed_private_metadata_counts": receipt.malformed_counts,
        "malformed_numeric_tolerance_replacements": receipt.malformed_numeric_tolerance_replacements,
        "removed_private_metadata_names": BLACKBOARD_PRIVATE_METADATA_TAGS,
        "scope": "application-private provenance metadata; this receipt does not claim LMS grading behavior",
        "authored": receipt.authored,
        "native_present_metadata": receipt.present,
        "pinned_absent_metadata": receipt.pinned_absent,
        "native_absent_metadata": receipt.native_absent,
        "malformed_native_projection": receipt.malformed_projection,
        "malformed_native_warnings": receipt.malformed_warnings,
    });
    let text = serde_json::to_string_pretty(&receipt).map_err(display_error)?;
    std::fs::write(
        temporary.join("blackboard_private_metadata_receipt.json"),
        text,
    )
    .map_err(display_error)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(display_error)?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256_json(value: &Value) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(display_error)
}

fn write_text2qti_repair_receipt(
    temporary: &Path,
    input: &Path,
    frozen_projection: &Value,
    authored_projection: &Value,
    restored_projection: &Value,
) -> Result<(), String> {
    let input_bytes = std::fs::read(input).map_err(display_error)?;
    let receipt = json!({
        "fixture": input.file_name().and_then(|name| name.to_str()).unwrap_or(""),
        "input_sha256": sha256_bytes(&input_bytes),
        "frozen_writer_reader": frozen_projection,
        "native_authored_sequence": authored_projection,
        "native_writer_reader": restored_projection,
    });
    let path = temporary.join("text2qti_multiblock_delimiter_repair.json");
    let text = serde_json::to_string_pretty(&receipt).map_err(display_error)?;
    std::fs::write(path, text).map_err(display_error)
}

fn text2qti_frozen_multiblock_receipt() -> Value {
    json!([{
        "kind": "MA",
        "question": "Which base pairs with A? *A) T B) C 2. Choose all base pairs.",
        "choices_list": [
            "A",
            "C",
            "U 3. How many chromatids follow replication? = 4.0 +- 0.01 4. The hereditary material is ____. * DNA",
        ],
        "answers_list": ["A", "C"],
        "min_answers_required": 1,
        "allow_all_correct": true,
    }])
}

fn text2qti_native_multiblock_receipt() -> Value {
    json!([
        {"kind":"MC","question":"Which base pairs with A?","choices_list":["T","C"],"answer_text":"T"},
        {"kind":"MA","question":"Choose all base pairs.","choices_list":["A","C","U"],"answers_list":["A","C"],"min_answers_required":1,"allow_all_correct":true},
        {"kind":"NUM","question":"How many chromatids follow replication?","answer_float":4.0,"tolerance_float":0.01,"tolerance_message":true},
        {"kind":"FIB","question":"The hereditary material is ____.","answers_list":["DNA"]},
    ])
}

fn native_reader_projection(bank: &ItemBank) -> Value {
    let items = bank
        .iter_ordered()
        .map(native_item_projection)
        .collect::<Vec<_>>();
    Value::Array(items)
}

fn native_item_projection(item: &Item) -> Value {
    let question = collapsed(item.common().question_text.as_str());
    match item.body() {
        ItemBody::Mc { choices, answer } => json!({
            "kind": "MC", "question": question,
            "choices_list": collapsed_list(choices), "answer_text": collapsed(answer),
        }),
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => json!({
            "kind": "MA", "question": question,
            "choices_list": collapsed_list(choices), "answers_list": collapsed_list(answers),
            "min_answers_required": min_answers_required, "allow_all_correct": allow_all_correct,
        }),
        ItemBody::Match { prompts, choices } => json!({
            "kind": "MATCH", "question": question,
            "prompts_list": collapsed_list(prompts), "choices_list": collapsed_list(choices),
        }),
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => json!({
            "kind": "NUM", "question": question,
            "answer_float": answer, "tolerance_float": tolerance,
            "tolerance_message": tolerance_message,
        }),
        ItemBody::Fib { answers } => json!({
            "kind": "FIB", "question": question, "answers_list": collapsed_list(answers),
        }),
        ItemBody::MultiFib { answers } => json!({
            "kind": "MULTI_FIB", "question": question,
            "answer_map": answers.iter().map(|(key, values)| {
                (key, collapsed_list(values))
            }).collect::<std::collections::BTreeMap<_, _>>(),
        }),
        ItemBody::Order { answers } => json!({
            "kind": "ORDER", "question": question,
            "ordered_answers_list": collapsed_list(answers),
        }),
    }
}

fn collapsed_list(values: &[String]) -> Vec<String> {
    values.iter().map(|value| collapsed(value)).collect()
}

fn collapsed(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn output_extension(engine: &str) -> &'static str {
    match engine {
        "blackboard_export_zip" => "zip",
        _ => "txt",
    }
}
