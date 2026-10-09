import init from "../generated/qti_wasm.js";

/** Initialize explicitly from release Wasm bytes in browsers, workers, or Node. */
export async function initialize(bytes: Uint8Array): Promise<void> {
  await init({ module_or_path: new Uint8Array(bytes) });
}

export { formats, convert, checkPackage, planRenderJobs, finishConvert } from "../generated/qti_wasm.js";
export type {
  CanvasSpec,
  DrawingDetails,
  PeptideQuery,
  RenderJob,
  RenderCompletion,
  RenderPlanResult,
  Artifact,
  CheckPackageResult,
  ConversionInput,
  ConvertRequest,
  ConvertResult,
  Diagnostic,
  DocumentOptions,
  FormatInfo,
  FormatInventory,
  IntegrityFinding,
  IntegrityReport,
  NamedBytes,
  PackageInput,
  Warning,
} from "../generated/qti_wasm.js";
