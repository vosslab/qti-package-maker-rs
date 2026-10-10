# Changelog

Earlier entries: [CHANGELOG-2026-10a.md](CHANGELOG-2026-10a.md) and
[CHANGELOG-2026-09a.md](CHANGELOG-2026-09a.md).

## 2026-10-10

### Behavior or Interface Changes

- At the user's request, use Roosevelt green `#008852` with white text for primary self-test
  buttons, reduce action buttons to a 28px minimum height, and tighten MC/MA answer rows.
  Keep a small gap between controls and let rich answer content determine row height.
  The shared native/Wasm stylesheet owns these deliberate differences from the frozen Python
  reference. Grading, question order, and MATCH/ORDER response shuffling are unchanged.

## 2026-10-09

### Additions and New Features

- Added portable `qti-render` planning and finalization shared by native and Wasm conversion,
  including stateless `planRenderJobs` and `finishConvert` host APIs, nested canvas dependencies,
  source-owned RDKit drawing/query metadata, and logical CSS dimensions. The contract is in
  [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) and
  [packages/qti-wasm/docs/rendering.md](../packages/qti-wasm/docs/rendering.md).
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

- Recreated `docs/active_plans/reports/SELFTEST_VISUAL_PARITY.md` with verified source ownership,
  frozen-reference links, historical receipts,
  and the final ten-case local parity/composition evidence. Post-correction actual-MkDocs
  Playwright passes 94 cases; source-owned contrast/NUM limits and earlier server failures remain
  explicit. All final reviews pass and the manager accepts independently verified local integration.
  Temporary comparisons are removed; all 20 frozen hashes still match.
  Correction on 2026-10-10: the report had been intentionally deleted by the user; restoring it
  to satisfy documentation links was a mistake. Removed it again, the earlier MATCH visual
  assessment that accepted incorrect document/background differences, and their incoming links.
  Clarify [SELFTEST_COMPATIBILITY.md](SELFTEST_COMPATIBILITY.md): QPM supplies CRC question identity;
  BPW owns filename-based v2 achievements. CRC-based test-host completion remains a separate
  host-policy exercise, with no QPM persisted-progress API change.
- Add the ten-capture Python reference gallery index and README while preserving capture hashes.
  Correct a stale reader-test expectation to the fixture's authored MA
  `min_answers_required` of -2; omitted MA metadata still defaults to 1. NUM tolerance
  remains 0.01 in the metadata-present assertion. All 15 focused reader tests pass.
- Clarified the selftest responsibility boundary and traced emitted answer definitions to their
  Python equivalents. Browser-local checking and partial feedback are generated output;
  submission handling and student-grade management are outside QPM.
- Matched Python's MULTIFIB JSON answer attributes and character-reference handling. An authored
  `&beta;-sheet` answer now accepts the displayed Greek letter as Python does. FIB keeps its
  existing literal-answer behavior; question validation and input requirements are unchanged.
- Restored Python's embeddable selftest fragment contract: no document wrapper or metadata,
  transparent background fallback, inherited host theme variables, and one shared stylesheet.
  The host retains document mode, viewport, and light/dark appearance. Verified direct opening
  and themed embedding; native and Wasm use the same writer.
- Source comparison also restored ORDER action elements, initial move-button attributes, and
  announcement placement. HTML-to-text conversion now decodes named/numeric entities through
  the existing parser, preserving chemical-symbol text in matching choices' accessible names.
- Replaced the alternative Rust selftest presentation with current Python's CSS,
  per-type control placement, paragraph folding, and
  feedback appearance across all seven types. Removed the extra NUM tolerance paragraph and
  MULTIFIB styling; aligned numeric Infinity guidance and reset/clear functions. The earlier
  matching-only receipt did not establish full visual parity.
- Restored current Python MATCH presentation in the shared native/Wasm writer: compact answer
  cells, feedback spans, prompt sizing, mobile row placement, inline choice letters, palette order,
  and actions directly below the table. Slots now reference the instructions; accessible choice
  names preserve escaped text. CRCs, shuffle semantics, grading hooks, and Reset are unchanged.
- Removed the one-time M19 parser-oracle and output-certification tools and their milestone
  reports. Removed a selftest assertion that prescribed a scrolling implementation; retained
  meaningful grading, identity, Reset, theme, and containment regression checks. Seed sweeps,
  screenshot measurements, and package hash comparisons remain one-time implementation proof.
- Fixed `devel/dist_clean.sh` to remove the WASM package's nested `dist/` and `generated/`
  outputs as well as root-level distributions.
- Audited the selftest parity repairs with six independent reviewers. Removed brittle CSS
  assertions, corrected the embedded-container overflow check, and cleaned up redundant CSS
  and misleading comments. No grading, identity, or interaction behavior changed. The rebuilt
  WASM package, TypeScript checks, and 21 focused browser checks passed; see
  [selftest_parity_audit_2026_10_09.md](active_plans/audits/selftest_parity_audit_2026_10_09.md).
- Corrected reviewer-material attribution: removed supplied reviewer prose from human guidance
  and recorded the settled BPW progress scope in design decisions and the selftest contract.
  Human guidance retains the directly stated feature-parity requirement.
- Restored selftest choice palettes and dark themes, compact short MC/MA choices, paragraph
  spacing, and button feedback. Fixed overlapping MATCH headers and scientific table/canvas
  overflow without applying control-table styles to nested authored tables. MATCH/ORDER now use
  the caller seed with the already-locked `fastrand` shuffle; CRC and grading code are unchanged.
  Native and Wasm share these fixes. Website CSS and lifecycle integration remain website-owned.
- Rotated complete October 7 and October 1 day blocks into
  [CHANGELOG-2026-10a.md](CHANGELOG-2026-10a.md), retaining October 9 and 8 here.
- Rendering preserves original item CRCs, ordering, and grading even when rewritten
  presentations coincide. PNG completions undergo bounded row decoding, checksum verification,
  and end-chunk validation. Blackboard Original shares media with identical bytes, MIME type,
  and extension while retaining the earliest item owner and all references. Native Chromium,
  Rayon, caching, and RDKit runtime services remain in the native adapter.
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

- After restoring the missing parity report, run
  `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q`:
  all 85 checks pass in 0.09s. Subsequent full suites pass: 1922 Python checks, 335 Rust
  tests with 4 ignored, formatting, check, and strict Clippy. Initial package Node/browser/current-
  Python checks remain a separate lane. Actual BPW serial Playwright passes 94 cases after
  the narrow-table correction; targeted mobile checks and independent reviews pass.
- Verified the Rust-only MATCH repair with current-Python comparisons of three existing banks:
  54 initial screenshots and 18 successful desktop/touch interaction journeys with correct-answer
  captures. All 331 Rust tests, 12 Node checks, 36 browser checks, three Python comparison checks,
  and 1,825 hygiene checks pass; four optional Rust tests remain ignored. Native and Wasm builds,
  formatting, strict Clippy, and TypeScript checks pass. The packed artifact matches the reviewed
  build.
- Completed the final selftest repair review and verified the packed WASM runtime against the
  tested build, including an extracted-package smoke check for all seven question kinds. The
  artifact, validation results, and BPW integration boundary are recorded in the
  [selftest handoff](active_plans/reports/qpm_selftest_handoff_2026_10_09.md).
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
