//! Stable render transport behavior across planning, completion and normal writing.

use qti_wasm::{
    Artifact, ConversionInput, ConvertRequest, ConvertResult, DocumentOptions, RenderCompletion,
    RenderPlanResult, finish_convert_request, plan_render_jobs_request,
};

// Complete 1x1 RGBA PNG, also used by the portable and native render contract tests.
fn png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4, 0, 9,
        251, 3, 253, 251, 94, 107, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
}

fn request() -> ConvertRequest {
    ConvertRequest {
        input_format: "bbq_text_upload".into(), output_format: "bbq_text_upload".into(),
        input: ConversionInput::File { name: "original.txt".into(), companions: Vec::new(),
            bytes: b"MC\t<table><tr><td>one</td></tr></table>\tA\tcorrect\tB\tincorrect\nMC\t<table><tr><td>two</td></tr></table>\tA\tincorrect\tB\tcorrect\n".to_vec() },
        allow_mixed: true, limit: None, output_name: Some("out.txt".into()),
        document: DocumentOptions::default(), shuffle_seed: 0,
    }
}

#[test]
fn equal_pngs_preserve_two_original_items_and_their_grading() {
    let original = request();
    let RenderPlanResult::Success {
        jobs,
        item_count,
        wrapper,
        ..
    } = plan_render_jobs_request(original.clone(), "2026-10-09")
    else {
        panic!("plan failed")
    };
    assert_eq!(item_count, 2);
    assert_eq!(jobs.len(), 2);
    assert!(wrapper.contains("qti-render-root"));
    let renders = jobs
        .into_iter()
        .map(|job| RenderCompletion {
            id: job.id,
            png: png(),
            width: 73.5,
            height: 22.25,
        })
        .collect();
    let ConvertResult::Success {
        artifact: Some(Artifact::File { primary, .. }),
        item_count,
        ..
    } = finish_convert_request(original, renders, "2026-10-09")
    else {
        panic!("finish failed")
    };
    assert_eq!(item_count, 2);
    let output = String::from_utf8(primary.bytes).unwrap();
    assert_eq!(output.lines().count(), 2);
    assert!(output.to_ascii_lowercase().contains(". a\tcorrect"));
    assert!(output.to_ascii_lowercase().contains(". a\tincorrect"));
    assert!(output.contains("73.5"));
    assert!(output.contains("22.25"));
}

#[test]
fn missing_completion_returns_an_actionable_error() {
    let result = finish_convert_request(request(), Vec::new(), "2026-10-09");
    let ConvertResult::Error { error, .. } = result else {
        panic!("missing render accepted")
    };
    assert_eq!(error.category, "render");
    assert!(error.message.contains("missing"), "{}", error.message);
}

#[test]
fn truncated_render_returns_a_job_error_before_packaging() {
    let original = request();
    let RenderPlanResult::Success { jobs, .. } =
        plan_render_jobs_request(original.clone(), "2026-10-09")
    else {
        panic!("plan failed")
    };
    let invalid_id = jobs[0].id.clone();
    let renders = jobs
        .into_iter()
        .map(|job| RenderCompletion {
            png: if job.id == invalid_id {
                png()[..8].to_vec()
            } else {
                png()
            },
            id: job.id,
            width: 73.5,
            height: 22.25,
        })
        .collect();
    let ConvertResult::Error { error, .. } =
        finish_convert_request(original, renders, "2026-10-09")
    else {
        panic!("truncated PNG packaged")
    };
    assert_eq!(error.category, "render");
    assert!(error.message.contains("invalid PNG"), "{}", error.message);
    assert!(error.message.contains(&invalid_id), "{}", error.message);
}
