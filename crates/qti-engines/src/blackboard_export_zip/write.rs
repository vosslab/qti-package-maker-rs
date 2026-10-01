//! Deterministic Blackboard Original pool-export package writer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::{EngineError, RenderHooks, WriteOutcome, render_bank};
use qti_core::media::{
    AssetKind, MediaAction, MediaWarning, apply_media_policy, rewrite_item_media,
};
use qti_core::{ArchiveEntry, ArchiveMap, ItemBody, ItemRenderView, build_zip};

use super::{KINDS, NAME};

const TOKEN_PREFIX: &str = "@X@EmbeddedFile.requestUrlStub@X@bbcswebdav/xid-";

pub(super) fn save_package(
    bank: &qti_core::ItemBank,
    output: Option<&Path>,
) -> Result<WriteOutcome, EngineError> {
    let output = output.unwrap_or_else(|| Path::new("blackboard-export.zip"));
    let mut warnings = Vec::new();
    for item in bank
        .iter_ordered()
        .filter(|item| !KINDS.contains(&item.kind()))
    {
        warnings.push(MediaWarning {
            engine_name: NAME.to_owned(),
            item_crc: item.crc().to_string(),
            src: "ORDER".to_owned(),
            resolved: "ORDER".to_owned(),
            action: MediaAction::KeptVerbatim,
            reason: "unsupported ORDER item was skipped".to_owned(),
        });
    }
    let collected = bank.collect_assets()?;
    let assets = collected.assets();
    // ASVS 5.3.2: emit generated csfiles paths only for sources retained in
    // this package's rendered pool.  Unsupported items have no ASI object and
    // therefore cannot safely own a CSResourceLinks parentId.
    let rendered_local_sources = bank
        .iter_ordered()
        .filter(|item| KINDS.contains(&item.kind()))
        .flat_map(|item| collected.dependencies_for(item.crc()).unwrap_or_default())
        .filter(|asset| asset.kind == AssetKind::Local)
        .map(|asset| asset.src.clone())
        .collect::<BTreeSet<_>>();
    let mut token_by_source = BTreeMap::new();
    let mut embedded = Vec::new();
    for (index, asset) in assets
        .iter()
        .filter(|asset| rendered_local_sources.contains(&asset.src))
        .enumerate()
    {
        let xid = format!("{}_{:01}", index + 1, 1);
        token_by_source.insert(asset.src.as_str(), format!("{TOKEN_PREFIX}{xid}"));
        embedded.push((xid, asset));
    }
    // Blackboard assigns every xid to the first item that refers to its source
    // in bank order.  The same ASI form is emitted in `skeleton`, so Learn can
    // resolve a CSResourceLinks parentId without relying on server metadata.
    let mut parent_by_source = BTreeMap::new();
    for item in bank
        .iter_ordered()
        .filter(|item| KINDS.contains(&item.kind()))
    {
        for asset in collected.dependencies_for(item.crc()).unwrap_or_default() {
            if token_by_source.contains_key(asset.src.as_str()) {
                parent_by_source
                    .entry(asset.src.to_owned())
                    .or_insert_with(|| format!("_{}_1", item.crc()));
            }
        }
    }
    for item in bank
        .iter_ordered()
        .filter(|item| KINDS.contains(&item.kind()))
    {
        let decision = apply_media_policy(
            qti_core::media::MediaPolicy::Package,
            collected.dependencies_for(item.crc()).unwrap_or_default(),
            NAME,
            &item.crc().to_string(),
        )
        .map_err(media_error)?;
        warnings.extend(decision.warnings);
    }
    let pre = |item: &qti_core::Item| {
        rewrite_item_media(item, |source| {
            token_by_source
                .get(source)
                .cloned()
                .unwrap_or_else(|| source.to_owned())
        })
        .map_err(media_error)
    };
    let items = render_bank(
        bank,
        KINDS,
        render_item,
        RenderHooks {
            pre_render: Some(&pre),
            post_render: None,
        },
    )?;
    let mut archive = ArchiveMap::new();
    archive.insert(
        "imsmanifest.xml".to_owned(),
        ArchiveEntry::Bytes(manifest().into_bytes()),
    );
    archive.insert(
        "res00002.dat".to_owned(),
        ArchiveEntry::Bytes(pool_document(&items).into_bytes()),
    );
    archive.insert(
        "res00005.dat".to_owned(),
        ArchiveEntry::Bytes(resource_links(&embedded, &parent_by_source)?.into_bytes()),
    );
    for (name, body) in fixed_sidecars() {
        archive.insert(name.to_owned(), ArchiveEntry::Bytes(body.into_bytes()));
    }
    archive.insert(
        ".bb-package-info".to_owned(),
        ArchiveEntry::Bytes(b"PackageFormatVersion=6.0\n".to_vec()),
    );
    archive.insert(
        ".bb-log-info".to_owned(),
        ArchiveEntry::Bytes(b"Blackboard pool export\n".to_vec()),
    );
    for (xid, asset) in embedded {
        let output_name = asset.output_name.as_deref().unwrap_or(&asset.src);
        let extension = extension(output_name);
        let base = format!("csfiles/home_dir/__xid-{xid}.{extension}");
        let bytes = asset.read_bytes().map_err(media_error)?;
        archive.insert(base.clone(), ArchiveEntry::Bytes(bytes));
        archive.insert(
            format!("{base}.xml"),
            ArchiveEntry::Bytes(lom_sidecar(&xid, output_name).into_bytes()),
        );
    }
    build_zip(output, &archive, ["res00001/"])
        .map(|path| WriteOutcome {
            path: Some(path),
            warnings,
        })
        .map_err(EngineError::from)
}

fn media_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::InvalidFormat {
        engine: NAME,
        format: "media",
        message: error.to_string(),
    }
}

fn extension(name: &str) -> &str {
    name.rsplit_once('.')
        .map_or("png", |(_, extension)| extension)
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn tag(name: &str, value: &str) -> String {
    format!("<{name}>{}</{name}>", xml(value))
}

fn item_id(item: &ItemRenderView) -> String {
    format!("_{}_1", item.crc())
}

fn smart(html: &str) -> String {
    format!(
        "<flow class=\"FORMATTED_TEXT_BLOCK\"><material><mat_extension><mat_formattedtext type=\"SMART_TEXT\">{}</mat_formattedtext></mat_extension></material></flow>",
        xml(&sanitize_question_html(html))
    )
}

/// Removes invalid whitespace-only nodes directly inside Blackboard table
/// structure. Blackboard Ultra can fail on one directly inside a styled `<tr>`;
/// cell text, comments, and other authored markup remain byte-for-byte intact.
fn sanitize_question_html(html: &str) -> String {
    if !html.to_ascii_lowercase().contains("<table") {
        return html.to_owned();
    }
    let mut sanitized = String::with_capacity(html.len());
    let mut stack = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        if let Some(raw_tag) = raw_text_tag(&stack) {
            let closing_tag = format!("</{raw_tag}");
            let Some(offset) = find_ascii_case_insensitive(&html[cursor..], &closing_tag) else {
                sanitized.push_str(&html[cursor..]);
                break;
            };
            let closing = cursor + offset;
            sanitized.push_str(&html[cursor..closing]);
            cursor = closing;
        }
        let Some(offset) = html[cursor..].find('<') else {
            push_table_text(&mut sanitized, &html[cursor..], &stack);
            break;
        };
        let tag_start = cursor + offset;
        push_table_text(&mut sanitized, &html[cursor..tag_start], &stack);
        let Some(tag_end) = html_tag_end(&html[tag_start..]) else {
            sanitized.push_str(&html[tag_start..]);
            break;
        };
        let tag_end = tag_start + tag_end;
        let tag = &html[tag_start..=tag_end];
        sanitized.push_str(tag);
        apply_html_tag(tag, &mut stack);
        cursor = tag_end + 1;
    }
    sanitized
}

fn push_table_text(output: &mut String, text: &str, stack: &[String]) {
    let is_table_whitespace = stack
        .last()
        .is_some_and(|tag| matches!(tag.as_str(), "table" | "thead" | "tbody" | "tfoot" | "tr"));
    if !is_table_whitespace || !text.chars().all(char::is_whitespace) {
        output.push_str(text);
    }
}

fn raw_text_tag(stack: &[String]) -> Option<&str> {
    match stack.last().map(String::as_str) {
        Some("script") => Some("script"),
        Some("style") => Some("style"),
        _ => None,
    }
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn html_tag_end(fragment: &str) -> Option<usize> {
    if fragment.starts_with("<!--") {
        return fragment.find("-->").map(|offset| offset + 2);
    }
    let mut quote = None;
    for (offset, byte) in fragment.bytes().enumerate().skip(1) {
        match (quote, byte) {
            (Some(delimiter), value) if value == delimiter => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
    }
    None
}

fn apply_html_tag(tag: &str, stack: &mut Vec<String>) {
    let content = tag[1..tag.len() - 1].trim();
    if content.starts_with('!') || content.starts_with('?') || content.is_empty() {
        return;
    }
    let closing = content.starts_with('/');
    let name = content
        .trim_start_matches('/')
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | ':'))
        .collect::<String>()
        .to_ascii_lowercase();
    if name.is_empty() {
        return;
    }
    if closing {
        if let Some(index) = stack.iter().rposition(|open| open == &name) {
            stack.truncate(index);
        }
        return;
    }
    if !content.ends_with('/') && !is_void_html_tag(&name) {
        stack.push(name);
    }
}

fn is_void_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn format_number(value: f64) -> String {
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

fn skeleton(
    item: &ItemRenderView,
    question_type: &str,
    response: String,
    processing: String,
) -> String {
    skeleton_with_metadata(item, question_type, response, processing, "")
}

fn skeleton_with_metadata(
    item: &ItemRenderView,
    question_type: &str,
    response: String,
    processing: String,
    grading_metadata: &str,
) -> String {
    format!(
        "<item title=\"{}\" maxattempts=\"0\"><itemmetadata>{}<bbmd_questiontype>{}</bbmd_questiontype>{grading_metadata}</itemmetadata><presentation><flow class=\"Block\"><flow class=\"QUESTION_BLOCK\">{}</flow><flow class=\"RESPONSE_BLOCK\">{response}</flow></flow></presentation>{processing}</item>",
        xml(&item.crc().to_string()),
        tag("bbmd_asi_object_id", &item_id(item)),
        xml(question_type),
        smart(&item.common().question_text)
    )
}

fn render_item(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let rendered = match item.body() {
        ItemBody::Mc { choices, answer } => choice_item(
            item,
            "Multiple Choice",
            choices,
            std::slice::from_ref(answer),
            false,
            None,
        ),
        ItemBody::Ma {
            choices,
            answers,
            min_answers_required,
            allow_all_correct,
        } => choice_item(
            item,
            "Multiple Answer",
            choices,
            answers,
            true,
            Some((*min_answers_required, *allow_all_correct)),
        ),
        ItemBody::Fib { answers } => fib_item(item, answers),
        ItemBody::MultiFib { answers } => multi_fib_item(item, answers),
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message,
        } => numeric_item(item, *answer, *tolerance, *tolerance_message),
        ItemBody::Match { prompts, choices } => match_item(item, prompts, choices),
        ItemBody::Order { .. } => return Ok(None),
    };
    Ok(Some(rendered))
}

fn processing(max_score: usize, branches: String) -> String {
    format!(
        "<resprocessing scoremodel=\"SumOfScores\"><outcomes><decvar varname=\"SCORE\" vartype=\"Decimal\" defaultval=\"0\" minvalue=\"0\" maxvalue=\"{max_score}.00000\"/></outcomes>{branches}</resprocessing>"
    )
}

fn score_branch(condition: String, score: &str, feedback: &str, title: Option<&str>) -> String {
    let title = title.map_or_else(String::new, |title| format!(" title=\"{title}\""));
    format!(
        "<respcondition{title}><conditionvar>{condition}</conditionvar><setvar variablename=\"SCORE\" action=\"Set\">{score}</setvar>{feedback}</respcondition>"
    )
}

fn feedback(identifier: &str) -> String {
    format!("<displayfeedback linkrefid=\"{identifier}\" feedbacktype=\"Response\"/>")
}

fn render_fib(fibtype: &str) -> String {
    format!(
        "<render_fib charset=\"us-ascii\" columns=\"0\" encoding=\"UTF_8\" fibtype=\"{fibtype}\" maxchars=\"0\" maxnumber=\"0\" minnumber=\"0\" prompt=\"Box\" rows=\"0\"/>"
    )
}

fn incorrect_branch() -> String {
    score_branch(
        "<other/>".to_owned(),
        "0",
        &feedback("incorrect"),
        Some("incorrect"),
    )
}

fn item_feedback() -> String {
    "<itemfeedback ident=\"correct\" view=\"All\"><flow_mat class=\"Block\"/></itemfeedback><itemfeedback ident=\"incorrect\" view=\"All\"><flow_mat class=\"Block\"/></itemfeedback>".to_owned()
}

fn choice_item(
    item: &ItemRenderView,
    typ: &str,
    choices: &[String],
    answers: &[String],
    multiple: bool,
    grading_options: Option<(usize, bool)>,
) -> String {
    let label_ids = choices
        .iter()
        .enumerate()
        .map(|(index, _)| format!("{}_label_{index}", item.crc()))
        .collect::<Vec<_>>();
    let labels = choices
        .iter()
        .zip(&label_ids)
        .map(|(choice, id)| {
            format!(
                "<response_label ident=\"{id}\" shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\">{}</response_label>",
                smart(choice)
            )
        })
        .collect::<String>();
    let correct_ids = answers
        .iter()
        .filter_map(|answer| choices.iter().position(|choice| choice == answer))
        .map(|index| &label_ids[index])
        .collect::<Vec<_>>();
    let response = format!(
        "<response_lid ident=\"response\" rcardinality=\"{}\" rtiming=\"No\"><render_choice shuffle=\"No\" minnumber=\"0\" maxnumber=\"0\">{labels}</render_choice></response_lid>",
        if multiple { "Multiple" } else { "Single" }
    );
    let correct_condition = if multiple {
        let all = label_ids
            .iter()
            .map(|id| {
                let equal = format!("<varequal respident=\"response\" case=\"No\">{id}</varequal>");
                if correct_ids.contains(&id) {
                    equal
                } else {
                    format!("<not>{equal}</not>")
                }
            })
            .collect::<String>();
        format!("<and>{all}</and>")
    } else {
        correct_ids
            .iter()
            .map(|id| format!("<varequal respident=\"response\" case=\"No\">{id}</varequal>"))
            .collect()
    };
    let mut branches = score_branch(
        correct_condition,
        "SCORE.max",
        &feedback("correct"),
        Some("correct"),
    );
    branches.push_str(&incorrect_branch());
    for id in &label_ids {
        // Blackboard's frozen exports put dangling displayfeedback links on
        // single-choice penalties.  The source has no corresponding authored
        // feedback body, so retain the predicate and zero-score penalty but
        // omit the invalid reference.
        branches.push_str(&score_branch(
            format!("<varequal respident=\"{id}\" case=\"No\"></varequal>"),
            "0",
            "",
            None,
        ));
    }
    let grading_metadata = grading_options.map_or_else(String::new, |(minimum, allow_all)| {
        format!(
            "<bbmd_qti_package_maker_ma_min_answers_required>{minimum}</bbmd_qti_package_maker_ma_min_answers_required><bbmd_qti_package_maker_ma_allow_all_correct>{allow_all}</bbmd_qti_package_maker_ma_allow_all_correct>"
        )
    });
    skeleton_with_metadata(
        item,
        typ,
        response,
        format!("{}{}", processing(1, branches), item_feedback()),
        &grading_metadata,
    )
}

fn fib_item(item: &ItemRenderView, answers: &[String]) -> String {
    let mut branches = String::new();
    let mut answer_feedback = String::new();
    for (index, answer) in answers.iter().enumerate() {
        let title = format!("{}_fib_answer_{index}", item.crc());
        branches.push_str(&format!(
            "<respcondition title=\"{title}\"><conditionvar><varequal respident=\"response\" case=\"No\">{}</varequal></conditionvar>{}{}</respcondition>",
            xml(answer), feedback("correct"), feedback(&title),
        ));
        answer_feedback.push_str(&format!("<itemfeedback ident=\"{title}\" view=\"All\"><solution view=\"All\" feedbackstyle=\"Complete\"><solutionmaterial><flow_mat class=\"Block\"/></solutionmaterial></solution></itemfeedback>"));
    }
    branches.push_str(&incorrect_branch());
    skeleton(
        item,
        "Fill in the Blank",
        format!(
            "<response_str ident=\"response\" rcardinality=\"Single\" rtiming=\"No\">{}</response_str>",
            render_fib("String")
        ),
        format!(
            "{}{}{}",
            processing(1, branches),
            item_feedback(),
            answer_feedback
        ),
    )
}

fn multi_fib_item(
    item: &ItemRenderView,
    answers: &std::collections::BTreeMap<String, Vec<String>>,
) -> String {
    let response = answers.keys().map(|key| format!("<response_str ident=\"{}\" rcardinality=\"Single\" rtiming=\"No\">{}</response_str>", xml(key), render_fib("String"))).collect::<String>();
    let groups = answers
        .iter()
        .map(|(key, values)| {
            format!(
                "<or>{}</or>",
                values
                    .iter()
                    .map(|value| format!(
                        "<varequal respident=\"{}\" case=\"No\">{}</varequal>",
                        xml(key),
                        xml(value)
                    ))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    let branches = format!(
        "{}{}",
        score_branch(
            format!("<and>{groups}</and>"),
            "SCORE.max",
            &feedback("correct"),
            Some("correct")
        ),
        incorrect_branch()
    );
    skeleton(
        item,
        "Fill in the Blank Plus",
        response,
        format!("{}{}", processing(answers.len(), branches), item_feedback()),
    )
}

fn numeric_item(
    item: &ItemRenderView,
    answer: f64,
    tolerance: f64,
    tolerance_message: bool,
) -> String {
    let correct_title = format!("{}_num_correct_0", item.crc());
    let lower_bound = format_number(answer - tolerance);
    let upper_bound = format_number(answer + tolerance);
    let answer = format_number(answer);
    let tolerance = format_number(tolerance);
    let condition = format!(
        "<vargte respident=\"response\">{lower_bound}</vargte><varlte respident=\"response\">{upper_bound}</varlte><varequal respident=\"response\" case=\"No\">{answer}</varequal>",
    );
    let branches = format!(
        "<respcondition title=\"{correct_title}\"><conditionvar>{condition}</conditionvar>{}</respcondition>{}",
        feedback("correct"),
        incorrect_branch()
    );
    let grading_metadata = format!(
        "<bbmd_qti_package_maker_num_tolerance>{tolerance}</bbmd_qti_package_maker_num_tolerance><bbmd_qti_package_maker_num_tolerance_message>{tolerance_message}</bbmd_qti_package_maker_num_tolerance_message>"
    );
    skeleton_with_metadata(
        item,
        "Numeric",
        format!(
            "<response_num ident=\"response\" rcardinality=\"Single\" rtiming=\"No\">{}</response_num>",
            render_fib("Decimal")
        ),
        format!("{}{}", processing(1, branches), item_feedback()),
        &grading_metadata,
    )
}

fn match_item(item: &ItemRenderView, prompts: &[String], choices: &[String]) -> String {
    let response = prompts.iter().enumerate().map(|(prompt_index, prompt)| {
        let labels = choices.iter().enumerate().map(|(choice_index, _)| format!("<response_label ident=\"{}_match_{prompt_index}_{choice_index}\" shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\"/>", item.crc())).collect::<String>();
        format!("<flow class=\"Block\"><response_lid ident=\"{}_match_prompt_{prompt_index}\" rcardinality=\"Single\" rtiming=\"No\"><render_choice shuffle=\"Yes\" minnumber=\"0\" maxnumber=\"0\"><flow_label class=\"Block\">{labels}</flow_label></render_choice></response_lid>{}</flow>", item.crc(), smart(prompt))
    }).collect::<String>();
    let right_choices = choices
        .iter()
        .map(|choice| format!("<flow class=\"Block\">{}</flow>", smart(choice)))
        .collect::<String>();
    let branches = prompts.iter().enumerate().map(|(index, _)| format!("<respcondition><conditionvar><varequal respident=\"{}_match_prompt_{index}\" case=\"No\">{}_match_{index}_{index}</varequal></conditionvar>{}</respcondition>", item.crc(), item.crc(), feedback("correct"))).collect::<String>() + &incorrect_branch();
    skeleton(
        item,
        "Matching",
        format!("{response}<flow class=\"RIGHT_MATCH_BLOCK\">{right_choices}</flow>"),
        format!("{}{}", processing(1, branches), item_feedback()),
    )
}

fn pool_document(items: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><questestinterop><assessment title=\"QTI package maker\"><section>{}</section></assessment></questestinterop>",
        items.join("")
    )
}

fn manifest() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest xmlns:bb=\"http://www.blackboard.com/content-packaging/\" identifier=\"main_manifest\"><organizations/><resources><resource bb:file=\"res00002.dat\" bb:title=\"QTI package maker\" identifier=\"res00002\" type=\"assessment/x-bb-qti-pool\" xml:base=\"res00002\"/><resource bb:file=\"res00005.dat\" bb:title=\"CSResourceLinks\" identifier=\"res00005\" type=\"course/x-bb-csresourcelinks\" xml:base=\"res00005\"/></resources></manifest>".to_owned()
}

fn fixed_sidecars() -> [(&'static str, String); 5] {
    [
        (
            "res00001.dat",
            "<?xml version=\"1.0\"?><COURSE/>".to_owned(),
        ),
        (
            "res00003.dat",
            "<?xml version=\"1.0\"?><ASSESSMENTCREATIONSETTINGS/>".to_owned(),
        ),
        (
            "res00004.dat",
            "<?xml version=\"1.0\"?><LEARNRUBRICS/>".to_owned(),
        ),
        (
            "res00006.dat",
            "<?xml version=\"1.0\"?><STDS_ALIGNMENTS/>".to_owned(),
        ),
        (
            "res00007.dat",
            "<?xml version=\"1.0\"?><COURSERUBRICASSOCIATIONS/>".to_owned(),
        ),
    ]
}

fn resource_links(
    embedded: &[(String, &qti_core::media::MediaAsset)],
    parent_by_source: &BTreeMap<String, String>,
) -> Result<String, EngineError> {
    let mut links = String::new();
    for (xid, asset) in embedded {
        let parent =
            parent_by_source
                .get(asset.src.as_str())
                .ok_or_else(|| EngineError::InvalidFormat {
                    engine: NAME,
                    format: "media",
                    message: format!("local asset '{}' has no owning item", asset.src),
                })?;
        links.push_str(&format!(
            "<cms_resource_link><parentId>{parent}</parentId><resourceId>{xid}</resourceId></cms_resource_link>"
        ));
    }
    Ok(format!(
        "<?xml version=\"1.0\"?><cms_resource_link_list>{links}</cms_resource_link_list>"
    ))
}

fn lom_sidecar(resource_id: &str, output_name: &str) -> String {
    const LOM_NAMESPACE: &str = "http://www.imsglobal.org/xsd/imsmd_rootv1p2p1";
    const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
    const LOM_SCHEMA_LOCATION: &str =
        "http://www.imsglobal.org/xsd/imsmd_rootv1p2p1 imsmd_rootv1p2p1.xsd";
    let output_name = output_name.rsplit('/').next().unwrap_or(output_name);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><lom xmlns=\"{LOM_NAMESPACE}\" xmlns:xsi=\"{XSI_NAMESPACE}\" xsi:schemaLocation=\"{LOM_SCHEMA_LOCATION}\"><relation><resource><identifier>{}#/courses/qti_package_maker/{}</identifier></resource></relation></lom>",
        xml(resource_id),
        xml(output_name)
    )
}

#[cfg(test)]
mod tests {
    use qti_core::{Item, ItemBody};

    use super::{format_number, numeric_item, sanitize_question_html};

    #[test]
    fn removes_only_invalid_table_structure_whitespace() {
        let input = "<table style=\"text-align:center\"><tr> <td> cell </td> <td><b>bold</b> text</td></tr> </table>";
        assert_eq!(
            sanitize_question_html(input),
            "<table style=\"text-align:center\"><tr><td> cell </td><td><b>bold</b> text</td></tr></table>"
        );
    }

    #[test]
    fn removes_comment_separated_table_whitespace_without_losing_authored_content() {
        let input = "<TABLE> <TR><!-- first cell --> <TD> cell text </TD> <!-- second cell --> <TD><b>bold</b> text</TD> </TR> </TABLE>";
        assert_eq!(
            sanitize_question_html(input),
            "<TABLE><TR><!-- first cell --><TD> cell text </TD><!-- second cell --><TD><b>bold</b> text</TD></TR></TABLE>"
        );
    }

    #[test]
    fn leaves_non_table_html_byte_for_byte_unchanged() {
        let input = "plain <b> text </b>";
        assert_eq!(sanitize_question_html(input), input);
    }

    #[test]
    fn numeric_grading_uses_frozen_six_decimal_xml_values() {
        assert_eq!(format_number(0.284_179_92), "0.28418");
        assert_eq!(format_number(5.734_747_26), "5.734747");
        assert_eq!(format_number(4.0), "4");

        let item = Item::new(
            "Precise numeric question".to_owned(),
            ItemBody::Num {
                answer: 0.284_179_92,
                tolerance: 0.0028,
                tolerance_message: true,
            },
        )
        .expect("valid numeric fixture");
        let xml = numeric_item(&item.render_view(), 0.284_179_92, 0.0028, true);
        assert!(xml.contains("<vargte respident=\"response\">0.28138</vargte>"));
        assert!(xml.contains("<varlte respident=\"response\">0.28698</varlte>"));
        assert!(xml.contains("<varequal respident=\"response\" case=\"No\">0.28418</varequal>"));
        assert!(xml.contains(
            "<bbmd_qti_package_maker_num_tolerance>0.0028</bbmd_qti_package_maker_num_tolerance>"
        ));
        assert!(!xml.contains("0.28417992"));
    }
}

#[cfg(test)]
mod grading_tests {
    use std::collections::BTreeSet;

    use qti_core::{Item, ItemBody};
    use regex::Regex;

    fn assert_displayfeedback_targets(xml: &str) {
        let references = Regex::new(r#"<displayfeedback linkrefid=\"([^\"]+)\""#)
            .expect("static reference regex is valid")
            .captures_iter(xml)
            .map(|capture| capture[1].to_owned())
            .collect::<BTreeSet<_>>();
        let targets = Regex::new(r#"<itemfeedback ident=\"([^\"]+)\""#)
            .expect("static target regex is valid")
            .captures_iter(xml)
            .map(|capture| capture[1].to_owned())
            .collect::<BTreeSet<_>>();

        assert!(references.is_subset(&targets));
    }

    use super::{render_fib, render_item};

    #[test]
    fn emits_blackboard_score_actions_and_feedback_for_every_supported_kind() {
        let items = [
            Item::new(
                "MC question".to_owned(),
                ItemBody::Mc {
                    choices: vec!["a".to_owned(), "b".to_owned()],
                    answer: "a".to_owned(),
                },
            ),
            Item::new(
                "MA question".to_owned(),
                ItemBody::Ma {
                    choices: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
                    answers: vec!["a".to_owned()],
                    min_answers_required: 1,
                    allow_all_correct: true,
                },
            ),
            Item::new(
                "FIB question".to_owned(),
                ItemBody::Fib {
                    answers: vec!["a".to_owned(), "b".to_owned()],
                },
            ),
            Item::new(
                "NUM question".to_owned(),
                ItemBody::Num {
                    answer: 4.0,
                    tolerance: 0.01,
                    tolerance_message: true,
                },
            ),
            Item::new(
                "MATCH question".to_owned(),
                ItemBody::Match {
                    prompts: vec!["a".to_owned(), "b".to_owned()],
                    choices: vec!["one".to_owned(), "two".to_owned()],
                },
            ),
            Item::new(
                "MULTI [left] [right]".to_owned(),
                ItemBody::MultiFib {
                    answers: [
                        ("left".to_owned(), vec!["L".to_owned()]),
                        ("right".to_owned(), vec!["R".to_owned()]),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
        ];
        for item in items {
            let item = item.expect("valid fixture");
            let xml = render_item(&item.render_view())
                .expect("rendering succeeds")
                .expect("supported kind renders");
            assert!(
                xml.contains(
                    "<resprocessing scoremodel=\"SumOfScores\"><outcomes><decvar varname=\"SCORE\""
                ),
                "{:?} omits SCORE outcome: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains("<setvar variablename=\"SCORE\" action=\"Set\">0</setvar>"),
                "{:?} omits zero-score fallback: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains(
                    "<displayfeedback linkrefid=\"incorrect\" feedbacktype=\"Response\"/>"
                ),
                "{:?} omits incorrect feedback: {xml}",
                item.kind(),
            );
            assert!(
                xml.contains("<itemfeedback ident=\"correct\" view=\"All\">")
                    && xml.contains("<itemfeedback ident=\"incorrect\" view=\"All\">"),
                "{:?} omits item feedback blocks: {xml}",
                item.kind(),
            );
            assert_displayfeedback_targets(&xml);
        }
    }

    #[test]
    fn emits_frozen_response_presentation_attributes() {
        assert_eq!(
            render_fib("Decimal"),
            "<render_fib charset=\"us-ascii\" columns=\"0\" encoding=\"UTF_8\" fibtype=\"Decimal\" maxchars=\"0\" maxnumber=\"0\" minnumber=\"0\" prompt=\"Box\" rows=\"0\"/>"
        );
        let item = Item::new(
            "Choice attributes".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid MC fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MC renders");
        assert!(xml.contains("<response_label ident=\""));
        assert!(xml.contains("shuffle=\"Yes\" rarea=\"Ellipse\" rrange=\"Exact\""));
    }

    #[test]
    fn multiple_answer_uses_all_choice_predicates_and_choice_ordered_penalties() {
        let item = Item::new(
            "MA question".to_owned(),
            ItemBody::Ma {
                choices: vec![
                    "correct".to_owned(),
                    "incorrect".to_owned(),
                    "also incorrect".to_owned(),
                ],
                answers: vec!["correct".to_owned()],
                min_answers_required: 1,
                allow_all_correct: true,
            },
        )
        .expect("valid MA fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MA renders");
        let first = format!("{}_label_0", item.crc());
        let second = format!("{}_label_1", item.crc());
        let third = format!("{}_label_2", item.crc());
        assert!(xml.contains(&format!(
            "<and><varequal respident=\"response\" case=\"No\">{first}</varequal><not><varequal respident=\"response\" case=\"No\">{second}</varequal></not><not><varequal respident=\"response\" case=\"No\">{third}</varequal></not></and>"
        )));
        let first_penalty = format!(
            "<varequal respident=\"{first}\" case=\"No\"></varequal></conditionvar><setvar"
        );
        let second_penalty = format!(
            "<varequal respident=\"{second}\" case=\"No\"></varequal></conditionvar><setvar"
        );
        assert!(xml.find(&first_penalty) < xml.find(&second_penalty));
    }

    #[test]
    fn emits_only_displayfeedback_references_with_itemfeedback_targets() {
        let item = Item::new(
            "MC question".to_owned(),
            ItemBody::Mc {
                choices: vec!["correct".to_owned(), "incorrect".to_owned()],
                answer: "correct".to_owned(),
            },
        )
        .expect("valid MC fixture");
        let xml = render_item(&item.render_view())
            .expect("rendering succeeds")
            .expect("MC renders");
        assert_displayfeedback_targets(&xml);
        assert!(!xml.contains(&format!(
            "<displayfeedback linkrefid=\"{}_label_0\"",
            item.crc()
        )));
        assert!(!xml.contains(&format!(
            "<displayfeedback linkrefid=\"{}_label_1\"",
            item.crc()
        )));
    }
}
