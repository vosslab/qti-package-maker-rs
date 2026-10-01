//! Blackboard Learn QTI 2.1 content-package writer.
//!
//! Each assessment item is a distinct XML resource below `qti21_items/`; local images are
//! copied to the ZIP root and every image reference is rewritten relative to that directory.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qti_core::media::{
    AssetKind, MediaPolicy, apply_media_policy, packageable_assets, rewrite_item_media,
};
use qti_core::{
    ArchiveEntry, ArchiveMap, ItemBody, ItemKind, ItemRenderView, ItemResource, ManifestConfig,
    QtiVersion, build_zip, generate_manifest,
};

use crate::{EngineError, EngineOptions, RenderHooks, WriteOutcome, Writer, render_bank};

mod fragment;

use fragment::{fragment, plain_text};

pub(crate) const NAME: &str = "blackboard_qti_v2_1";
const ITEM_DIRECTORY: &str = "qti21_items";
const QTI_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1";
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
const QTI_SCHEMA: &str = "http://www.imsglobal.org/xsd/imsqti_v2p1 http://www.imsglobal.org/xsd/qti/qtiv2p1/imsqti_v2p1.xsd";
const KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
    ItemKind::Order,
];

/// Creates the fixed-registry Blackboard QTI 2.1 writer.
///
/// `html_to_image` is a request to the shared conversion layer. The CLI performs that conversion
/// once before it fans a bank out to writers; direct library callers use the same facade there.
pub fn boxed_writer(options: EngineOptions) -> Box<dyn Writer> {
    Box::new(BlackboardQti21Writer {
        html_to_image: options.html_to_image,
    })
}

struct BlackboardQti21Writer {
    html_to_image: bool,
}

impl Writer for BlackboardQti21Writer {
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
            // M19 performs conversion once before a shared bank is sent to each package writer.
            // Refusing the direct writer path prevents a caller from believing an unconverted
            // bank was rasterized merely because this engine option was accepted.
            return Err(invalid(
                "html-to-image",
                "HTML conversion must be performed by the shared conversion pass",
            ));
        }
        let output = output
            .unwrap_or_else(|| Path::new("qti21-package.zip"))
            .to_path_buf();

        // Collect and apply the policy before creating the archive map. `build_zip` subsequently
        // preflights all source bytes before replacing `output`, so a data URI or unreadable local
        // file cannot leave a partial package behind (ASVS 2.2.1 / 5.3.2).
        let collected = bank.collect_assets()?;
        let mut all_assets = collected.assets().to_vec();
        for asset in &mut all_assets {
            if asset.kind == AssetKind::Local && asset.output_name.is_none() {
                return Err(invalid(
                    "media",
                    format!("asset '{}' has no package name", asset.src),
                ));
            }
        }
        let names = all_assets
            .iter()
            .filter_map(|asset| {
                asset
                    .output_name
                    .as_deref()
                    .map(|name| (asset.src.as_str(), name))
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
            .map_err(|error| invalid("media", error.to_string()))?;
            // `render_bank` visits renderable items in bank order, preserving the same ordering
            // for externally useful media diagnostics.
            warnings.borrow_mut().extend(decision.warnings);
            rewrite_item_media(item, |src| {
                names
                    .get(src)
                    .map_or_else(|| src.to_owned(), |name| format!("../{name}"))
            })
            .map_err(|error| invalid("HTML", error.to_string()))
        };
        let rendered = render_bank(
            bank,
            KINDS,
            render_item,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: None,
            },
        )?;

        let packaged_assets = all_assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Local)
            .cloned()
            .collect::<Vec<_>>();
        let packageable = packageable_assets(&packaged_assets)
            .map_err(|error| invalid("media", error.to_string()))?;
        let mut map = ArchiveMap::new();
        let mut item_resources = Vec::with_capacity(rendered.len());
        for (position, item) in rendered.iter().enumerate() {
            let file_name = format!("item_{:05}.xml", position + 1);
            let path = format!("{ITEM_DIRECTORY}/{file_name}");
            map.insert(
                path.clone(),
                ArchiveEntry::Bytes(item.xml.as_bytes().to_vec()),
            );
            let dependencies = collected
                .dependencies_for(&item.crc)
                .unwrap_or_default()
                .iter()
                .filter_map(|dependency| {
                    packaged_assets
                        .iter()
                        .position(|asset| asset.src == dependency.src)
                })
                .collect();
            item_resources.push(ItemResource {
                href: path,
                asset_indexes: dependencies,
            });
        }
        if item_resources.is_empty() {
            return Err(invalid("QTI 2.1", "bank contains no renderable items"));
        }
        map.insert(
            format!("{ITEM_DIRECTORY}/assessment_meta.xml"),
            ArchiveEntry::Bytes(assessment_meta("QTI 2.1", rendered.len()).into_bytes()),
        );
        for asset in packageable {
            let name = asset
                .output_name
                .clone()
                .expect("packageable assets have names");
            let bytes = asset
                .read_bytes()
                .map_err(|error| invalid("media", error.to_string()))?;
            map.insert(name, ArchiveEntry::Bytes(bytes));
        }
        let manifest = generate_manifest(&ManifestConfig {
            package_name: "QTI 2.1".to_owned(),
            version: QtiVersion::V2p1,
            items: item_resources,
            assets: &packaged_assets,
            metadata_date: Some("2026-09-30".to_owned()),
        })?;
        map.insert("imsmanifest.xml".to_owned(), ArchiveEntry::Bytes(manifest));
        build_zip(&output, &map, BTreeSet::<String>::new())?;
        Ok(WriteOutcome {
            path: Some(output),
            warnings: warnings.into_inner(),
        })
    }
}

struct RenderedItem {
    xml: String,
    crc: qti_core::ItemCrc,
}

fn render_item(item: &ItemRenderView) -> Result<Option<RenderedItem>, EngineError> {
    let xml = match item.body() {
        ItemBody::Mc { choices, answer } => {
            choice_item(item, choices, std::slice::from_ref(answer), 1)?
        }
        ItemBody::Ma {
            choices, answers, ..
        } => choice_item(item, choices, answers, answers.len())?,
        ItemBody::Match { prompts, choices } => match_item(item, prompts, choices),
        ItemBody::Num {
            answer, tolerance, ..
        } => numeric_item(item, *answer, *tolerance),
        ItemBody::Fib { answers } => fib_item(item, answers),
        ItemBody::MultiFib { answers } => multi_fib_item(item, answers),
        ItemBody::Order { answers } => order_item(item, answers),
    };
    Ok(Some(RenderedItem {
        xml,
        crc: *item.crc(),
    }))
}

fn choice_item(
    item: &ItemRenderView,
    choices: &[String],
    answers: &[String],
    max: usize,
) -> Result<String, EngineError> {
    let identifiers = answers
        .iter()
        .map(|answer| {
            choices
                .iter()
                .position(|choice| choice == answer)
                .map(|index| format!("answer_{}", index + 1))
                .ok_or_else(|| invalid("QTI 2.1", "correct answer is absent from choices"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let declaration = response_declaration(
        "identifier",
        if identifiers.len() == 1 {
            "single"
        } else {
            "multiple"
        },
        "RESPONSE",
        &identifiers,
        "",
    );
    let choices = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            format!(
                "<simpleChoice fixed=\"true\" identifier=\"answer_{}\"><p>{}</p></simpleChoice>",
                index + 1,
                fragment(choice)
            )
        })
        .collect::<String>();
    let body = format!(
        "<itemBody><div>{}</div><choiceInteraction maxChoices=\"{max}\" responseIdentifier=\"RESPONSE\" shuffle=\"true\">{choices}</choiceInteraction></itemBody>",
        fragment(&item.common().question_text)
    );
    Ok(assessment_item(
        item,
        &declaration,
        &body,
        &standard_processing(),
    ))
}

fn match_item(item: &ItemRenderView, prompts: &[String], choices: &[String]) -> String {
    let pairs = prompts
        .iter()
        .enumerate()
        .map(|(index, _)| format!("prompt_{:03} choice_{:03}", index + 1, index + 1))
        .collect::<Vec<_>>();
    let values = pairs
        .iter()
        .map(|pair| format!("<value>{}</value>", xml(pair)))
        .collect::<String>();
    let map_entries = pairs
        .iter()
        .map(|pair| {
            format!(
                "<mapEntry mapKey=\"{}\" mappedValue=\"1\"/>",
                xml_attr(pair)
            )
        })
        .collect::<String>();
    let declaration = format!(
        "<responseDeclaration baseType=\"directedPair\" cardinality=\"multiple\" identifier=\"RESPONSE\"><correctResponse>{values}</correctResponse><mapping defaultValue=\"0\">{map_entries}</mapping></responseDeclaration>"
    );
    let prompt_nodes = prompts.iter().enumerate().map(|(index, prompt)| format!("<simpleAssociableChoice identifier=\"prompt_{:03}\" fixed=\"true\" matchMax=\"1\" matchMin=\"0\"><p>{}</p></simpleAssociableChoice>", index + 1, fragment(prompt))).collect::<String>();
    let choice_nodes = choices.iter().enumerate().map(|(index, choice)| format!("<simpleAssociableChoice identifier=\"choice_{:03}\" fixed=\"true\" matchMax=\"{}\" matchMin=\"0\"><p>{}</p></simpleAssociableChoice>", index + 1, prompts.len(), fragment(choice))).collect::<String>();
    let stem = fragment(&item.common().question_text);
    let body = format!(
        "<itemBody><div>{stem}</div><matchInteraction responseIdentifier=\"RESPONSE\" shuffle=\"true\" maxAssociations=\"{}\"><prompt>{}</prompt><simpleMatchSet>{prompt_nodes}</simpleMatchSet><simpleMatchSet>{choice_nodes}</simpleMatchSet></matchInteraction></itemBody>",
        prompts.len(),
        plain_text(&item.common().question_text)
    );
    assessment_item(
        item,
        &declaration,
        &body,
        "<responseProcessing template=\"http://www.imsglobal.org/question/qti_v2p1/rptemplates/map_response\"/>",
    )
}

fn numeric_item(item: &ItemRenderView, answer: f64, tolerance: f64) -> String {
    let answer = number(answer);
    let tolerance = number(tolerance);
    let declaration = response_declaration("float", "single", "RESPONSE", &[answer], "");
    let body = entry_body(item, "RESPONSE");
    let processing = format!(
        "<responseProcessing><responseCondition><responseIf><equal toleranceMode=\"absolute\" tolerance=\"{tolerance} {tolerance}\" includeLowerBound=\"true\" includeUpperBound=\"true\"><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></equal><setOutcomeValue identifier=\"SCORE\"><baseValue baseType=\"float\">100</baseValue></setOutcomeValue></responseIf><responseElse><setOutcomeValue identifier=\"SCORE\"><baseValue baseType=\"float\">0</baseValue></setOutcomeValue></responseElse></responseCondition></responseProcessing>"
    );
    assessment_item(item, &declaration, &body, &processing)
}

fn fib_item(item: &ItemRenderView, answers: &[String]) -> String {
    let values = answers
        .iter()
        .map(|answer| format!("<value>{}</value>", xml(answer)))
        .collect::<String>();
    let mappings = answers
        .iter()
        .map(|answer| {
            format!(
                "<mapEntry mapKey=\"{}\" caseSensitive=\"false\" mappedValue=\"100.0\"/>",
                xml_attr(answer)
            )
        })
        .collect::<String>();
    let declaration = format!(
        "<responseDeclaration baseType=\"string\" cardinality=\"single\" identifier=\"RESPONSE\"><correctResponse>{values}</correctResponse><mapping>{mappings}</mapping></responseDeclaration>"
    );
    assessment_item(
        item,
        &declaration,
        &entry_body(item, "RESPONSE"),
        &standard_processing(),
    )
}

fn multi_fib_item(
    item: &ItemRenderView,
    answers: &std::collections::BTreeMap<String, Vec<String>>,
) -> String {
    let declarations = answers
        .iter()
        .map(|(key, values)| response_declaration("string", "single", key, &values[..1], ""))
        .collect::<String>();
    let mut stem = item.common().question_text.clone();
    let mut processing = String::from("<responseProcessing>");
    let score = 100.0 / answers.len() as f64;
    for key in answers.keys() {
        stem = stem.replace(
            &format!("[{key}]"),
            &format!(
                "<textEntryInteraction responseIdentifier=\"{}\"/>",
                xml_attr(key)
            ),
        );
        let predicate = if answers[key].len() == 1 {
            format!(
                "<match><variable identifier=\"{}\"/><correct identifier=\"{}\"/></match>",
                xml_attr(key),
                xml_attr(key)
            )
        } else {
            let alternatives = answers[key].iter().map(|answer| {
                format!("<match><variable identifier=\"{}\"/><baseValue baseType=\"string\">{}</baseValue></match>", xml_attr(key), xml(answer))
            }).collect::<String>();
            format!("<or>{alternatives}</or>")
        };
        processing.push_str(&format!("<responseCondition><responseIf>{predicate}<setOutcomeValue identifier=\"SCORE\"><sum><variable identifier=\"SCORE\"/><baseValue baseType=\"float\">{score:.2}</baseValue></sum></setOutcomeValue></responseIf></responseCondition>"));
    }
    processing.push_str("</responseProcessing>");
    let body = format!("<itemBody><div>{}</div></itemBody>", fragment(&stem));
    assessment_item_with_score_default(item, &declarations, &body, &processing, Some("0"))
}

fn order_item(item: &ItemRenderView, answers: &[String]) -> String {
    let identifiers = (1..=answers.len())
        .map(|index| format!("choice_{index:03}"))
        .collect::<Vec<_>>();
    let declaration = response_declaration("identifier", "ordered", "RESPONSE", &identifiers, "");
    let nodes = answers
        .iter()
        .enumerate()
        .map(|(index, answer)| {
            format!(
                "<simpleChoice identifier=\"choice_{:03}\"><p>{}</p></simpleChoice>",
                index + 1,
                fragment(answer)
            )
        })
        .collect::<String>();
    let body = format!(
        "<itemBody><div>{}</div><orderInteraction responseIdentifier=\"RESPONSE\" shuffle=\"true\"><prompt>{}</prompt>{nodes}</orderInteraction></itemBody>",
        fragment(&item.common().question_text),
        plain_text(&item.common().question_text)
    );
    assessment_item(
        item,
        &declaration,
        &body,
        "<responseProcessing template=\"http://www.imsglobal.org/question/qti_v2p1/rptemplates/match_correct\"/>",
    )
}

fn assessment_item(
    item: &ItemRenderView,
    declaration: &str,
    body: &str,
    processing: &str,
) -> String {
    assessment_item_with_score_default(item, declaration, body, processing, None)
}

fn assessment_item_with_score_default(
    item: &ItemRenderView,
    declaration: &str,
    body: &str,
    processing: &str,
    score_default: Option<&str>,
) -> String {
    let identifier = format!("QUE_{}_{:04x}", item.crc(), item.common().item_number);
    let score_outcome = score_default.map_or_else(
        || {
            "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"/>"
                .to_owned()
        },
        |value| {
            format!(
                "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"><defaultValue><value>{}</value></defaultValue></outcomeDeclaration>",
                xml(value)
            )
        },
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><assessmentItem xmlns=\"{QTI_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{QTI_SCHEMA}\" title=\"{identifier}\" adaptive=\"false\" timeDependent=\"false\" identifier=\"{identifier}\">{declaration}{score_outcome}{body}{processing}</assessmentItem>"
    )
}

fn response_declaration(
    base_type: &str,
    cardinality: &str,
    identifier: &str,
    values: &[String],
    suffix: &str,
) -> String {
    let values = values
        .iter()
        .map(|value| format!("<value>{}</value>", xml(value)))
        .collect::<String>();
    format!(
        "<responseDeclaration baseType=\"{base_type}\" cardinality=\"{cardinality}\" identifier=\"{}\"><correctResponse>{values}</correctResponse>{suffix}</responseDeclaration>",
        xml_attr(identifier)
    )
}

fn entry_body(item: &ItemRenderView, identifier: &str) -> String {
    format!(
        "<itemBody><div>{}</div><p><textEntryInteraction responseIdentifier=\"{}\"/></p></itemBody>",
        fragment(&item.common().question_text),
        xml_attr(identifier)
    )
}

fn standard_processing() -> String {
    "<responseProcessing><responseCondition><responseIf><match><variable identifier=\"RESPONSE\"/><correct identifier=\"RESPONSE\"/></match></responseIf></responseCondition></responseProcessing>".to_owned()
}

fn assessment_meta(title: &str, count: usize) -> String {
    let references = (1..=count)
        .map(|number| {
            format!(
                "<assessmentItemRef identifier=\"item_{number:05}\" href=\"item_{number:05}.xml\"/>"
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><assessmentTest xmlns=\"{QTI_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{QTI_SCHEMA}\" identifier=\"assessment_meta\" title=\"{}\"><testPart identifier=\"test_part\" navigationMode=\"nonlinear\" submissionMode=\"simultaneous\"><assessmentSection identifier=\"section_part\" visible=\"false\" title=\"Question Pool\">{references}</assessmentSection></testPart></assessmentTest>",
        xml_attr(title)
    )
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_attr(value: &str) -> String {
    xml(value).replace('"', "&quot;").replace('\'', "&apos;")
}

fn number(value: f64) -> String {
    value.to_string()
}

fn invalid(format: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::io::Read;

    use qti_core::media::MediaAction;
    use qti_core::{Item, ItemBank, ItemBody, MediaBaseDir};

    use super::{BlackboardQti21Writer, KINDS, boxed_writer, fragment};
    use crate::{EngineOptions, Writer};

    fn seven_item_bank() -> ItemBank {
        let mut bank = ItemBank::new(true);
        let bodies = [
            ItemBody::Mc {
                choices: vec!["a".into(), "b".into()],
                answer: "b".into(),
            },
            ItemBody::Ma {
                choices: vec!["a".into(), "b".into(), "c".into()],
                answers: vec!["a".into(), "c".into()],
                min_answers_required: 1,
                allow_all_correct: false,
            },
            ItemBody::Match {
                prompts: vec!["one".into(), "two".into()],
                choices: vec!["one".into(), "two".into()],
            },
            ItemBody::Num {
                answer: 4.0,
                tolerance: 0.1,
                tolerance_message: false,
            },
            ItemBody::Fib {
                answers: vec!["answer".into()],
            },
            ItemBody::MultiFib {
                answers: BTreeMap::from([
                    ("one".into(), vec!["one".into()]),
                    ("two".into(), vec!["two".into()]),
                ]),
            },
            ItemBody::Order {
                answers: vec!["first".into(), "second".into(), "third".into()],
            },
        ];
        for (index, body) in bodies.into_iter().enumerate() {
            let question = if index == 5 {
                "MULTI [one] [two]".into()
            } else {
                format!("Question number {index}")
            };
            bank.add_item(Item::new(question, body).expect("valid item"))
                .expect("add item");
        }
        bank
    }

    #[test]
    fn plain_prompt_preserves_displayed_entities_and_literal_markup() {
        assert_eq!(
            super::plain_text("<strong>&Delta;G &minus; T</strong>"),
            "&lt;strong&gt;\u{0394}G \u{2212} T&lt;/strong&gt;"
        );
        assert_eq!(
            super::plain_text("A &amp; B &unknown;"),
            "A &amp; B &amp;unknown;"
        );
    }

    #[test]
    fn multi_fib_alternatives_use_single_correct_value_and_explicit_predicates() {
        let answers = BTreeMap::from([("order".into(), vec!["kfr".into(), "rfk".into()])]);
        let item = Item::new(
            "Gene order: [order]".into(),
            ItemBody::MultiFib {
                answers: answers.clone(),
            },
        )
        .expect("valid alternative answers");
        let xml = super::multi_fib_item(&item.render_view(), &answers);
        let mut reader = quick_xml::Reader::from_str(&xml);
        let mut correct_response = false;
        let mut declared_values = 0;
        let mut alternatives = 0;
        loop {
            match reader.read_event().expect("well-formed QTI") {
                quick_xml::events::Event::Start(node)
                    if node.name().as_ref() == "correctResponse" =>
                {
                    correct_response = true
                }
                quick_xml::events::Event::End(node)
                    if node.name().as_ref() == "correctResponse" =>
                {
                    correct_response = false
                }
                quick_xml::events::Event::Start(node)
                    if correct_response && node.name().as_ref() == "value" =>
                {
                    declared_values += 1
                }
                quick_xml::events::Event::Start(node) if node.name().as_ref() == "match" => {
                    alternatives += 1
                }
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(declared_values, 1, "single-cardinality declaration");
        assert_eq!(
            alternatives, 2,
            "both authored answers have grading predicates"
        );
        assert!(xml.contains(">kfr</baseValue>"));
        assert!(xml.contains(">rfk</baseValue>"));
    }

    fn emitted_multi_fib_score(xml: &str, correct: &[&str]) -> f64 {
        let correct = correct.iter().copied().collect::<BTreeSet<_>>();
        let mut score = 0.0;
        for condition in xml.split("<responseCondition>").skip(1) {
            let response = condition
                .split("<variable identifier=\"")
                .nth(1)
                .and_then(|tail| tail.split_once('\"'))
                .map(|(identifier, _)| identifier)
                .expect("response condition variable");
            if !correct.contains(response) {
                continue;
            }
            let action = condition
                .split("<setOutcomeValue identifier=\"SCORE\">")
                .nth(1)
                .expect("SCORE action");
            let value = action
                .split("<baseValue baseType=\"float\">")
                .nth(1)
                .and_then(|tail| tail.split_once("</baseValue>"))
                .map(|(value, _)| value.parse::<f64>().expect("float contribution"))
                .expect("float contribution");
            if action.starts_with("<sum><variable identifier=\"SCORE\"/>") {
                score += value;
            } else {
                score = value;
            }
        }
        score
    }

    fn write_png(path: &std::path::Path) {
        fs::write(
            path,
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248,
                207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68,
                174, 66, 96, 130,
            ],
        )
        .expect("image");
    }

    fn archive_text(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> String {
        let mut text = String::new();
        archive
            .by_name(name)
            .unwrap_or_else(|_| panic!("archive entry '{name}'"))
            .read_to_string(&mut text)
            .expect("read archive text");
        text
    }

    #[test]
    fn writes_all_seven_types_with_local_media_and_no_integrity_errors() {
        let temporary = tempfile::tempdir().expect("temp directory");
        fs::write(
            temporary.path().join("figure.png"),
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 248,
                207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68,
                174, 66, 96, 130,
            ],
        )
        .expect("image");
        let mut bank = seven_item_bank();
        bank.set_media_base_dir(Some(MediaBaseDir::external(temporary.path())));
        let first = bank.get(0).expect("first").clone();
        let mut image_bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(temporary.path()));
        image_bank
            .add_item(
                Item::new(
                    format!("{} <img src=\"figure.png\"/>", first.common().question_text),
                    first.body().clone(),
                )
                .expect("image item"),
            )
            .expect("add image item");
        for item in bank.iter_ordered().skip(1) {
            image_bank.add_item(item.clone()).expect("add item");
        }
        let output = temporary.path().join("package.zip");
        BlackboardQti21Writer {
            html_to_image: false,
        }
        .save_package(&image_bank, Some(&output))
        .expect("package");
        let mut archive =
            zip::ZipArchive::new(fs::File::open(&output).expect("open zip")).expect("zip");
        assert_eq!(archive.len(), 10);
        let first_item = archive_text(&mut archive, "qti21_items/item_00001.xml");
        assert!(first_item.contains("correctResponse><value>answer_2"));
        assert!(first_item.contains("src=\"../figure.png\""));
        let multiple_answer = archive_text(&mut archive, "qti21_items/item_00002.xml");
        assert!(multiple_answer.contains("cardinality=\"multiple\""));
        assert!(multiple_answer.contains("<value>answer_1</value><value>answer_3</value>"));
        let matching = archive_text(&mut archive, "qti21_items/item_00003.xml");
        // QTI directed-pair values are represented as whitespace-separated identifiers, as in
        // the frozen Python writer's `create_response_declaration_MATCH` helper.
        assert!(matching.contains("<value>prompt_001 choice_001</value>"));
        assert!(matching.contains("mapKey=\"prompt_002 choice_002\" mappedValue=\"1\""));
        let numeric = archive_text(&mut archive, "qti21_items/item_00004.xml");
        assert!(numeric.contains("baseType=\"float\""));
        assert!(numeric.contains("tolerance=\"0.1 0.1\""));
        let fib = archive_text(&mut archive, "qti21_items/item_00005.xml");
        assert!(fib.contains("mapKey=\"answer\" caseSensitive=\"false\" mappedValue=\"100.0\""));
        let multi_fib = archive_text(&mut archive, "qti21_items/item_00006.xml");
        assert!(multi_fib.contains("responseIdentifier=\"one\""));
        assert!(multi_fib.contains("responseIdentifier=\"two\""));
        assert!(multi_fib.contains(
            "<outcomeDeclaration baseType=\"float\" cardinality=\"single\" identifier=\"SCORE\"><defaultValue><value>0</value></defaultValue></outcomeDeclaration>"
        ));
        let contribution = "<sum><variable identifier=\"SCORE\"/><baseValue baseType=\"float\">50.00</baseValue></sum>";
        let correct_blank_count = multi_fib.matches(contribution).count();
        assert_eq!(correct_blank_count, 2);
        assert_eq!(emitted_multi_fib_score(&multi_fib, &[]), 0.0);
        assert_eq!(emitted_multi_fib_score(&multi_fib, &["one"]), 50.0);
        assert_eq!(emitted_multi_fib_score(&multi_fib, &["two"]), 50.0);
        assert_eq!(emitted_multi_fib_score(&multi_fib, &["one", "two"]), 100.0);
        let overwrite_mutant = multi_fib
            .replace("<sum><variable identifier=\"SCORE\"/>", "")
            .replace("</sum>", "");
        assert_eq!(
            emitted_multi_fib_score(&overwrite_mutant, &["one", "two"]),
            50.0
        );
        let ordered = archive_text(&mut archive, "qti21_items/item_00007.xml");
        assert!(ordered.contains("cardinality=\"ordered\""));
        assert!(ordered.contains("<value>choice_001</value><value>choice_002</value>"));
        let entries = qti_integrity::check_package(&output);
        assert!(
            entries
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error),
            "integrity errors: {entries:#?}"
        );
    }

    #[test]
    fn policies_reject_data_uri_before_output() {
        let temporary = tempfile::tempdir().expect("temp directory");
        let bank = seven_item_bank();
        let item = bank.get(0).expect("item");
        let mut media_bank = ItemBank::new(true);
        media_bank
            .add_item(
                Item::new(
                    format!(
                        "{} <img src=\"data:image/png;base64,AA==\"/>",
                        item.common().question_text
                    ),
                    item.body().clone(),
                )
                .expect("data item"),
            )
            .expect("add");
        let output = temporary.path().join("must-not-exist.zip");
        let error = BlackboardQti21Writer {
            html_to_image: false,
        }
        .save_package(&media_bank, Some(&output))
        .expect_err("data URI rejected");
        assert!(error.to_string().contains("data URI"));
        assert!(!output.exists());
    }

    #[test]
    fn html_to_image_option_requires_the_shared_conversion_pass() {
        let temporary = tempfile::tempdir().expect("temp directory");
        let error = boxed_writer(EngineOptions {
            html_to_image: true,
            ..EngineOptions::default()
        })
        .save_package(
            &seven_item_bank(),
            Some(&temporary.path().join("output.zip")),
        )
        .expect_err("writer must not rerun conversion");
        assert!(error.to_string().contains("shared conversion pass"));
    }

    #[test]
    fn collision_safe_media_stays_with_its_item_and_remote_urls_are_preserved() {
        let temporary = tempfile::tempdir().expect("temp directory");
        fs::create_dir_all(temporary.path().join("left")).expect("left directory");
        fs::create_dir_all(temporary.path().join("right")).expect("right directory");
        write_png(&temporary.path().join("left/figure.png"));
        write_png(&temporary.path().join("right/figure.png"));
        let mut bank =
            ItemBank::with_media_base_dir(true, MediaBaseDir::external(temporary.path()));
        for question in [
            "First image <img src=\"left/figure.png\"/>",
            "Second image <img src=\"right/figure.png\"/>",
            "Remote image <img src=\"https://example.test/image.png\"/>",
        ] {
            bank.add_item(
                Item::new(
                    question.into(),
                    ItemBody::Mc {
                        choices: vec!["first".into(), "second".into()],
                        answer: "first".into(),
                    },
                )
                .expect("item"),
            )
            .expect("add item");
        }
        let output = temporary.path().join("collision.zip");
        let outcome = BlackboardQti21Writer {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("package");
        assert_eq!(outcome.path.as_deref(), Some(output.as_path()));
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].item_crc,
            bank.get(2).expect("third item").crc().to_string()
        );
        assert_eq!(outcome.warnings[0].action, MediaAction::KeptVerbatim);
        assert_eq!(outcome.warnings[0].src, "https://example.test/image.png");
        let mut archive =
            zip::ZipArchive::new(fs::File::open(&output).expect("open zip")).expect("zip archive");
        assert!(archive.by_name("figure.png").is_ok());
        assert!(archive.by_name("figure(1).png").is_ok());
        let mut first = String::new();
        archive
            .by_name("qti21_items/item_00001.xml")
            .expect("first XML")
            .read_to_string(&mut first)
            .expect("read first XML");
        let mut second = String::new();
        archive
            .by_name("qti21_items/item_00002.xml")
            .expect("second XML")
            .read_to_string(&mut second)
            .expect("read second XML");
        let mut third = String::new();
        archive
            .by_name("qti21_items/item_00003.xml")
            .expect("third XML")
            .read_to_string(&mut third)
            .expect("read third XML");
        assert!(first.contains("../figure.png"));
        assert!(second.contains("../figure(1).png"));
        assert!(third.contains("https://example.test/image.png"));
        assert!(
            qti_integrity::check_package(&output)
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error)
        );
    }

    #[test]
    fn makes_html_named_entities_safe_for_qti_xml() {
        assert_eq!(
            fragment(
                "<p title=\"&Gamma;&#34;\">&alpha; &amp; &#946; &#38; &#60; &AMP; &notARealEntity;</p>",
            ),
            "<p title=\"\u{393}&quot;\">\u{3B1} &amp; \u{3B2} &amp; &lt; &amp; &amp;notARealEntity;</p>"
        );
        let raw_html = fragment(
            "<script>const terminator=\"]]>\";for(let i=0;i<items.length;i++){ready&&draw(i);}</script><td rowspan=2><a href=\"https://example.test/?a=1&b=2\">link</a></td>",
        );
        assert!(raw_html.contains("<![CDATA[const terminator=\"]]]]><![CDATA[>\";for(let i=0;i<items.length;i++){ready&&draw(i);}]]>"));
        assert!(raw_html.contains("rowspan=\"2\""));
        assert!(raw_html.contains("a=1&amp;b=2"));
        let xml = format!("<root>{raw_html}</root>");
        let mut reader = quick_xml::Reader::from_str(&xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(error) => panic!("writer fragment must be XML: {error}"),
            }
        }
        let temporary = tempfile::tempdir().expect("temp directory");
        let mut bank = ItemBank::new(true);
        bank.add_item(
            Item::new(
                "<p title=\"&Gamma;&#34;\">&alpha; &amp; &#946; &#38; &#60; &AMP; &notARealEntity;</p>"
                    .into(),
                ItemBody::Mc {
                    choices: vec!["one".into(), "two".into()],
                    answer: "one".into(),
                },
            )
            .expect("entity-bearing item"),
        )
        .expect("add item");
        let output = temporary.path().join("entities.zip");
        BlackboardQti21Writer {
            html_to_image: false,
        }
        .save_package(&bank, Some(&output))
        .expect("package");
        let mut archive =
            zip::ZipArchive::new(fs::File::open(&output).expect("open zip")).expect("zip archive");
        let item = archive_text(&mut archive, "qti21_items/item_00001.xml");
        assert!(item.contains("title=\"\u{393}&quot;\""));
        assert!(item.contains("\u{3B1} &amp; \u{3B2} &amp; &lt; &amp; &amp;notARealEntity;"));
        assert!(
            qti_integrity::check_package(&output)
                .iter()
                .all(|violation| violation.severity != qti_integrity::Severity::Error)
        );
    }

    #[test]
    fn declares_every_supported_kind() {
        assert_eq!(
            BlackboardQti21Writer {
                html_to_image: false,
            }
            .supported_kinds(),
            KINDS
        );
    }
}
