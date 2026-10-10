//! Embeddable, self-contained HTML practice fragments.
//!
//! The writer intentionally keeps the assessment item immutable.  It creates an
//! [`qti_core::ItemRenderView`] at the output boundary, inlines local images,
//! and emits a fragment with no runtime dependency on Python or a
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
        let fragment = render_item(&view, context.shuffle_seed)?;
        let controls = item_control_assets(item.kind(), item.crc().to_string().as_str());
        // Python emits fragments: the host owns document mode, metadata, and theme.
        // Install the shared Python CSS once, at the same boundary as its theme script.
        let styles = serde_json::to_string(&format!("{BASE_CSS}{CSS}"))
            .expect("static CSS serializes as a JSON string");
        let html = format!(
            "<script>(function() {{if (document.getElementById('qti-selftest-theme')) return;var style = document.createElement('style');style.id = 'qti-selftest-theme';style.textContent = {styles};(document.head || document.documentElement).appendChild(style);}})();</script>\n<div class=\"qti-selftest\">\n{fragment}\n<script>{CONTROLS}</script>{controls}</div>\n"
        );
        // Match Python's character references: fragments carry no charset declaration.
        let mut bytes = Vec::with_capacity(html.len());
        for character in html.chars() {
            if character.is_ascii() {
                bytes.push(character as u8);
            } else {
                bytes.extend_from_slice(format!("&#{};", character as u32).as_bytes());
            }
        }
        Ok(WriteOutcome {
            artifact: Some(WriteArtifact::File {
                primary: NamedFile::new(context.output_name(), bytes)?,
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

fn render_item(item: &ItemRenderView, seed: u64) -> Result<String, EngineError> {
    let crc = item.crc().to_string();
    // Match Python's format_question_text, including paragraph-to-line-break folding.
    // CSS margin suppression leaves separate paragraph boxes and changes authored styles.
    let adjacent_paragraphs =
        regex::Regex::new(r"</p>\s*<p>").expect("static adjacent paragraph expression");
    let stem = adjacent_paragraphs.replace_all(&item.common().question_text, "<br/>");
    let mut html = format!(
        "<div class=\"qti-selftest-item\" id=\"question_html_{crc}\" data-crc=\"{crc}\" data-kind=\"{}\">\n<div id=\"statement_text_{crc}\">{stem}</div>\n",
        kind_name(item.kind())
    );
    match item.body() {
        ItemBody::Mc { choices, answer } => {
            html.push_str("<form>\n");
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
            html.push_str("<form>\n");
            html.push_str(&choice_list(
                &crc,
                choices,
                |choice| answers.contains(choice),
                true,
            ));
        }
        ItemBody::Fib { answers } => {
            html.push_str("<form>\n");
            html.push_str(&format!(
                "<input type=\"text\" id=\"fib_input_{crc}\" class=\"qti-input qti-fib-input\" autocomplete=\"off\" placeholder=\"Enter your answer\" data-answers=\"{}\">",
                encode_answers(answers)
            ));
        }
        ItemBody::Num {
            answer, tolerance, ..
        } => {
            html.push_str("<div>\n");
            html.push_str(&format!("<input type=\"text\" id=\"num_input_{crc}\" class=\"qti-input qti-num-input\" inputmode=\"decimal\" pattern=\"[0-9]*[.,]?[0-9]*\" placeholder=\"Enter a number\">"));
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
        ItemBody::Match { prompts, .. } => html.push_str(&match_prompts(&crc, prompts)),
        ItemBody::Order { answers } => html.push_str(&order_controls(&crc, answers, seed)),
    }
    html.push_str(&actions(item.kind(), &crc));
    match item.kind() {
        ItemKind::Mc | ItemKind::Ma | ItemKind::Fib => html.push_str("</form><br/>\n"),
        ItemKind::Num => html.push_str("</div><br/>\n"),
        _ => {}
    }
    if let ItemBody::Match { choices, .. } = item.body() {
        html.push_str(&format!("<p class=\"qti-control-instructions\" id=\"instructions_{crc}\">Drag a choice to a row, or click a choice and then a row. With a slot focused, type a letter to move that choice here from other rows.</p>"));
        html.push_str(&match_choices(&crc, choices, seed));
    }
    if item.kind() == ItemKind::Match {
        html.push_str(
            "<div class=\"qti-sr-only\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>",
        );
    }
    html.push_str("</div>");
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

fn has_rich_content(html: &scraper::Html) -> bool {
    let selector = scraper::Selector::parse("table, div, p, ul, ol, svg, canvas, img")
        .expect("static rich-content selector");
    html.select(&selector).next().is_some()
}

fn choice_layout(choices: &[String]) -> &'static str {
    if choices.len() <= 3
        || choices.iter().any(|choice| {
            let fragment = scraper::Html::parse_fragment(choice);
            has_rich_content(&fragment)
                || fragment
                    .root_element()
                    .text()
                    .collect::<String>()
                    .chars()
                    .count()
                    > 50
        })
    {
        ""
    } else if choices.len() <= 5 {
        "qti-auto-grid-compact"
    } else {
        "qti-auto-grid"
    }
}

fn scroll_attributes(content: &str) -> &'static str {
    let rich = scraper::Selector::parse("table, div, pre, svg, canvas, img")
        .expect("static rich answer selector");
    if scraper::Html::parse_fragment(content)
        .select(&rich)
        .next()
        .is_some()
    {
        " tabindex=\"0\" role=\"group\" aria-label=\"Answer content\""
    } else {
        ""
    }
}

fn actions(kind: ItemKind, crc: &str) -> String {
    let controls = format!(
        "{}<div id=\"result_{crc}\" class=\"qti-feedback-result\">&#160;</div>\n",
        buttons(kind, crc)
    );
    if matches!(kind, ItemKind::Match | ItemKind::Order) {
        format!("<div class=\"qti-game-actions\">{controls}</div>\n")
    } else {
        controls
    }
}

// Shuffle presentation indices only: source item contents, CRC, and grading tokens stay fixed.
fn shuffled_indices(len: usize, seed: u64) -> Vec<usize> {
    let mut indices = (0..len).collect::<Vec<_>>();
    let mut rng = fastrand::Rng::with_seed(seed);
    // Fixed-width draws keep native and wasm32 arrangements identical for the same seed.
    for end in 1..len {
        indices.swap(end, rng.u64(..=end as u64) as usize);
    }
    indices
}

fn choice_list(
    crc: &str,
    choices: &[String],
    correct: impl Fn(&String) -> bool,
    multiple: bool,
) -> String {
    let input_type = if multiple { "checkbox" } else { "radio" };
    let layout = choice_layout(choices);
    let mut html = format!("<ul id=\"choices_{crc}\" class=\"{layout}\">");
    for (index, choice) in choices.iter().enumerate() {
        let option_number = if multiple { index + 1 } else { index };
        let id = format!("option_{crc}_{option_number}");
        let letter = letter(index);
        html.push_str(&format!("<li><input type=\"{input_type}\" id=\"{id}\" name=\"answer_{crc}\" data-correct=\"{}\"><label for=\"{id}\"><span style=\"font-weight: bold;\">{letter}.</span> <div class=\"qti-choice-content\">{choice}</div></label></li>\n", correct(choice)));
    }
    html.push_str("</ul>\n");
    html
}

fn match_prompts(crc: &str, prompts: &[String]) -> String {
    let mut html = "<div class=\"qti-match-layout\"><table class=\"qti-match-table\"><thead><tr><th scope=\"col\"><span class=\"qti-sr-only\">Feedback</span></th><th scope=\"col\">Your Choice</th><th scope=\"col\">Prompt</th></tr></thead><tbody>".to_owned();
    let rich = scraper::Selector::parse("table, div, pre, svg, canvas, img")
        .expect("static rich prompt selector");
    for (index, prompt) in prompts.iter().enumerate() {
        let token = format!("{crc}_{:03}", index + 1);
        let number = index + 1;
        // Diagrams use the available prompt width; prose retains Python's compact line length.
        let fragment = scraper::Html::parse_fragment(prompt);
        let (class, scroll) = if fragment.select(&rich).next().is_some() {
            (
                " qti-match-prompt-rich",
                format!(" tabindex=\"0\" role=\"group\" aria-label=\"Prompt {number} content\""),
            )
        } else {
            ("", String::new())
        };
        html.push_str(&format!("<tr class=\"qti-match-row\"><td class=\"qti-match-feedback\"><span class=\"feedback\"></span></td><td class=\"qti-match-answer\"><button type=\"button\" class=\"qti-match-slot\" data-correct=\"{token}\" data-prompt=\"{number}\" aria-label=\"Assign a choice to prompt {number}\" aria-describedby=\"prompt_{crc}_{number} instructions_{crc}\">Drop Your Choice Here</button></td><td class=\"qti-match-prompt\" id=\"prompt_{crc}_{number}\"><div class=\"qti-match-prompt-content{class}\"{scroll}>{number}. {prompt}</div></td></tr>"));
    }
    html.push_str("</tbody></table></div>");
    html
}

fn match_choices(crc: &str, choices: &[String], seed: u64) -> String {
    let mut html = format!(
        "<ul id=\"choiceList_{crc}\" class=\"qti-match-bank\" aria-label=\"Answer choices\">"
    );
    // Match choices are displayed in a shuffled order, but their token always retains the
    // original pair position.  A slot grades against that source token, never its display letter.
    for (display_index, source_index) in shuffled_indices(choices.len(), seed)
        .into_iter()
        .enumerate()
    {
        let choice = &choices[source_index];
        let letter = letter(display_index);
        let palette = (display_index + 1) % 5 + 1;
        // ASVS 1.2.1: escape the plain choice name at its HTML attribute boundary.
        let name = escape_attribute(&qti_core::make_question_pretty(choice));
        html.push_str(&format!("<li><button type=\"button\" class=\"qti-match-choice qti-choice-{palette}\" data-value=\"{crc}_{:03}\" data-letter=\"{letter}\" draggable=\"true\" aria-pressed=\"false\" aria-label=\"Select {letter}. {name}\"><span class=\"qti-choice-content\"><strong>{letter}.</strong> {choice}</span></button></li>", source_index + 1));
    }
    html.push_str("</ul>");
    html
}

fn order_controls(crc: &str, answers: &[String], seed: u64) -> String {
    let mut html = "<p class=\"qti-control-instructions\">Drag and drop rows to arrange the answers, or use Move up and Move down. You can also use the arrow keys while a move button is focused.</p><ol class=\"qti-order-list\" aria-label=\"Your answer order\">".to_owned();
    for (display_index, index) in shuffled_indices(answers.len(), seed)
        .into_iter()
        .enumerate()
    {
        let answer = &answers[index];
        let position = display_index + 1;
        let scroll = scroll_attributes(answer);
        html.push_str(&format!("<li class=\"qti-order-row qti-choice-{}\" data-value=\"{crc}_{:03}\" draggable=\"true\"><span class=\"feedback\"></span><strong class=\"qti-order-position\">{position}</strong><div class=\"qti-choice-content\"{scroll}>{answer}</div><div class=\"qti-order-actions\">", position % 5 + 1, index + 1));
        for (direction, label, relative, disabled) in [
            ("up", "Move up", "earlier", position == 1),
            ("down", "Move down", "later", position == answers.len()),
        ] {
            let disabled = if disabled { " disabled" } else { "" };
            html.push_str(&format!("<button type=\"button\" class=\"qti-order-move\" data-direction=\"{direction}\" aria-label=\"Move item {position} {relative}\"{disabled}>{label}</button>"));
        }
        html.push_str("</div></li>");
    }
    html.push_str("</ol><div class=\"qti-sr-only\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>");
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
            let answers = serde_json::to_string(values).expect("string answers serialize as JSON");
            // Match Python's MULTIFIB JSON attribute and format_html_lxml ampersand handling:
            // the browser decodes authored character references before JSON.parse reads them.
            // ASVS 1.2.1/1.2.3: keep quotes and markup escaped at the attribute boundary.
            let answers = escape_attribute(&answers).replace("&amp;", "&");
            result.push_str(&format!(
                "<input type=\"text\" class=\"fib-blank\" name=\"{}\" id=\"fib_blank_{crc}_{occurrence}\" placeholder=\"{}\" data-answers=\"{}\">",
                escape_attribute(name),
                escape_attribute(name),
                answers
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

fn buttons(kind: ItemKind, crc: &str) -> String {
    let check = format!(
        "<button type=\"button\" class=\"qti-btn\" onclick=\"checkAnswer_{crc}()\">Check Answer</button>\n"
    );
    match kind {
        ItemKind::Match | ItemKind::Order => format!(
            "<div class=\"qti-game-buttons\">{check}<button type=\"button\" class=\"qti-btn qti-btn-reset\" onclick=\"resetGame_{crc}()\">Reset</button>\n</div>"
        ),
        ItemKind::Ma => format!(
            "{check}<button type=\"button\" class=\"qti-btn qti-btn-reset\" onclick=\"clearSelection_{crc}()\">Clear Selection</button>\n"
        ),
        _ => check,
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
            let rendered = render_item(&item(body).render_view(), 0).expect("render");
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
        let fib_html = render_item(&fib.render_view(), 0).expect("render FIB");
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
        let number_html = render_item(&number.render_view(), 0).expect("render NUM");
        assert!(number_html.contains(&format!("id=\"num_input_{}\"", number.crc())));

        let multiple = item(ItemBody::Ma {
            choices: vec!["A".into(), "B".into(), "C".into()],
            answers: vec!["A".into()],
            min_answers_required: 1,
            allow_all_correct: false,
        });
        let multiple_html = render_item(&multiple.render_view(), 0).expect("render MA");
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
        let rendered = render_item(&item.render_view(), 0).expect("render");
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
    fn selftest_fragment_inlines_local_assets_and_preserves_external_urls() {
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
        for document_tag in ["<!doctype", "<html", "<head", "<body", "<meta", "<title"] {
            assert!(!page.to_ascii_lowercase().contains(document_tag));
        }
        assert!(page.contains("<div class=\"qti-selftest\">"));
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
        let rendered = render_item(&item.render_view(), 0).expect("render");
        assert!(rendered.contains("window.authorHook=1"));
        assert!(rendered.contains("href=\"https://example.test\""));
        assert!(rendered.contains("onclick=\"window.authorHook=2\""));
    }

    #[test]
    fn match_choice_tokens_keep_source_pairing_after_display_shuffle() {
        let item = item(ItemBody::Match {
            prompts: vec!["A".into(), "C".into()],
            choices: vec!["T".into(), "G".into()],
        });
        let crc = item.crc().to_string();
        let html = render_item(&item.render_view(), 0).expect("render matching question");
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
                let letter = choice.value().attr("data-letter").expect("choice letter");
                let text = choice
                    .select(&choice_content_selector)
                    .next()
                    .expect("choice content")
                    .text()
                    .collect::<String>();
                (
                    choice
                        .value()
                        .attr("data-value")
                        .expect("choice token")
                        .to_owned(),
                    text.strip_prefix(&format!("{letter}. "))
                        .expect("display letter precedes choice text")
                        .to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(prompts["1. A"], format!("{crc}_001"));
        assert_eq!(prompts["2. C"], format!("{crc}_002"));
        assert_eq!(choices[&format!("{crc}_001")], "T");
        assert_eq!(choices[&format!("{crc}_002")], "G");
        assert_eq!(choices.len(), 2);
    }

    #[test]
    fn match_choice_accessible_name_preserves_text_without_creating_attributes() {
        let html = super::match_choices(
            "c0de",
            &["<b>A &amp; B</b> \"quoted\" &mdash;H<sub>2</sub>PO<sub>4</sub>&ndash;".into()],
            0,
        );
        let document = Html::parse_fragment(&html);
        let selector = Selector::parse("button").expect("button selector");
        let choice = document.select(&selector).next().expect("choice");
        assert_eq!(
            choice.value().attr("aria-label"),
            Some("Select A. A & B \"quoted\" \u{2014}H2PO4\u{2013}")
        );
        assert_eq!(
            choice.text().collect::<String>(),
            "A. A & B \"quoted\" \u{2014}H2PO4\u{2013}"
        );
    }

    #[test]
    fn writer_has_all_seven_kinds() {
        assert_eq!(HtmlSelftestWriter.supported_kinds().len(), 7);
    }
}
