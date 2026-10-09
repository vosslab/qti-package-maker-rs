//! Standalone, accessible HTML practice writer.
//!
//! The writer intentionally keeps the assessment item immutable.  It creates an
//! [`qti_core::ItemRenderView`] at the output boundary, inlines local images,
//! and emits one complete document with no runtime dependency on Python or a
//! network service.

use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use qti_core::media::{
    AssetKind, AssetSource, MediaAsset, MediaError, apply_media_policy, rewrite_item_media,
};
use qti_core::{ItemBank, ItemBody, ItemKind, ItemRenderView, NamedFile};

use crate::{EngineError, WriteArtifact, WriteContext, WriteOutcome, Writer};

pub(crate) const NAME: &str = "html_selftest";
const BASE_CSS: &str = include_str!("assets/base_styles.css");
const CSS: &str = include_str!("assets/control_styles.css");
const CONTROLS: &str = include_str!("assets/controls.js");
const DRAG_CONTROLS: &str = include_str!("assets/drag_controls.js");
const MATCH_CONTROLS: &str = include_str!("assets/match_controls.js");
const ORDER_CONTROLS: &str = include_str!("assets/order_controls.js");

/// Creates the HTML self-test writer for the compile-time engine registry.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(HtmlSelftestWriter)
}

struct HtmlSelftestWriter;

impl Writer for HtmlSelftestWriter {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> qti_core::media::MediaPolicy {
        crate::engine(NAME)
            .expect("registered selftest engine")
            .media_policy
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(NAME)
            .expect("registered selftest engine")
            .supported_kinds
    }

    fn write_package(
        &self,
        bank: &qti_core::ItemBank,
        source: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        let item = pick_item(bank, context.shuffle_seed)?;
        // Only the emitted practice question needs payloads from the caller's source.
        let mut selected = ItemBank::new(true);
        selected.add_item(item.clone())?;
        let assets = selected.collect_assets(source)?;
        let dependencies = assets.dependencies_for(item.crc()).unwrap_or_default();
        // Python passes pre-existing data URIs through without invoking the package-file policy.
        // Apply the shared decision only to dependencies that this writer can package or warn on.
        let policy_assets = dependencies
            .iter()
            .filter(|asset| asset.kind != AssetKind::DataUri)
            .cloned()
            .collect::<Vec<_>>();
        let warnings = apply_media_policy(
            self.media_policy(),
            &policy_assets,
            NAME,
            &item.crc().to_string(),
        )
        .map_err(|source| EngineError::InvalidFormat {
            engine: NAME,
            format: "HTML media",
            message: source.to_string(),
        })?
        .warnings;
        let source_map = dependencies
            .iter()
            .map(|asset| Ok((asset.src.clone(), asset_data_uri(asset)?)))
            .collect::<Result<BTreeMap<_, _>, EngineError>>()?;
        let view = rewrite_item_media(item, |src| {
            source_map
                .get(src)
                .cloned()
                .unwrap_or_else(|| src.to_owned())
        })
        .map_err(media_error)?;
        // Question, choice, and prompt fields are authored HTML.  Preserve them as the Python
        // writer does: this output is a self-test renderer, not an HTML sanitizer.
        let fragment = render_item(&view)?;
        let controls = item_control_assets(item.kind(), item.crc().to_string().as_str());
        let document = format!(
            "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n<title>QTI self-test</title><style>{BASE_CSS}{CSS}</style></head><body><main class=\"qti-selftest\">{fragment}</main><script>{CONTROLS}</script>{controls}</body></html>\n"
        );
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary: NamedFile::new(context.output_name(), document.into_bytes())?,
                companions: Vec::new(),
            }),
            warnings,
        })
    }
}

fn item_control_assets(kind: ItemKind, crc: &str) -> String {
    // The assets follow the current Python self-test interactions with a sentinel CRC.
    let substitute = |template: &str| template.replace("{{CRC}}", crc);
    match kind {
        ItemKind::Match => format!(
            "{}{}",
            substitute(DRAG_CONTROLS),
            substitute(MATCH_CONTROLS)
        ),
        ItemKind::Order => format!(
            "{}{}",
            substitute(DRAG_CONTROLS),
            substitute(ORDER_CONTROLS)
        ),
        _ => String::new(),
    }
}

fn pick_item(bank: &qti_core::ItemBank, shuffle_seed: u64) -> Result<&qti_core::Item, EngineError> {
    if bank.is_empty() {
        return Err(EngineError::InvalidFormat {
            engine: NAME,
            format: "item bank",
            message: "cannot save html_selftest output from an empty item bank".to_owned(),
        });
    }
    let candidates = bank
        .iter_ordered()
        .filter(|item| {
            crate::engine(NAME)
                .expect("registered selftest engine")
                .supported_kinds
                .contains(&item.kind())
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(EngineError::InvalidFormat {
            engine: NAME,
            format: "item bank",
            message: "no supported assessment item could be rendered".to_owned(),
        });
    }
    // The host supplies variation once; identical seeds select identical practice questions.
    Ok(candidates[(shuffle_seed % candidates.len() as u64) as usize])
}

fn media_error(source: MediaError) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format: "HTML media",
        message: source.to_string(),
    }
}

fn asset_data_uri(asset: &MediaAsset) -> Result<String, EngineError> {
    match asset.kind {
        AssetKind::DataUri => Ok(asset.src.clone()),
        AssetKind::External => Ok(asset.src.clone()),
        AssetKind::Local => {
            let mime = asset
                .mime_type
                .as_deref()
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "HTML media",
                    message: format!("local image '{}' has no MIME type", asset.src),
                })?;
            let bytes = asset.read_bytes().map_err(media_error)?;
            Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
        }
    }
}

fn render_item(item: &ItemRenderView) -> Result<String, EngineError> {
    let crc = item.crc().to_string();
    let stem = &item.common().question_text;
    let mut html = format!(
        "<div class=\"qti-selftest-item\" id=\"question_html_{crc}\" data-crc=\"{crc}\" data-kind=\"{}\"><div id=\"statement_text_{crc}\" class=\"qti-statement\">{stem}</div>",
        kind_name(item.kind())
    );
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            html.push_str(&choice_list(
                &crc,
                choices,
                |choice| choice == answer,
                false,
            ));
        }
        ItemBody::Ma {
            choices, answers, ..
        } => {
            html.push_str(&choice_list(
                &crc,
                choices,
                |choice| answers.contains(choice),
                true,
            ));
        }
        ItemBody::Fib { answers } => {
            html.push_str(&format!(
                "<input id=\"fib_input_{crc}\" class=\"qti-input qti-fib-input\" autocomplete=\"off\" placeholder=\"Enter your answer\" aria-label=\"Your answer\" data-answers=\"{}\">",
                encode_answers(answers)
            ));
        }
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => {
            if *tolerance_message {
                html.push_str(&format!(
                    "<p>Answer must be within &plusmn;{tolerance}.</p>"
                ));
            }
            html.push_str(&format!("<input id=\"num_input_{crc}\" class=\"qti-input qti-num-input\" inputmode=\"decimal\" pattern=\"[0-9]*[.,]?[0-9]*\" placeholder=\"Enter a number\" aria-label=\"Numeric answer\" data-answer=\"{answer}\" data-tolerance=\"{tolerance}\">"));
            // Keep JS lookup simple and avoid serializing untrusted data into executable code.
            html = html.replacen(
                "data-kind=\"num\"",
                &format!(
                    "data-kind=\"num\" data-answer=\"{answer}\" data-tolerance=\"{tolerance}\""
                ),
                1,
            );
        }
        ItemBody::MultiFib { answers } => {
            html = inject_blanks(html, answers, &crc);
        }
        ItemBody::Match { prompts, choices } => {
            html.push_str(&match_controls(&crc, prompts, choices))
        }
        ItemBody::Order { answers } => html.push_str(&order_controls(&crc, answers)),
    }
    html.push_str(buttons(item.kind()));
    html.push_str(&format!(
        "<div id=\"result_{crc}\" class=\"qti-feedback-result\" aria-live=\"polite\"></div>"
    ));
    html.push_str(
        "<div class=\"qti-sr-only\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div></div>",
    );
    Ok(html)
}

fn kind_name(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Mc => "mc",
        ItemKind::Ma => "ma",
        ItemKind::Match => "match",
        ItemKind::Num => "num",
        ItemKind::Fib => "fib",
        ItemKind::MultiFib => "multi-fib",
        ItemKind::Order => "order",
    }
}

fn choice_list(
    crc: &str,
    choices: &[String],
    correct: impl Fn(&String) -> bool,
    multiple: bool,
) -> String {
    let input_type = if multiple { "checkbox" } else { "radio" };
    let mut html = format!("<ul id=\"choices_{crc}\">");
    for (index, choice) in choices.iter().enumerate() {
        let option_number = if multiple { index + 1 } else { index };
        let id = format!("option_{crc}_{option_number}");
        let letter = letter(index);
        html.push_str(&format!("<li><input type=\"{input_type}\" id=\"{id}\" name=\"answer_{crc}\" data-correct=\"{}\"><label for=\"{id}\"><strong>{letter}.</strong><span class=\"qti-choice-content\">{choice}</span></label></li>", correct(choice)));
    }
    html.push_str("</ul>");
    html
}

fn match_controls(crc: &str, prompts: &[String], choices: &[String]) -> String {
    let mut html = "<p class=\"qti-control-instructions\">Drag a choice to a row, or click a choice and then a row. With a slot focused, type a letter to move that choice here.</p><table class=\"qti-match-table\"><thead><tr><th>Feedback</th><th>Your choice</th><th>Prompt</th></tr></thead><tbody>".to_owned();
    for (index, prompt) in prompts.iter().enumerate() {
        let token = format!("{crc}_{:03}", index + 1);
        html.push_str(&format!("<tr class=\"qti-match-row\"><td class=\"feedback\"></td><td class=\"qti-match-answer\"><button type=\"button\" class=\"qti-match-slot\" data-correct=\"{token}\" data-prompt=\"{}\" aria-label=\"Assign a choice to prompt {}\" aria-describedby=\"prompt_{crc}_{}\">Drop Your Choice Here</button></td><td class=\"qti-match-prompt\" id=\"prompt_{crc}_{}\">{}. {prompt}</td></tr>", index + 1, index + 1, index + 1, index + 1, index + 1));
    }
    html.push_str("</tbody></table><ul class=\"qti-match-bank\" aria-label=\"Answer choices\">");
    // Match choices are displayed in a shuffled order, but their token always retains the
    // original pair position.  A slot grades against that source token, never its display letter.
    for (display_index, (source_index, choice)) in choices.iter().enumerate().rev().enumerate() {
        let letter = letter(display_index);
        let palette = display_index % 5 + 1;
        html.push_str(&format!("<li><button type=\"button\" class=\"qti-match-choice qti-choice-{palette}\" data-value=\"{crc}_{:03}\" data-letter=\"{letter}\" draggable=\"true\" aria-pressed=\"false\"><strong>{letter}.</strong> <span class=\"qti-choice-content\">{choice}</span></button></li>", source_index + 1));
    }
    html.push_str("</ul>");
    html
}

fn order_controls(crc: &str, answers: &[String]) -> String {
    let mut html = "<p class=\"qti-control-instructions\">Drag and drop rows to arrange the answers, or use Move up and Move down. You can also use the arrow keys while a move button is focused.</p><ol class=\"qti-order-list\" aria-label=\"Your answer order\">".to_owned();
    // Reverse rather than shuffle: every author answer stays present, and output selection still
    // varies at the bank level.  A deterministic starting order supports reproducible tests.
    for (index, answer) in answers.iter().enumerate().rev() {
        let position = answers.len() - index;
        html.push_str(&format!("<li class=\"qti-order-row qti-choice-{}\" data-value=\"{crc}_{:03}\" data-initial=\"{position}\" draggable=\"true\"><span class=\"feedback\"></span><strong class=\"qti-order-position\">{position}</strong><span class=\"qti-choice-content\">{answer}</span><span class=\"qti-order-actions\"><button type=\"button\" class=\"qti-order-move\" data-direction=\"up\">Move up</button><button type=\"button\" class=\"qti-order-move\" data-direction=\"down\">Move down</button></span></li>", position % 5 + 1, index + 1));
    }
    html.push_str("</ol>");
    html
}

fn inject_blanks(
    html: String,
    answers: &std::collections::BTreeMap<String, Vec<String>>,
    crc: &str,
) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html.as_str();
    let mut occurrence = 1;
    while let Some(open) = remaining.find('[') {
        result.push_str(&remaining[..open]);
        let after_open = &remaining[open + 1..];
        let Some(close) = after_open.find(']') else {
            result.push_str(&remaining[open..]);
            return result;
        };
        let name = &after_open[..close];
        if let Some(values) = answers.get(name) {
            result.push_str(&format!(
                "<input class=\"qti-input fib-blank\" name=\"{}\" id=\"fib_blank_{crc}_{occurrence}\" aria-label=\"{}\" autocomplete=\"off\" placeholder=\"{}\" data-answers=\"{}\">",
                escape_attribute(name),
                escape_attribute(name),
                escape_attribute(name),
                encode_answers(values)
            ));
            occurrence += 1;
        } else {
            result.push_str(&remaining[open..open + close + 2]);
        }
        remaining = &after_open[close + 1..];
    }
    result.push_str(remaining);
    result
}

fn buttons(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Match | ItemKind::Order => {
            "<div class=\"qti-game-actions\"><button type=\"button\" class=\"qti-btn\" data-action=\"grade\">Check Answer</button><button type=\"button\" class=\"qti-btn qti-btn-reset\" data-action=\"reset\">Reset</button></div>"
        }
        ItemKind::Ma => {
            "<button type=\"button\" class=\"qti-btn\" data-action=\"grade\">Check Answer</button><button type=\"button\" class=\"qti-btn qti-btn-reset\" data-action=\"reset\">Clear Selection</button>"
        }
        _ => {
            "<button type=\"button\" class=\"qti-btn\" data-action=\"grade\">Check Answer</button>"
        }
    }
}

fn encode_answers(answers: &[String]) -> String {
    STANDARD.encode(answers.join("\u{1f}"))
}

fn letter(index: usize) -> char {
    char::from_u32(u32::from(b'A') + index as u32).unwrap_or('?')
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{HtmlSelftestWriter, Writer, boxed_writer, render_item};
    use crate::{DocumentMetadata, WriteArtifact, WriteContext};
    use qti_core::media::MemoryAssets;
    use qti_core::{Item, ItemBank, ItemBody};
    use scraper::{Html, Selector};
    use std::collections::BTreeMap;

    fn context(seed: u64) -> WriteContext {
        WriteContext::new(
            "practice/custom.html",
            DocumentMetadata {
                title: "Practice".into(),
                date: "2026-10-08".into(),
            },
            seed,
        )
        .expect("context")
    }

    fn item(body: ItemBody) -> Item {
        let question = if matches!(body, ItemBody::MultiFib { .. }) {
            "Question [blank]"
        } else {
            "Question <b>stem</b>"
        };
        Item::new(question.into(), body).expect("valid")
    }

    #[test]
    fn renders_every_item_kind_with_local_answer_data() {
        let cases = [
            ItemBody::Mc {
                choices: vec!["A".into(), "B".into(), "C".into()],
                answer: "A".into(),
            },
            ItemBody::Ma {
                choices: vec!["A".into(), "B".into(), "C".into()],
                answers: vec!["A".into()],
                min_answers_required: 1,
                allow_all_correct: false,
            },
            ItemBody::Match {
                prompts: vec!["P1".into(), "P2".into()],
                choices: vec!["C1".into(), "C2".into()],
            },
            ItemBody::Num {
                answer: 2.0,
                tolerance: 0.1,
                tolerance_message: true,
            },
            ItemBody::Fib {
                answers: vec!["yes".into()],
            },
            ItemBody::MultiFib {
                answers: BTreeMap::from([("blank".into(), vec!["yes".into()])]),
            },
            ItemBody::Order {
                answers: vec!["first".into(), "second".into(), "third".into()],
            },
        ];
        for body in cases {
            let rendered = render_item(&item(body).render_view()).expect("render");
            assert!(rendered.contains("data-kind"));
            assert!(rendered.contains("Check Answer"));
        }
    }

    #[test]
    fn preserves_python_question_and_input_identifiers() {
        let fib = item(ItemBody::Fib {
            answers: vec!["yes".into()],
        });
        let fib_crc = fib.crc().to_string();
        let fib_html = render_item(&fib.render_view()).expect("render FIB");
        assert!(fib_html.starts_with(&format!(
            "<div class=\"qti-selftest-item\" id=\"question_html_{fib_crc}\""
        )));
        assert!(fib_html.contains(&format!("id=\"statement_text_{fib_crc}\"")));
        assert!(fib_html.contains(&format!("id=\"fib_input_{fib_crc}\"")));
        assert!(fib_html.contains(&format!("id=\"result_{fib_crc}\"")));

        let number = item(ItemBody::Num {
            answer: 2.0,
            tolerance: 0.1,
            tolerance_message: false,
        });
        let number_html = render_item(&number.render_view()).expect("render NUM");
        assert!(number_html.contains(&format!("id=\"num_input_{}\"", number.crc())));

        let multiple = item(ItemBody::Ma {
            choices: vec!["A".into(), "B".into(), "C".into()],
            answers: vec!["A".into()],
            min_answers_required: 1,
            allow_all_correct: false,
        });
        let multiple_html = render_item(&multiple.render_view()).expect("render MA");
        assert!(multiple_html.contains(&format!("id=\"option_{}_1\"", multiple.crc())));
        assert!(!multiple_html.contains("Reveal answer"));
        assert!(multiple_html.contains("Clear Selection"));
    }

    #[test]
    fn repeated_multi_fib_blanks_have_unique_occurrence_identifiers() {
        let item = Item::new(
            "Fill [gene], then repeat [gene], and finally [trait].".into(),
            ItemBody::MultiFib {
                answers: BTreeMap::from([
                    ("gene".into(), vec!["A".into()]),
                    ("trait".into(), vec!["dominant".into()]),
                ]),
            },
        )
        .expect("valid multi FIB");
        let crc = item.crc().to_string();
        let rendered = render_item(&item.render_view()).expect("render");
        let document = Html::parse_fragment(&rendered);
        let inputs = Selector::parse(".fib-blank").expect("input selector");
        let ids = document
            .select(&inputs)
            .map(|input| input.value().attr("id").expect("input ID").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                format!("fib_blank_{crc}_1"),
                format!("fib_blank_{crc}_2"),
                format!("fib_blank_{crc}_3"),
            ]
        );
        assert!(rendered.contains("name=\"gene\""));
        assert!(rendered.contains("name=\"trait\""));
    }

    #[test]
    fn selftest_document_inlines_local_assets_and_preserves_external_urls() {
        let mut assets = MemoryAssets::new();
        assets
            .insert("figure.png", b"not a real image".to_vec())
            .expect("local asset");
        let mut bank = ItemBank::new(true);
        bank.add_item(Item::new("See <img src=\"figure.png\" /><img src=\"https://example.test/figure.png\" /><a href=\"https://example.test/source\">source</a>".into(), ItemBody::Fib {
            answers: vec!["yes".into()],
        }).expect("valid"))
        .expect("add");
        let outcome = boxed_writer()
            .write_package(&bank, &assets, &context(0))
            .expect("write");
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].src, "https://example.test/figure.png");
        let Some(WriteArtifact::File {
            primary,
            companions,
        }) = outcome.artifact
        else {
            panic!("selftest file");
        };
        assert_eq!(primary.name(), "practice/custom.html");
        assert!(companions.is_empty());
        let page = String::from_utf8(primary.into_parts().1).expect("HTML bytes");
        assert!(page.contains("<!doctype html>"));
        assert!(page.contains("qti-feedback-result"));
        assert!(page.contains("data:image/png;base64,bm90IGEgcmVhbCBpbWFnZQ=="));
        assert!(page.contains("src=\"https://example.test/figure.png\""));
        assert!(page.contains("href=\"https://example.test/source\""));
    }

    #[test]
    fn supplied_seed_selects_one_question_without_reading_other_question_media() {
        let mut bank = ItemBank::new(true);
        bank.add_item(
            Item::new(
                "Plain question".into(),
                ItemBody::Fib {
                    answers: vec!["yes".into()],
                },
            )
            .expect("plain item"),
        )
        .expect("add plain");
        bank.add_item(
            Item::new(
                "Image question <img src='missing.png'/>".into(),
                ItemBody::Fib {
                    answers: vec!["no".into()],
                },
            )
            .expect("image item"),
        )
        .expect("add image");
        let assets = MemoryAssets::new();
        let first = boxed_writer()
            .write_package(&bank, &assets, &context(0))
            .expect("plain selection");
        assert_eq!(
            first,
            boxed_writer()
                .write_package(&bank, &assets, &context(2))
                .expect("same selection")
        );
        let Some(WriteArtifact::File { primary, .. }) = first.artifact else {
            panic!("HTML file");
        };
        let html = std::str::from_utf8(primary.bytes()).expect("UTF-8");
        assert!(html.contains("Plain question"));
        assert!(!html.contains("Image question"));
        assert!(
            boxed_writer()
                .write_package(&bank, &assets, &context(1))
                .is_err()
        );
    }

    #[test]
    fn empty_bank_keeps_selftest_error() {
        let error = boxed_writer()
            .write_package(&ItemBank::new(true), &MemoryAssets::new(), &context(0))
            .expect_err("empty selftest");
        assert!(error.to_string().contains("empty item bank"));
    }

    #[test]
    fn authored_markup_is_preserved_for_python_parity() {
        let item = Item::new(
            "<script>window.authorHook=1</script><a href=\"https://example.test\" onclick=\"window.authorHook=2\">x</a>".into(),
            ItemBody::Fib { answers: vec!["yes".into()] },
        ).expect("valid");
        let rendered = render_item(&item.render_view()).expect("render");
        assert!(rendered.contains("window.authorHook=1"));
        assert!(rendered.contains("href=\"https://example.test\""));
        assert!(rendered.contains("onclick=\"window.authorHook=2\""));
    }

    #[test]
    fn match_choice_tokens_keep_source_pairing_after_display_shuffle() {
        let html = super::match_controls(
            "c0de",
            &["A".to_owned(), "C".to_owned()],
            &["T".to_owned(), "G".to_owned()],
        );
        let document = Html::parse_fragment(&html);
        let prompt_selector = Selector::parse(".qti-match-prompt").expect("prompt selector");
        let slot_selector = Selector::parse(".qti-match-slot").expect("slot selector");
        let choice_selector = Selector::parse(".qti-match-choice").expect("choice selector");
        let choice_content_selector =
            Selector::parse(".qti-choice-content").expect("choice content selector");
        let prompts = document
            .select(&prompt_selector)
            .zip(document.select(&slot_selector))
            .map(|(prompt, slot)| {
                (
                    prompt.text().collect::<String>().trim().to_owned(),
                    slot.value()
                        .attr("data-correct")
                        .expect("slot token")
                        .to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let choices = document
            .select(&choice_selector)
            .map(|choice| {
                (
                    choice
                        .value()
                        .attr("data-value")
                        .expect("choice token")
                        .to_owned(),
                    choice
                        .select(&choice_content_selector)
                        .next()
                        .expect("choice content")
                        .text()
                        .collect::<String>()
                        .trim()
                        .to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(prompts["1. A"], "c0de_001");
        assert_eq!(prompts["2. C"], "c0de_002");
        assert_eq!(choices["c0de_001"], "T");
        assert_eq!(choices["c0de_002"], "G");
        let display = document
            .select(&choice_selector)
            .map(|choice| {
                choice
                    .select(&choice_content_selector)
                    .next()
                    .expect("choice content")
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(display, ["G", "T"], "the bank is shuffled for display");
    }

    #[test]
    fn writer_has_all_seven_kinds() {
        assert_eq!(HtmlSelftestWriter.supported_kinds().len(), 7);
    }
}
