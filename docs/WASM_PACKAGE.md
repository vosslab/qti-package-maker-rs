# WebAssembly package

`packages/qti-wasm` is a private local ES-module package named `@vosslab/qti-wasm`. It builds the
same Rust core, integrity checker, four readers, and eleven writers used by the native CLI.
Generated declarations come from Rust `tsify` transport types and wasm-bindgen exports. TypeScript
owns initialization and host examples; conversion algorithms remain in Rust.

The package targets `wasm32-unknown-unknown`, not WASI. Screenshot and molecule rendering require
native host services and are available through the CLI. This workflow builds local artifacts;
it does not publish an npm package or deploy a site.

## Build and verify

Use Rust 1.98.1 or later and Node 24 for the tested package workflow:

```bash
rustup target add wasm32-unknown-unknown
cd packages/qti-wasm
npm ci
npm run build
npm run typecheck
npm test
npx playwright install chromium firefox webkit
npm run test:browser
```

`npm ci` installs the lockfile's local build tools, including wasm-pack. Build invokes
`wasm-pack build --target web --release`, emits Rust-generated declarations and Wasm into
`generated/`, then compiles TypeScript and copies the generated runtime into `dist/`.
`typecheck` includes `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, and
`verbatimModuleSyntax`; workers use a separate WebWorker library configuration.

Missing toolchain targets, dependencies, browsers, or failed checks are failures of the selected
lane. Install or correct the reported prerequisite and rerun. The native CLI builds independently
with Cargo and requires no Node or Wasm setup; see [INSTALL.md](INSTALL.md).

## Initialize explicitly

Load the Wasm bytes once per JavaScript realm before calling any operation. Browser and worker
examples use a URL resolved against the importing module; serve the build over HTTP:

```typescript
import { convert, formats, initialize } from "./dist/src/index.js";

const response = await fetch(new URL("./dist/generated/qti_wasm_bg.wasm", import.meta.url));
if (!response.ok) throw new Error(`Wasm download failed: ${response.status}`);
await initialize(new Uint8Array(await response.arrayBuffer()));
console.log(formats().formats);
```

Node loads the same module and bytes with `readFile`; no browser globals or network loader are
needed. A worker initializes its own realm. The built examples provide these paths:

```bash
npm run example:node -- INPUT INPUT_FORMAT OUTPUT_FORMAT OUTPUT_DIRECTORY [COMPANION ...]
npm run example:browser
```

For example, run from `packages/qti-wasm`:

```bash
npm run example:node -- ../../bbq-demo-questions.txt bbq_text_upload canvas_qti_v1_2 /tmp/qti-demo
```

The Node example writes only new files under its supplied destination. For a PLE directory
artifact it writes the returned entries directly into that destination; it adds no native
ownership manifest. The browser interface uses a worker for conversion and offers downloads. It
accepts ordinary multiple-file selection; after selecting companions, edit each relative name to
match authored HTML such as `images/cell.png`. It does not currently offer a directory picker.

## Convert owned bytes

The generated API accepts a discriminated request and returns a discriminated result:

```typescript
const result = convert({
  inputFormat: "bbq_text_upload",
  outputFormat: "canvas_qti_v1_2",
  input: {
    kind: "file",
    name: "questions.txt",
    bytes: new TextEncoder().encode("MC\tWhich base pairs with A?\tT\tcorrect\tC\tincorrect\n"),
    companions: [],
  },
  allowMixed: true,
  outputName: "practice.zip",
  document: { title: "Genetics", date: "2026-10-08" },
  shuffleSeed: 0,
});
if (result.status === "error") throw new Error(result.error.message);
console.log(result.itemCount, result.warnings, result.artifact);
```

`input.kind: "file"` carries source bytes and optional named companion assets. Blackboard export
also accepts `input.kind: "entries"` with a logical name and named extracted ZIP members.
Supply images with the exact relative POSIX spelling used in the authored HTML. The Node companion
argument is a basename: `npm run example:node -- INPUT INPUT_FORMAT OUTPUT_FORMAT OUTPUT_DIRECTORY
cell.png` reads the host file `cell.png` and uses its basename as the logical name. Use
`ASSET_NAME=PATH` when the logical name differs from the host path: `images/cell.png=assets/cell.png`
reads `assets/cell.png` but supplies the nested logical name `images/cell.png`. Names allow
spaces and UTF-8, and reject absolute paths, traversal, backslashes, colons, empty components,
controls, and trailing slashes. File companions permit identical duplicate names and bytes;
conflicting companion payloads fail. Extracted package entries reject every duplicate name,
including identical payloads. The lower-level Rust memory provider also permits repeated identical
insertions.

Inputs are copied into Rust ownership during the synchronous call. Returned `Uint8Array` values
are independent owned copies; changing an input or an earlier output cannot mutate later output.
A worker may transfer its copies using an explicit transfer list, which detaches those arrays in
the sending realm. Neither engine providers nor package loading fetch authored external media.

The returned artifact is either `null`, a `file` with `primary` and `companions`, or a `directory`
with `name` and `entries`. Companion names are relative to the primary file's parent. Directory
entries are relative to the artifact directory. PLE returns compact question JSON and associated
media bytes; native CLI persistence alone adds `.qpm-ple-native-json` and verified replacement.
The package caller chooses how to store or download artifacts. A ZIP primary can be downloaded
as a Blob; directory exports require saving all named entries together.
The browser example offers one ZIP download envelope for a directory or a file with companions,
preserving every relative name and byte payload. A directory envelope includes its artifact name
as the root folder. A single primary without companions downloads directly. This host packaging
does not change the shared `file`/`directory` artifact API or reinterpret question content.

## Defaults and diagnostics

Omitted `outputName` uses the shared registry default. Omitted document title is `Exam`; omitted
date is UTC today, resolved once in the Wasm host adapter. Omitted `shuffleSeed` is zero.
`allowMixed` defaults to false and omitted `limit` retains all items. Supply output name, title,
date, and seed explicitly for reproducible native/Wasm comparisons. The seed is an unsigned
32-bit transport value; shared Rust context stores it as `u64`.

Context metadata affects exam YAML title/heading/date. ZIP package titles preserve their
established fixed text. Reference/placeholder-only formats can report media decisions without
reading companion payloads; text2qti reads companions because it emits media files.

Expected request, parser, media, and conversion failures return `status: "error"` with a typed
category and available format, item, and source provenance. Optional `logicalName` retains the
outer input/output name separately from an authored media or archive-member `source`. Success warnings retain reader then
writer order. Artifact absence preserves the writer's established no-output behavior; `itemCount`
is the trimmed bank count, not the number of representable items for every writer. Initialization
or unexpected transport serialization failures can throw a JavaScript exception.
Unknown request fields are rejected at the JavaScript boundary before tsify deserialization,
matching the native Rust transport's serde contract, including nested document/file fields.

## Check package bytes

```typescript
import { checkPackage } from "./dist/src/index.js";

const checked = checkPackage({ kind: "zip", bytes: zipBytes });
if (checked.status === "error") throw new Error(checked.error.message);
console.log(checked.report.errors, checked.report.warnings, checked.report.entryCount);
```

An extracted package uses `{ kind: "entries", entries: namedFiles }`. The shared integrity input
layer validates names, duplicates, member types, supported compression, and bounded sizes before
inspection. Transport input allows at most 10,000 files, 32 MiB per named file, and 256 MiB total;
ZIP/document byte input is also bounded at 256 MiB. `checkPackage` evaluates package structure,
media, and grading bindings. PLE source JSON remains a handoff to PLE's decoder rather than a QTI
ZIP integrity target.

## Acceptance and measurements

`npm test` exercises Node host transport and compares the native Rust transport oracle with generated
Wasm across every reader/writer, supported kinds, warnings, artifacts, and failures; it needs Cargo
and the repository checkout. `npm run test:parity` is a convenience command for running just the
parity tests. `npm run test:browser` runs Chromium,
Firefox, and WebKit acceptance, including workers and downloads. `npm run measure` reports the
Wasm, loader, and wrapper raw/compressed sizes with runtime version information. Repeated conversion timings
are recorded separately with their workload and commands. Exact source-bound results and open gates live in
[shared_engine_delivery.md](active_plans/reports/shared_engine_delivery.md).

The opt-in `RUST_RELEASE_WASM=1` lane in
[rust_release_check.sh](../devel/rust_release_check.sh) runs the portable target check and the
package build, typecheck, Node host/parity, and browser commands. No publication is part of that lane.
