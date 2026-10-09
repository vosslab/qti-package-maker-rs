# HTML-to-image contract

`--html-to-image` converts selected HTML tables and static RDKit canvases to packaged PNGs for
Canvas QTI 1.2, Blackboard QTI 2.1, Blackboard Original exports, and PLE Native JSON. Table PNGs are rendered by a
local Chromium instance controlled from Rust. Chromium's rendering of the source HTML and CSS is
the presentation authority. Python PNG layout is useful diagnostic evidence, not an acceptance
target.

The production binary uses `chromiumoxide` to launch Chromium lazily and reuse it during one
conversion. It always runs headlessly, preferring a dedicated headless shell on `PATH` or in the
standard local Playwright browser cache (newest installed revision first). Otherwise it discovers
a full Chromium browser and runs that headlessly. Set `QTI_CHROMIUM` to an executable path when automatic local-browser discovery is
unsuitable. Chromium is required only when the selected input actually needs table conversion;
ordinary package generation needs neither Chromium, Python, nor Node.js.

The renderer supplies the selected table in a controlled static document, uses a white page
background, waits for the document to be ready, and captures the table's bounds. It does not run
author-supplied scripts or fetch network resources. Wrapper choices such as viewport size and
fonts serve readable output; they are not a Python-image or pixel-equivalence contract.

Static RDKit canvases remain a separate path. The converter validates the bounded canvas script,
uses the local RDKit shim to create its PNG, and inlines that PNG before Chromium renders a table
that contains it. Existing sugar-library PNG/SVG exports stay images and do not enter the HTML
table renderer.

## Shared rendering ownership

[crates/qti-render/src/lib.rs](../crates/qti-render/src/lib.rs) owns portable selection, canvas
parsing, deterministic job planning, naming, the static wrapper, and completion validation.
Its finalizer rewrites presentation fields while retaining the original bank's CRCs, order,
source item numbers, and grading, including when different items acquire identical presentations.
Generated images carry logical CSS dimensions and responsive sizing; PNG pixel dimensions may
be larger than their displayed dimensions.

The native adapter in
[html_to_image/mod.rs](../crates/qti-native/src/html_to_image/mod.rs) retains Chromium, Rayon,
the render cache, and runtime RDKit loading. Its read-only `AssetSource` is rooted at the input
parent. Generated assets combine with recovered/input providers through
`qti-engines::AssetOverlay` before writing.

The Wasm package exposes stateless `planRenderJobs(originalRequest)` and
`finishConvert(originalRequest, renders)`. A browser host executes planned canvas and table jobs;
Rust owns the source interpretation and packaging. Plans identify canvas dependencies inside
tables and supply `drawingDetails` plus optional source-owned `peptideQuery` metadata for RDKit.
The final call reparses the original request, reconstructs bindings, and overlays generated,
recovered, and companion assets. See
[packages/qti-wasm/docs/rendering.md](../packages/qti-wasm/docs/rendering.md) for the host API.

Every completion supplies a job ID, PNG bytes, and positive finite logical CSS dimensions.
The portable finalizer checks the actual PNG signature, decodes rows with the PNG decoder's
default allocation limit, verifies checksums and the end chunk, and rejects malformed bytes.
Missing, duplicate, unknown, or invalid completions return typed render diagnostics.

Selection follows eligible selected writers' supported kinds. A table's display-image sources
are inlined only when that table is rendered; unrelated references remain lazy. FIB/MULTIFIB
accepted-answer literals are grading fields and stay outside rendering-media scans. The core
`ItemBank::with_rewritten_items` map preserves source identity, kind, order, and source item numbers.
Blackboard Original packaging shares media only when bytes, MIME type, and extension all match.
It retains the earliest item's media ownership and every rewritten reference while preserving
source grading.
Shared-engine correction and acceptance status is recorded in
[shared_engine_delivery.md](active_plans/reports/shared_engine_delivery.md).

## Acceptance evidence

The two table inputs below demonstrate why the previous custom renderer is superseded:

- `3057867908e9cff30bd1536d9056db77e89c3ae85309540d928eaaab8e516c56` is a gel whose band uses
  `box-shadow`; the custom renderer rejected that property.
- `08b73c60946140c7370d9a3ab8d990f833052e384b09b68ae0867fe9715bd360` lost authored colors and
  circles.

The four user-classified malformed HTML sources and two obsolete HTML-sugar sources are excluded
from table-rendering acceptance. Their identifiers and the rationale are recorded in
[../refactor_progress.md](../refactor_progress.md).

Acceptance checks readable content, correct browser rendering of valid source HTML/CSS, package
integrity, media references, and grading. It does not compare pixels or require a Chromium result
to match Python's older human-readable PNG output. Benchmarks measure browser lifecycle and
conversion cost to guide improvements; no fixed speed threshold is a release gate.

Run `cargo xtask table-gallery` after `cargo xtask table-corpus` to build
`output_tables/gallery/run-<timestamp>/index.html`. A gallery is review evidence, not a permanent
test suite. Regenerate it after a rendering change rather than preserving image goldens.
