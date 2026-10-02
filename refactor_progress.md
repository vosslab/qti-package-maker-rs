# Rust port progress

## Implementation complete: 2026-10-01

The complete amended plan is archived in [docs/archive/rust_port_plan.md](docs/archive/rust_port_plan.md).

- M1-M12: core, integrity, ten writers, four readers, CLI, media ownership, and static self-test
  behavior are implemented. The earlier reviewed contract receipts below remain applicable.
- M13: 1,709 plain inputs across ten formats have zero findings. The refreshed Chromium run
  has 721 packages with zero integrity errors/warnings and zero public-content/grading
  differences. Reference defects and harmless presentation differences are classified in
  [docs/PARITY.md](docs/PARITY.md).
- M14: macOS and Linux Rust tests, strict Clippy, formatting, and release builds pass. Both
  Linux Rust architectures build. All 88 source/manifest/font inputs in the Linux capture
  verification still match current files. Manual release preparation is documented.
- M15-M19: baseline, selection/cache conversion, static RDKit rendering, and Chromium table
  rendering are verified. All 290 gallery entries render; the two reported failures and
  oversized tables are fixed. User visual acceptance is recorded in the archived plan.
- Final corpus evidence: `output_tables/native_table_bench/run-23373/receipt.json`,
  `integrity_receipt.json`, `semantic_receipt.json`, and `comparison_receipt.json`.
- The first-screenshot delay is isolated and documented; temporary instrumentation is removed.
  The restored production CLI hash equals the verified corpus binary hash.
- Final closeout documentation/source checks pass (506 checks, seven existing source-size
  advisories), release metadata confirms all six packages use 26.10.0, and Git diff checks pass.
- Human commits/publication, real LMS imports, and native amd64 browser execution remain
  delivery or external validation steps. None is claimed by the local implementation evidence.

The dated sections below retain the implementation history; earlier pending states are superseded
by this final result.


## User visual review: 2026-10-01

The user rejects maintaining a custom HTML renderer. This supersedes the native-raster
direction below; earlier automated and reviewer passes do not establish visual acceptance.
M17 is rejected and M19 rendering acceptance is reopened. The user selected Chromium, now implemented through chromiumoxide. The custom rasterizer
source and its implementation-specific tests are removed; bundled fonts remain. Earlier build/package evidence remains valid for that source revision.

Real failures in gallery `run-1790798094`:

- `08b73c60946140c7370d9a3ab8d990f833052e384b09b68ae0867fe9715bd360`
  (user reports missing colors and circles).
- `3057867908e9cff30bd1536d9056db77e89c3ae85309540d928eaaab8e516c56`
  (user reports rejection of CSS `box-shadow` on a gel band).

The reported gel error is `unsupported Property 'box-shadow'` near:

```html
<div style="height:7px; background-color:#99dbfb; border-radius:4px; box-shadow:0 0 2px #99dbfb;"></div>
```

These are the user's observed failures. The earlier agent description of sizing/alignment
came from saved gallery PNGs and did not describe the reported failing render.

User-excluded malformed source HTML:

- `02c975d7591dc5c002674c8ba3897ccb735a5136ea9e0b806d34e4e29f2353da`
- `1b149ba0b156c800e409c327f74ab18bb3ed35e65f5289a0487e9cf03b0db517`
- `242efdc8c99781b30d1c99f186a619bf7247399693b5132ac81cd2bdc315471e`
- `2d14d062599326632953d8ef2ce5e9d209ef4fc099811d87e6c941fdedefd505`

User-excluded HTML sugars; use the sugar library's PNG/SVG export:

- `29df173763b6ebdc1465a0b799cd721084a20ccf0d1f1210759027d090dabaeb`
- `3785b5c2a6fda118b08b1c82b33d226bf8f62713528f5415a8f38e72af64deac`

## Chromium replacement verification

- Production tables use one lazy, serialized Chromium session. Canvas RDKit rendering,
  conversion caching, media ownership, and package fan-out remain in Rust.
- The two reported cases export successfully to all three ZIP formats; all six packages
  pass integrity. Extracted PNGs show colored pathway circles and gel shadows.
  Evidence: `tests/_temp/chromium_acceptance/receipt.json` and the adjacent PNGs.
- Gallery `output_tables/gallery/run-1790859291/index.html` renders all 290 entries through
  Chromium. The six user exclusions above remain exclusions; Python appearance is not a gate.
- Workspace tests pass (248 tests; five optional runtime tests ignored), strict all-target
  Clippy passes, and the optimized workspace builds. Current logs are
  `tests/_temp/chromium_workspace_tests.log`, `chromium_clippy.log`, and
  `chromium_release_build.log`.
- One-time checks confirm authored image event handlers cannot change the screenshot,
  missing-browser errors identify `QTI_CHROMIUM`, and plain exports need no browser.
- Final release replay also blocks authored meta-refresh redirects and passes integrity for
  all 12 packages (two rendering failures plus two boundary fixtures, three formats each).
  `tests/_temp/chromium_acceptance/boundary_receipt.json` binds the release binary.
  Final release-mode strict Clippy passes (`tests/_temp/chromium_final_clippy.log`).
  Python checks passed 1,497 cases; the two disk-budget failures passed on focused rerun
  after `cargo clean --profile dev`. Release binaries and review artifacts are retained.
- Earlier platform and corpus receipts below predate this replacement and are historical.
- Fresh Linux snapshot `output_tables/chromium_linux/source` passes all 245 current workspace
  tests (five optional runtime tests ignored), release-mode strict Clippy, formatting, and the
  optimized workspace build. Both x86_64 CLI binaries cross-compile. `build_receipt.json` records
  zero changed snapshot files at validation and the successful container exit.
- Linux arm64 Chromium runs as a non-root user with its sandbox enabled. Both reported failures
  and both boundary fixtures produce 12 packages with clean integrity checks; script/redirect
  fixtures cannot change their table images. The colored circles and gel shadows were visually
  inspected. Evidence: `output_tables/chromium_linux/runtime_receipt.json` and adjacent PNGs.
- Native Linux amd64 Chromium remains unverified: the Mac emulator fails before browser startup.
  The local release helper now selects its native architecture and installs Chromium; it runs
  validation as a non-root user and selects the matching Debian RDKit library directory.
- The updated helper's prerequisite-only run passes at
  `output_tables/linux_release_check/run_10365`: Rust 1.98.1, Python 3.13.5, Chromium 154,
  the Debian arm64 RDKit shim, and a non-root sandboxed browser launch. This is prerequisite
  validation, not a full committed-source release pass. Markdown/ASCII/shell checks pass (299).
- Final expanded prerequisite run `output_tables/linux_release_check/run_18179` also passes
  imports of the pinned Python package interface, table/canvas renderers, and RDKit as the
  non-root validation user. Missing comparison dependencies and Debian lxml 5.3's annotation
  incompatibility are corrected in the disposable container. The current `devel/make_release.py`
  dry-run passes for 26.10 (`tests/_temp/chromium_release_dry_run.log`); no release is published.
- The user's positive Chromium review is recorded in the active plan checklist.
- A one-time oversized-table case exposed off-viewport content becoming white in screenshots.
  The chromiumoxide element helper scrolls and double-counts offsets, and does not request
  capture beyond the viewport. The adapter now captures explicit document bounds without
  scrolling and enables beyond-viewport capture. All four corner colors pass in the corrected
  3600-by-2000 PNG (`tests/_temp/chromium_oversized/receipt.json`). No permanent browser test
  was added. Benchmark `output_tables/native_table_bench/run-10923` was deliberately stopped;
  its `STOPPED.json` marks the incomplete evidence. Gallery and corpus proof need refreshing.
- Corrected-capture verification passes: 248 macOS workspace tests, strict release Clippy,
  optimized builds, the two reported cases and script/redirect fixtures in all three formats,
  and the oversized-table check. The refreshed gallery is
  `output_tables/gallery/run-1790861624/index.html` (290 of 290 rendered).
- Linux correction proof is at `output_tables/chromium_capture_linux/receipt.json`: 245 tests,
  strict Clippy, formatting, builds for arm64 and x86_64, unchanged build-input hashes, and six
  clean oversized-table packages. Both Rust architectures retain all four corners. The x86_64
  CLI runs under emulation against native arm64 Chromium; this does not claim a native amd64
  browser run. Documentation and source checks pass (513; seven existing size warnings).
- Corrected full-corpus benchmark `output_tables/native_table_bench/run-23373` completed with
  exit zero and no CLI or library conversion failures. All 721 ZIPs pass integrity with zero
  errors/warnings; all public-content/grading projections agree with run 80228. Input provenance
  matches all 181 Python baseline inputs. The final integrity, semantic, and comparison receipts
  are retained beside the run receipt. This supersedes the interim checks below.
- CLI batches take 915.990/909.205 seconds versus Python 197.234/445.820. Separate library
  conversion passes take 26.771/27.148 seconds. A focused sampled corpus case reproduces a
  10.5-second wait inside table rendering. Temporary instrumentation isolates it to screenshot
  capture (DOM/fonts/geometry about 22 ms; activation about 1 ms). Chrome for Testing
  153.0.8010.12 shows behavior consistent with upstream first-capture stall issue 1859. This
  is a documented performance limitation, with no speculative workaround or speed gate.
  Instrumentation was removed and the production source restored.
- The completed Blackboard phase contains 181 packages, all with zero integrity errors or
  warnings (`run-23373/blackboard_integrity_receipt.json`). The benchmark remains live;
  the final full-run receipts above supersede this intermediate result.
- Interim public-content/grading comparison covers 87 completed packages with zero differences
  from verified run 80228 (`run-23373/semantic_progress.json`). This is partial evidence only.
  The Python baseline report now matches the retained baseline receipt: 197.234 seconds for
  Blackboard and 445.820 seconds for three formats; no new Python benchmark was run.

## Historical snapshot before Chromium acceptance

The following table and open-work list preserve the earlier custom-renderer review state.
Their pending labels are historical, superseded by the implementation-complete section above.

Authority: [rust_port_plan.md](docs/archive/rust_port_plan.md), with carried-forward
requirements in [rust_port_plan_v1.md](docs/archive/rust_port_plan_v1.md).
Python reference: `55e5f368777f7809fe2e91b5d070caf6df0cb581` (2026-09-30 inspection).
Native development target: `aarch64-apple-darwin`, Rust 1.98.1.

| Milestone | State | Evidence or remaining gate |
| --- | --- | --- |
| M1 workspace and dependencies | Accepted | Seven-crate workspace, recorded dependency decisions, canonical lockfile pinning, locked workspace check/build verified |
| M2 item model and CRC | Repaired and reverified | NUM reader now preserves the pinned Python tolerance note. Fresh sweep: 203 files, 1,113 items, seven synthetic cases, all identities agree; all 181 structural inputs are covered with exact path/SHA agreement. Six BBQ tests and strict engine lint pass; the later M19 structural v7 receipt covers the corrected traversal |
| M3 validation and bank | Implemented and reviewed | Validated constructors and RAII bank; boolean numeric inputs rejected by typed Serde; all validation failures tested |
| M4 media assets | Implemented and reviewed | Path boundaries, naming, provenance and retained ownership verified in core suite |
| M5 ZIP and manifest | Accepted | Manifest paths validated; atomic ZIP replacement; independent review and 60 core tests pass |
| M6 integrity oracle | Accepted | Independent review, 26 tests, bounded ZIP/tree inspection and 23-package pinned agreement |
| M7 engine traits | Accepted | Independent architecture review; three probe engines and hook control-flow evidence |
| M8 text engines | Accepted | Text writer/readback contracts pass the final workspace tests and full 1,709-input plain corpus comparison |
| M9 QTI writers | Accepted | All seven QTI 2.1 kinds agree under semantic projection and native package integrity; independently accepted bounded frozen MULTI_FIB repair preserves authored blanks and additive grading, with native scores 0/50/50/100. Final plain and converted corpus integration are reconciled; all current packages pass integrity |
| M10 Blackboard pool | Accepted | All six real package pairs agree under explicit frozen-only feedback repair; 19 focused tests pass and native feedback references resolve. Independent final writer/reader review passes; temporary carrier edits preserve public score/content/media, and score mutations remain detected. Full corpus integration is reconciled, including all 178 converted Blackboard grading pairs |
| M11 HTML self-test | Accepted | Certified frozen assets and fresh seven-kind grading/interaction browser receipt accepted independently |
| M12 registry and CLI | Accepted | Four readers/ten writers; CLI process contracts and full plain corpus integration pass |
| M13 parity integration | Verified with documented reference repairs | Full plain run `parity_6191` passes 1,709 inputs and ten formats with zero findings. Current release HTML replay exports 537 clean packages; grading agrees for all 178 Blackboard, 178 Canvas, and 181 QTI 2.1 pairs after five source-bound reference repairs. Presentation findings are reconciled in `docs/PARITY.md`; raw serialization differences remain visible in the diagnostic harness. Final workspace checks pass |
| M14 release readiness | In progress | Manual `devel/make_release.py` path selected; no GitHub release workflow. Current macOS workspace tests, strict all-target Clippy, formatting, and optimized workspace build pass; release metadata passes. Local release validation now locks all xtask dependency resolution. All 1,556 Python checks pass including the latest comparison changes. CLI checks pass after removing the equivalent-version startup blocker; raster checks and strict lint pass with the current cssparser API. Final Linux arm64 snapshot `output_tables/linux_current/run_1790825204` passes formatting, strict Clippy, 320 tests, and optimized workspace build with matching current build-input hashes. Committed-HEAD release validation remains |
| M15 corpus and Python baseline | Implemented | 181 BBQ, 290 tables, 58 canvases; pinned repaired Python baseline has zero failures in both output modes |
| M16 raster stack spike | Accepted | Architect reviewed three actual hard fixtures and native font/raster stack |
| M17 native table rasterizer | Awaiting user visual signoff | Gallery run-1790798094 renders all 290 native/reference pairs; independent full screening and direct repaired-scene review pass |
| M18 RDKit canvases | Accepted | Independent review of persistent macOS and emulated Linux runtime receipts, all native error classes and 58 canvases |
| M19 conversion and benchmark | Awaiting user visual signoff | Structural v7 and parser differential independently accepted. Current run-80228 binds exactly to all 181 Python-baseline inputs: native CLI walls 18.124/19.202 seconds versus Python 197.234/445.820 seconds. All 721 ZIPs pass integrity with zero errors/warnings. All 537 converted packages are reconciled as recorded under M13; user visual signoff remains |

Current verification

- macOS: locked workspace tests, strict all-target Clippy, formatting, and optimized workspace
  build pass. All 1,556 Python checks pass (`tests/_temp/root_final_python_verification.log`).
- Linux arm64: the current-source snapshot at
  `output_tables/linux_current/run_1790825204` passes the same Rust checks, with 320 tests
  passing and five explicitly ignored native-runtime tests. Its `verification_receipt.json`
  records the pinned image, source manifest hash, and commands. Build-input hashes match the final source, including the alt-text and report changes. This is source-snapshot evidence, not a committed-HEAD release.
- Plain corpus comparison `tests/_temp/parity_6191` completed successfully: 1,709 input
  banks agree across seven frozen-CLI and three registered-adapter formats, with zero findings.
  This run precedes the image-only alt-text spacing fix. HTML run
  `tests/_temp/parity_7405` completed with 263 findings across 537 inputs. All writer/integrity
  failures are frozen-Python failures. The current 537-package release replay reconciles every
  finding through source-bound repairs and presentation classification; all packages pass integrity.
- The current three-format HTML-to-image preflight is `tests/_temp/parity_5989`.
  It passes with zero findings. Earlier failed or interrupted runs remain historical evidence.
- Independent final Blackboard writer/reader review passes (`tests/_temp/blackboard_final_review.md`).
  The temporary carrier-separation proof confirms that private authoring state cannot hide a
  changed public score program or media payload.

Structural and performance evidence

The independently accepted structural v7 receipt covers all 181 inputs, 515 items, and 3,426
fields with zero failures. It replaces the rejected v2 receipt. The independent static-parser
review accepted 141 real contexts and 23 hostile cases with zero disagreements, bound to the
frozen selector module and corpus hashes. These establish structural and parser behavior;
they do not replace the full format comparisons.

The current native benchmark `run-80228` uses the exact 181 Python-baseline inputs and
measures 18.124 seconds for Blackboard and 19.202 seconds for three formats, compared with
197.234 and 445.820 seconds in Python. All 721 ZIPs passed integrity with zero errors or
warnings. The run directory holds the captured release binary, receipt, input-bound comparison,
and per-package integrity receipt. Both CLI and separately measured library conversions have
zero failures. The earlier missing-shim run `run-70816` failed and is not acceptance evidence.

Open completion work

- Documentation audit: all WP-G1 named documents exist; engine authoring includes the Rust
  `template_class` translation, and the test guide separates permanent contracts, tooling
  evidence, and one-time proof. Both repositories now cross-link the Python and Rust projects.
  Markdown/ASCII checks pass (311 checks); both repositories pass `git diff --check`.
- Complete the final workspace/release audit. The current 537-package HTML replay is
  reconciled: all exports and integrity checks pass, all public grading comparisons agree,
  and presentation differences are source-preserving or incidental. Five Python failures
  are resolved by source-bound reference repairs. Full classification is recorded in
  `tests/_temp/native_html_replay_current/presentation_review.md` and `docs/PARITY.md`.
  The raw harness still reports serialization/reference differences; it has not been
  changed to hide them. Final macOS and Linux arm64 checks pass. Linux x86_64 cross-compilation passes
  from the same verified source snapshot. Both CLIs run under amd64 emulation and produce
  three table-containing packages that pass integrity.
- Record the user's visual decision on `output_tables/gallery/run-1790798094/index.html`.
- Complete the documented manual release validation at the user's committed HEAD and close
  the plan's remaining documentation/verification checklist. Git commits and publication
  remain under the user's control.
- Temporary-check cleanup is complete: 73 untracked proof/captured-source files were
  classified, archived with per-file SHA-256 verification, and removed from `tests/_temp/`.
  Results remain at their recorded paths. Recoverable source and its manifest are in
  `output_tables/implementation_proof_archive/retired_checks.tar.gz` and `receipt.json`.
  No checks were promoted to permanent tests. After acceptance, archive the plan using the
  repository's prescribed Git move procedure.

Real LMS imports are external release validation, as specified by the plan. Local XML and
package checks do not claim a real LMS import. The failed emulated amd64 compiler run is
historical environment evidence. Linux x86_64 binaries are cross-built on arm64 and runtime
checked under amd64 emulation; no native x86 hardware run is claimed.
