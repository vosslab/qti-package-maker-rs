//! Stable identity, dependency, completion, and CSS sizing contracts.

use base64::Engine;
use qti_core::media::MemoryAssets;
use qti_core::{Item, ItemBank, ItemBody, ItemKind};

use crate::{RenderCompletion, RenderError, RenderJobKind, finish_bank, plan_bank, prepare_table};

fn png() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=").expect("valid PNG")
}
fn item(stem: &str) -> Item {
    Item::new(
        stem.into(),
        ItemBody::Mc {
            choices: vec!["yes".into(), "no".into()],
            answer: "yes".into(),
        },
    )
    .expect("valid item")
}
fn canvas(id: &str) -> String {
    format!(
        r#"<canvas id="canvas_{id}" width="120" height="80"></canvas><script>let smiles="CC(=O)NCC(=O)O";let mol=RDKitModule.get_mol(smiles);let mdetails={{}};mdetails["bonds"]=getPeptideBonds(mol);mdetails["atoms"]=[0];mdetails["highlightColour"]=[0,1,.5];mdetails["legend"]="peptide";mdetails["explicitMethyl"]=true;mol.draw_to_canvas_with_highlights(canvas,JSON.stringify(mdetails));</script>"#
    )
}
fn completions(plan: &crate::BankRenderPlan) -> Vec<RenderCompletion> {
    plan.jobs
        .iter()
        .map(|job| RenderCompletion {
            id: job.id.clone(),
            png: png(),
            width: 120.5,
            height: 80.25,
        })
        .collect()
}

#[test]
fn identical_rendered_presentations_preserve_distinct_original_items_and_grading() {
    let mut bank = ItemBank::new(false);
    // Different authored markup becomes the same normalized rendered presentation.
    for stem in [
        "<table><tr><td>x</td></tr></table>",
        "<table><tbody><tr><td>x</td></tr></tbody></table>",
    ] {
        bank.add_item(item(stem)).expect("add distinct source");
    }
    let plan = plan_bank(&bank, &[ItemKind::Mc], &MemoryAssets::new()).expect("plan");
    assert_eq!(plan.jobs.len(), 2);
    assert_ne!(plan.jobs[0].id, plan.jobs[1].id);
    assert_eq!(plan.jobs[0].content_hash, plan.jobs[1].content_hash);
    let repeated = plan_bank(&bank, &[ItemKind::Mc], &MemoryAssets::new()).expect("repeat");
    assert_eq!(plan.jobs, repeated.jobs);
    let (output, assets) = finish_bank(&bank, &plan, &completions(&plan)).expect("finish");
    assert_eq!(output.len(), 2);
    for (source, result) in bank.iter_ordered().zip(output.iter_ordered()) {
        assert_eq!(source.crc(), result.crc());
        assert_eq!(source.body(), result.body());
        assert_eq!(source.common().item_number, result.common().item_number);
        assert!(result.common().question_text.contains("width=\"120.5\""));
        assert!(result.common().question_text.contains("height=\"80.25\""));
    }
    assert_eq!(assets.entries().len(), 2);
}

#[test]
fn nested_canvas_dependencies_are_resolved_and_only_visible_images_are_materialized() {
    let mut bank = ItemBank::new(false);
    bank.add_item(item(&format!(
        "<table><tr><td>{}</td></tr></table>{}",
        canvas("nested"),
        canvas("standalone")
    )))
    .expect("add");
    let plan = plan_bank(&bank, &[ItemKind::Mc], &MemoryAssets::new()).expect("plan");
    assert_eq!(plan.jobs.len(), 3);
    let table = plan
        .jobs
        .iter()
        .find(|job| job.kind == RenderJobKind::Table)
        .expect("table");
    assert_eq!(table.dependencies, [plan.jobs[0].id.clone()]);
    let source = plan.jobs[0].canvas_spec.as_ref().expect("molecule spec");
    assert_eq!(source.drawing_details()["atoms"], serde_json::json!([0]));
    let query = source.peptide_query().expect("source-owned query");
    assert_eq!((query.smarts, query.bond_atoms), ("CC(=O)NC", [1, 3]));
    let completed = completions(&plan);
    let html = prepare_table(table, &completed).expect("prepared dependencies");
    assert!(html.contains("data:image/png;base64,"));
    assert!(html.contains("width=\"120.5\""));
    assert!(!html.contains("qti-render:"));
    let (output, assets) = finish_bank(&bank, &plan, &completed).expect("finish");
    assert_eq!(
        assets.entries().len(),
        2,
        "nested PNG is consumed by its table"
    );
    let stem = &output.get(0).expect("item").common().question_text;
    assert!(!stem.contains("<script"));
    assert!(!stem.contains("<canvas"));
    assert!(!stem.contains("qti-render:"));
}

#[test]
fn completion_errors_name_missing_duplicate_unknown_invalid_jobs_and_wrong_original() {
    let mut bank = ItemBank::new(false);
    bank.add_item(item("<table><tr><td>x</td></tr></table>"))
        .expect("add");
    let plan = plan_bank(&bank, &[ItemKind::Mc], &MemoryAssets::new()).expect("plan");
    let completed = completions(&plan);
    assert!(matches!(
        finish_bank(&bank, &plan, &[]),
        Err(RenderError::MissingCompletion(_))
    ));
    assert!(matches!(
        finish_bank(&bank, &plan, &[completed[0].clone(), completed[0].clone()]),
        Err(RenderError::DuplicateCompletion(_))
    ));
    let mut wrong = completed[0].clone();
    wrong.id = "other".into();
    assert!(matches!(
        finish_bank(&bank, &plan, &[wrong]),
        Err(RenderError::UnknownCompletion(_))
    ));
    for (width, bytes) in [(f64::NAN, png()), (0.0, png()), (120.0, vec![7])] {
        let mut invalid = completed[0].clone();
        invalid.width = width;
        invalid.png = bytes;
        assert!(matches!(
            finish_bank(&bank, &plan, &[invalid]),
            Err(RenderError::InvalidCompletion(_))
        ));
    }
    let mut other = ItemBank::new(false);
    other.add_item(item("different")).expect("other");
    assert!(matches!(
        finish_bank(&other, &plan, &completed),
        Err(RenderError::PlanMismatch)
    ));
}

#[test]
fn incomplete_or_corrupt_pngs_fail_before_rewriting_or_inlining() {
    let mut bank = ItemBank::new(false);
    bank.add_item(item(&format!(
        "<table><tr><td>{}</td></tr></table>",
        canvas("nested")
    )))
    .expect("add");
    let plan = plan_bank(&bank, &[ItemKind::Mc], &MemoryAssets::new()).expect("plan");
    let table = plan
        .jobs
        .iter()
        .find(|job| job.kind == RenderJobKind::Table)
        .expect("table");
    let complete = png();
    let mut corrupt = complete.clone();
    corrupt[45] ^= 1; // Damage the image stream while retaining its signature and header.
    for bytes in [
        complete[..8].to_vec(),
        complete[..complete.len() - 12].to_vec(),
        corrupt,
    ] {
        let mut renders = completions(&plan);
        renders[0].png = bytes;
        for error in [
            finish_bank(&bank, &plan, &renders).expect_err("invalid PNG rewritten"),
            prepare_table(table, &renders).expect_err("invalid PNG inlined"),
        ] {
            assert!(
                matches!(error, RenderError::InvalidCompletion(ref id) if id == &renders[0].id)
            );
        }
    }
}
