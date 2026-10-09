//! Canvas-compatible QTI 1.2 ZIP package writer.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qti_core::media::{
    AssetKind, AssetSource, MediaPolicy, apply_media_policy, packageable_assets, rewrite_item_media,
};
use qti_core::{
    EntryMap, ItemBody, ItemKind, ItemRenderView, ItemResource, ManifestConfig, NamedFile,
    QtiVersion, encode_zip, generate_manifest,
};

use crate::{
    EngineError, RenderHooks, WriteArtifact, WriteContext, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "canvas_qti_v1_2";
const ITEM_PATH: &str = "canvas_qti12_questions/canvas_qti12_questions.xml";
const MEDIA_DIRECTORY: &str = "media";
const CDATA_SPLIT: &str = concat!("]]", "]]><![CDATA[>");
const SCORE_OUTCOME: &str = "<outcomes><decvar maxvalue=\"100\" minvalue=\"0\" varname=\"SCORE\" vartype=\"Decimal\"/></outcomes>";

/// Creates the QTI writer.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(CanvasWriter)
}

struct CanvasWriter;
impl Writer for CanvasWriter {
    fn name(&self) -> &'static str {
        NAME
    }
    fn media_policy(&self) -> MediaPolicy {
        crate::engine(NAME)
            .expect("registered Canvas writer")
            .media_policy
    }
    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(NAME)
            .expect("registered Canvas writer")
            .supported_kinds
    }

    fn write_package(
        &self,
        bank: &qti_core::ItemBank,
        assets: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        // Unsupported items cannot contribute package media or require source reads.
        let mut media_bank = qti_core::ItemBank::new(true);
        for item in bank
            .iter_ordered()
            .filter(|item| self.supported_kinds().contains(&item.kind()))
        {
            media_bank.add_item(item.clone())?;
        }
        let collected = media_bank.collect_assets(assets)?;
        let mut assets = collected.assets().to_vec();
        for asset in &mut assets {
            if asset.kind == AssetKind::Local {
                let leaf = asset
                    .output_name
                    .clone()
                    .ok_or_else(|| EngineError::InvalidFormat {
                        engine: NAME,
                        format: "media",
                        message: format!("asset '{}' has no output name", asset.src),
                    })?;
                asset.output_name = Some(format!("{MEDIA_DIRECTORY}/{leaf}"));
            }
        }
        let source_names = assets
            .iter()
            .filter_map(|asset| {
                asset
                    .output_name
                    .as_ref()
                    .map(|name| (asset.src.as_str(), name.as_str()))
            })
            .collect::<BTreeMap<_, _>>();
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &qti_core::Item| {
            let dependencies = collected.dependencies_for(item.crc()).unwrap_or_default();
            let decision = apply_media_policy(
                MediaPolicy::Package,
                dependencies,
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: error.to_string(),
            })?;
            warnings.borrow_mut().extend(decision.warnings);
            rewrite_item_media(item, |src| {
                source_names
                    .get(src)
                    .map_or_else(|| src.to_owned(), |name| format!("../{name}"))
            })
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "HTML",
                message: error.to_string(),
            })
        };
        let items = render_bank(
            bank,
            self.supported_kinds(),
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;
        // Canvas reports zero saved assessment items for an ORDER-only bank and
        // does not create a package.  Keep that observable CLI outcome rather
        // than manufacturing an empty ZIP from the requested output path.
        if items.is_empty() {
            return Ok(WriteOutcome {
                artifact: None,
                warnings: warnings.into_inner(),
            });
        }
        let document = qti_document(&items);
        let mut map = EntryMap::new();
        map.insert(ITEM_PATH.to_owned(), document.into_bytes());
        map.insert(
            "canvas_qti12_questions/assessment_meta.xml".to_owned(),
            assessment_meta().into_bytes(),
        );
        let packaged_assets = assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Local)
            .cloned()
            .collect::<Vec<_>>();
        let local_assets =
            packageable_assets(&packaged_assets).map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: error.to_string(),
            })?;
        for asset in local_assets {
            map.insert(
                asset
                    .output_name
                    .clone()
                    .expect("packageable media has output name"),
                asset
                    .read_bytes()
                    .map_err(|error| EngineError::InvalidFormat {
                        engine: NAME,
                        format: "media",
                        message: error.to_string(),
                    })?,
            );
        }
        let dependency_indexes = (0..packaged_assets.len()).collect();
        let manifest = generate_manifest(&ManifestConfig {
            package_name: "Canvas QTI 1.2".to_owned(),
            version: QtiVersion::V1p2,
            items: vec![ItemResource {
                href: ITEM_PATH.to_owned(),
                asset_indexes: dependency_indexes,
            }],
            assets: &packaged_assets,
            metadata_date: context.document.date.clone(),
        })?;
        map.insert("imsmanifest.xml".to_owned(), manifest);
        let bytes = encode_zip(&map, std::iter::empty::<&str>())?;
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary: NamedFile::new(context.output_name().to_owned(), bytes)?,
                companions: Vec::new(),
            }),
            warnings: warnings.into_inner(),
        })
    }
}

fn qti_document(items: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<questestinterop><assessment ident=\"root_assessment\" title=\"Canvas QTI 1.2\"><section ident=\"root_section\">{}</section></assessment></questestinterop>\n",
        items.join("\n")
    )
}

fn assessment_meta() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<quiz xmlns=\"http://canvas.instructure.com/xsd/cccv1p0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://canvas.instructure.com/xsd/cccv1p0 https://canvas.instructure.com/xsd/cccv1p0.xsd\" identifier=\"assessment_meta\"><title>Canvas QTI 1.2</title><assignment identifier=\"assignment_name\"><title>Canvas QTI 1.2</title></assignment></quiz>\n".to_owned()
}

fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let kind = match item.kind() {
        ItemKind::Mc => "multiple_choice",
        ItemKind::Ma => "multiple_answer",
        ItemKind::Match => "matching",
        ItemKind::Num => "numeric",
        ItemKind::Fib => "fib",
        ItemKind::MultiFib => "multi_fib",
        ItemKind::Order => {
            return Err(EngineError::UnsupportedItemKind {
                engine: NAME,
                kind: ItemKind::Order,
            });
        }
    };
    let metadata = format!(
        "<itemmetadata><qtimetadata><qtimetadatafield><fieldlabel>question_type</fieldlabel><fieldentry>{kind}_question</fieldentry></qtimetadatafield></qtimetadata></itemmetadata>"
    );
    let presentation = presentation(item)?;
    let processing = processing(item)?;
    Ok(Some(format!(
        "<item ident=\"{}_{}\" title=\"{}\">{metadata}{presentation}{processing}</item>",
        kind,
        item.common().item_number,
        item.crc()
    )))
}

fn presentation(item: &ItemRenderView) -> Result<String, EngineError> {
    let stem = material(&item.common().question_text);
    let value = match item.body() {
        ItemBody::Mc { choices, .. } | ItemBody::Ma { choices, .. } => format!("{stem}<response_lid ident=\"response1\" rcardinality=\"{}\"><render_choice>{}</render_choice></response_lid>", if matches!(item.body(), ItemBody::Mc { .. }) { "Single" } else { "Multiple" }, choices.iter().enumerate().map(|(index, choice)| format!("<response_label ident=\"choice_{:03}\">{}</response_label>", index + 1, material(choice))).collect::<String>()),
        ItemBody::Match { prompts, choices } => format!("{stem}{}", prompts.iter().enumerate().map(|(index, prompt)| format!("<response_lid ident=\"response_{:03}\" rcardinality=\"Single\">{}<render_choice>{}</render_choice></response_lid>", index + 1, material(prompt), choices.iter().enumerate().map(|(choice_index, choice)| format!("<response_label ident=\"choice_{:03}\">{}</response_label>", choice_index + 1, material(choice))).collect::<String>())).collect::<String>()),
        ItemBody::Num { .. } => format!(
            "{stem}<response_str ident=\"response1\" rcardinality=\"Single\"><render_fib fibtype=\"Decimal\"><response_label ident=\"answer1\" rshuffle=\"No\"/></render_fib></response_str>"
        ),
        ItemBody::Fib { .. } => format!(
            "{stem}<response_str ident=\"response1\" rcardinality=\"Single\"><render_fib fibtype=\"String\"><response_label ident=\"answer1\" rshuffle=\"No\"/></render_fib></response_str>"
        ),
        ItemBody::MultiFib { answers } => {
            let responses = answers
                .iter()
                .enumerate()
                .map(|(blank_index, (blank, values))| {
                    let response_id = format!("response_{}", blank_index + 1);
                    let choices = values
                        .iter()
                        .enumerate()
                        .map(|(answer_index, answer)| {
                            format!(
                                "<response_label ident=\"{response_id}_choice_{:03}\"><material><mattext texttype=\"text/plain\">{}</mattext></material></response_label>",
                                answer_index + 1,
                                xml(answer)
                            )
                        })
                        .collect::<String>();
                    format!(
                        "<response_lid ident=\"{response_id}\"><material><mattext>{}</mattext></material><render_choice>{choices}</render_choice></response_lid>",
                        xml(blank)
                    )
                })
                .collect::<String>();
            format!("{stem}{responses}")
        }
        ItemBody::Order { .. } => return Err(EngineError::UnsupportedItemKind { engine: NAME, kind: ItemKind::Order }),
    };
    Ok(format!("<presentation>{value}</presentation>"))
}

fn processing(item: &ItemRenderView) -> Result<String, EngineError> {
    let body = match item.body() {
        ItemBody::Mc { choices, answer } => {
            let index = choices
                .iter()
                .position(|choice| choice == answer)
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "QTI 1.2",
                    message: "MC answer is absent from choices".to_owned(),
                })?;
            response("response1", &format!("choice_{:03}", index + 1))
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            let conditions = choices
                .iter()
                .enumerate()
                .map(|(index, choice)| {
                    let value = format!("choice_{:03}", index + 1);
                    if answers.contains(choice) {
                        format!("<varequal respident=\"response1\">{value}</varequal>")
                    } else {
                        format!("<not><varequal respident=\"response1\">{value}</varequal></not>")
                    }
                })
                .collect::<String>();
            format!(
                "<respcondition><conditionvar><and>{conditions}</and></conditionvar><setvar varname=\"SCORE\" action=\"Set\">100</setvar></respcondition>"
            )
        }
        ItemBody::Match { prompts, .. } => {
            let score = 100.0 / prompts.len() as f64;
            prompts
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let response_id = format!("response_{:03}", index + 1);
                    let choice_id = format!("choice_{:03}", index + 1);
                    format!(
                        "<respcondition><conditionvar><varequal respident=\"{response_id}\">{choice_id}</varequal></conditionvar><setvar varname=\"SCORE\" action=\"Add\">{score:.2}</setvar></respcondition>"
                    )
                })
                .collect()
        }
        ItemBody::Num {
            answer, tolerance, ..
        } => {
            let answer_text = qti_number(*answer);
            let lower = qti_number(*answer - tolerance);
            let upper = qti_number(*answer + tolerance);
            format!(
                "<respcondition continue=\"No\"><conditionvar><or><varequal respident=\"response1\">{answer_text}</varequal><and><vargte respident=\"response1\">{lower}</vargte><varlte respident=\"response1\">{upper}</varlte></and></or></conditionvar><setvar action=\"Set\" varname=\"SCORE\">100</setvar></respcondition>"
            )
        }
        ItemBody::Fib { answers } => format!(
            "<respcondition continue=\"No\"><conditionvar>{}</conditionvar><setvar action=\"Set\" varname=\"SCORE\">100</setvar></respcondition>",
            answers
                .iter()
                .map(|answer| format!(
                    "<varequal respident=\"response1\">{}</varequal>",
                    xml(answer)
                ))
                .collect::<String>()
        ),
        ItemBody::MultiFib { answers } => {
            let score = 100.0 / answers.len() as f64;
            answers
                .iter()
                .enumerate()
                .map(|(blank_index, (_, values))| {
                    let response_id = format!("response_{}", blank_index + 1);
                    let values = values
                        .iter()
                        .enumerate()
                        .map(|(answer_index, _)| {
                            format!(
                                "<varequal respident=\"{response_id}\">{response_id}_choice_{:03}</varequal>",
                                answer_index + 1
                            )
                        })
                        .collect::<Vec<_>>();
                    let condition = if values.len() == 1 {
                        values.into_iter().next().expect("one response label")
                    } else {
                        format!("<or>{}</or>", values.concat())
                    };
                    format!(
                        "<respcondition><conditionvar>{condition}</conditionvar><setvar varname=\"SCORE\" action=\"Add\">{score:.2}</setvar></respcondition>"
                    )
                })
                .collect()
        }
        ItemBody::Order { .. } => {
            return Err(EngineError::UnsupportedItemKind {
                engine: NAME,
                kind: ItemKind::Order,
            });
        }
    };
    Ok(format!(
        "<resprocessing>{SCORE_OUTCOME}{body}</resprocessing>"
    ))
}
fn response(identifier: &str, value: &str) -> String {
    format!(
        "<respcondition><conditionvar><varequal respident=\"{identifier}\">{}</varequal></conditionvar><setvar action=\"Set\" varname=\"SCORE\">100</setvar></respcondition>",
        xml(value)
    )
}

fn qti_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}
fn material(value: &str) -> String {
    // Preserve authored HTML as markup while preventing an author-controlled CDATA terminator
    // from closing `mattext` and altering package structure.
    let cdata = xml_characters(value).replace("]]>", CDATA_SPLIT);
    format!("<material><mattext texttype=\"text/html\"><![CDATA[{cdata}]]></mattext></material>")
}

fn xml_characters(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            matches!(character, '\t' | '\n' | '\r')
                || ('\u{20}'..='\u{D7FF}').contains(character)
                || ('\u{E000}'..='\u{FFFD}').contains(character)
                || ('\u{10000}'..='\u{10FFFF}').contains(character)
        })
        .collect()
}

fn xml(value: &str) -> String {
    xml_characters(value)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{SCORE_OUTCOME, boxed_writer, material, presentation, processing};
    use crate::{DocumentMetadata, WriteArtifact, WriteContext, WriteOutcome};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use qti_core::media::MemoryAssets;
    use qti_core::{Item, ItemBank, ItemBody};
    use quick_xml::{Reader, events::Event};
    use std::collections::BTreeMap;
    use std::io::Read;

    fn context(name: &str) -> WriteContext {
        WriteContext::new(
            name.to_owned(),
            DocumentMetadata {
                title: "Canvas QTI 1.2".to_owned(),
                date: "2026-09-30".to_owned(),
            },
            0,
        )
        .expect("context")
    }

    fn primary(outcome: &WriteOutcome) -> &qti_core::NamedFile {
        match outcome.artifact.as_ref().expect("package artifact") {
            WriteArtifact::File {
                primary,
                companions,
            } => {
                assert!(companions.is_empty());
                primary
            }
            WriteArtifact::Directory { .. } => panic!("ZIP must be a file"),
        }
    }

    fn all_or_nothing_score(correct: &[&str], selected: &[&str]) -> u8 {
        let correct = correct
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let selected = selected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        u8::from(correct == selected) * 100
    }

    fn numeric_condition_accepts(input: &str, answer: f64, tolerance: f64) -> bool {
        input.parse::<f64>().is_ok_and(|value| {
            value == answer || (value >= answer - tolerance && value <= answer + tolerance)
        })
    }

    fn fib_condition_accepts(input: &str, answers: &[&str]) -> bool {
        answers.contains(&input)
    }

    fn assert_well_formed_xml(document: &str) {
        let mut reader = Reader::from_reader(document.as_bytes());
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(error) => panic!("malformed XML: {error}; document: {document}"),
            }
            buffer.clear();
        }
    }

    #[test]
    fn package_covers_six_kinds_skips_order_and_handles_local_and_external_media() {
        let assets = MemoryAssets::from_entries(BTreeMap::from([(
            "local.png".to_owned(),
            STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAF/gL+X9VR/QAAAABJRU5ErkJggg==").expect("inline PNG"),
        )])).expect("memory assets");
        let mut bank = ItemBank::new(true);
        let mut multi = BTreeMap::new();
        multi.insert("blank".to_owned(), vec!["value".to_owned()]);
        let items = vec![
            Item::new(
                "MULTIPLE <img src=\"local.png\" />".into(),
                ItemBody::Mc {
                    choices: vec!["first".into(), "second".into(), "third".into()],
                    answer: "second".into(),
                },
            ),
            Item::new(
                "MULTI ANSWER".into(),
                ItemBody::Ma {
                    choices: vec!["first".into(), "second".into(), "third".into()],
                    answers: vec!["first".into()],
                    min_answers_required: 0,
                    allow_all_correct: false,
                },
            ),
            Item::new(
                "MATCH".into(),
                ItemBody::Match {
                    prompts: vec!["prompt1".into(), "prompt2".into()],
                    choices: vec!["choice1".into(), "choice2".into()],
                },
            ),
            Item::new(
                "NUM".into(),
                ItemBody::Num {
                    answer: 2.0,
                    tolerance: 0.1,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "FIB <img src=\"https://example.test/external.png\" />".into(),
                ItemBody::Fib {
                    answers: vec!["value".into()],
                },
            ),
            Item::new(
                "MULTI [blank]".into(),
                ItemBody::MultiFib { answers: multi },
            ),
            Item::new(
                "ORDER SHOULD NOT RENDER".into(),
                ItemBody::Order {
                    answers: vec!["one".into(), "two".into(), "three".into()],
                },
            ),
        ];
        for item in items {
            bank.add_item(item.expect("valid item")).expect("bank item");
        }
        let outcome = boxed_writer()
            .write_package(&bank, &assets, &context("canvas.zip"))
            .expect("package");
        assert_eq!(primary(&outcome).name(), "canvas.zip");
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].src, "https://example.test/external.png");
        let violations = qti_integrity::check_package(primary(&outcome).bytes());
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "integrity violations: {violations:#?}"
        );
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(primary(&outcome).bytes())).expect("ZIP");
        assert!(archive.file_names().any(|name| name == "media/local.png"));
        let mut media_bytes = Vec::new();
        archive
            .by_name("media/local.png")
            .expect("packaged image")
            .read_to_end(&mut media_bytes)
            .expect("image bytes");
        assert_eq!(
            media_bytes.as_slice(),
            assets.get("local.png").expect("source image")
        );
        let mut item_xml = String::new();
        archive
            .by_name("canvas_qti12_questions/canvas_qti12_questions.xml")
            .expect("item XML")
            .read_to_string(&mut item_xml)
            .expect("XML text");
        assert_eq!(item_xml.matches("<item ").count(), 6);
        assert_eq!(item_xml.matches(SCORE_OUTCOME).count(), 6);
        assert!(
            item_xml.split("<setvar").skip(1).all(|fragment| fragment
                .split('>')
                .next()
                .is_some_and(|tag| tag.contains("varname=\"SCORE\""))),
            "every response processor must target Canvas's declared SCORE outcome: {item_xml}"
        );
        assert!(item_xml.contains("../media/local.png"));
        assert!(item_xml.contains("https://example.test/external.png"));
        assert!(!item_xml.contains("ORDER SHOULD NOT RENDER"));
    }

    #[test]
    fn uses_explicit_output_name_and_manifest_date_with_existing_format_title() {
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "Choose.".to_owned(),
                ItemBody::Mc {
                    choices: vec!["first".to_owned(), "second".to_owned()],
                    answer: "first".to_owned(),
                },
            )
            .expect("item"),
        )
        .expect("bank item");
        let context = WriteContext::new(
            "custom/canvas.payload".to_owned(),
            DocumentMetadata {
                title: "Caller document".to_owned(),
                date: "2040-01-02".to_owned(),
            },
            42,
        )
        .expect("context");
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::default(), &context)
            .expect("package");
        assert_eq!(primary(&outcome).name(), "custom/canvas.payload");
        let entries =
            qti_integrity::read_zip_entries(primary(&outcome).bytes()).expect("ZIP entries");
        let manifest = std::str::from_utf8(&entries["imsmanifest.xml"]).expect("manifest text");
        assert!(manifest.contains("2040-01-02"));
        assert!(manifest.contains("Canvas QTI 1.2"));
        assert!(!manifest.contains("Caller document"));
    }

    #[test]
    fn mc_uses_qti_default_continue_and_declared_score_outcome() {
        let item = Item::new(
            "Choose ATP.".to_owned(),
            ItemBody::Mc {
                choices: vec!["ADP".to_owned(), "ATP".to_owned()],
                answer: "ATP".to_owned(),
            },
        )
        .expect("MC item");

        let processing = processing(&item.render_view()).expect("MC processing");
        assert!(processing.starts_with(
            "<resprocessing><outcomes><decvar maxvalue=\"100\" minvalue=\"0\" varname=\"SCORE\" vartype=\"Decimal\"/></outcomes><respcondition><conditionvar><varequal respident=\"response1\">choice_002</varequal>"
        ));
        assert!(!processing.contains("<respcondition continue=\"No\">"));
        assert!(processing.contains("<setvar action=\"Set\" varname=\"SCORE\">100</setvar>"));
    }

    #[test]
    fn removes_invalid_xml_control_characters_from_serialized_material() {
        let material = material("A\0B\u{1}C\tD\nE");
        assert_eq!(
            material,
            "<material><mattext texttype=\"text/html\"><![CDATA[ABC\tD\nE]]></mattext></material>"
        );
        assert_well_formed_xml(&format!("<root>{material}</root>"));
    }

    #[test]
    fn order_only_completes_without_creating_a_canvas_package() {
        let mut bank = ItemBank::new(false);
        bank.add_item(
            Item::new(
                "ORDER <img src=\"missing.png\" />".into(),
                ItemBody::Order {
                    answers: vec!["one".into(), "two".into(), "three".into()],
                },
            )
            .expect("ORDER item"),
        )
        .expect("bank item");
        let output = "order-only.zip";
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::default(), &context(output))
            .expect("ORDER omission");
        assert_eq!(outcome.artifact, None);
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn multi_fib_uses_sorted_choice_lids_and_scores_declared_labels() {
        let mut answers = BTreeMap::new();
        answers.insert("zeta".to_owned(), vec!["z-one".to_owned()]);
        answers.insert(
            "alpha".to_owned(),
            vec!["a-first".to_owned(), "a-second".to_owned()],
        );
        let item = Item::new(
            "Complete [alpha] then [zeta].".to_owned(),
            ItemBody::MultiFib { answers },
        )
        .expect("MULTI_FIB item");
        let view = item.render_view();
        let presentation = presentation(&view).expect("presentation");
        let processing = processing(&view).expect("processing");

        assert!(presentation.contains(
            "<response_lid ident=\"response_1\"><material><mattext>alpha</mattext></material><render_choice>"
        ));
        assert!(presentation.contains(
            "response_1_choice_001\"><material><mattext texttype=\"text/plain\">a-first"
        ));
        assert!(presentation.contains(
            "response_1_choice_002\"><material><mattext texttype=\"text/plain\">a-second"
        ));
        assert!(presentation.contains(
            "<response_lid ident=\"response_2\"><material><mattext>zeta</mattext></material>"
        ));
        assert!(presentation.contains("response_2_choice_001"));
        assert!(!presentation.contains("response_str"));

        assert!(processing.contains(
            "<or><varequal respident=\"response_1\">response_1_choice_001</varequal><varequal respident=\"response_1\">response_1_choice_002</varequal></or>"
        ));
        assert!(
            processing
                .contains("<varequal respident=\"response_2\">response_2_choice_001</varequal>")
        );
        assert_eq!(
            processing
                .matches("<setvar varname=\"SCORE\" action=\"Add\">50.00</setvar>")
                .count(),
            2
        );
        assert!(!processing.contains(">a-first</varequal>"));
        assert!(!processing.contains(">z-one</varequal>"));
    }

    #[test]
    fn match_uses_labeled_pairs_for_partial_credit_and_produces_an_integral_package() {
        let item = Item::new(
            "Match each molecule to its role.".to_owned(),
            ItemBody::Match {
                prompts: vec!["ATP".to_owned(), "DNA".to_owned()],
                choices: vec!["energy carrier".to_owned(), "genetic material".to_owned()],
            },
        )
        .expect("MATCH item");
        let view = item.render_view();
        let presentation = presentation(&view).expect("presentation");
        let processing = processing(&view).expect("processing");

        assert!(presentation.contains(
            "<response_lid ident=\"response_001\" rcardinality=\"Single\"><material><mattext texttype=\"text/html\"><![CDATA[ATP]]></mattext></material><render_choice><response_label ident=\"choice_001\""
        ));
        assert!(presentation.contains(
            "<response_lid ident=\"response_002\" rcardinality=\"Single\"><material><mattext texttype=\"text/html\"><![CDATA[DNA]]></mattext></material><render_choice><response_label ident=\"choice_001\""
        ));
        assert!(processing.contains("<varequal respident=\"response_001\">choice_001</varequal>"));
        assert!(processing.contains("<varequal respident=\"response_002\">choice_002</varequal>"));
        assert_eq!(
            processing
                .matches("<setvar varname=\"SCORE\" action=\"Add\">50.00</setvar>")
                .count(),
            2
        );

        let output = "match.zip";
        let mut bank = ItemBank::new(false);
        bank.add_item(item).expect("bank item");
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::default(), &context(output))
            .expect("MATCH package");
        assert_eq!(primary(&outcome).name(), output);
        let violations = qti_integrity::check_package(primary(&outcome).bytes());
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "integrity violations: {violations:#?}"
        );
    }

    #[test]
    fn ma_requires_all_and_only_the_authored_correct_choices() {
        let item = Item::new(
            "Select both products.".to_owned(),
            ItemBody::Ma {
                choices: vec!["ATP".to_owned(), "ADP".to_owned(), "NADH".to_owned()],
                answers: vec!["ATP".to_owned(), "NADH".to_owned()],
                min_answers_required: 0,
                allow_all_correct: false,
            },
        )
        .expect("MA item");
        let presentation = presentation(&item.render_view()).expect("presentation");
        let processing = processing(&item.render_view()).expect("processing");

        assert!(presentation.contains("<response_label ident=\"choice_001\""));
        assert!(presentation.contains("<response_label ident=\"choice_002\""));
        assert!(presentation.contains("<response_label ident=\"choice_003\""));
        assert!(processing.contains(
            "<and><varequal respident=\"response1\">choice_001</varequal><not><varequal respident=\"response1\">choice_002</varequal></not><varequal respident=\"response1\">choice_003</varequal></and>"
        ));
        assert!(processing.contains("<setvar varname=\"SCORE\" action=\"Set\">100</setvar>"));
        assert_eq!(
            all_or_nothing_score(&["choice_001", "choice_003"], &["choice_001", "choice_003"]),
            100,
            "the authored correct set earns full credit"
        );
        assert_eq!(
            all_or_nothing_score(&["choice_001", "choice_003"], &["choice_001"]),
            0,
            "a partial selection fails the required <and>"
        );
        assert_eq!(
            all_or_nothing_score(
                &["choice_001", "choice_003"],
                &["choice_001", "choice_002", "choice_003"],
            ),
            0,
            "an unselected-choice violation fails the <not> condition"
        );
        assert_eq!(
            all_or_nothing_score(&["choice_001", "choice_003"], &[]),
            0,
            "an empty selection fails the required correct-choice conditions"
        );

        let output = "ma.zip";
        let mut bank = ItemBank::new(false);
        bank.add_item(item).expect("bank item");
        let outcome = boxed_writer()
            .write_package(&bank, &MemoryAssets::default(), &context(output))
            .expect("MA package");
        let violations = qti_integrity::check_package(primary(&outcome).bytes());
        assert!(
            violations
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "integrity violations: {violations:#?}"
        );
    }

    #[test]
    fn num_and_fib_present_and_grade_the_frozen_predicates() {
        let numeric = Item::new(
            "How many ATP?".to_owned(),
            ItemBody::Num {
                answer: 5.0,
                tolerance: 0.5,
                tolerance_message: true,
            },
        )
        .expect("NUM item");
        let numeric_presentation = presentation(&numeric.render_view()).expect("NUM presentation");
        let numeric_processing = processing(&numeric.render_view()).expect("NUM processing");
        assert!(numeric_presentation.contains(
            "<response_str ident=\"response1\" rcardinality=\"Single\"><render_fib fibtype=\"Decimal\"><response_label ident=\"answer1\" rshuffle=\"No\"/></render_fib></response_str>"
        ));
        assert!(numeric_processing.contains(
            "<or><varequal respident=\"response1\">5.0</varequal><and><vargte respident=\"response1\">4.5</vargte><varlte respident=\"response1\">5.5</varlte></and></or>"
        ));
        assert!(numeric_condition_accepts("5", 5.0, 0.5), "center");
        assert!(numeric_condition_accepts("4.5", 5.0, 0.5), "lower bound");
        assert!(numeric_condition_accepts("5.5", 5.0, 0.5), "upper bound");
        assert!(
            !numeric_condition_accepts("4.49", 5.0, 0.5),
            "outside range"
        );
        assert!(!numeric_condition_accepts("five", 5.0, 0.5), "non-number");

        let fib = Item::new(
            "The molecule is ____.".to_owned(),
            ItemBody::Fib {
                answers: vec!["ATP".to_owned(), "adenosine triphosphate".to_owned()],
            },
        )
        .expect("FIB item");
        let fib_presentation = presentation(&fib.render_view()).expect("FIB presentation");
        let fib_processing = processing(&fib.render_view()).expect("FIB processing");
        assert!(fib_presentation.contains(
            "<response_str ident=\"response1\" rcardinality=\"Single\"><render_fib fibtype=\"String\"><response_label ident=\"answer1\" rshuffle=\"No\"/></render_fib></response_str>"
        ));
        assert!(fib_processing.contains(
            "<conditionvar><varequal respident=\"response1\">ATP</varequal><varequal respident=\"response1\">adenosine triphosphate</varequal></conditionvar><setvar action=\"Set\" varname=\"SCORE\">100</setvar>"
        ));
        assert!(fib_condition_accepts(
            "ATP",
            &["ATP", "adenosine triphosphate"]
        ));
        assert!(fib_condition_accepts(
            "adenosine triphosphate",
            &["ATP", "adenosine triphosphate"]
        ));
        assert!(!fib_condition_accepts(
            "ADP",
            &["ATP", "adenosine triphosphate"]
        ));
    }
}
