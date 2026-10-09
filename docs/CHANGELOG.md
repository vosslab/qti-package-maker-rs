# Changelog

Earlier entries: [CHANGELOG-2026-09a.md](CHANGELOG-2026-09a.md).

## 2026-10-09

### Additions and New Features

- Added independent selftest identity and browser grading contracts, including host-wrapped
  grading and question-level completion across variant replacement. Current-Python execution
  belongs to a separate migration check. The contract and drift investigation are recorded in
  [SELFTEST_COMPATIBILITY.md](SELFTEST_COMPATIBILITY.md).

### Behavior or Interface Changes

- Generated selftests grade through `window.checkAnswer_<CRC>` so the website can observe
  completion. Repeated initialization preserves wrapped hooks; functions resolve the currently
  mounted question. Statement IDs and distinct repeated MULTIFIB blank IDs are restored.
- Check buttons remain usable after correct answers. NUM Enter grades once, MULTIFIB Enter
  does not grade, and FIB has no extra Enter-grading handler. MATCH supports repeated click/drop
  guesses and keyboard letter moves. Clear/reset actions affect question controls and feedback.
- Current Python source is a transitional migration reference with source-bound receipts.
  Routine Rust release checks run independently of Python QPM. Important behaviors are protected
  by independent Rust and browser tests so Python can eventually leave active workflows.

### Fixes and Maintenance

- Recorded the preference to minimize dependencies to reduce the supply chain attack surface
  in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).
- Removed unused direct `sha2` from `qti-integrity` and `quick-xml` from `xtask`, and kept
  `qti-engines`' direct `zip` dependency only for its tests. Disabled Scraper's standalone CLI
  feature and ZIP's unused Zopfli encoder while retaining HTML diagnostics and the existing
  ordinary Deflate backend. No dependency version upgrades or converter API changes.
- The dependency cleanup reduces external Rust lockfile packages from 250 to 246, removing
  `getopts`, `unicode-width`, `zopfli`, and `simd-adler32`. All 323 Rust tests pass, with four
  optional tests ignored; strict Clippy, the Wasm target check, formatting, and 1,819 Python
  hygiene checks pass. Browser and native rendering acceptance were not rerun for this cleanup.
- Adapted ZIP filename handling to the already-locked ZIP 9 API. Decode failures remain typed
  package-input violations. The fallback archive test asserts QPM's size rejection independently
  of which central directory the dependency chooses.

### Removals and Deprecations

- Removed Rust-only Reveal controls that supplied and graded answers. No compatibility layer
  preserves the earlier hook bypass or obsolete button behavior.
- Removed the historical Python pin from active reference resolution and ordinary development
  setup. Historical receipts retain their source identity; they are not current parity evidence.
- Removed historical comparator repair modules and expected-defect branches. Current comparisons
  report actual Python/Rust differences, including reference defects, without rewriting outputs.

### Decisions and Failures

- The initial Rust port bypassed its own public grading hooks. Frozen-source and static-answer
  comparisons failed to detect both that regression and later useful Python control changes.
  The correction protects the current consumer contract without copying obsolete interfaces or
  making Python a permanent test dependency.

### Developer Tests and Notes

- Completed six fresh independent Plan, Test, Style, Docs, Legacy, and Comment audit passes.
  Setup docs now reference manifest requirements instead of duplicating transient tool versions.
  Release scripts record versions and rely on actual builds/tests instead of separate Rust,
  Python, C++, or Node version gates. The Linux image follows the workspace compiler requirement.
  Removed an incidental initial-order assertion from the
  browser test. Two low-priority filename issues remain; no blocker/high/medium findings remain.
  Scope, evidence, and dispositions are in
  [the selftest audit](active_plans/audits/selftest_six_pass_audit_2026_10_09.md).
- Verification passes: 325 Rust tests (four optional tests ignored), strict Clippy and formatting,
  1,810 Python hygiene checks, nine Node tests, and 30 Chromium/Firefox/WebKit browser tests.
  The separate current-Python browser lane passes all seven kinds in all three engines; 551
  question CRCs agree. Broader migration findings remain visible. The installed local tarball,
  old-output reproduction, source identities, and website handoff are recorded in
  [SELFTEST_REGRESSION_VERIFICATION.md](active_plans/reports/SELFTEST_REGRESSION_VERIFICATION.md).

## 2026-10-08

### Additions and New Features

- Added a shared Rust native/WebAssembly conversion pipeline with the same four readers and
  eleven writers. `qti-native` owns filesystem input, rendering, context, and persistence;
  `qti-wasm` exposes Rust-generated TypeScript transport for format inventory, conversion,
  and integrity checking. The private local package includes Node, browser, and worker examples,
  companion-name editing, and multi-file ZIP downloads. Final source-bound acceptance is tracked in
  [shared_engine_delivery.md](active_plans/reports/shared_engine_delivery.md).
- Documented the shared native/browser Rust architecture, byte reader/writer contracts,
  provider-owned media, generated TypeScript API, local package build, workers, and downloads.
  Added [WASM_PACKAGE.md](WASM_PACKAGE.md) and a source-bound shared-engine delivery report.
- Added the explicit `RUST_RELEASE_WASM=1` local release lane for portable target checks,
  package installation/build, strict TypeScript, Node host/native-Wasm parity tests, and
  Chromium/Firefox/WebKit tests.
  Selected missing prerequisites fail with a correction-and-rerun message; native CLI setup
  continues to use Cargo independently of Node or Wasm tooling.

### Behavior or Interface Changes

- Recorded the portable migration from filesystem readers/writers to logical named bytes,
  explicit context, and `WriteArtifact` outputs. Native filesystem, PLE directory ownership,
  rendering, and time resolution belong to `qti-native`; Wasm consumes the same four readers
  and eleven writers. Browser omission defaults are documented, with explicit values for
  reproducible calls. Native local media must resolve under the input parent directory,
  including authored absolute paths and symlinks; migration guidance uses a common input root.
  Reference/placeholder policies inspect metadata without payload reads; native rendering filters
  selected supported kinds and preserves FIB grading literals and source item numbers.

### Fixes and Maintenance

- Removed an incidental allocation-count assertion and a one-time ignored browser-proof generator,
  documented Wasm transport and asset-provider ownership contracts, clarified companion-name and
  browser-picker behavior, and removed qti-native's unused direct `markup5ever` dependency.
- Applied the six-pass PLE Native JSON audit cleanup: reduced the internal source model to its
  emitted-only serialization surface, removed one direct private-helper test, clarified the
  eligible `--html-to-image` writer rule, and repaired the PLE-issues handoff link. Final integrated
  Rust tests pass (282 passed, 5 intentionally ignored), as do strict Clippy, formatting, and the
  Python suite (1,576 passed with no failures or advisories). The earlier PLE corpus receipts remain
  historical and do not cover the cleanup bytes; current source hashes and gates are recorded in
  `tests/_temp/cohesive_splits_gate_receipt.md`.
- Split seven files flagged by the size advisory into modules by responsibility, preserving public
  APIs, comments, tests, and behavior. The Blackboard QTI XML escaping helper now has one owner in
  `fragment.rs`. Fresh specification, quality, and integration reviews accepted all splits. Final
  Rust tests (282 passed, 5 intentionally ignored), strict Clippy, formatting, 37 source hashes,
  Python (1,576 passed with no failures or advisories), and parity on 59 banks with zero divergences
  pass. Architecture and file-structure docs and the active split ledger describe the new modules.
- The 2026-10-07 PLE corpus summary remains historical. Its verification, gates, and classification
  records live under Git-ignored local `output_ple_native_json/` and are unavailable in fresh
  clones; they do not establish coverage of the bytes changed by this audit cleanup.

### Developer Tests and Notes

- Completed the fresh Plan, Test, Style, Docs, Legacy, and Comment audit. After bounded cleanup,
  323 Rust tests pass (four optional tests ignored), strict Clippy/formatting and 1,819 Python
  checks pass, and the rebuilt package passes eight Node/parity tests, 15 browser tests, and
  installed-tarball runtime/type checks. The Wasm binary remains byte-identical to prior acceptance.
  Findings, unchanged pre-existing notes, and evidence boundaries are recorded in
  [the audit report](active_plans/audits/shared_engine_six_pass_audit_2026_10_08.md).
- Shared-engine final corrected source passes 323 active Rust tests with five existing optional
  tests ignored, strict formatting/Clippy, and all 1,581 Python tests. Current identity-bound
  native fixture parity covers 59 banks with zero divergences; three actual shim tests, 58
  canvases, and four-format rendering/persistence correction receipts pass. Full current native
  corpus parity covers 1,699 projected banks from 180 BBQ inputs with zero divergences; rendered
  table parity passes across three ZIP formats. Comparable native timings preserve input/output
  checks, and the small historical collision increase does not reproduce in alternating runs.
  Completed contract and execution records are archived; evidence remains tracked in
  [shared_engine_delivery.md](active_plans/reports/shared_engine_delivery.md).
- The final local Wasm package passes build, strict TypeScript, eight Node host/native-Wasm parity
  tests, 15 Chromium/Firefox/WebKit tests, and an actual installed nine-file npm tarball consumer's
  runtime and positive/negative strict TypeScript checks. Source/artifact hashes, component sizes,
  runtime versions, and browser smoke observations are bound in the delivery report; no package
  publication is implied.

## 2026-10-07

- Reproduced the clean Graphify 0.9.80 reclustering crash and validated an upstream
  fix without forcing writes: six Cargo aliases reconcile with AST package nodes,
  preserving all production symbols and ten dependency links. The candidate passes
  the exact fresh mapping command and 252 focused tests. Restore the official package
  after validation, following the user's preference against third-party patches and
  wrapper workarounds. The cited Graphify evidence artifact is unavailable in this checkout.

- Added the PLE Native JSON writer documentation: seven-kind library and directory handoff,
  `ple-<content_name>/` naming, associated media, HTML-to-image participation, and validation
  ownership. Documented the settled text-matching and URL-inventory decisions, human directions,
  and a PLE follow-up for URLs used in multiple HTML roles. Corrected operational PLE import,
  API, roadmap, and checklist wording against source; M15 results are recorded in the active plan.
- Added the write-only `ple_native_json` engine for all seven item kinds. Its public in-memory
  export returns each question's JSON, item number, CRC, associated file bytes, and warnings.
  The writer stages a directory of `item_NNNNN.json` and `media/` files, replaces only a verified
  prior engine-owned directory, and refuses foreign contents. Ownership is established by a hidden
  `.qpm-ple-native-json` marker with hashes of generated files, not by JSON shape alone. Registry
  and CLI selection now
  include the eleventh writer, default `ple-<content_name>` name, and shared `--html-to-image`
  conversion. `--all` selects eleven writers, though unsupported kinds can leave a writer with no
  rendered output.
- Verified the PLE Native JSON handoff with final CLI SHA-256
  `0475e805b75186ffe9dea7e91678105b27c5cb394d1b306cee3c585a8713eeee`:
  373 attempts yielded 359 outputs and 359 passing decoder receipts. All 708 JSON questions
  passed PLE `parse` and `compile`, across all seven kinds; receipts account for 447 associated-file
  uses, 448 display `src` references, and 32 external inventory entries. Exact ownership-manifest
  hashes match all 1,155 generated JSON/media files. A separate representative run covered 31
  JSON files and 85 assets; these are not the main smoke counts. Final 59-bank parity comparison
  has zero divergences, and all 76 regenerated parity inputs are byte-identical to those used in
  the final smoke. Evidence and independent classification are retained in
  `output_ple_native_json/verification/` and `output_ple_native_json/m14_classification.md`.
- Classified all 14 rejected CLI attempts as QPM capability gaps: 13 script-bearing banks reject
  plain mode but pass `--html-to-image` with identical source bytes; one `data:` image cannot be
  bundled under Package policy. The inherited entity-name media behavior is also documented below.
  No QPM mapping defect, PLE decoder rejection, or PLE limit hit was observed. The corpus harvest
  attempted 205 generators and emitted 180 BBQ banks; 17 gave no BBQ output and eight failed
  upstream, so the 180 emitted banks define the corpus smoke scope.
- Hardened PLE directory publication against case-insensitive path aliases such as
  `media/A.png` and `media/a.png`. The writer now verifies exact staged manifest paths and hashes
  before publish; a filesystem-aware regression failed before the fix and passes now. Independent
  `stage_respec` and `stage_requality` reviews accepted D8. Whole-change `final_spec` and
  `final_quality` found no code blockers. Final-tree workspace tests, strict all-target Clippy,
  and formatting pass (logs in `output_ple_native_json/gates/`). The verification archive retains
  1,145 files (7.3 MB), all checked against `files.sha256`, and records 19 QPM source plus 11 PLE
  decoder/grading hashes with reconstruction guidance. Eight temporary harness source files and
  two bytecode files were removed. The final Python suite passes 1,514 tests with seven existing
  near-limit advisories, and `git diff --check` passes. Fresh `final_integration_acceptance`
  found no unresolved implementation or plan blockers; M15 is complete.
- Documented the PLE display-only media boundary after a temporary reproduction showed that
  whole-bank media traversal can mistake FIB grading literals for image references and alter
  accepted answers. The writer reuses shared media primitives on mapped display fields.
- Recorded a shared QPM media capability gap found during Native JSON review: an authored local
  `<img src="a&amp;b.png">` is passed through by
  [resolve.rs](../crates/qti-core/src/media/resolve.rs) as the literal filesystem name
  `a&amp;b.png`, while HTML viewers interpret the source as `a&b.png`. The existing bank collector
  shares this scanner, so packaging a literal `a&amp;b.png` file can bind the wrong image and a
  source file named `a&b.png` can be missed. Native JSON now rejects mismatches with item context;
  the shared media-layer correction remains outside this writer scope.
- Repaired two dangling Graphify documentation links in this changelog and
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) by stating that the referenced evidence artifact is
  unavailable in this checkout. No replacement proof was inferred.
- Reflowed comments and blank lines in `devel/run_linux_release_check.sh` from 106 to 99 lines;
  its meaningful command lines remain identical. Replaced 19 imported handoff/audit links with
  canonical PLE GitHub links after checking their targets and headings. Handoff prose and Git
  disposition remain unchanged. Baseline spec and quality reviews accepted these narrow repairs.

### Fixes and Maintenance

- Rotated the intact 2026-09-30 day block into
  [CHANGELOG-2026-09a.md](CHANGELOG-2026-09a.md), keeping the two newest days here.
- Synchronized shared style guides, tests, and repository support files from the starter template.
## 2026-10-01

- Preserved the headless-shell speed comparison in
  [WEBSITE_EXPORT_BENCHMARK.md](WEBSITE_EXPORT_BENCHMARK.md), including methodology, per-round
  totals, source provenance, exclusions, and the boundary between permanent summary and local evidence.

- Made headless rendering an explicit fixed behavior and automatically prefer Chromium headless
  shell from `PATH` or the installed Playwright cache. No new modes or settings were added.
  The repeated 12-set website export benchmark now takes 18.38 seconds in Rust versus 31.64 in
  Python (median of three rounds), with 204 matched successful exports and unchanged validation
  checks. This supersedes the earlier full-browser speed comparison for the automatic shell path.
  Evidence: `output_website_speed/automatic_headless/`; recorded the user's speed and simplicity
  requirements. Existing engine tests and strict Clippy pass; no permanent tests added.

- Measured website-style Rust/Python exports using 12 copied question sets (570 questions),
  three alternating-order rounds, and fresh output directories. Median matched totals were
  50.29 seconds for Rust and 30.99 for Python: Rust was faster without HTML-to-image, while
  Chromium capture delays dominated image-enabled exports. Verified ZIP/XML integrity and
  matching question/image counts; excluded unsupported ordering exports. Temporary scripts
  live in `tests/_temp/website_speed/` and evidence in `output_website_speed/browser_allowed/`.
  This measures converter subprocesses, not the entire website build; no permanent tests added.

- Closed the actionable audit follow-ups: Linux release receipts now record installed Python
  packages and Playwright's Chromium version alongside system Chromium. Archived and verified
  the one-time Chromium probe sources, binaries, and input fixture before retiring their loose
  copies; retained validation receipts and rendered outputs. No dependency pins or permanent
  tests were added.

- Six independent audit passes identified and corrected a stale M13-incomplete statement,
  ambiguous historical progress labels, and archived-plan references. Dependency-environment
  reproducibility and temporary-probe cleanup were identified for follow-up; no permanent tests added.

- Completed the amended Rust feature-parity plan and archived both plan documents with Git
  moves. Updated architecture, file layout, README, and progress documentation to reflect
  Chromium, final corpus validation, and the remaining external release-validation boundaries.

- Synchronized the written Python benchmark measurements with the retained 181-input baseline
  receipt so the Chromium comparison uses the documented source measurements.

- Fixed missing content in Chromium screenshots of tables larger than the initial viewport.
  A temporary oversized-table check exposed white areas where authored cells should appear.
  Capture now uses document bounds once and explicitly includes content beyond the viewport;
  all four corners remain visible. Updated the renderer cache identity. The earlier corpus
  benchmark was stopped and retained as incomplete evidence; no permanent browser test was added.
  Corrected capture passes macOS and Linux checks, including both Linux Rust architectures,
  six oversized-table packages, and a refreshed 290-table gallery. The full refreshed corpus
  produces 721 packages with zero integrity errors/warnings or public-content/grading
  differences. Recorded the slower CLI benchmark and isolated its wait to the first screenshot
  capture, consistent with an upstream Chromium report. Kept this as a documented performance
  limitation; temporary instrumentation was removed and no permanent test was added.

- Repaired the Linux comparison environment: install the pinned Python reference's missing
  dependencies and browser, set its explicit checkout path, and use current lxml because Debian
  13's lxml 5.3 fails on the reference's annotations. Added a prerequisite import check so this
  environment failure is reported before Rust compilation. The prerequisite run and the 26.10
  manual release dry-run pass. The Rust runtime remains Python-free.

- Updated Linux release validation for Chromium: use the native container architecture, install
  the browser, retain its sandbox through a non-root runtime user, and locate RDKit libraries
  for that architecture. Fresh Linux arm64 checks pass 245 workspace tests, strict Clippy,
  formatting, release builds, and 12 Chromium-rendered package integrity checks. Both x86_64
  CLIs cross-compile; amd64 Chromium runtime validation remains open because the Mac emulator
  fails before browser startup.

- Chromium replacement verification passes: all 290 gallery tables render; the reported pathway
  colors/circles and gel shadows render correctly in three-format exports. All 12 final smoke
  packages pass integrity, including script/redirect boundary fixtures. Workspace tests (248),
  strict Clippy, formatting, and optimized builds pass. Repository checks pass after removing
  stale debug artifacts; no permanent browser or screenshot tests were added.

- Replaced custom HTML layout with a shared Chromium session controlled through Rust.
  Removed the custom rasterizer implementation and its implementation-specific tests; kept
  bundled fonts and the existing conversion, cache, media, and RDKit canvas contracts.
  Chromium now preserves the reported gel shadows and pathway colors/circles. Installed Chrome
  or Chromium is required for table images; `QTI_CHROMIUM` can select its executable.
  Python table appearance is reference material, not an exact-match acceptance requirement.

- Corrected the visual-failure record from the user's clarification: the gel rejects
  `box-shadow`; the pathway loses colors and circles. Earlier saved-image observations
  did not describe the user's failing render.

- Recorded the user's 2026-10-01 rejection of custom HTML rendering, two genuine gallery
  failures, four malformed-source exclusions, and two obsolete HTML-sugar exclusions.
  Native rendering acceptance is reopened; existing automated checks do not override this review.

- Restored the date heading required by the changelog parser so the vendored commit helper
  can detect added entries and seed a commit message.

- Retired 73 ignored implementation-proof and captured-source files from the temporary test
  workspace. Their source is preserved in a verified ignored archive; result receipts remain
  available for review. No permanent tests were added.

- Updated the roadmap to reflect completed platform, corpus, and benchmark verification and
  the manual release workflow. The progress ledger now names temporary-check retirement and
  plan archival as closeout work and points to the final Python verification log.

- Completed the Python repository's reciprocal Rust-port link. Clarified that the optional
  HTML release-comparison command still exits nonzero on documented reference and serialization
  findings, and corrected stale parity/progress wording to reflect completed semantic review.

- Built both Linux x86_64 release CLIs from the final verified source snapshot using an arm64 Linux host and Debian cross-compiler. Both run under amd64 Podman emulation; a real native-table conversion emits all three package formats with PNGs and clean integrity checks. The vendored manual release dry-run passes with draft notes at `/tmp/qti-rust-26.09-release-notes.md`; it correctly reports that archives use committed HEAD and omit current uncommitted changes. No release was published.

- Final current-source verification passes on macOS arm64 and Linux arm64: formatting, strict workspace/all-target Clippy, workspace tests, and optimized builds. The Linux snapshot `run_1790825204` records 320 passing tests and unchanged build-input hashes; all 1,556 Python checks pass with nine existing warnings. Linux x86_64 release compilation is being verified separately. Updated installation guidance to distinguish these completed checks from remaining release and visual decisions.

- Reconciled the five HTML reference failures with source-bound frozen rebuilds. The two Canvas MULTI_FIB item structures reproduce exactly; correcting only their uniquely resolved literal-answer references yields matching grading. The established media-staging repair makes the three horse-image exports agree with native semantics and image contracts. All five repaired Python packages pass integrity. Completed the remaining QTI 2.1 presentation classification without changing native output or adding permanent tests.

- Converted-package public grading agrees for all 175 valid Canvas and 180 valid QTI 2.1 reference pairs; four Python reference failures remain separately recorded. Temporary Canvas presentation review accounts for all 126 differing fields: 92 incidental paragraph/serialization/choice-prefix differences and 34 fields where Rust preserves authored text lost by Python. Source hashes and exact visible-text matches substantiate the latter. No production compatibility workaround or permanent test was added.

- Classified the current HTML replay's 69 image-placement findings with temporary evidence: 49 contain only unstyled paragraph/known choice-prefix differences; the other 20 packages preserve source text that Python drops after rendered tables. All 68 affected fields match source text outside rendered tables/canvases, with source hashes verified. Broader semantic comparison remains open; production behavior and permanent tests are unchanged.

- Re-exported all 537 HTML corpus inputs with the current release binary: zero export failures and all 537 packages pass integrity without errors or warnings. Image-contract replay confirms the whitespace defect is resolved; remaining alt differences are redundant choice prefixes in eight packages. One-time converted Blackboard grading checks agree for all 177 available Python/native package pairs and detect changed score maxima and correct choices. Presentation comparison still has outstanding findings; these receipts do not claim full HTML parity.

- Current native benchmark `run-80228` completes all 181 inputs in both modes with zero CLI or library conversion failures: 18.124 seconds for Blackboard and 19.202 seconds for three formats. Exact path/hash binding matches the pinned Python baseline (197.234/445.820 seconds). All 721 emitted ZIPs pass integrity with zero errors or warnings; per-package hashes and checker identity are retained in the run directory. Formatting and strict xtask Clippy pass after the benchmark report correction.

- Removed a hard-coded success claim from the native benchmark report after a missing-shim run exposed the contradiction. Reports now direct readers to measured failure/output counts and the detailed receipt, including when a benchmark fails. No permanent test was added for this prose correction.

- Full plain-export corpus verification passes: `tests/_temp/parity_6191` compares 1,709 input banks across seven frozen-CLI formats and three registered-adapter formats with zero findings. This run precedes the image-only alt-text spacing fix; HTML-to-image findings remain under review.

- Independent final Blackboard writer/reader review passes after checking the pinned Python authoring validators and the established private-metadata decision. An ignored one-time proof verifies that carrier edits/removal preserve public score/content/media and that a public score mutation is detected. Broader HTML triage identified a real table alt-text bug: removing non-ASCII whitespace before collapsing it fuses distinct labels; the library now converts whitespace to ASCII spaces before filtering other characters. The existing regression expectation now preserves the word boundary. Focused tests, formatting, strict engine lint, and a real release-CLI boxplot conversion plus integrity check pass. The active plain run keeps its unchanged debug CLI binary.

- Refreshed native macOS RDKit verification: all three runtime success/error tests and the full 58-canvas rendering corpus pass with the installed shim. Temporary replay classifies the first 24 HTML comparison findings as observed wrapper/entity/loader differences while retaining content, image, style, and score mutations; this is diagnostic evidence, not a full corpus pass.

- Current Linux arm64 verification passed formatting, strict all-target Clippy, 320 workspace tests, and an optimized workspace build. The read-only source snapshot and command/image receipt are at `output_tables/linux_current/run_1790822794`; all build-input hashes still match. Consolidated the progress tracker to distinguish current proof, historical timings, and remaining completion work.

- Reverified the current macOS workspace after the dependency API and version-startup changes: formatting, strict all-target Clippy, workspace all-target tests, the locked optimized workspace build, and all 1,556 Python checks pass. Started a separate Linux arm64 verification from a hashed current-source snapshot; its result remains pending.

- Corrected development comparisons to normalize Canvas choice HTML serialization consistently with question HTML and accept removal of the unused RDKit loader after rasterization. Content, choice identity, scoring, and image checks remain active. A fresh browser-backed three-format HTML-to-image preflight passes with zero findings; retained replay and grading-program mutation checks also pass. No permanent tests or production behavior were added.

- Rewrote README through the readme-docs side job around instructor workflows: one editable BBQ source, Canvas/Blackboard packages, readable review output, and student practice. The copyable example creates all four documented artifacts and its Canvas package passes the native checker. README first-paragraph/link checks pass (69 tests); deeper implementation details remain linked from the landing page.

- Used vendored `devel/bump_version.py -cA`, which writes `VERSION` as `26.09` and Cargo as `26.9.0`. Removed the redundant compile-time string-equality startup guard that blocked conversion for equivalent version spellings. Release metadata validation now reuses the vendored Cargo normalization. Recorded the user's instruction to prioritize meaningful correctness and delivery issues.

- Recorded the user's clarification that parity means working like the Python package, not reproducing every nuance. Remaining comparison findings must demonstrate a practical content, grading, media, interoperability, or usability consequence before motivating implementation changes or new lasting tests.

- Resolved the retained plain comparator findings: styled color swatches now retain distinct visual identity, block-boundary layout whitespace is treated as equivalent, and NUM self-test markup may match its SHA-bound authored source when frozen Python merges paragraphs and spreads heading styles. The observed NUM output matches the authored DOM exactly. Comparator self-tests pass; fresh full verification remains required. Python cleanup verification passed 1,556 tests with nine existing line-limit warnings.

- Fresh full plain comparison completed: 1,709 inputs, 12 findings (11 self-test markup comparisons and one QTI2 visual-choice comparison), with stable executable and helper hashes in `tests/_temp/parity_40500/divergences.json`. The prior baseline had 199 findings. Applied the prevalidated helper annotation/import cleanup after completion. One-time markup diagnosis locates block-boundary whitespace/paragraph differences; final decisions and correction remain pending. Recorded the user's openness to small local implementations when simpler than dependencies.

- Recorded the user's preference for latest versions. The cssparser compatibility fix follows that preference by adapting to the locked current API instead of downgrading; reproducible builds continue to use `Cargo.lock` and `--locked`.

- The permitted full HTML run stopped before corpus generation because locked `cssparser` 0.38 removes `ParserInput`. Updated the rasterizer to construct `Parser` directly from the source string; all 74 rasterizer tests pass. The plain run is still using its original binary; rebuilding that CLI and retrying HTML waits for the plain run to finish so its executable identity stays stable.

- Diagnosed the first fresh HTML corpus run (`parity_47447`): it ended with 537 writer-outcome findings because the sandbox blocked frozen Python's Chromium MachPort startup. This is execution-environment evidence, not a product parity result. The unchanged single-case renderer succeeds with approved execution permission, and the full HTML corpus run has restarted in that permitted environment. The plain run remains active.

- Applied the user's KISS/test-lifetime guidance to the two newly added parity test modules. Moved their 15 checks into ignored `tests/_temp/rebuild_verification/check_*.py` for explicit one-time execution: these prove the rebuild's comparison rules and frozen-Python repairs, rather than lasting product behavior. Updated the progress tracker to classify that evidence correctly. Fresh optimized workspace build and all three native RDKit runtime checks also pass.

- Refreshed the M13/M14/M19 rows in `refactor_progress.md` with current retained-case evidence, live full-corpus artifact locations, and fresh Rust checks. Explicitly marked historical Linux and benchmark/integrity results as predating current writer changes, with final refreshes still required.

- Added permanent human-readable projection tests covering equivalent entity spelling, numeric padding/grouping, and border width, plus rejection of changed values, labels, row order, answer marks, emphasis, long-number precision, and tolerance. All 15 human-readable/Canvas focused tests pass through the normal Python bootstrap; the Canvas tests also pass with the explicit frozen oracle.

- Added permanent Canvas reference-repair tests using a generated two-blank source question. All six checks pass: the repair records source/archive-bound answer mappings, while changed answer text, score, namespace, response identifier, and source content are rejected. The running corpus comparators were left fixed.

- Updated the local Rust release validation script to invoke every `xtask` through `cargo run --locked -p xtask`, matching the locked build/test dependency policy. Shell syntax, workspace formatting, and release metadata checks pass; all seven workspace packages declare version 26.9.0. Fresh corpus runs remain active.

- Restored the checkout disk budget by relocating two historical Linux debug-build caches to `/Users/vosslab/.cache/qti-rust-stale-linux-debug`, preserving snapshot sources and release artifacts. Disk-budget and dependency checks pass (66 tests); relocation receipt: `tests/_temp/root_linux_debug_cache_relocation.json`. Fresh plain and HTML-to-image corpus runs are underway with comparator sources held stable; remaining helper annotations and unused imports will be fixed after these runs finish.

- Fresh Rust workspace verification passes 324 tests and strict all-target Clippy. Started a fresh full plain corpus run with stable comparator sources. Python policy verification reports 1531 passes and 13 failures covering helper annotations, an undeclared HTML parser dependency, two unused imports, and generated checkout disk usage; declared the development-only `lxml` dependency. Remaining fixes and full corpus results are pending.

- All 111 retained human-readable findings now compare equally (`tests/_temp/root_human_final_receipt.json`). Reused source-bound frozen regeneration for the four nested-label cases; recognized conventional thousands grouping only inside complete table cells and numeric equivalence only in the generated tolerance note. Added changed-value/malformed-grouping checks. Extracted comparator mutation checks into a focused module to restore the source line limit, and removed an arbitrary one-second self-test gate while retaining elapsed-time reporting. Fresh broad verification remains required.

- Added bounded human-readable fancy-grid comparison: complete grid blocks retain frame topology, cell order, labels, and exact numeric value while tolerating border width and decimal padding. Exact Decimal tuple identities avoid precision rounding. Retained replay resolves 105 of 111 human-readable findings (`tests/_temp/root_human_table_receipt.json`); permanent mutation checks reject changed values, labels, cell order, and numbers outside tables. Six findings and fresh full verification remain open.

- Compare human-readable HTML after browser entity decoding while retaining markup and answer marks. A retained-corpus probe resolves 58 of 111 findings (`tests/_temp/root_human_dom_probe.json`); focused checks accept equivalent entity spelling and reject changed grading marks, numbers, and emphasis. Remaining human-readable differences and fresh full verification remain open.

- Verified the integrated label fallback rejects changed native grading/content, changed frozen content/namespace, and stale source SHA (`tests/_temp/root_choice_repair_mutations.json`). Permanent boundary checks retain nested markup attributes and later authored label-like text, reject decimals and unlabelled choices, and restrict correction to the demonstrated frozen-strip no-op. The normal comparator self-test entry point passes. Fresh broad plain/HTML verification and human-readable semantic comparison remain open.

- Integrated a source-bound nested-choice-label fallback for plain MC comparisons. It requires one MC source item, unchanged source SHA, the demonstrated pinned-strip no-op on every choice and answer, unique corrected choice identity, and reproduced original frozen semantics before comparing corrected regeneration against strict native output. All 63 retained comparisons across nine formats pass (`tests/_temp/root_choice_repair_integrated_receipt.json`). Self-test grading, question structure, images, source selection, and native control usability remain checked. Human-readable and HTML-to-image lanes remain separate; negative mutation coverage and fresh broad verification are pending.

- Reproduced all 63 non-human-readable frozen artifacts across the seven label-defect sources before correcting their choices. Original full semantic projections/readback and self-test question/grading structures agree; source-corrected regeneration then agrees with all 63 native outputs (`tests/_temp/root_label_corrected_writer_receipt.json`). Frozen Blackboard reproduction retains its existing bounded feedback repair on both frozen sides. Human-readable rendering remains separate, and the label correction is still a scratch prototype awaiting integration and negative mutation coverage.

- Regenerated all ten frozen writer formats from source-bound MC choices after removing only the demonstrated nested label that the pinned stripper leaves untouched. Complete semantic comparisons agree with native output for 63 of 70 cases across nine formats, including QTI scoring, Blackboard readback, and self-test grading/question structure (`tests/_temp/root_label_corrected_writer_receipt.json`). Seven human-readable cases remain separate rendering work. This is scratch evidence; source-bound correction and exact original-artifact verification are not yet integrated.

- Verified the seven duplicated-label cases against unchanged authored source hashes. Each bank contains one MC item; all 34 choices retain their nested HTML display label because the pinned prefix-strip function is a no-op (`tests/_temp/root_choice_label_source_probe.json`). The next repair must derive corrected choices from those source fields, retain all surrounding markup/content, and verify writer output and grading rather than globally dropping repeated text.

- Replayed the actual three Canvas source cases through the fresh 21-input harness: Canvas semantic and integrity findings are zero, with stable executable/helper hashes (`tests/_temp/parity_31034/divergences.json`). The stricter self-test comparison exposed generated blank-input attributes and insignificant list-container spaces; normalized blanks by authored name, effective type, and all non-generated attributes while retaining separate answer/usability checks. All three source-bound self-tests now agree (`tests/_temp/root_multifib_structure_replay.json`). Permanent changed-name/type/required-attribute checks and comparator self-tests pass. Human-readable findings remain open.

- Extracted parity ZIP integrity accounting into a focused Rust module and run it after semantic comparison creates source-repair receipts. Canvas accounting now requires matching source/archive SHA values, a nonempty correction list, exactly one known dangling-reference finding per correction, and zero native integrity errors. Other findings remain failures. Focused `xtask` tests pass, including changed source/archive and unexpected/missing error rejection. Actual three-case harness replay remains required.

- Integrated frozen Canvas MULTI_FIB reference repair behind exact namespace-preserving pinned item reproduction and unchanged source SHA verification. Full strict projection agrees for all three retained cases (`tests/_temp/root_canvas_repair_integrated_receipt.json`). Changed answer text, grading weight, predicate namespace, response identifier, and source SHA are rejected (`tests/_temp/root_canvas_repair_mutations.json`). Native projection remains strict. Raw archive hashes and each reference correction are recorded beside the Python output. Integrity accounting and permanent repair mutation coverage remain open.

- Bound the three retained Canvas MULTI_FIB defects to their unchanged authored source SHA receipts and regenerated each item with the pinned writer. All three archived item structures reproduce exactly, including numeric blanks and both alternative gene-order answers (`tests/_temp/root_canvas_source_binding_probe.json`). The 14-reference scratch repair agrees with native full semantic projection; integration still requires namespace-preserving source verification, negative mutation checks, and explicit integrity repair accounting.

- Diagnosed all three retained Canvas grading-reference failures as frozen Python MULTI_FIB answer text used in place of declared choice identifiers; native output already resolves those references. A scratch copy replaces only 14 unambiguous text-to-label references across the three items and then agrees with native output across the full strict semantic projection (`tests/_temp/root_canvas_reference_probe.json`). Source-bound repair grammar, adversarial checks, and integrity classification remain required before accepting these cases in the broad parity harness.

- Integrated browser-DOM paragraph comparison for Blackboard HTML tokens while retaining every other semantic/readback field and separate script/style payloads and paths. Fresh retained replay resolves all 19 layout findings and preserves all seven duplicated-label findings (`tests/_temp/root_bb_dom_integrated_receipt.json`). Added permanent changed-text, image-reference/alt-text, table-order, style, and script-content/placement checks to the normal oracle self-test entry point, together with the strict QTI 2.1 self-tests. The integrated self-test entry point passes.

- Integrated source-bound self-test statement structure comparison. It preserves element order, attributes, emphasis, script/style payloads, comments, table structure, and meaningful inline spaces while accepting unstyled paragraph/line-break layout and insignificant table-container whitespace. All nine retained selected-item comparisons pass (`tests/_temp/root_structure_integrated_receipt.json`); permanent mutations detect changed script sources/content/placement, emphasis, paragraph/table styling, table order, and missing inline spaces. Comparator self-tests pass. Broad corpus verification remains open.

- Integrated the native self-test usability check for disabled controls, disabled fieldset ancestry, and read-only answer inputs. The browser's first-legend fieldset exemption remains supported. All nine retained native self-tests pass, and permanent comparator checks reject each unusable-control mutation (`tests/_temp/root_control_integrated_receipt.json`). Comparator self-tests pass; question-structure normalization remains a tested scratch prototype pending integration.

- Completed fresh full plain verification of 1,709 projected inputs: 199 findings remain, with unchanged executable and helper hashes (`tests/_temp/parity_88370/divergences.json`). Seven source cases produce cross-writer differences; the first inspected case exposes frozen Python duplicated display labels rather than a native missing answer. Human-readable, Blackboard DOM, and three Canvas grading-reference findings remain open. Integrated structural disambiguation for repeated visible MATCH diagrams within each matching set; permanent comparator checks detect changed correct pairs and reject identical ambiguous diagrams. Comparator self-tests pass.

- Prepared scratch comparator improvements while preserving the active full-corpus run's executable and helpers unchanged. The Blackboard DOM probe agrees on all 19 retained cases and detects changed text, image references/alt text, table order, styling, and script content/placement. The MATCH probe demonstrates a collapsed correct-pair difference for distinct diagrams with identical visible labels and tests structural disambiguation within each matching set. Source-bound self-test structure probes agree on nine retained selected items while detecting changed scripts, emphasis, table order/styling, and missing inline spaces; authored thin spaces remain distinct. A separate control probe rejects disabled/read-only answer controls. These probes remain unintegrated, and full parity is still unproven (`tests/_temp/root_bb_dom_mutations.json`, `tests/_temp/root_match_visual_probe.json`, `tests/_temp/root_selftest_layout_probe.json`, `tests/_temp/root_selftest_enabled_controls.json`).

- Extended implied QTI paragraph closure to retain attributed leading paragraphs while promoting block content. All four retained list/table/script packages now agree fully (`tests/_temp/root_qti_paragraph_all_four_receipt.json`). Exact text is preserved; mutations for text, paragraph style, script body/placement, and foreign namespaces are detected (`tests/_temp/root_qti_paragraph_dom_mutations.json`). Existing comparator self-tests pass. Starting fresh full plain corpus verification.

- Added bounded QTI body normalization for HTML-implied closure of unstyled paragraphs before block content. It preserves exact concatenated text, text tails, child ordering, and every nonparagraph node; attributed paragraphs remain unchanged. Three retained paired packages, including RDKit script placement, now agree fully in the prototype; one attributed-paragraph case remains open (`tests/_temp/root_qti_paragraph_tree_receipt.json`). Applied normalization only to a copied item body, leaving grading declarations untouched.

- Verified eight QTI writer tests including the new plain-prompt entity regression. Rebuilt CLI and replayed the actual thermodynamics source: full strict paired projection now agrees (`tests/_temp/root_qti_plain_prompt_replay.json`). Remaining list/table cases contain block elements inside paragraph XML on the native side versus HTML-implied closed paragraphs in frozen output; preserving text tails and script placement is required for the next normalization step.

- Corrected QTI 2.1 plain-text prompt entity handling for MATCH/ORDER: authored HTML entities are converted to their displayed characters before XML serialization, while literal markup remains escaped text. This prevents double-escaped Greek/math entities in duplicated prompt text. Focused writer verification follows.

- Fresh native replay resolved nine previously retained QTI differences; remaining cases were diagnosed directly. Corrected visible-text extraction to ignore whitespace-only XML pretty-print indentation containing newlines while retaining authored inline spacing. Four MathML/restriction-map packages now agree across full paired strict projection (`tests/_temp/root_qti_indentation_receipt.json`); MathML text and inline-space-removal mutations remain detected. Comparator self-tests pass.

- Diagnosed the remaining duplicate-text QTI choice case: five phylogenetic diagrams contain only four distinct leaf-label strings, but all five structural identities are distinct and match the frozen output. Strict choice dereferencing now uses structural identities for repeated diagram text; full paired projection agrees (`tests/_temp/root_qti_duplicate_visible_integrated.json`). Existing comparator self-tests pass. Retained package replay is historical artifact evidence; fresh native corpus replay remains required.

- Normalized QTI empty, unstyled paragraph wrappers before QTI block content while preserving all child/tail content and attributed paragraphs. Both multi-alternative gene-order packages now agree across the full repaired strict projection (`tests/_temp/root_qti_multivalue_normalized_pair.json`). Mutation checks detect changed authored text, blank references, removed blanks, and styled paragraphs, while accepting equivalent block nesting (`tests/_temp/root_qti_paragraph_mutation_receipt.json`). Seven native QTI tests pass; comparator grammar self-tests pass.

- Extended the bounded frozen MULTI_FIB grammar repair to retain every distinct nonempty answer alternative in explicit OR predicates while correcting single-cardinality declarations. Existing nine negative grammar probes pass. Both affected current native packages agree with repaired frozen grading/interactions; paragraph token serialization remains different (`tests/_temp/root_qti_multivalue_repaired_pair.json`). Added a permanent native declaration/alternative regression; initial test compilation exposed quick-xml name comparison API differences, corrected and rerunning. Engine strict clippy passed before the new regression.

- Verified current native QTI outputs for both retained gene-order cases with strict projection and direct predicate evaluation: each authored alternative earns 100 overall with the other three blanks correct; an incorrect gene order earns 75, preserving 25-point blank contributions (`tests/_temp/root_qti_multivalue_current/receipt.json`). Each single-cardinality declaration contains exactly one correctResponse value. Frozen alternative-answer source repair still needs extension before paired corpus comparison.

- Verified six QTI writer tests after the alternative-answer fix. Current native replay exposed the strict comparator lacking the standard OR predicate; added bounded recursive OR projection requiring at least two predicates and rejecting extra attributes/text. Actual multi-alternative grading proof remains pending.

- Fresh QTI visual-comparison fixture run passed 59 banks with zero divergences (`tests/_temp/parity_80326/divergences.json`). Fixed MULTI_FIB alternatives in native QTI 2.1 output: each single-cardinality blank declares one correct value, while an explicit OR of string matches accepts every authored alternative. Single-answer grading remains equivalent. Two retained corpus declarations exposed this source-inherited defect (`tests/_temp/root_qti_multivalue_diagnosis.json`); focused verification follows.

- Applied diagram identity to nontext choice markup: both retained pedigree packages now agree across all strict semantic fields (33 items; `tests/_temp/root_qti_visual_full_pair_receipt.json`). Actual package mutations for diagram styling, namespace, grading weight, and unknown references are rejected. Independent review found foreign paragraph namespace erasure; repaired flattening to exact QTI paragraph wrappers and verified the foreign-wrapper mutation plus all 33 paired items (`tests/_temp/root_qti_foreign_wrapper_receipt.json`).

- Normalized unstyled empty paragraph wrappers only inside nontext QTI diagram identity. All 33 retained paired pedigree items now agree on interaction and grading semantics; only whole-question markup serialization remains different (`tests/_temp/root_qti_visual_wrapper_receipt.json`). Cell styling, wrapper styling, and namespace mutations change identity; empty wrapper and generated outer-ID changes do not (`tests/_temp/root_qti_visual_wrapper_mutations.json`). Comparator self-tests and `pyflakes` pass; independent review requested.

- Paired replay of the two retained pedigree packages now parses and validates both sides, but whole-choice projections still differ because frozen HTML closes empty paragraph wrappers before tables while native XML retains nested paragraph wrappers. All 165 actual diagram table subtrees match exactly after namespace-preserving structural projection (`tests/_temp/root_qti_visual_subtree_receipt.json`). This establishes the next normalization target, not full package parity.

- Integrated namespace-preserving structural identities for nontext QTI choices into strict semantic dereferencing. Empty choices and duplicate identifiers remain errors; visual attributes remain bound. Both retained visual-choice packages now pass the full item/grading projection (33 assessment items; `tests/_temp/root_qti_visual_integrated_receipt.json`). Existing QTI comparator self-tests and generated-ID/style-change probes pass; full paired parity replay remains pending.

- Verified integrated native accessible-control validation against nine retained outputs and a fresh 59-bank fixture replay with zero divergences (`tests/_temp/parity_74977/divergences.json`). Prototyped namespace-preserving structural identities for QTI choices without text: all 165 retained visual choices have supported diagram structure; generated outer IDs do not affect identity, visual style mutations are detected, and empty choices fail (`tests/_temp/root_qti_visual_identity_receipt.json`). Prototype is not yet promoted into the parity comparator.

- Added native self-test accessible-control association validation: controls need nonempty names, IDs must be unique, and labelledby/describedby references must resolve. Nine retained native selection outputs pass; missing labels, empty names, and broken description references fail (`tests/_temp/root_selftest_controls/receipt.json`). The image-check fixture replay passed all 59 banks with zero divergences (`tests/_temp/parity_73109/divergences.json`). Full source-derived control semantics remain under development.

- Strengthened source-selected self-test comparisons with ordered authored-image content, alternative-text, and title checks. Empty, external, and escaping media references fail. Focused mutations reject changed alt text, changed image content, removed images, and external references (`tests/_temp/root_selftest_images/receipt.json`); `pyflakes` passes. Structured prompt and non-MATCH accessible-control verification remains open.

- Fresh fixture verification after the MATCH mutation-probe update passed: 59 banks, zero divergences (`tests/_temp/parity_70918/divergences.json`). Independent review accepted the bounded text2qti and Okla source-defect contracts, but requires structured prompt/media and non-MATCH accessible-control checks before random self-test corpus selection can establish parity. Diagnosed all 19 remaining retained Blackboard comparisons: readback and every semantic field except HTML token serialization agree; DOM normalization still needs explicit mutation proof.

- Updated the MATCH comparator mutation probe to check visible grading text and accessible names independently, matching the source-bound comparison contract.

- Resolved the random self-test MATCH table-label comparison by using visible authored choices for grading and separately validating explicit accessible names against source-rendered aliases or the visible choice. All nine retained random selections now validate, including full nucleotide prompts; a wrong accessible name remains rejected. `pyflakes` passes.

- Strengthened self-test MATCH projection with DOM-based direct-row-cell extraction, preserving nested nucleotide prompt text previously lost by regex cell matching. Eight valid random-selection cases still pass; unknown source CRC, changed question, and changed answer mutations are rejected. Full nucleotide prompt is retained; its accessibility-label difference remains open. `pyflakes` passes.

- Added source-bound random self-test selection checks: each output CRC must identify one authored item, and its statement/answer projection must match that item rendered by the pinned writer. Eight of nine retained selection differences validate; the nucleotide MATCH case still fails because frozen accessibility labels include table-frame characters. That finding remains open rather than being ignored. `pyflakes` passes.

- Verified the integrated source-bound Okla and text2qti comparisons in a fresh 59-bank fixture run: zero divergences with unchanged binary/support hashes (`tests/_temp/parity_66856/divergences.json`). Random self-test corpus selection remains a separate unresolved comparison contract.

- Added a bounded Okla source-defect receipt for the pinned MATCH reader splitting HTML closing tags at the first slash. Acceptance requires identical paired outputs, unchanged bound source, exact regenerated frozen bytes, the specific closing-tag precondition, and XMLSyntaxError on both outputs. All eight retained cases satisfy those checks; changed native bytes remain rejected. `pyflakes` and diff checks pass.

- Replayed all seven retained text2qti failures: 291 authored items recovered, all native readbacks match. Changed source binding and modified export are rejected. Fresh integrated fixture run passes 59 banks with zero divergences and unchanged helper/binary hashes (`tests/_temp/parity_64652/divergences.json`).

- Added source-bound frozen text2qti readback repair for missing question separators. It requires byte-exact reconstruction from the pinned writer and unchanged source SHA, retains the raw artifact, and records per-block hashes. Actual retained 20-question failure now matches native readback; mutated raw export is rejected. Both helper modules pass `pyflakes`.

- Added input path, input SHA, index, and retained artifact-directory attribution to per-bank parity findings. A focused test verifies correct binding without attributing global checks to a bank; Cargo check, focused test, and strict xtask Clippy pass. The repaired nested-label CLI replay also passes all four actual corpus cases with content and plot marks retained.

- Verified the shared nested-prefix normalization across the full engine library suite: 119 tests pass after the human-readable duplicated-label repair.

- Rebuilt both native CLIs after the nested human-readable choice-label repair. Replaced the numeric-spelling regression literal with its equivalent Rust Unicode escape; focused core string-related tests pass (6). Actual four-case corpus replay remains pending.

- Fresh integrated plain parity fixtures pass: 59 banks, zero divergences (`tests/_temp/parity_60633/divergences.json`). Independently verified native binary, oracle, and every support-helper hash stayed unchanged throughout the run; full plain/HTML corpus certification remains pending.

- Render nested authored multiple-choice labels only once in human-readable output. The shared
  choice normalizer now removes an `A.`-style prefix after leading table markup, so the writer's
  own choice label remains the single visible label. Focused core and human-readable regressions
  cover the boxplot shape.

- Promoted the independently approved package-image contract to `xtask/support/parity_images.py` and integrated the parity oracle wrapper. The promoted implementation passes renamed-image and changed-raster acceptance, missing references/order/alt/attribute/namespace/CDATA mutation checks, and retained Blackboard/QTI2 package comparisons. Both support modules pass `pyflakes`; full corpus acceptance remains pending.

- Repaired the scratch image comparator namespace loss identified by independent review. Placement now retains complete canonical element names; foreign namespace and `xml:lang` mutation checks pass alongside raw CDATA, image references, and retained-package probes. Independent approval remains required before promotion.

- Preserve authored numeric spelling and grouped values while rendering readable HTML tables.
  The frozen `tabulate` display-only conversion can remove comma grouping or trailing zeroes;
  retain source text such as `4,400` and `2.80` because it is clearer and does not change the
  number's meaning. A focused inline-table regression covers both forms.

- Fixed corpus projection media authority: projected banks use isolated source-origin directories and retain referenced relative images. Verified equal records from two origins preserve different same-named images across all ten projections; the previously failing projected horse bank passes all ten native writers and all three ZIP integrity checks. Full projection verification retains all 181 source banks, 1,709 writer projections, and unchanged authored records (`tests/_temp/root_projection_full/receipt.json`). `pyflakes` passes.

- Independently checked the stable-helper Blackboard replay receipt: all 47 writer/comparator invocations complete, 28 match and 19 paragraph-structure cases remain. Human replay mapped all 146 original findings; displayed table-text differences are being reviewed against practical content/readability requirements before further changes.

- Verified the retained horse-coat media failure against its original source bank: native Blackboard conversion and integrity checking pass, all XML parses, and the source PNG is packaged unchanged. Retained source/binary/package hashes in `tests/_temp/root_horse_direct/receipt.json`; Expanded replay passes all ten native writers and all three ZIP integrity checks (`tests/_temp/root_horse_direct/all_writers/receipt.json`); projected media authority still needs repair.

- Carried practical content, grading, upload-validation, and image-reference gates into the canonical active Rust plan; updated progress with the terminal HTML corpus findings and independently verified QTI2 XML repairs. Markdown-link and README checks pass (69 tests).

- Limit the HTML-to-image corpus lane to its three ZIP-producing packaging writers before
  writer receipt comparison. The lane retains every corpus bank but no longer treats intentionally
  skipped text, YAML, Aiken, or browser-only formats as missing native writer outcomes.

- Rebuild the native converter after corpus writer repairs and independently replay all six
  QTI 2.1 invalid-XML cases (0058, 0084, 0093, 0103, 0113, 1705). Every XML member parses and
  every rebuilt package passes native integrity. The combined engine library passes 118 tests.
  Retain the terminal HTML corpus receipt separately: its non-packaging receipt expectations
  are harness defects, while remaining package differences require explicit review.

- Restore frozen Blackboard NUM XML canonicalization: answer, tolerance, and lower/upper
  grading bounds now use six fixed decimals with trailing zeros trimmed. Replace the narrow
  table-whitespace regex with an HTML token walk that removes direct table-structure whitespace
  through comments while preserving comments and all cell content. Focused NUM grading and
  comment-separated table regressions pass.

- Repair Blackboard QTI 2.1 XML serialization for authored HTML fragments: canonicalize HTML
  attributes, retain raw script and style payloads as XML CDATA, and escape non-entity ampersands.
  This keeps JavaScript comparisons, URL query strings, and legacy unquoted table attributes
  well-formed without changing their rendered meaning.

- Restore frozen human-readable text normalization for inline word boundaries and readable ASCII
  tables by using the shared Python-compatible normalizer instead of flattening HTML with a DOM
  text iterator.

- Classify the 47 Blackboard semantic-projection differences in retained full-parity receipt
  `parity_48349`: eleven NUM exports bypass the frozen six-decimal canonical XML form, changing
  the recovered answer or tolerance and score-window bounds; the other 36 retain whitespace-only
  nodes in table structure that the frozen lxml sanitizer removes. Reapplying the frozen sanitizer
  makes all 36 HTML cases equal, including comment-separated cells, so repair the writer rather
  than weaken semantic comparison.

- Retain the terminal full 1,709-bank parity receipt with 275 differences. Assign exact
  native QTI XML failures, human-readable differences and Blackboard differences to their
  owners; keep comparator and projected-media fixture diagnosis separate. Preserve the
  still-running HTML corpus executable and helpers unchanged.

- Refresh the release-readiness ledger with the completed current macOS checks and fresh
  isolated Linux arm64/amd64 evidence; retain committed-HEAD release validation and full
  parity as outstanding requirements.

- Record independent acceptance of fresh M19 performance/integrity and seven-kind QTI
  receipts. Review identifies a generated-image-name false-failure in the HTML comparator;
  retain live runs unchanged and defer the semantic reference fix until receipts finish.

- Refresh the isolated current-worktree Linux receipt: a manifest-bound arm64
  snapshot passes formatting, strict linting, all-target tests, and release
  builds, then cross-builds and executes the two x86_64 CLIs in the AMD64
  runtime. Retain this as snapshot evidence until the user commits the release
  source and final committed-HEAD validation can run.

- Align the parity workstream wording with the existing conversion contract and human
  guidance: generated image names may differ, while placement, authored alt text, unique
  package names, and resolving PNG references remain required.

- Pass the browser-permitted HTML-to-image fixture lane with zero divergences: all three
  packaging formats agree structurally for its real input bank. Preserve receipt
  `tests/_temp/parity_64427/divergences.json` and start the full converted-corpus parity lane.

- Recheck current workspace formatting, release metadata, and strict all-target Clippy:
  all pass. Retain the terminal sandbox-denied HTML-to-image fixture result and rerun
  the unchanged check with local browser execution permitted; keep full corpus parity live.

- Complete the fresh native benchmark: Blackboard export takes 19.313 seconds and three
  formats take 22.002 seconds, both with zero failures. Verify exact ordered input hashes
  and manifest against the pinned Python baseline (197.234 and 445.820 seconds); refresh
  the M19 decision with current binary hashes and all-721-package integrity evidence.

- Certify all 721 freshly rebuilt native benchmark ZIPs: zero package integrity errors,
  zero warnings, unchanged output hashes, and unchanged checker binary. Retain the separate
  integrity receipt beside benchmark run-44801 while performance metrics finish.

- Verify the completed XML helper hygiene repairs with the full repository test directory:
  1,406 tests pass with eight source-size advisory warnings. Rebuild both native release
  binaries successfully before rerunning the corpus performance and integrity certification.

- Independently replay all seven current QTI 2.1 package pairs through the reviewed frozen
  MULTI_FIB repair and strict native projection: all agree and all seven native packages
  pass the package integrity checker. The repair rejects nine grammar mutations. Fresh
  repository checks pass 1,404 cases with two remaining Blackboard helper hygiene failures.

- Independently prove the repaired frozen MULTI_FIB projection equals native output for its
  two authored blanks. Full repository checks expose six helper-hygiene/guidance failures;
  shorten human guidance to the user's words and assign safe XML parsing, annotation, and
  shebang fixes to helper owners before rerunning the suite.

- Reject non-finite Blackboard private numeric tolerances in comparator carrier validation,
  matching the native typed contract. Add direct regressions for NaN, infinities, overflow,
  and negative tolerance; close the independent six-kind review's narrow validation finding.

- Refresh the milestone ledger with independently replayed six-kind Blackboard and QTI 2.1
  package results and actual emitted MULTI_FIB scoring evidence. Keep the frozen-source
  repair proof, fresh fixture gate, and complete corpus gate explicitly open.

- Independently replay all six Blackboard package pairs with explicit frozen-only repairs:
  all agree; MC records two repaired frozen feedback edges and the other kinds record none.
  Replay the six valid frozen QTI 2.1 kinds successfully, including numerically equivalent
  NUM values. Keep malformed frozen MULTI_FIB, private-carrier proof binding, and the full
  corpus run as separate requirements; use the accepted 13360 carrier receipt, not rejected
  earlier numeric-parsing evidence.

- Make Blackboard projection strict for native packages by default. Only the frozen-Python
  comparison explicitly enables the reviewed dangling-feedback repair, with repair provenance
  available to receipts. Independently reject the real native ZIP mutation and add a permanent
  helper selftest proving explicit frozen repair agrees with the corrected package.

- Pass a fresh locked all-target workspace Rust test run after the Blackboard feedback and
  QTI additive-scoring repairs. Independently reproduce that the current Blackboard comparator
  still normalizes a native dangling-feedback mutant; require explicit frozen-only repair
  handling before accepting its six-type package comparison or the full corpus gate.

- Independently pass all 19 focused Blackboard tests, generate a fresh native MC pool,
  and verify every emitted feedback link resolves to an itemfeedback target. Strict native
  projection retains all four grading conditions. Require frozen-only repair provenance in
  the comparator so a native dangling-reference regression cannot be normalized away.

- Generate a fresh native MULTI_FIB QTI 2.1 ZIP and independently evaluate its emitted
  score expressions: neither blank correct yields 0, either blank yields 50, and both
  yield 100. Replacing accumulation with assignment yields 50 for both and fails the
  expected result. Keep this focused grading proof separate from the corpus gate.

- Independently run the six focused QTI 2.1 tests successfully, then check the additive
  score initialization against the official QTI 2.1 binding. Identify an invalid
  `outcomeDeclaration@defaultValue` attribute: the standard defines a `defaultValue` element,
  with omitted numeric defaults initialized to zero. Reopen the writer and comparator fix
  rather than treating well-formed XML or passing integrity tests as format validity.

- Repair QTI 2.1 MULTI_FIB scoring so each correct blank adds its contribution to SCORE from
  a declared zero default. The strict QTI projection recognizes only the corresponding
  `sum(SCORE, float contribution)` form and records the outcome default as grading semantics.

- Clarify the user's upload requirement: validate XML, target-format package structure,
  resource references, and grading, and distinguish those checks from actual LMS import
  evidence. Preserve valid repairs when frozen output is malformed.

- Record the user's requirement for practical acceptance gates and amend the active plan:
  compare content, grading, interoperability, accessibility, and usable rendering rather
  than bytes, pixels, or invented timing cutoffs. Distinguish evidence-binding hashes from
  output equality; assess equivalent XML defaults and generated identifiers by behavior.

- Record fresh strict Blackboard six-type package comparison as failed: MATCH agrees;
  remaining kinds expose missing render attributes, unresolved generated-feedback identity,
  and the separately approved private-metadata carrier. Keep exact repairs and proof lanes
  distinct from package equality. Review frozen QTI 2.1 MULTI_FIB's invalid interaction and
  QTI 1 scoring markup; expose native replacement scoring where per-blank additive scoring
  is required, and reopen the production fix with partial/full-answer regressions.

- Independently compare freshly retained QTI 2.1 MC, MA, MATCH, and FIB Python/Rust package
  pairs: each projects one item and agrees. Keep the all-seven receipt incomplete: the
  frozen MULTI_FIB artifact fails strict parsing on its lowercase interaction attribute,
  requiring the existing named repair contract to be proven without weakening the parser.

- Derive the development helper import root from `source_me.sh` itself, preserving frozen
  oracle precedence while allowing qualified comparator imports from any caller directory.
  Source the script from `/private/tmp` and successfully run both comparator selftests;
  replace the stale disabled-import comments with the active bootstrap contract.

- Re-run repository Python checks after qualified helper imports: all 1,350 tests pass
  with seven source-size warnings. Record the reviewed bounded Blackboard MA label-reference
  grammar; require explicit reference resolution rather than a source-defect exception.
  Independently expose a remaining Blackboard fragment comparator gap where moving text
  into a bold element produces the same projection, and require ordered text ownership.

- Independently replay the repaired QTI 2.1 namespace and leaf-value checks: foreign choices,
  foreign interactions, and nested correct-response markup now reject explicitly. Both QTI
  and Blackboard comparator adversarial selftests pass. Re-run all 17 focused Blackboard
  engine tests successfully after score-action and negated-choice reader repairs; exact
  frozen-package grading comparison remains a separate acceptance requirement.

- Independently reject the current QTI 2.1 comparator after three direct mutations remain
  accepted: foreign-namespace choices, foreign-namespace interactions, and child markup inside
  correct-response values. Reopen namespace-exact structural discovery and leaf-value checks;
  a passing helper selftest and one real package pair do not cover these invariants.

 - Repair Blackboard Original response processing for every supported question kind: declare SCORE outcomes, preserve frozen correct and incorrect score actions, and emit the paired feedback branches. The reader now ignores MA predicates under `<not>` so the exact frozen all-choice condition preserves distractors on native round trips. Seventeen focused Blackboard tests and strict engine Clippy pass.

- Add the frozen Blackboard choice-label and text-entry renderer attributes for all six native response shapes. Repair the frozen writer's dangling per-choice Blackboard feedback links: retain each zero-score label penalty and emit `displayfeedback` only for authored `itemfeedback` targets.
- Independently replay the strengthened QTI 2.1 projector against the retained real Python
  and Rust package pair at parity input 0005: both project one item and agree across the full
  projected fields. Its adversarial selftest passes. Keep seven-kind real-artifact coverage
  and the complete corpus gate separate from this focused evidence.

- Bind parity receipts to the initial and final SHA-256 inventory of every Python support
  module, including the new QTI 2.1 and Blackboard comparators. Reject helper additions,
  deletions, or content changes during a run. The focused inventory mutation regression
  and strict all-target xtask Clippy pass.

- Expose a Blackboard pool grading gap through independent package projection: native answer
  conditions omit the frozen writer's score actions. Keep full parity acceptance open while
  the production writer is repaired across supported question kinds. Independently demonstrate
  that the initial QTI 2.1 projection also misses a hidden-choice style mutation; require
  structured fragment preservation before accepting the strengthened comparator.

- Align human-readable rendering with the frozen Python source checks for RDKit and MathML in
  MC/MA choices, MATCH fields, FIB answers, and ORDER answers. An all-skipped bank returns no
  document instead of producing unsupported canvas markup.

- Refreshed the M12 audit to replace its stale process-test gaps with the actual
  eight passing CLI contract tests, keeping full export parity acceptance under
  the separate, incomplete M13 gate.

- Verified Blackboard's paired private-metadata regression on the same emitted
  package: present fields preserve authored MA/NUM options, while removing all
  four fields restores the frozen reader defaults and bound-derived tolerance.
  Fifteen focused package tests and strict engine lint pass.

- Verified locked all-target workspace tests after BBQ and Okla readers restored
  the pinned MA defaults (`min_answers_required=1`, `allow_all_correct=true`).
  Started the renewed full 1,709-bank parity gate; its terminal acceptance remains
  separate from these focused and workspace checks.

- Bound the remaining reader comparisons to explicit reviewed contracts: BBQ
  and Okla must restore frozen MA defaults; Blackboard private metadata must
  prove authored-value preservation and frozen behavior when absent; text2qti
  retains the previously approved blank-delimiter repair with both actual
  frozen and native readback projections. No general reader normalization is
  used to accept these differences.

- Restore the pinned Python defaults for MA grading options omitted by BBQ and Oklahoma BQGen
  reader formats: require one answer and allow all choices to be correct.

- Accepted the independently reproduced M19 structural v7 receipt: all 181
  inputs, 515 items, 3,426 fields, 510 tables, 141 canvases, and 650 generated
  images agree with the bound frozen selection authority with zero failures.
  Canvas grading comparisons now pass; fixture parity identified remaining
  reader-default and source-preservation differences for separate resolution.

- Verified locked all-target workspace tests after the Canvas outcome declaration
  and MC continuation repairs, including the renewed structural audit scanner
  controls and grading-harness integration.

- Reverified the complete repository base test lane after the structural-marker
  and grading-oracle updates: 1,350 tests pass, including source-size and build
  cache budget checks.

- Verified the benchmark after the Canvas `SCORE` target repair: both 181-input
  modes completed without failures and retained exact pinned baseline provenance.
  Kept final acceptance open for the newly identified MC `continue` mismatch and
  independent structural provenance review.

- Verified locked all-target workspace tests after the Canvas `SCORE` target
  repair. The renewed structural receipt covers all 181 inputs with zero
  failures; independent acceptance remains pending.

- Verified the post-NUM-repair native benchmark: both 181-input modes complete
  with zero failures, exact pinned baseline input provenance, and 721 ZIPs with
  zero integrity errors or warnings. Final evidence remains open pending the
  Canvas grading-target repair discovered by the stricter parity comparator.

- Repair Canvas QTI 1.2 FIB response processing to set the declared `SCORE`
  variable, declare the required Decimal outcome, and retain the frozen MC default `continue`
  behavior in strict grading output.

- Retained the post-repair benchmark failure caused by selecting an obsolete
  RDKit shim separately from performance acceptance. Started the replacement
  run with the certified macOS arm64 shim recorded in the structural audit.

- Verified locked all-target workspace tests and strict workspace Clippy after
  the NUM source-identity repair and its seven focused reader regressions.

- Reverified the numeric-reader identity repair against the pinned Python
  oracle: 203 files, 1,113 items, and seven synthetic cases agree, covering all
  181 structural corpus inputs with exact source path and SHA-256 matching.
  Six focused reader tests and strict engine lint pass.

- Diagnosed the full corpus checker's exponential grading enumeration: bank
  1,202 contains 32 matching items with 33,554,432 assignments each. Retained
  the diagnostic run as incomplete and required an exact symbolic comparison
  before renewing the full parity gate.

- Reopened source-reader CRC acceptance after the structural audit identified
  numeric tolerance-note removal as the cause of 14 mismatches with the pinned
  Python reader. Recorded the defect without suppressing failed item pairing.

- Verified formatting, strict workspace lint, all-target tests, and optimized
  builds on native Linux arm64 from an isolated current-source snapshot. A
  separate lane cross-built both Linux x86-64 CLI binaries. This evidence does
  not substitute for committed-HEAD release or full corpus parity validation.

- Verify the Debian 13 RDKit SDK release-build prerequisite with g++ 14.2.0,
  Boost 1.83, Cairo 1.18.4, and RDKit 202503.1-4; record the required
  `CAIRO_PREFIX=/usr` include path for the native shim recipe. The Linux x86_64
  CLI cross-build and runtime conversion are retained as target-ABI evidence.

- Verified the current locked optimized workspace build on macOS. Recorded the
  renewed structural audit's failing DOM/path checks without treating the earlier
  weaker receipt as acceptance.

- Verified locked tests and strict Clippy for every workspace target after the
  MATCH/LOM repairs and parity tooling split; corpus acceptance remains separate.

- Verified strict xtask lint for all targets after the writer-outcome module split
  and temporary structural-checker corrections.

- Verified the complete repository base test lane: 1,350 tests pass after safe
  build-cache cleanup and the parity writer-outcome module split.

- Updated the progress ledger after independent reproduction and acceptance of the
  M19 parser differential, including its actual corpus provenance and failure guard.

- Recorded the audit traversal correction: pinned Python production converts MC
  answer text as well as choices, matching Rust; the temporary named-field oracle
  omitted those leaves. Retained the emulated Linux Rust toolchain SIGSEGV as a
  failed release-validation lane.

- Recorded the fresh 59-bank parity fixture pass after MATCH mapping and Blackboard
  LOM sidecar repairs, including media outcomes and all four native reader roundtrips.

- Clarified that M19 still requires the real-and-hostile parser differential receipt
  alongside positional structural proof and visual signoff.

- Made local release validation run the full corpus parity gate as well as fixtures,
  and made its Cargo metadata check honor the committed lockfile. Verified all seven
  workspace packages still share release metadata version 26.9.0.

- Updated the progress ledger with the verified benchmark and the independent review
  requirements for positional structural certification.
- Verified the fresh Python/native benchmark manifest and all 181 input hashes; recorded plain-run comparisons of 5.50x for Blackboard and 15.96x for three-format output, with zero failures in both lanes.
