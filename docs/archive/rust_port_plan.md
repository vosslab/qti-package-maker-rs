# Plan: Rust port of qti-package-maker with native table rasterization

## Completion: 2026-10-01

Implementation is complete under the Chromium amendment and the user's manual-release direction.
The refreshed corpus has 721 integrity-clean ZIPs and zero public-content/grading differences;
all 290 gallery entries render. macOS and Linux builds/checks pass, including Linux x86_64
cross-builds. The measured Chromium first-capture delay is a documented performance limitation.
Real LMS imports, native amd64 browser validation, human commits, and publication remain delivery
activities, not claims made by this implementation receipt.
See [refactor_progress.md](../../refactor_progress.md) for the final evidence and historical ledger.


## Context

`rust-port-plan.md` (untracked at the repo root, written against Python commit `29fdd36`,
2026-08-10) planned a CLI-parity Rust port of `qti-package-maker`. The Python package has moved
since through deliberate upgrades, including one whole subsystem the old plan never mentions:
`html_to_image/`. It turns every
HTML table, and every RDKit `<canvas>`, into a packaged PNG so Blackboard Ultra, which strips table
styles and runs no item JavaScript, still shows gels, restriction maps, and wells.

`html_to_image` is the slowest part of the Python package, and the slowness is structural:

- `TableRenderer` launches Playwright Chromium (`render_table.py:131-133`).
- Each of `-1`, `-2`, `-B` calls `transform.convert_bank` independently
  (`canvas_qti_v1_2/engine_class.py:261`, `blackboard_qti_v2_1/engine_class.py:254`,
  `blackboard_export_zip/engine_class.py:152`): a three-format run launches Chromium three times
  and rasterizes every table three times.
- Every table opens a new page, re-parses ~225 KB of base64 `@font-face` CSS, runs two
  `evaluate` round trips plus a locator screenshot, then closes the page, serially
  (`render_table.py:151-178`).
- The render cache is per item (`transform.py:209`); identical tables in different items render
  again.

The tables themselves are simple. A survey of the 77 `biology-problems` generator files that emit
`<table>` found: `border` / `border-*` (200+), `background-color` / `bgcolor`, `text-align` /
`align` / `valign`, `color`, `border-collapse`, `padding` / `cellpadding` / `cellspacing`,
`font-size`, `font-weight`, fixed `width` / `height`, `colspan` (70), `rowspan` (32),
`font-family: monospace`, and inline `sub`, `sup`, `b`, `i`, `span`, `br`, `p`. The one exotic
case, `table_curve_lib.py`, draws sigmoid curves from fixed-size cells using `border-radius:50%`
dots, `1px dashed` guide borders, `font-size:0`, and `overflow:hidden`. The user states the scope
directly: very basic tables with alignment, monospace, color, fixed widths and heights, and
colspan/rowspan. The acceptance truth is equally direct: the images must look like HTML tables.

A browser engine is far more machinery than that subset needs, and it is the source of the
startup, IPC, and install cost. This plan replaces it with a native Rust rasterizer for a declared
HTML/CSS table subset, and updates the rest of the port plan to the current Python code.

## 2026-10-01 Chromium rendering amendment

This amendment supersedes the earlier native-rasterizer direction wherever the two conflict. The
custom renderer has two demonstrated content failures: gel
`3057867908e9cff30bd1536d9056db77e89c3ae85309540d928eaaab8e516c56` rejected `box-shadow`, and
pathway `08b73c60946140c7370d9a3ab8d990f833052e384b09b68ae0867fe9715bd360` lost colors and
circles. HTML/CSS is a moving target, so this port uses local Chromium through Rust's
`chromiumoxide` 0.9.1 rather than maintaining a partial browser engine.

- Table rendering launches Chromium lazily, reuses it during a conversion, and supports
  `QTI_CHROMIUM` as an executable-path override. It needs no Python or Node.js production runtime.
- A controlled static wrapper renders the selected table. Readable output and Chromium's rendering
  of valid source HTML/CSS are authoritative; the Python PNG and pixel equality are not.
- Static RDKit canvases continue through their bounded parser and local shim before table render.
  Existing sugar-library PNG/SVG exports bypass HTML rendering.
- The four user-classified malformed HTML sources and two obsolete HTML sugars recorded in
  [../../refactor_progress.md](../../refactor_progress.md) are outside table acceptance.
- Benchmark evidence measures browser lifecycle and conversion cost. It guides practical
  improvements but does not require a fixed time or Rust faster than Python.

Earlier M16/M17 work-package detail and native gallery evidence remain historical context. This
amendment defines the implementation and acceptance direction from here forward.

All remaining references below to a native subset, `qti-raster`, native layout tests, typed
unsupported-table errors, or a Rust-faster requirement describe the superseded proposal. They are
retained to preserve the plan's history and evidence trail; they are not current gates or work.

## Objectives

- Deliver a Rust `bbq-converter` binary with feature parity with `tools/bbq_converter.py` at the
  current Python head, including `--html-to-image`, the new long-form flag aliases, and the `bez-`
  output naming.
- Render `--html-to-image` tables with a native Rust rasterizer that needs no browser, no Python,
  and no network, and whose output reads as an HTML table rendering of the same markup.
- Make `--html-to-image` markedly faster than the Python path on the harvested table corpus,
  shown by measuring both on the same inputs and machine, with the time attributed per stage.
- Rasterize each distinct table once per run, whatever the number of output formats, and render
  distinct tables in parallel.
- Reject any table markup outside the supported subset with a typed error that names the item,
  the feature, and a snippet, before any output is written.
- Render RDKit canvases, standalone and inside tables, from the same static `CanvasSource`
  extraction Python uses, never executing item JavaScript.
- Carry forward every settled decision of `rust-port-plan.md` that the Python upgrades
  since 2026-08-10 left intact, and update the ones they superseded.

## Design philosophy

The core trade-off: give up "whatever Chromium does" in exchange for a closed, declared subset that
the port renders fast and without a browser runtime dependency. This works only
because the input really is closed: the tables come from a known family of generators, and the
visual goal is a correct HTML table, not agreement with Chromium's pixels. An explicit allowlist
turns "we hope the renderer handles it" into a checked contract. Markup outside the list fails
loudly, with the feature named, so the list grows deliberately. This is **fix the design, not the
symptom** and **ground requirements in actual needs**.

Rejected alternative: keep Chromium and drive it well from Rust over CDP (for example with
`chromiumoxide`). That means one launch per run, fonts loaded once, all tables batched onto one
page, and clip screenshots. It would remove most of the per-table overhead with near-zero fidelity
risk, but it keeps a browser download as a hard dependency of a flag, a process-launch floor of
roughly a second, and serialized screenshots. It also contradicts the single-static-binary
objective. It stays documented as the recovery path if the native renderer fails its review gate
(see `## Risk register`).

Rejected alternative: a general Rust HTML engine (Blitz: Stylo + Taffy + Parley). It is broader
than needed and pre-1.0. A general engine also accepts markup silently where the subset contract
should reject it.

Speed comes from two separate levers, and the plan measures both rather than assuming them:

- Structural: convert once per run, cache by content hash, parallelize. These fix the Python
  design regardless of which renderer is used.
- Per-table: a native layout-and-paint pipeline replaces a browser round trip.

- Evidence strategy for uncertain methods: the table corpus is harvested from the real generators
  before any renderer code exists (M15). The Python baseline is timed per stage on that corpus
  (M15). The text and raster stack is chosen by a spike on the hardest corpus tables (M16). The
  RDKit binding is chosen by a spike against the canvas corpus (M18). Every decision records its
  command, its input, and its observed result in `docs/DEPENDENCY_DECISIONS.md`.

## Scope

- Carry forward the old plan's workspace, crates, milestones M1-M14, media contract, parity
  authority, trait shape, and test classification. The amendments listed in
  `## Changes carried from Python upgrades` apply.
- Add crate `qti-raster`: HTML table fragment in, PNG bytes out. It knows nothing about items,
  banks, or engines.
- Add module `qti-engines::html_to_image`: table and canvas selection, the static RDKit script
  parser, bank conversion, image naming, alt text, a content-hash render cache, and parallel
  rendering.
- Add the RDKit canvas renderer behind a runtime-loaded native library, so the binary builds and
  runs without RDKit until a canvas is actually converted.
- Add the `--html-to-image` CLI flag with Python's engine restrictions, and convert the bank once
  per run before fanning out to the engines.
- Add `cargo xtask table-corpus`, `cargo xtask table-bench`, and `cargo xtask table-gallery`
  (a side-by-side review page).
- Add `docs/HTML_TO_IMAGE.md` documenting the supported subset as a contract.

## Non-goals

- Leave PyO3 bindings to the next plan (user decision). Downstream Python callers keep importing
  the Python package. Recorded imports the next plan must cover: `item_bank`, `item_types`,
  `validator`, `string_functions`, `yaml_tools`, `color_wheel`, `anti_cheat`,
  `bbq_text_upload.read_package/write_item`, `human_readable.write_item`, `package_interface`,
  `html_to_image.render_table` (including `render_mathml_png`), `selectors`, and `render_canvas`.
- Leave `TableRenderer.render_mathml_png` to the PyO3 plan. Only exam-formatting-tools calls it,
  and only through Python. `qti-raster` keeps its inline layout open so a MathML subset can be
  added there later.
- Leave `color_theory/`, `color_wheel.py`, `anti_cheat.py`, and `franken_bptools.py` in Python for
  this plan. No engine and no CLI path uses them; they are library exports for the PyO3 plan.
- Match table appearance, not Chromium pixels. No golden-pixel tests. User decision:
  legible and correct.
- Support only the declared table subset. General HTML, external stylesheets, `<script>`,
  CSS positioning, floats, and web fonts beyond the two bundled faces are rejected by name.
- Modify nothing in the Python repository. Python defects found during the port are reported
  with evidence in the changelog and handled under the parity authority.

## Current state summary

Python facts that changed since the old plan, from the code survey:

| Area | Old plan assumed | Code now (Python head) | Port consequence |
| --- | --- | --- | --- |
| `html_to_image/` | absent | `transform`, `selectors`, `render_table`, `render_canvas` | New M15-M19 |
| Engines | 10 | same 10; `canvas_qti_v1_2`, `blackboard_qti_v2_1`, `blackboard_export_zip` take `html_to_image` options | Engine options struct on those three writers |
| BB export reader | one module | split into `read_package`, `read_items`, `read_media` | M10's five-file split still fits; map to the new Python files |
| `html_selftest` | one engine | adds `control_styles`, `drag_controls`, `match_controls`, `order_controls` (native drag, keyboard, touch; MATCH slots 180x44) | WP-E1 grows; embed JS/CSS assets verbatim |
| NUM validation | finite floats | rejects booleans and non-finite answer and tolerance | WP-A3 adds two error variants |
| CLI flags | WP-F2 list | adds `--html-to-image`, `--input_file`, `--output_file`, `--question_limit`, `--<engine_name>` on every shortcut, a startup version check, silent skip of engines that cannot write | WP-F2 amended |
| Output names | `qti12-`, `selftest-` named | `qti12-`, `qti21-`, `bez-` (new), `selftest-`, `human-*.html`, `bbq-`, `aiken-`, `exam-*.yaml`, `okla-`, `text2qti-` | WP-F2 asserts all ten |
| Dependencies | lxml, pyyaml, crcmod, num2words | adds required `playwright` (+ `playwright install chromium`), optional `rdkit` | Rust needs neither Chromium nor Python |
| Fonts | none | Atkinson Hyperlegible Next and Mono variable TTFs (OFL), embedded into table screenshots | `qti-raster` embeds both via `include_bytes!` with their OFL files |

Python `html_to_image` behavior the port reproduces:

- Every outermost `<table>` in every HTML field becomes one PNG; nested tables render inside it
  (`selectors.py:147`).
- Canvases render first. A canvas inside a table is inlined into the table's render and is not
  packaged separately; standalone canvases are packaged (`transform.py:214-241`).
- Names are `<original item crc16>_<family>_<n>.png`, with `n` counted per item and per family
  (`transform.py:226`). The rebuilt item gets a new CRC; the PNG keeps the old one.
- Alt text: tables use whitespace-collapsed ASCII cell text, or `table drawing` when empty.
  Canvases use `SMILES <s>` or `<legend> (SMILES <s>)`.
- A renderer error raises before any media directory is created (two-phase conversion).
- Serialization re-escapes non-ASCII as character references, so item CRCs stay ASCII.
- Tables render on white with the Atkinson Next font as the default family, at device scale
  factor 2, and the image is clipped to the table's border box.

Suspected Python defect (unconfirmed, not run): `convert_bank` builds a new `ItemBank` without
carrying `media_base_dir` (`transform.py:359`). A bank with existing local `<img>` files probably
fails to resolve them after conversion. WP-T5 reproduces it against Python first. If it is
confirmed, package integrity (tier 2 of the parity authority) outranks Python behavior (tier 3):
the port carries the base directory forward, and the finding is recorded for an upstream fix.

## Changes carried from Python upgrades

The Python package gained deliberate upgrades after the old plan was written; the port targets
them. Amendments to `rust-port-plan.md`. Everything not listed here carries forward unchanged.

- WP-A3: add `ValidationError` variants for a boolean or non-finite NUM answer and tolerance.
- WP-D4a..e: map the five Rust source files onto Python's new `read_package` / `read_items` /
  `read_media` split. Keep the five-file Rust granularity.
- WP-E1: port the four new control modules. Embed their JS and CSS as verbatim assets
  (`include_str!`) so the harness compares behavior, not re-typed code.
- WP-F2: add the flags and naming listed in `## Current state summary`. `--html-to-image` is
  accepted only when a requested engine is one of `canvas_qti_v1_2`, `blackboard_qti_v2_1`, or
  `blackboard_export_zip`. It exits with code 2 when `-o` targets another engine, and other
  engines in a multi-format run ignore it, as `bbq_converter.py:156-181` does. Survey the Python
  `_check_version()` and port its observable behavior.
- WP-F4 parity harness: add `--html-to-image` runs. Compare converted item HTML structurally:
  every table becomes an `<img>` at the same position with equivalent authored alt text, and
  each image reference resolves to its packaged PNG. Generated names may differ when unique
  within the package. PNG content is judged by the M17 review gate, not by the harness.
- M12 registry: the three packaging writers take an `EngineOptions { html_to_image: bool }`.
- Remove the old plan's ungrounded requirements rather than carry them:
  - every line-count figure;
  - the 250-character README limit (house style sets 350);
  - the "release within the last year" YAML crate rule (use "actively maintained, round-trips
    the `exam_yaml` shape");
  - any byte-identical or pixel-identical expectation. Structural XML and ZIP equivalence was
    already the old plan's rule and stays.
  Each surviving gate names the failure it prevents.

## Supported table subset

This contract is what `qti-raster` implements and what `docs/HTML_TO_IMAGE.md` publishes. M15's
corpus harvest confirms or extends it before M17 freezes it. Anything outside it is a typed
`UnsupportedTableFeature` error naming the tag, attribute, or property.

| Class | Supported |
| --- | --- |
| Table structure | `table`, `caption`, `thead`, `tbody`, `tfoot`, `tr`, `td`, `th`, `colgroup`/`col` if the corpus uses them; nested tables inside cells |
| Table attributes | `border`, `cellpadding`, `cellspacing`, `width`, `height`, `align`, `valign`, `bgcolor`, `colspan`, `rowspan` |
| Inline content | text, `span`, `b`, `strong`, `i`, `em`, `sub`, `sup`, `br`, `p`, `div` (block), `ul`/`ol`/`li` if present in cells, `img` with a `data:` PNG/SVG source (canvas output) |
| Box properties | `width`, `height`, `max-width`, `padding[-side]`, `margin` (block content in cells), `overflow: hidden`, `table-layout`, `border-collapse`, `border-spacing`, `vertical-align`, `text-align`, `white-space`, `line-height` |
| Borders | `border`, `border-{top,right,bottom,left}`, `border-width/style/color`; styles `none`, `solid`, `dashed`, `dotted`, `double`; `border-radius` (cell background clip, including `50%` dots) |
| Paint and text | `background`, `background-color`, `color`, `font-size` (including `0`), `font-weight` (normal/bold/numeric), `font-style`, `font-family` resolved to Atkinson Next (default and `sans-serif`) or Atkinson Mono (`monospace`) |
| Colors | hex (3/6/8), `rgb()`/`rgba()`, CSS named colors |
| Units | `px`, `%`, `em`, unitless zero, and `pt` if the corpus uses it |

Layout follows CSS 2.1 section 17: the automatic table layout algorithm (min/max content widths,
colspan distribution, specified widths honored), the fixed table layout when requested, both
border models, and rowspan height distribution. Default styles match the HTML user-agent
defaults for these elements (for example `th` bold and centered; `td` padding 1px under
`cellpadding` absence; `table` border-spacing 2px in the separated model), recorded as named
constants in one module.

## Architecture boundaries and ownership

New and changed components. The rest follows the old plan's layout.

```text
crates/
  qti-core/        (unchanged responsibilities)
  qti-integrity/   (unchanged)
  qti-raster/      NEW: parse -> style -> table layout -> inline layout -> paint -> PNG
    src/lib.rs            facade: render_table_png(html, &RasterConfig) -> Result<Png, RasterError>
    src/subset.rs         allowlist and UnsupportedTableFeature
    src/style.rs          inline style + presentational attribute cascade, UA defaults
    src/table_layout.rs   CSS 2.1 section 17 auto/fixed layout, spans, border models
    src/inline_layout.rs  parley text runs, sub/sup, br, bold/italic, alignment
    src/paint.rs          tiny-skia backgrounds, borders, radius, glyphs, images
    src/fonts.rs          embedded Atkinson Next + Mono, OFL files alongside
  qti-molecule/    NEW: CanvasSource -> PNG via runtime-loaded RDKit (M18 decides binding)
  qti-engines/
    src/html_to_image/  selectors.rs, canvas_script.rs, convert.rs, naming.rs, cache.rs
  qti-cli/         --html-to-image; converts once before engine fan-out
xtask/             + table_corpus.rs, table_bench.rs, table_gallery.rs
```

- `qti-raster` depends on no other workspace crate. It is testable alone and reusable by the
  PyO3 plan and exam-formatting-tools.
- `qti-molecule` loads RDKit at runtime, so every crate builds unconditionally (the old plan's
  "every engine compiles" decision). A canvas conversion without RDKit returns
  `MoleculeError::RdkitUnavailable` naming the install step, matching Python's lazy
  `ImportError`.
- `html_to_image::convert_bank(&ItemBank, &ConvertOptions) -> Result<ItemBank, HtmlToImageError>`
  is pure over its input. It renders everything into memory first, then builds the new bank, as
  Python's two phases do. The CLI calls it once; the three writers call it only when invoked with
  `html_to_image` directly as library users.
- The render cache is keyed by sha256 of the prepared fragment plus the renderer version. It is
  shared across items and across a whole run; names stay per-item as Python assigns them.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1-M14 | as in `rust-port-plan.md`, with the amendments above | as in the old plan |
| M15 | `xtask/table_corpus.rs`, `xtask/table_bench.rs`, `tests/corpus/tables/` (generated, gitignored) | Corpus and baseline recorded with commands |
| M16 | spike crate outside the workspace; `docs/DEPENDENCY_DECISIONS.md` | Stack chosen on the hardest corpus tables |
| M17 / WS-Raster | `crates/qti-raster/` | Subset contract implemented; review gate passed |
| M18 | `crates/qti-molecule/` | RDKit binding chosen and canvas corpus rendered |
| M19 | `qti-engines/src/html_to_image/`, `qti-cli` flag | Conversion parity; measured faster than Python |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1-M14 | Core port | As in `rust-port-plan.md`, amended | CLI parity for all ten engines |
| M15 | Table corpus and baseline | Harvest every table and canvas the generators emit; time Python per stage | A real-world corpus and a measured Python baseline |
| M16 | Raster stack spike | Prototype the hardest tables with the candidate text and raster crates | Stack chosen by evidence |
| M17 | Native table rasterizer | `qti-raster` implements the subset contract | Every corpus table renders and looks like an HTML table |
| M18 | Molecule canvases | RDKit binding spike, then `qti-molecule` | Every corpus canvas renders offline without JS |
| M19 | html_to_image conversion | Selection, conversion, naming, cache, parallelism, CLI flag | Conversion parity with Python, measured faster on the same corpus |

Dependency summary: M15 and M16 start with M1 and run beside M2-M6. M17 needs M16. M18 needs
M15. M19 needs M17, M18, M3 (bank), M7 (engine options shape), and M12 (CLI). M13's integration
gate includes M19.

### Milestone: M15 table corpus and baseline

- Depends on: M1 (workspace for `xtask`). The Python repo and `biology-problems` are present
  locally.
- Deliverables: WP-T1, WP-T2.
- Entry criteria: none beyond M1.
- Exit criteria: `cargo xtask table-corpus` regenerates a corpus of BBQ files from every
  `biology-problems` generator whose output contains `<table>` or an RDKit `<canvas>`, using
  fixed seeds. It also writes a feature census: every tag, attribute, CSS property, unit, and
  color form used, with counts and one example each. `cargo xtask table-bench` times the Python
  `--html-to-image` path on that corpus and records wall time, table count, browser launches,
  and per-stage time (launch, page setup, font load, screenshot, conversion bookkeeping).
  Results go in `docs/active_plans/reports/html_to_image_baseline.md`.
- Parallel-plan ready: yes. WP-T1 (corpus) and WP-T2 (benchmark harness) are independent until
  the final timed run, which needs both; maximum 2.

### Milestone: M16 raster stack spike

- Depends on: M15 feature census (the hardest tables are chosen from it).
- Deliverables: WP-T3.
- Exit criteria: `docs/DEPENDENCY_DECISIONS.md` records the text stack (parley vs cosmic-text),
  the painter (tiny-skia), the HTML parser (html5ever via a tree builder; M1's `lol_html` stays
  the media rewriter), the CSS value parser (`cssparser` vs hand-rolled for the subset), and the
  PNG encoder settings. Each record names its command and result. The spike renders three corpus
  tables: a monospace sequence table with colspan, a gel lane table with colored fixed-size
  cells, and a `table_curve_lib` curve with dashed guides and radius dots. Each rendering is
  judged against the Python PNG in a gallery page.
- Parallel-plan ready: no. One owner comparing candidates on the same three inputs.

### Milestone: M17 native table rasterizer

- Depends on: M16 (stack), M15 (corpus and census).
- Deliverables: WP-R1..WP-R5.
- Workstreams: WS-Raster.
- Exit criteria:
  - Every corpus table renders without error, or fails with `UnsupportedTableFeature` for a
    construct that `architect` has explicitly ruled out of the subset. The ruling is recorded in
    `docs/HTML_TO_IMAGE.md`.
  - Permanent layout tests pass (see `## Test and verification strategy`).
  - Review gate: `cargo xtask table-gallery` produces an HTML page showing, for every corpus
    table, the source HTML rendered live by the reviewer's browser beside the Rust PNG, with the
    Python PNG as a third column. `image_evaluator` screens the full set and lists suspect tables.
    The user then reviews the gallery and signs off that the tables look like HTML tables. That
    sign-off is the acceptance truth.
- Parallel-plan ready: yes, after WP-R1 fixes the internal types (styled tree, layout boxes,
  display list). WP-R2..WP-R4 run concurrently; WP-R5 integrates. Maximum 3.

### Milestone: M18 molecule canvases

- Depends on: M15 (canvas corpus).
- Deliverables: WP-M1, WP-M2.
- Exit criteria: the binding decision is recorded. Every corpus canvas renders offline, with
  every `CanvasSource` option honored: legend, `explicitMethyl`, atom and bond highlights,
  highlight colour, and peptide-bond SMARTS highlighting with its "no match" error. The four
  Python validation errors for out-of-range highlight indices and unparseable SMILES are
  reproduced as typed errors. With RDKit absent, a table-only bank still converts and a canvas
  bank fails with `RdkitUnavailable`.
- Parallel-plan ready: no. WP-M2 implements the binding WP-M1 selects.

### Milestone: M19 html_to_image conversion

- Depends on: M17, M18, M3 (bank and media directory), M7 (engine trait and options), M12 (CLI).
- Deliverables: WP-T4, WP-T5, WP-T6.
- Exit criteria:
  - The static canvas script parser is checked against real and hostile input.
    Every corpus canvas is accepted, because real generator output must convert. Every
    negative-set script is rejected, because that parser is what keeps item JavaScript from
    running. Error wording is free to differ.
  - On the corpus, every table and canvas is replaced by an `<img>` at the same position as in
    Python's output, with alt text carrying the table's cell text or the SMILES. Surrounding
    markup survives, loader scripts are removed, and the HTML stays ASCII so item CRCs hold.
    File names only need to be unique within the package.
  - Packages built with `-1 -2 -B --html-to-image` pass `qti-integrity`, including the media
    trace.
  - The run log shows one conversion per run.
  - `cargo xtask table-bench` shows Rust faster than the WP-T2 Python baseline on the same
    corpus and machine, with a per-stage breakdown.
- Parallel-plan ready: yes. WP-T4 (selection and canvas parser) and WP-T5 (conversion core) run
  concurrently against a shared `Fragment` type fixed first; WP-T6 (CLI wiring and benchmark)
  follows. Maximum 2.

## Workstream breakdown

### Workstream: WS-Raster (M17)

- Goal: implement the supported table subset from parsed HTML to PNG bytes.
- Owner: `expert_coder`, one fresh subagent per work package.
- Work packages: WP-R1..WP-R5.
- Needs: M16 stack decisions; the M15 corpus and census.
- Provides: `qti_raster::render_table_png`.
- Review boundary, when modifying the repository: `crates/qti-raster/` only; each package owns
  its named source files.

## Work packages

### Work package: WP-T1 harvest the table and canvas corpus

- Owner: `coder`.
- Touch points: `xtask/src/table_corpus.rs`; generated output under `output_tables/`
  (root-anchored ignore rule).
- Depends on: M1.
- Acceptance criteria: runs every `biology-problems` generator that emits a table or canvas,
  with fixed seeds and a small question count. It extracts every outermost table fragment and
  canvas record, deduplicated by content hash, and writes the census described in M15. Its
  command is reproducible and recorded.
- Obvious follow-ons: re-run it whenever a generator changes and diff the census.

### Work package: WP-T2 measure the Python baseline

- Owner: `tester`.
- Touch points: `xtask/src/table_bench.rs`, `docs/active_plans/reports/html_to_image_baseline.md`.
- Depends on: WP-T1.
- Acceptance criteria: times Python `bbq_converter.py -1 -2 -B --html-to-image` and a single
  `-B` run on the corpus, recording the machine. A temporary instrumented copy in
  `tests/_temp/` attributes time to browser launch, per-table page work, and conversion
  bookkeeping. The report states where Python's time goes. It sets no numeric multiplier: the
  comparison exists to show the Rust path is faster on the same work and to point optimization
  at the stage that dominates.
- Obvious follow-ons: WP-T6 re-runs the same bench against Rust.

### Work package: WP-T3 raster stack spike

- Owner: `expert_coder`.
- Touch points: spike crate outside the workspace; `docs/DEPENDENCY_DECISIONS.md`.
- Depends on: WP-T1.
- Acceptance criteria: the M16 exit criteria. Decision rules:
  - Text stack: correct shaping and metrics for both Atkinson variable fonts at weights 400 and
    700, line breaking inside fixed-width cells, and sub/sup baseline shift, all with the least
    code. Speed is the tiebreaker.
  - Painter: dashed and dotted borders, rounded background clipping, and anti-aliased text
    compositing.
  - CSS parser: covers the census value forms with the least code.
- Obvious follow-ons: retain the manager-selected dependency requirements and refresh the tracked
  `Cargo.lock`; the lockfile is the exact tested pin, per `docs/RUST_STYLE.md` section 16.

### Work package: WP-R1 subset parser and style cascade

- Owner: `expert_coder`.
- Touch points: `qti-raster/src/{lib.rs, subset.rs, style.rs}`.
- Depends on: WP-T3.
- Acceptance criteria: parses a fragment into a styled tree. Presentational attributes map to
  their CSS equivalents; inline styles override them; UA defaults come from one constants
  module; inheritance applies to `color`, `font-*`, `text-align`, `white-space`, and
  `line-height`. Every construct outside the subset returns `UnsupportedTableFeature` with the
  tag, attribute, or property and a snippet. Defines the styled-tree, layout-box, and
  display-list types that WP-R2..R4 consume.
- Obvious follow-ons: none.

### Work package: WP-R2 table layout

- Owner: `expert_coder`.
- Touch points: `qti-raster/src/table_layout.rs`.
- Depends on: WP-R1.
- Acceptance criteria: CSS 2.1 section 17 grid construction with colspan and rowspan, auto and
  fixed layout, specified widths and heights honored as minimums, percentage widths, both border
  models with collapsed-border conflict resolution, `cellspacing`/`border-spacing`,
  `cellpadding`, `vertical-align` within rows, and nested tables laid out as cell content.
- Obvious follow-ons: none.

### Work package: WP-R3 inline and text layout

- Owner: `expert_coder`.
- Touch points: `qti-raster/src/{inline_layout.rs, fonts.rs}`.
- Depends on: WP-R1.
- Acceptance criteria: min and max content widths for the table algorithm; line breaking at the
  final cell width; `white-space: nowrap/pre`; `text-align`; bold, italic, and numeric weights on
  the variable fonts; sub/sup at the UA baseline shift and size; `br`, and `p`/`div` block
  margins; `font-size: 0` and `line-height: 0` collapsing a cell to its specified height;
  `&nbsp;` and entity decoding; both fonts embedded with their OFL files.
- Obvious follow-ons: none.

### Work package: WP-R4 paint and encode

- Owner: `coder`.
- Touch points: `qti-raster/src/paint.rs`.
- Depends on: WP-R1.
- Acceptance criteria: paints the display list on white, clipped to the table border box.
  Covers backgrounds, `border-radius` clipping (including `50%`), solid, dashed, dotted, and
  double borders, `overflow: hidden` clipping, glyph runs, and embedded `data:` images (PNG
  decode, SVG via `resvg`). Encodes PNG with settings chosen in WP-T3.
- Obvious follow-ons: none.

### Work package: WP-R5 integrate, gallery, and review

- Owner: `expert_coder` (integration), `image_evaluator` (screen), user (sign-off).
- Touch points: `qti-raster/src/lib.rs`, `xtask/src/table_gallery.rs`, `docs/HTML_TO_IMAGE.md`.
- Depends on: WP-R2, WP-R3, WP-R4.
- Acceptance criteria: the M17 exit criteria. `docs/HTML_TO_IMAGE.md` publishes the subset table
  and every out-of-subset ruling.
- Obvious follow-ons: fixes from the review loop go back to the owning file's package.

### Work package: WP-M1 RDKit binding spike

- Owner: `expert_coder`.
- Touch points: spike crate; `docs/DEPENDENCY_DECISIONS.md`.
- Depends on: WP-T1.
- Acceptance criteria: compares three candidates on the canvas corpus.
  - (a) RDKit MinimalLib C API (`librdkitcffi`), loaded with `libloading`, drawing SVG through
    its JSON details (legend, highlights, colour, `explicitMethyl`), rasterized with `resvg`.
  - (b) `rdkit-sys` plus a small `cxx` shim over `MolDraw2DCairo` or `MolDraw2DSVG`.
  - (c) the Python RDKit in a subprocess, as a floor for comparison only.
  - Decision rule: supports every `CanvasSource` option including the peptide SMARTS match;
    installs on macOS arm64 and Linux x86_64 from a documented package source (Homebrew,
    conda-forge, or a pinned release); loads at runtime so builds need no RDKit. Among passing
    candidates, prefer the smallest install footprint, then speed.
  - Escalation: RDKit canvases must be solved. If neither (a) nor (b) passes cleanly, the spike
    stops without choosing. It writes `docs/active_plans/decisions/rdkit_canvas_options.md`
    with each candidate's evidence, blockers, and install cost, and hands it to the user, who
    has offered alternative solutions. Candidate (c) is listed there as an option, not a
    default. M18 stays blocked until the user decides; M17 and the table path of M19 proceed
    meanwhile.
  - Survey first: before comparing bindings, list which generators emit canvases and how many
    corpus items use them (from WP-T1), so the decision weighs real usage.
- Obvious follow-ons: document the install step in `docs/INSTALL.md`.

### Work package: WP-M2 molecule renderer

- Owner: `coder`.
- Touch points: `crates/qti-molecule/`.
- Depends on: WP-M1.
- Acceptance criteria: the M18 exit criteria.
  `render_canvas_png(&CanvasSource) -> Result<Vec<u8>, MoleculeError>` produces an image at the
  canvas's declared width and height, because the replacement `<img>` carries no size attributes
  and its pixel size is its display size.
- Obvious follow-ons: none.

### Work package: WP-T4 selection and static canvas parser

- Owner: `coder`.
- Touch points: `qti-engines/src/html_to_image/{selectors.rs, canvas_script.rs}`.
- Depends on: M3.
- Acceptance criteria: outermost-table selection in document order; RDKit canvas-to-script
  matching with Python's three-step sibling search; the static script grammar with every
  Python rejection (single `get_mol`, single draw call, static SMILES, empty `mdetails`, allowed
  keys, no duplicates, integer lists, RGB range, the 4096 limits); CDN loader removal; ASCII
  re-serialization. Verified by the corpus plus negative-set agreement run in M19.
- Obvious follow-ons: none.

### Work package: WP-T5 conversion core

- Owner: `expert_coder`.
- Touch points: `qti-engines/src/html_to_image/{convert.rs, naming.rs, cache.rs}`.
- Depends on: M3; WP-T4 for the shared `Fragment` type.
- Acceptance criteria: canvases before tables; canvas PNGs inlined as `data:` images into the
  table render and dropped from the package unless still referenced; per-item, per-family
  counters and `<crc>_<family>_<n>.png` names from the original CRC (Python's convention, kept so
  packaged files stay recognizable; the requirement is uniqueness); alt text rules; the per-item
  identical-string reuse Python has; a run-wide content-hash cache; parallel rendering of distinct
  fragments with `rayon` and results reassembled in document order; two-phase behavior (all
  renders succeed before the new bank exists); the new bank carries forward `media_base_dir`
  after WP-T5 first reproduces the suspected Python defect and `architect` rules on it.
- Obvious follow-ons: report the upstream defect to the Python repo's changelog owner.

### Work package: WP-T6 CLI wiring and speed verification

- Owner: `coder`.
- Touch points: `qti-cli`, `xtask/src/table_bench.rs`.
- Depends on: WP-T4, WP-T5, M12.
- Acceptance criteria: the `--html-to-image` rules from the amended WP-F2; the CLI converts once
  and passes the converted bank to each packaging writer; `cargo xtask table-bench` reports the
  Rust times beside the WP-T2 baseline, showing Rust faster on the same work.
- Obvious follow-ons: none.

## Acceptance criteria and gates

- Apply the user's 2026-09-30 guidance: parity gates assess equivalent content, grading,
  interoperability, accessibility, and usable output. Generated identifiers, formatting,
  equivalent explicit/default attributes, and harmless implementation differences do not
  require byte equality. Visual review assesses readable, faithful tables without pixel
  equality. Timing evidence uses the specified same-corpus baseline rather than an invented
  absolute cutoff. Any stricter comparator must identify the practical invariant it proves;
  unsupported differences are findings to assess, not automatic failures. Hashes bind evidence
  to inputs and implementations; they do not require independent outputs to share bytes.
- Upload validation requires well-formed XML, the target format's required structure,
  consistent resource/media references, and correct grading. Record actual LMS import
  evidence separately; local integrity or structural checks alone do not prove acceptance
  by an LMS. Invalid frozen XML is a documented source defect to repair, not a requirement
  to reproduce malformed output.
- Per-patch gate: `cargo fmt --check`, `cargo check`, the owning package's `cargo test`, and
  `cargo clippy -- -D warnings`.
- Integration gate (M13, now including M19): the old plan's integration gate, plus
  `--html-to-image` packages passing `qti-integrity`, conversion structural parity on the
  corpus, and the benchmark comparison.
- Review gates:
  - M16: `architect` accepts the stack decisions.
  - M17: `image_evaluator` screens the gallery, then the user signs off that the tables look
    like HTML tables.
  - M18: `architect` accepts the RDKit binding and its install story, or escalates to the
    user per WP-M1.
  - M19: `reviewer` audits the canvas script parser, because it is the security boundary that
    keeps item JavaScript from ever executing.
- Failure plan for the speed comparison: if Rust is not faster on the same corpus, the design
  premise is wrong. `table-bench` names the dominant stage: parse, layout, text, paint, encode,
  or conversion bookkeeping. The owning work package fixes that stage. A result that is faster
  but still slow enough to annoy a user in practice goes to the user as a finding, not a
  failed gate.
- Failure plan for the M17 review gate: suspect tables become inline layout tests in the owning
  work package, then the gallery is regenerated. If the same class of failure survives two rounds
  (for example auto-layout width distribution), `architect` applies the scrap-vs-fix criteria
  and may invoke the CDP-Chromium recovery path in `## Risk register`.

## Test and verification strategy

Permanent `cargo test` (fast, inline, offline), following the house test policy:

- `qti-raster` layout invariants, asserted on layout geometry rather than pixels, each one a
  table behavior the user named or the corpus depends on:
  - A cell with a specified width and height gets at least that size.
  - A colspan cell spans exactly its columns.
  - A rowspan cell covers its rows.
  - A `font-size:0` cell stays at its specified height (the curve tables depend on it).
  - `text-align` and `vertical-align` move a text run toward the named side.
  - Each unsupported construct returns `UnsupportedTableFeature` naming itself.
- One paint smoke test: a colored cell's background color appears in the PNG.
- Canvas script parser: accept and reject cases, one per Python rejection rule.
- Conversion: names, alt text, canvas-before-table ordering, the intermediate-canvas drop,
  `media_base_dir` carry-forward, and two-phase failure leaving no media directory.
- No golden-image tests: pixel goldens pin incidental rendering and are fragile.

Tooling (`cargo xtask`, on demand and in CI where Python and `biology-problems` exist):
`table-corpus`, `table-bench`, `table-gallery`, and the M19 parser agreement run.

One-time proof: the gallery sign-off and the baseline report are recorded in `docs/CHANGELOG.md`;
the corpus itself is regenerated rather than committed.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Auto table layout diverges visibly from browsers | High: tables look wrong | Gallery shows wrong column widths on spans | `expert_coder` | Implement CSS 2.1 section 17 as written; span cases are permanent tests; two failed review rounds trigger scrap-vs-fix |
| Corpus misses a construct used in the field | Medium: a user's table errors | `UnsupportedTableFeature` reported | `architect` | Error names the feature and snippet; the census re-runs on generator changes; extending the subset is an explicit ruling |
| Native renderer cannot pass review | High: flag unusable | Same failure class across two review rounds | `architect` | Recovery path: `qti-raster` gains a second backend driving system Chrome over CDP, with one launch, batched pages, and clip screenshots; the subset check stays in front of it |
| No reasonable RDKit binding | High: canvas parity blocked | WP-M1 finds no candidate passing its rule | user | Spike stops and escalates with evidence in `docs/active_plans/decisions/rdkit_canvas_options.md`; the user chooses among candidates or supplies another solution; table work continues unblocked |
| Canvas parser accepts something Python rejects | High: executes or misreads untrusted script content | Agreement run disagrees | `reviewer` | Grammar ported rule by rule; negative set; independent audit |
| Rust not faster than Python | Medium: objective unmet | `table-bench` comparison | `coder` | Stage timings name the bottleneck; structural levers (once per run, cache, `rayon`) are independent of the renderer |
| Font metrics differ from Chromium | Low: slightly different wrapping | Gallery shows different line breaks | `expert_coder` | Same embedded fonts; the legible-and-correct bar tolerates wrap differences |
| Python upgrades land during the port | Medium: port targets a moving head | New Python commits after a milestone's pin | `architect` | Pin the Python commit per milestone; re-run the upgrade survey before M8-M11 dispatch and before M13, and fold new features into the plan |

## Rollout and release checklist

- [x] Originally copied this plan and its predecessor into `docs/active_plans/active/`.
      Both are now archived here as `rust_port_plan.md` and `rust_port_plan_v1.md`.
- [x] M15 baseline report written, naming where Python's time goes.
- [x] M16 and M18 decisions in `docs/DEPENDENCY_DECISIONS.md`; the Chromium amendment
      supersedes the original custom-renderer choice.
- [x] M17 replacement accepted: on 2026-10-01 the user said "good work" and observed that
      the Chromium renderer matches the Python Chromium renderer. The new gallery renders
      all 290 tables; the two reported failures also pass focused macOS and Linux visual review.
- [x] M19 benchmark comparison recorded beside the baseline. Run 23373 has 721 clean
      packages and zero content/grading differences. The measured first-screenshot wait is
      documented as a Chromium performance limitation; this measurement does not claim a speedup.
- [x] `docs/HTML_TO_IMAGE.md`, `docs/INSTALL.md` (RDKit and Chromium), and `docs/USAGE.md` updated.
- [x] Python `convert_bank` `media_base_dir` finding reported with a reproduction in
      [../active_plans/reports/html_to_image_baseline.md](../active_plans/reports/html_to_image_baseline.md) and
      [../PARITY.md](../PARITY.md).

## Documentation close-out requirements

- Completed plan: `docs/archive/rust_port_plan.md`; the original proposal remains at
  `docs/active_plans/majestic-shimmying-tiger.md`. The progress tracker is
  `refactor_progress.md`, including M15-M19 evidence.
- docs/CHANGELOG.md entry: per milestone, with `### Decisions and Failures` recording:
  - native subset rasterizer over Chromium, and why;
  - convert-once-per-run;
  - the content-hash cache;
  - the RDKit binding choice;
  - each out-of-subset ruling;
  - the Python defect finding.
- `docs/DESIGN_DECISIONS.md`: the subset-contract decision, with `Owner` naming
  `docs/HTML_TO_IMAGE.md`.
- `docs/HUMAN_GUIDANCE.md`: record in the user's words that the Rust port needs feature parity or
  no one will use it; that the tables are very basic (alignment, monospace, color, fixed widths
  and heights, col and row spans); that the tables need to look like HTML tables, with no
  truth beyond that; that RDKit canvases must be figured out, and the user offers other
  solutions if no reasonable one is found; that the Python changes since the first plan
  are well-designed upgrades, not drift; and that plan gates must be grounded in reality, with
  no byte- or pixel-equivalence expectations for improvements, no arbitrary thresholds, and no
  expected line counts.
- Archive / closure notes: `git mv` the plan to `docs/archive/` on M14 plus M19 completion.

## Patch plan and reporting format

- Patches 1-28: as in the old plan.
- Patch 29 (WP-T1), patch 30 (WP-T2): concurrent with M2-M6.
- Patch 31 (WP-T3).
- Patch 32 (WP-R1), then patches 33-35 (WP-R2, WP-R3, WP-R4) concurrent, then patch 36 (WP-R5).
- Patch 37 (WP-M1), patch 38 (WP-M2): concurrent with M17.
- Patch 39 (WP-T4) and patch 40 (WP-T5) concurrent, then patch 41 (WP-T6).

Each patch reports its work-package ID, files touched, the Cargo commands run with results, the
gates passed, and, for raster patches, the gallery tables it changed.

## Open questions and decisions needed

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: `architect` for subset rulings, an RDKit binding that
    passes the WP-M1 rule cleanly, and the Python-defect ruling; the user for the M17 visual
    sign-off and for RDKit whenever no candidate passes cleanly.
  - Evidence and decision rule: census counts and gallery evidence for subset rulings; the
    WP-M1 rule for RDKit; a WP-T5 reproduction for the defect.
- Non-blocking follow-up:
  - Backport convert-once and one-page batching to the Python package as a quick win for current
    users. It needs a separate Python-repo plan, because this plan leaves Python unmodified.
  - Python's replacement `<img>` carries no width or height while tables render at device scale
    2, so LMSes may show table images at double size. The gallery review should check how they
    display in context. Any change is a user decision, because it alters Python-visible
    behavior.
  - The PyO3 plan picks up `render_mathml_png`, `color_wheel`, `anti_cheat`, and the library
    imports listed in `## Non-goals`.
