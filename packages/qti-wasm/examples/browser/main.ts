import { zipSync } from "fflate";
import type { Artifact, ConvertRequest, FormatInfo, NamedBytes } from "../../src/index.js";
import type { WorkerResponse } from "./worker_protocol.js";

function element<T extends Element>(selector: string): T {
  const result = document.querySelector<T>(selector);
  if (!result) throw new Error(`Missing example element: ${selector}`);
  return result;
}

const form = element<HTMLFormElement>("#conversion");
const inputFile = element<HTMLInputElement>("#input-file");
const companionFiles = element<HTMLInputElement>("#companion-files");
const companionNames = element<HTMLOListElement>("#companion-names");
const inputFormat = element<HTMLSelectElement>("#input-format");
const outputFormat = element<HTMLSelectElement>("#output-format");
const button = element<HTMLButtonElement>("#convert");
const status = element<HTMLParagraphElement>("#status");
const diagnostics = element<HTMLPreElement>("#diagnostics");
const downloads = element<HTMLUListElement>("#downloads");
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
const objectUrls: string[] = [];
let availableFormats: FormatInfo[] = [];
let companions: { file: File; name: HTMLInputElement }[] = [];

companionFiles.addEventListener("change", () => {
  companionNames.replaceChildren();
  companions = Array.from(companionFiles.files ?? []).map((file, index) => {
    const name = document.createElement("input");
    name.type = "text";
    name.value = file.webkitRelativePath || file.name;
    name.setAttribute("aria-describedby", "companion-help");
    const label = document.createElement("label");
    label.append(`Relative name for companion ${index + 1} (${file.name}) `, name);
    const item = document.createElement("li");
    item.append(label);
    companionNames.append(item);
    return { file, name };
  });
});

function clearDownloads(): void {
  for (const url of objectUrls) URL.revokeObjectURL(url);
  objectUrls.length = 0;
  downloads.replaceChildren();
}

function addDownload(file: NamedBytes): void {
  const url = URL.createObjectURL(new Blob([new Uint8Array(file.bytes)], { type: "application/octet-stream" }));
  objectUrls.push(url);
  const link = document.createElement("a");
  link.href = url;
  // ASVS 3.2.1: generated content is downloaded, never inserted as authored HTML.
  link.download = file.name;
  // ASVS 1.2.1, 3.2.2: authored names and diagnostics are rendered as text.
  link.textContent = file.name;
  const item = document.createElement("li");
  item.append(link);
  downloads.append(item);
}

function addArtifact(artifact: Artifact): void {
  if (artifact.kind === "file" && artifact.companions.length === 0) {
    addDownload(artifact.primary);
    return;
  }
  const entries: Record<string, Uint8Array> = Object.create(null) as Record<string, Uint8Array>;
  let name: string;
  if (artifact.kind === "directory") {
    name = artifact.name;
    for (const file of artifact.entries) entries[`${artifact.name}/${file.name}`] = file.bytes;
  } else {
    name = artifact.primary.name;
    entries[name] = artifact.primary.bytes;
    const parent = name.slice(0, name.lastIndexOf("/") + 1);
    for (const file of artifact.companions) entries[`${parent}${file.name}`] = file.bytes;
  }
  // This ZIP only persists the returned artifact; relative names and payloads come from Rust.
  addDownload({ name: `${name.slice(name.lastIndexOf("/") + 1)}.zip`, bytes: zipSync(entries) });
  const contents = document.createElement("ul");
  contents.setAttribute("aria-label", "Archive contents");
  for (const entry of Object.keys(entries)) {
    const item = document.createElement("li");
    item.textContent = entry;
    contents.append(item);
  }
  downloads.lastElementChild?.append(contents);
}

function populate(select: HTMLSelectElement, formats: FormatInfo[]): void {
  for (const format of formats) {
    const option = document.createElement("option");
    option.value = format.name;
    option.textContent = format.name;
    select.append(option);
  }
}

worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
  const message = event.data;
  if (message.type === "ready") {
    availableFormats = message.inventory.formats;
    populate(inputFormat, availableFormats.filter((format) => format.canRead));
    populate(outputFormat, availableFormats.filter((format) => format.canWrite));
    inputFormat.value = "bbq_text_upload";
    outputFormat.value = "blackboard_qti_v2_1";
    status.textContent = "Ready";
    status.dataset.initMs = String(message.initMs);
    button.disabled = false;
    return;
  }
  button.disabled = false;
  if (message.type === "error") {
    status.textContent = "Conversion failed";
    diagnostics.textContent = message.message;
    return;
  }
  status.dataset.conversionMs = String(message.conversionMs);
  const result = message.result;
  diagnostics.textContent = result.warnings.map((warning) => `${warning.stage}: ${warning.category}: ${warning.message}`).join("\n");
  if (result.status === "error") {
    status.textContent = "Conversion failed";
    diagnostics.textContent += `\n${result.error.category}: ${result.error.message}`;
    return;
  }
  status.textContent = `Converted ${result.itemCount} item(s)`;
  if (!result.artifact) return;
  addArtifact(result.artifact);
};
worker.onerror = (event) => {
  status.textContent = "Worker failed";
  diagnostics.textContent = event.message;
  button.disabled = true;
};

async function namedBytes(file: File, name = file.webkitRelativePath || file.name): Promise<NamedBytes> {
  return { name, bytes: new Uint8Array(await file.arrayBuffer()) };
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  void submit().catch((error: unknown) => {
    status.textContent = "Conversion failed";
    diagnostics.textContent = error instanceof Error ? error.message : String(error);
    button.disabled = false;
  });
});

async function submit(): Promise<void> {
  clearDownloads();
  diagnostics.textContent = "";
  button.disabled = true;
  status.textContent = "Converting in worker...";
  const file = inputFile.files?.[0];
  if (!file) throw new Error("Select a question file");
  const inputCompanions = await Promise.all(companions.map(({ file, name }) => namedBytes(file, name.value)));
  const output = availableFormats.find((format) => format.name === outputFormat.value && format.canWrite);
  if (!output) throw new Error("Select an output format");
  const input = await namedBytes(file);
  const request: ConvertRequest = {
    inputFormat: inputFormat.value,
    outputFormat: outputFormat.value,
    input: { kind: "file", ...input, companions: inputCompanions },
    allowMixed: true,
    outputName: output.defaultOutputName,
    document: { title: "Browser question bank", date: "2026-01-01" },
    shuffleSeed: 0,
  };
  worker.postMessage(request);
}

window.addEventListener("beforeunload", () => {
  clearDownloads();
  worker.terminate();
});
