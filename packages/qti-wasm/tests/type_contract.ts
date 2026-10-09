import { checkPackage, convert, formats } from "@vosslab/qti-wasm";
import type { ConvertRequest, ConvertResult, NamedBytes } from "@vosslab/qti-wasm";

const request: ConvertRequest = {
  inputFormat: "bbq_text_upload",
  outputFormat: "ple_native_json",
  input: { kind: "file", name: "bank.txt", bytes: new Uint8Array() },
};
const result = convert(request);
if (result.status === "success") {
  if (result.artifact?.kind === "file") {
    const bytes: Uint8Array = result.artifact.primary.bytes;
    void bytes;
  } else if (result.artifact?.kind === "directory") {
    const entries: NamedBytes[] = result.artifact.entries;
    void entries;
  }
  // @ts-expect-error success results do not expose an error diagnostic
  result.error;
} else {
  const message: string = result.error.message;
  const logicalName: string | undefined = result.error.logicalName;
  void [message, logicalName];
  // @ts-expect-error error results do not expose an artifact
  result.artifact;
}
const checked = checkPackage({ kind: "zip", bytes: new Uint8Array() });
if (checked.status === "success") checked.report.errors.map((finding) => finding.message);
else checked.error.message;
formats().formats.filter((format) => format.canRead);

// @ts-expect-error bytes must be Uint8Array, not an ordinary number array
const wrongBytes: NamedBytes = { name: "x", bytes: [1, 2] };
// @ts-expect-error input kinds are a closed generated discriminant
const wrongInput: ConvertRequest = { ...request, input: { kind: "url", name: "x", bytes: new Uint8Array() } };
// @ts-expect-error exact optional properties reject explicitly undefined booleans
const undefinedOption: ConvertRequest = { ...request, allowMixed: undefined };
void [wrongBytes, wrongInput, undefinedOption];

// Render API consumes the original request and owned PNG completions without handles.
import { planRenderJobs, finishConvert } from "@vosslab/qti-wasm";
import type { RenderCompletion, RenderPlanResult } from "@vosslab/qti-wasm";
const renderPlan: RenderPlanResult = planRenderJobs(request);
if (renderPlan.status === "success") {
  const completions: RenderCompletion[] = renderPlan.jobs.map((job) => ({
    id: job.id, png: new Uint8Array(), width: 100, height: 50,
  }));
  const finished: ConvertResult = finishConvert(request, completions);
  void finished;
}
