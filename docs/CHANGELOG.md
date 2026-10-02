# Changelog

## 2026-10-01

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

## 2026-09-30

### Additions and New Features

- Add an ignored M19 static-canvas parser differential receipt. The pinned Python snapshot and
  Rust extractor agree on all 141 production-transform field occurrences and 23 hostile grammar
  cases, including the approved receiver-agnostic spelling; the receipt retains input pin and
  every disagreement rather than normalizing either result. The three repeated source strings are
  intentionally separate MC answer-field contexts; the corpus has 138 unique canvas records.
- Accept the M19 parser differential after independently reproducing its frozen fixture digest,
  all 181 input-hash checks, and zero disagreement result; retain corrected structural proof and
  user visual sign-off as separate exits.
- Preserve fingerprint-relevant MA and NUM source fields through Blackboard
  pool writer-reader round trips with explicitly application-private metadata;
  Blackboard scoring remains the ordinary response/resprocessing payload.
- Add the validated, identity-preserving Item presentation derivation used by HTML-to-image;
  preserve source CRC serialization while validating every rewritten display field.
- Add M19 conversion metrics with explicit cache outcomes, native renderer stage spans, and
  metrics-bearing conversion failures that preserve the original typed error.
- Audit M19 conversion and native benchmark evidence: accept the immutable speed receipt and
  one-pass fan-out design, while requiring non-overlapping stage accounting and the remaining
  structural, integrity, one-pass, and user-visual exit receipts.
- Make each successful native HTML-to-image pass observable in the CLI conversion report and
  verbose receipt. The three-format packaging path records exactly one shared pass before writer
  fan-out; quiet receipts remain unchanged.
- Compile the complete engine-authoring worked example against current Rust crate artifacts;
  verify all 1,125 repository checks pass after the documentation additions.
- Add durable Rust-format documentation: engine inventory and authoring contract, including a
  complete pedagogical trait-object writer, format and question-type coverage, media behavior,
  roadmap, and Rust-specific three-class test guidance. Keep the active plan and progress ledger
  as the authority for unaccepted parity and release work.
- Persist PNG-member and media-bearing-package counts in benchmark receipts, inspecting ZIPs
  after stopping the CLI wall timer so evidence collection does not alter measured conversion time.
- WP-G1: Add evidence-based [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and
  [FILE_STRUCTURE.md](FILE_STRUCTURE.md) for the seven-crate Rust workspace, public facades,
  renderer and media boundaries, development tooling, generated evidence, and remaining gates.
- Verify current self-built CLI parity across 56 question-kind projections, including Canvas
  grading behavior; correct the documented frozen defect attribution to MULTI_FIB.
- Record accepted workspace/dependency evidence and completed CLI process contracts, including
  ten real all-format artifacts, ambiguous prefix diagnostics, and actionable writer failure.
- Add a related-projects guide that identifies the Python converter as this Rust port's upstream
  and links visitors to the supported text2qti, QTI, and Moodle Aiken interoperability resources.
- Capture a private immutable executable and fresh output directory for native benchmarks,
  verify its hash after all runs, and prevent concurrent Cargo rebuilds or stale artifacts from
  contaminating timing receipts.
- Clarify the carried M1 dependency criterion: the manager-selected Cargo requirements remain
  ranges and the tracked lockfile is the exact tested pin; verify locked workspace check and build.
- Audit native benchmark fidelity: mark the earlier single-format timing as packaging-only
  because its CLI invocation omitted `--html-to-image`; require corrected current-binary reruns.
- Verify the repaired renderer with 74 raster tests and a fresh 290-table gallery; independent
  visual screening confirms gel bands, titration labels, and pedigree clipping are repaired.
  Retain the plan's user visual-signoff gate as pending.
- Correct the native benchmark's ORDER capability explanation: Canvas omits the unsupported
  output; Blackboard export emits its empty valid package and diagnostic.
- Remove stale incremental compiler caches under the Cargo build lock to restore the target
  disk budget; verify all 1,115 repository checks pass after cleanup.
- Accept independently reviewed seven-kind HTML self-test browser proof and certified frozen
  assets; verify the full Rust workspace and strict Clippy after decorated-box integration.
  Generate a fresh complete 290-table gallery for visual review.
- Repair Canvas MULTI_FIB as sorted choice-based `response_lid` interactions and score their
  declared response-label identifiers, preserving source answer order for each blank.
- Repair Canvas MATCH partial-credit processing so each authored prompt/choice position scores
  its declared response and choice identifiers.
- Repair Canvas MA scoring to require every selected answer and reject every unselected choice
  before awarding the all-or-nothing score.
- Restore Canvas NUM and FIB response controls and grading predicates, including decimal bounds
  and multiple acceptable string answers.
- Sanitize invalid XML control characters from Canvas QTI serialized text while retaining legal
  whitespace, then verify the serialization with a safe XML parser.
- Match the pinned human-readable image description order: placeholder, authored alt text, then
  source path.
- Match frozen human-readable FIB output by enclosing authored underscore blanks in brackets;
  extend the existing question-kind regression check with the actual prompt rendering.
- Refresh README, installation, usage and Rust guidance for the native binaries, all ten writers,
  four readers, optional RDKit shim, and explicit remaining certification gates. Verify actual
  CLI help and inspection commands plus focused documentation checks.
- Record the expanded parity harness and frozen Canvas MULTI_FIB response-reference defect without
  weakening package integrity or omitting the question kind from coverage.
- Reverify all 23 pinned integrity-oracle cases after integration. Resolve self-test asset
  provenance against the frozen Python import path, accept the four exact payloads, and begin
  existing-subset decorated-box layout/painting without extending CSS grammar.
- Verify the integrated workspace test suite after writer and metric migrations; the first
  ten-format MC differential fixture passes. Keep expanded question-kind/media parity and visual
  acceptance open, and retract map-scale findings contradicted by exact-file pixel measurements.
- Complete a 290-table native/reference gallery and diagnose remaining visual issues; correct
  double-scaled relative line-height, retain decorated-box painting as open implementation work,
  and recheck disputed map geometry against original-size images.
- Land the differential parity harness with explicit seven-format CLI and three-format registered
  oracle lanes, supported-kind fixtures, ORDER-only outcome checks, and semantic comparisons.
  Declare the pinned Python oracle as a development dependency; native production remains Rust.
- Expose differential parity in xtask help and update the progress ledger for implemented CLI
  binaries and prepared release gates; retain parity, visual and speed acceptance as open work.
- Add Linux/macOS release workflow and local release-check scripts with required pinned-oracle,
  differential parity, native RDKit, metadata, formatting, lint, test and binary-build gates.
  Clear the Blackboard optional-output constructor mismatch during writer-outcome integration.
- Prepare release metadata with CalVer `26.9.0`, synchronized `VERSION`, and inherited GitHub
  repository metadata; wire the differential parity xtask entry and correct native shim script
  execution permissions. Keep actual CI and parity results as remaining gates.
- Wire owned document metadata through the ten-writer/four-reader registry and exam YAML factory;
  begin per-format differential harness implementation and independent text-engine review.
- Start the native CLI delivery with typed argument parsing and local document-date metadata;
  add Cargo-managed Clap and time dependencies. Accept the persisted RDKit cross-build/runtime
  evidence and keep specialized renderer geometry defects open after independent visual review.
- Verify the integrated workspace test suite, advance the progress ledger to accepted M7,
  and start independent engine parity and rendered-gallery reviews. Restore the Human writer
  registry's Python `ReferenceWarn` declaration while retaining readable image descriptions.
- Return structured writer outcomes with provenance-complete media warnings instead of printing
  diagnostics from library code. Preserve each writer's pinned no-output behavior, including
  Canvas ORDER-only completion without a ZIP artifact.
- WP-E1: Add the standalone `html_selftest` writer for all seven item kinds. It inlines local
  media, keeps existing data URIs local, replaces remote image URLs without fetching them, and
  provides accessible click, drag, keyboard, reveal, grading, and reset controls. Focused Rust,
  JavaScript syntax, and a seven-item Playwright proof pass.
- WP-G0: Add the object-safe engine registry, generic rendering hooks, typed reader outcomes,
  and the complete human-readable, Canvas QTI 1.2, and Blackboard BBQ probe engines. The Canvas
  probe emits a manifest-backed ZIP that the independent integrity checker accepts; BBQ records
  line-located recovery warnings and round-trips an item fingerprint.
- Implement reviewed media handling, deterministic ZIP assembly and QTI manifests. The core
  suite passes 59 tests; manifest path validation and atomic ZIP replacement are follow-up
  review corrections before the engine gate.
- Complete the corrected real-generator sweep: 181 BBQ files, 290 unique tables and 58
  canvases, with source revision and per-generator hashes. Keep failed and no-output cases
  explicit; rerun baseline timings against this final corpus.
- Accept the native raster dependency spike using three real hard fixtures and start the
  production parser/style crate. Start the optional native RDKit production adapter; Linux
  runtime proof remains a release gate.
- Add native table-grid, inline/font shaping and paint/encode modules as coordinated work
  packages. Their focused integration and full-corpus gallery checks remain in progress.
- Wire the native PNG renderer through parsed table, inline, image, nested-table, and
  collapsed-border-segment display assembly. The gallery now retains timestamped review runs,
  includes bounded boxplot and restriction-digest scene routes, and records all 290 source
  hashes, routes, results, and compatibility no-ops.
- Implement outermost-table selection and static canvas extraction. A pinned 154-record
  differential proof agrees; the arbitrary draw-receiver grammar finding needs an explicit
  architecture ruling before final parser acceptance.
- WP-M1: Select a runtime-loaded native RDKit C ABI shim after a Rust 2024 spike rendered all
  58 harvested canvases and reproduced the CanvasSource validation errors. Record the optional
  install footprint and supported macOS arm64/Linux x86_64 package sources.
- WP-M2: Add `qti-molecule`, its safe runtime loader, ABI v1 native shim source, install contract,
  and optional native-shim integration tests. The production macOS arm64 shim renders the corpus;
  a cross-built Debian amd64 shim passes its two option/error tests and the 58-canvas public API
  corpus test under Podman x86 emulation. Public CanvasSource validation rejects non-finite or
  out-of-range RGB components before FFI.
- WP-A0: Add the Rust 2024 workspace, four component crates, Cargo `xtask` alias, active plan
  copies, and [refactor_progress.md](../refactor_progress.md) for M1-M19.
- Add a GitHub README through an independent readme-docs agent, clearly marking the converter
  as in progress. Its focused README and Markdown checks pass (36 tests).
- WP-A1/A3/A4: Implement seven item shapes, fallible validation, semantic fingerprints, ordered
  banks, and temporary-media ownership. Architecture review corrections are in progress.
- WP-A2: Implement CRC16-XMODEM and Python string helper translations; differential proof is
  in progress.
- WP-T1/T2: Add real-generator corpus and Python benchmark development tooling; corpus sweep
  and timing evidence are in progress.

### Decisions and Failures

- Accept M19's hash-matched native speed comparison against the fresh pinned-Python baseline;
  retain the frozen positional structural proof, parser differential receipt, and user visual
  sign-off as the remaining completion exits.
- Preserve the Python hierarchy through Rust crate/module boundaries; use exhaustive item
  enums and typed errors instead of runtime string dispatch.
- Keep the full parity plan as the objective. Workspace build success does not prove converter,
  package integrity, rendering, or performance parity.
- Choose wildcard direct dependency requirements; Cargo.lock records the exact tested graph.
- Pin development-oracle certification to `55e5f368777f7809fe2e91b5d070caf6df0cb581`
  using an isolated snapshot when the user's Python checkout advances. Preserve that checkout.
  Allow an explicit oracle-root override in the development Python bootstrap.
- Default Cargo registry writes hit the filesystem sandbox for newly downloaded dependencies.
  Use the writable `CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo` cache.
- Use a native arm64 Rust compiler plus Debian amd64 cross compiler/sysroot when Podman emulation
  cannot execute `rustc`; then run the resulting ELF x86-64 shim and corpus tests in the amd64
  runtime container. This preserves an actual target-ABI proof and records the emulated platform.
- Architect review found raw media filenames affected fingerprints, arbitrary serialized
  secondary strings could change CRC identity, and equal-path merges could lose temporary
  ownership. Correct these contracts before using the model in engine work.
- Architect rules that merge keeps Python's source item numbers; explicit renumbering remains
  a separate operation. Shared temporary ownership fixes Rust bank lifetime safety.
- Reproduce converted-bank media-base loss against the pinned Python snapshot. The accepted
  Rust conversion contract renders in memory first, then creates an owned media directory and
  copies existing images, preserving caller-owned input directories.

### Developer Tests and Notes

- Complete the pinned, repaired Python timing lane on all 181 inputs with zero failures:
  158.022 seconds for `-B`, 371.674 seconds for `-1 -2 -B`. Preserve the unpatched
  media-base failure separately; the repair is confined to development tooling.
- The SceneLeaf integration now compiles across native parsing, table layout, inline metrics
  and painting. The preserved 290-entry gallery currently has 255 native PNGs and 35 explicit
  typed failures; remaining support and visual review are required before M17 acceptance.
- Remove unreachable boolean numeric validation variants under the M3 architecture ruling;
  typed deserialization rejects them before domain validation. The core suite passes 62 tests.
- Final CRC differential sweep uses an isolated pinned Python snapshot: 191 files, 1,108
  items and seven synthetic shapes agree. Native HTML-to-text preserves semantics across all
  290 tables; differences are whitespace and numeric presentation.
- The integrity crosscheck agrees on 23 packages, including independently generated negatives,
  against the same pinned snapshot. Independent review still requires complete category
  regressions, namespace checks and documented inspection bounds before M6 acceptance.
- ZIP and manifest safety corrections pass independent review and the 60-test core suite.
- Initial workspace `cargo build` and `cargo xtask --help` pass. Final core and tooling checks
  remain in progress; no completed parity, native-raster, or speed gate is claimed.

- Correct the progress ledger to record the accepted immutable native speed evidence and current
  56-case fixture coverage while retaining the full-corpus, metric, and visual acceptance gates.

- Require the tracked dependency lockfile for release Clippy, tests, and builds, including the
  native RDKit lane, so certification cannot silently resolve a different dependency set.

- Inspect all 721 captured native conversion packages and retain eight orphaned Blackboard
  media-parent errors as an explicit failed certification receipt. Harden the certifier to fail
  on checker process failures and changing checker bytes, and hash each ZIP before checking it.

- Record every native benchmark input hash and the corpus manifest hash, rejecting mutation
  during measurement so a captured run can be tied to the exact source corpus.

- Remove the Rust GitHub Actions workflow at the user's request; retain local validation and
  use the existing manual `devel/make_release.py` release preparation path.

- Document the concrete local Rust validation and manual source-release commands, including
  notes-file and dry-run behavior of the existing release helper.

- Restore executable permissions for the local release validation and certification scripts.
  Remove stale incremental build caches under the Cargo lock to satisfy the target disk budget.

- Make benchmark commands fail after retaining their diagnostic receipts when any conversion
  fails; failed browser or renderer runs cannot be certified as performance baselines.

- Declare the parity oracle's hardened XML parser as a development dependency.

- Verify the updated tooling and release documentation with the complete repository suite:
  1,350 checks pass, including security, dependency declarations, executable permissions,
  source-length policy, and disk budget.

- Preserve each native benchmark's complete JSON receipt beside its private executable and
  outputs, so subsequent measurements cannot overwrite its input provenance or metrics.

- Remove the final obsolete GitHub workflow entry from the documented repository tree.

- Verify all eight CLI process contracts against the integrated identity fix, including the
  explicit one-conversion receipt and three package artifacts.

- Complete fresh native measurements against all 181 hash-bound inputs with zero failures:
  35.845 seconds for Blackboard and 27.940 seconds for three formats; preserve the private
  binary, source provenance, per-stage metrics, and 721 package outputs for certification.

- Certify all 721 fresh native conversion ZIPs with zero integrity errors or warnings and
  unchanged package/checker hashes, closing the package-integrity exit.
- Declare the development tool's default binary explicitly so `cargo xtask` continues working
  while ignored one-time audit binaries are present.

- Verify the integrated identity and reader work with `cargo test --workspace --all-targets
  --locked`; all enabled tests pass. Native RDKit environment tests remain separately certified.

- Mark the earlier M19 speed acceptance as historical: current release acceptance requires a
  fresh benchmark and package certification after the integrated reader and Canvas grading fixes.

- Record the complete 1,709-bank parity run as failed with 547 strict divergences; keep M13
  acceptance open while comparator parsing and writer-outcome differences are investigated.

- Tighten named Blackboard reader receipts to require exactly one of each private field,
  verify removal and malformed-value transitions in the emitted archives, and record the
  exact replacement count. Independent source review accepts this contract; fixture execution
  and the full M13 parity gate remain separate requirements.

- Match frozen Aiken production behavior by accepting raw RDKit canvas and table markup;
  its Python validator returns before the dormant rejection checks. A corpus-shaped regression
  and all nine focused Aiken tests pass, along with strict engine Clippy.

- Restore frozen constructor defaults for MA items read from text2qti: one required answer
  and all-correct selections allowed. Nine focused tests and strict engine Clippy pass.

- Verify the integrated engine changes with locked all-target tests: all 109 engine tests
  pass. The separate browser proof remains explicitly ignored outside its Playwright lane.

- Reject missing or conflicting writer outcomes in the parity harness, including identical
  invalid receipts on both sides. Add adversarial checks distinguishing successful no-output
  behavior from an emitted artifact. Both locked adversarial tests pass after the report-module
  build fix.

- Verify CLI integration after the text-writer repairs: eight unit tests and eight actual
  process contracts pass, including all ten outputs and one conversion before package fan-out.

- Declare the already locked `markup5ever` dependency directly for QTI named-entity decoding;
  corpus investigation found native `&alpha;` made an emitted item invalid XML. Offline engine
  dependency resolution passes; the writer correction and its regressions are in progress.

- Verify the revised parity tooling with locked all-target xtask tests: all nine tool tests
  and three structural scanner tests pass. Reader fixture execution remains a separate gate.

- Complete the revised 59-bank deterministic parity gate with zero divergences and unchanged
  CLI/oracle hashes. Retain typed Blackboard metadata and text2qti repair receipts for independent
  artifact review; full-corpus parity remains open.

- Correct QTI 2.1 HTML entity serialization using the full named-entity table and XML-safe
  escaping of decoded values. Six focused tests pass, including attribute quotes and numeric
  metacharacters; the retained alpha-entity corpus case now produces one item and passes integrity.
- Keep named reader-proof acceptance open: independent replay found a floating-point readback
  mismatch in the receipt-bound Blackboard fallback artifact despite the green fixture run.

- Enable `serde_json`'s exact float round-trip parser after proving its default parser changed
  the pinned Blackboard readback to an adjacent value. Require full native/pinned fallback
  projection equality; regenerate the receipt instead of rounding away the mismatch.

- Verify the integrated workspace after entity decoding and exact numeric parsing: locked
  all-target tests and strict workspace Clippy pass. Apply Rust 2024 formatting to the QTI
  writer; workspace formatting and diff checks pass.

- Independently replay and accept the regenerated named Blackboard/text2qti reader receipt:
  the exact stripped archive reproduces the full pinned numeric projection, nondefault MA
  preservation, and precise malformed-NUM recovery. M13 remains open for MATCH display-label
  comparison and a fresh full-corpus gate.

- Run the full Python repository checks after the dependency and receipt changes: 1,350 tests
  pass; seven warnings identify source files approaching the existing line limit.

- Compare self-test MATCH answers through their actual choice tokens, accounting for generated
  display letters while rejecting duplicate and unknown tokens. All ten retained self-test
  artifact pairs agree; adversarial wrong-pair, unknown-token, and changed-label checks pass.

- Identify incomplete QTI 2.1 grading coverage in the differential comparator. Require a
  separate vocabulary-specific projection of stems, interactions, declarations, correct
  responses, and scoring before corpus acceptance; equal reader failures alone cannot prove
  package parity. Record the independent strict review ruling without accepting a new exception.

- Refresh the progress ledger to distinguish the repaired MATCH fixture pass from the
  still-open full parity gate, and replace stale structural v2 status with the independently
  accepted v7 receipt. Review the new QTI 2.1 comparator for omitted grading metadata and
  fail-closed namespace handling before accepting its results.
