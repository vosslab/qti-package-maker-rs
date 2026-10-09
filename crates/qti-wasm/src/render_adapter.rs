//! Stateless render orchestration over the original parsed bank and shared writer.

use qti_engines::{AssetOverlay, ReadOutcome};
use qti_render::{BankRenderPlan, RenderJobKind};

use crate::adapter::{self, MAX_ENTRY_BYTES, MAX_FILES, MAX_TOTAL_BYTES, PreparedRequest};
use crate::diagnostics;
use crate::{
    ConvertRequest, ConvertResult, Diagnostic, RenderCompletion, RenderJob, RenderPlanResult,
};

fn read_and_plan(
    request: &ConvertRequest,
    prepared: &PreparedRequest,
) -> Result<(ReadOutcome, BankRenderPlan), Box<Diagnostic>> {
    let read = qti_engines::read_bank(
        &request.input_format,
        prepared.input(),
        request.allow_mixed,
        request.limit.map(|limit| limit as usize),
    )
    .map_err(|error| Box::new(diagnostics::engine_error(&error)))?;
    let assets = AssetOverlay {
        memory: &read.assets,
        fallback: &prepared.assets,
    };
    let entry = qti_engines::engine(&request.output_format).expect("prepared output format");
    let plan =
        qti_render::plan_bank(&read.bank, entry.supported_kinds, &assets).map_err(render_error)?;
    Ok((read, plan))
}

fn render_error(error: qti_render::RenderError) -> Box<Diagnostic> {
    Box::new(Diagnostic::request("render", error.to_string()))
}

/// Creates owned host jobs without retaining a Rust handle or mutable bank.
pub fn plan_render_jobs_request(request: ConvertRequest, default_date: &str) -> RenderPlanResult {
    match plan(&request, default_date) {
        Ok(result) => result,
        Err(error) => RenderPlanResult::Error {
            error: *error,
            warnings: Vec::new(),
        },
    }
}

fn plan(request: &ConvertRequest, date: &str) -> Result<RenderPlanResult, Box<Diagnostic>> {
    let prepared = adapter::prepare_request(request, date)?;
    let (read, plan) = read_and_plan(request, &prepared)?;
    Ok(RenderPlanResult::Success {
        jobs: plan
            .jobs
            .into_iter()
            .map(|job| RenderJob {
                id: job.id,
                kind: match job.kind {
                    RenderJobKind::Table => "table",
                    RenderJobKind::Canvas => "canvas",
                }
                .into(),
                html: job.html,
                canvas_spec: job.canvas_spec.map(Into::into),
                content_hash: job.content_hash,
                dependencies: job.dependencies,
            })
            .collect(),
        wrapper: qti_render::static_document(""),
        item_count: read.bank.len(),
        warnings: read
            .warnings
            .into_iter()
            .map(|warning| {
                diagnostics::read_warning(warning, &request.input_format, &prepared.name)
            })
            .collect(),
    })
}

/// Rebuilds original bindings, validates completions, and writes through the normal writer.
pub fn finish_convert_request(
    request: ConvertRequest,
    renders: Vec<RenderCompletion>,
    date: &str,
) -> ConvertResult {
    match finish(&request, renders, date) {
        Ok(result) => result,
        Err(error) => ConvertResult::Error {
            error: *error,
            warnings: Vec::new(),
        },
    }
}

fn finish(
    request: &ConvertRequest,
    renders: Vec<RenderCompletion>,
    date: &str,
) -> Result<ConvertResult, Box<Diagnostic>> {
    if renders.len() > MAX_FILES {
        return Err(Box::new(Diagnostic::request(
            "inputLimit",
            "render completions exceed 10000 files",
        )));
    }
    let mut total = 0;
    for render in &renders {
        adapter::bound_size(render.png.len(), MAX_ENTRY_BYTES)?;
        total += render.png.len();
        adapter::bound_size(total, MAX_TOTAL_BYTES)?;
    }
    let prepared = adapter::prepare_request(request, date)?;
    let (read, plan) = read_and_plan(request, &prepared)?;
    let renders = renders
        .into_iter()
        .map(|render| qti_render::RenderCompletion {
            id: render.id,
            png: render.png,
            width: render.width,
            height: render.height,
        })
        .collect::<Vec<_>>();
    let (bank, generated) =
        qti_render::finish_bank(&read.bank, &plan, &renders).map_err(render_error)?;
    let originals = AssetOverlay {
        memory: &read.assets,
        fallback: &prepared.assets,
    };
    let assets = AssetOverlay {
        memory: &generated,
        fallback: &originals,
    };
    let write = qti_engines::write_bank(&request.output_format, &bank, &assets, &prepared.context)
        .map_err(|error| Box::new(diagnostics::engine_error(&error)))?;
    Ok(ConvertResult::Success {
        artifact: write.artifact.map(adapter::artifact),
        item_count: read.bank.len(),
        warnings: read
            .warnings
            .into_iter()
            .map(|warning| {
                diagnostics::read_warning(warning, &request.input_format, &prepared.name)
            })
            .chain(write.warnings.into_iter().map(diagnostics::write_warning))
            .collect(),
    })
}
