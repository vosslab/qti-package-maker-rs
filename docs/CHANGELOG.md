## 2026-09-19

### Additions and New Features

- Added `githubpages` as a repository type inheriting `typescript` and `website`.
- Added the reset interview follow-up that promotes a selected `typescript` repository to
  `githubpages` when it uses the template's Pages build, parallel to Python's PyPI follow-up.
- Added mandatory base-lane disk-budget checks for machine-wide Podman storage and Rust `target/`
  build artifacts. The Podman guard ships universally; the target guard ships through the Rust
  overlay.
- Added the consumer-owned TypeScript `.prettierignore.local` noexist seed for repository-specific
  exclusions. Propagation ships it once and never overwrites it.
- Added a non-blocking 900-999 line advisory band to the source-file gate. Advisories appear in
  pytest's warnings summary and in `report_source_file_line_limit_warnings.txt`.

### Behavior or Interface Changes

- Moved the Pages build, deployment seed, preview server, and build-aware Playwright runner from
  the generic TypeScript overlay to `templates/githubpages/`. Plain TypeScript repositories no
  longer receive those four build-specific files or package aliases that point to them. Existing
  Pages consumers change `REPO_TYPE` from `typescript` to `githubpages`; generic consumers can
  remove old copies explicitly because propagation does not guess ownership of existing files.
- Refactored the PyPI publishing tool into a thin coordinator plus focused project, release, and
  distribution modules. CLI modes and release order remain unchanged, while validated import names
  now travel as subprocess arguments instead of interpolated Python source.
- Reduced the propagated Human Guidance seed to its managed instructions and the Design Decisions
  seed to its managed instructions plus a blank entry template. Receiving repositories now start
  with fresh ledgers instead of inheriting starter-template history.
- Updated the vendored TypeScript aggregate check and the seeded `format:write` script to pass
  `.gitignore`, `.prettierignore`, and `.prettierignore.local` through repeated `--ignore-path`
  flags. Existing consumers receive the check update automatically but must update their owned
  `package.json` script manually.
- Removed the closing divider from the rendered `.gitignore` LOCAL banner so all following content
  is visibly repository-owned. Legacy four-line banners remain accepted and converge without
  losing their body.
- Replaced the bare 1000-line failure with guidance to split a file into cohesive modules by
  responsibility instead of trimming it to 999 lines.

### Fixes and Maintenance

- Split changelog parsing records and state machines into `devel/changelog_parse.py`, leaving
  `changelog_lib.py` as the stable public API for file mutation, Git, and console helpers.
- Kept PyPI tokens out of source, command arguments, and logs during the module split. Preserved
  list-based subprocess execution, ordered preflight gates, and explicit production confirmation.
- Applied the independent split audit's low-risk findings: removed an unused project-metadata
  field, corrected import ordering, and restored Google-style boundary docstrings.

### Decisions and Failures

- Made GitHub Pages a child capability instead of an assumption of every TypeScript repository.
  A Pages consumer writes `REPO_TYPE=githubpages`; inheritance supplies TypeScript automatically,
  so a redundant `githubpages,typescript` marker is unnecessary.
- Kept both disk guards in ordinary pytest rather than E2E. Podman storage is machine-wide, so its
  guard ships to every repository; `target/` is repository-local Rust output, so its guard ships
  only to Rust repositories. Budget failures direct developers to inspect and remove stale data.
- Chose a companion Prettier ignore file over a LOCAL section inside the overwrite-owned
  `.prettierignore`. This matches the `eslint.config.local.js` precedent without adding a new
  propagation bucket or merge format. Recorded this template-only decision and the related human
  guidance in the `meta/docs/` ledgers rather than the propagated consumer seeds.
- Rejected a generic vendor-directory exclusion because repository review boundaries vary, and
  rejected a hard-coded PLE path because product-specific paths do not belong in shared tooling.
  Removed the superseded root proposal after recording the settled decision.
- Exempted `docs/CHANGELOG.md` and `docs/CHANGELOG-*.md` from the advisory band because the
  changelog rotation policy already owns their intended size.
- The first focused gitignore run exposed a non-idempotent three-line banner parse. Moving opening
  divider recognition outside legacy-END handling made both old and new inputs converge.
- The completion audit found that `devel/markdown_section_sizes.py` lacked the whole-file
  propagation warning, preventing the plan's required full-suite verification. Added the exact
  vendored header after its required shebang.

### Developer Tests and Notes

- Temporary pure-behavior checks passed for metadata resolution, import-name validation, pip index
  parsing, and repository selection, then were removed. A plan assertion confirms every PyPI helper
  ships only to PyPI consumers. The focused suite passes 1,291 checks; the full suite passes 2,520
  tests without warnings after the changelog parser split.
- The focused gitignore, line-limit, and folder-convention suite passes all 233 tests and emits the
  expected two `UserWarning` advisories for `devel/changelog_lib.py` at 940 lines and
  `templates/pypi/devel/submit_to_pypi.py` at 990 lines. Changelog archives remain silent.
- The full meta suite passes all 581 tests. The full repository suite passes all 2,434 tests with
  the two expected non-blocking line-limit warnings.
- A dry-run propagation to the local Fold-Spacer TypeScript/Rust consumer reports the new
  `.prettierignore.local` as a noexist copy and `check_codebase.sh` as an overwrite update, with no
  errors. Shell syntax, documentation bucket isolation, meta-content isolation, and
  `git diff --check` pass.

## 2026-09-18

### Fixes and Maintenance

- Restored the canonical inline-first fixture policy in `docs/PYTEST_STYLE.md` after the testing
  guidance rewrite accidentally removed it. The policy covers custom `@pytest.fixture` functions
  and on-disk test data, keeps its closed three-case durable allowlist, and requires explicit human
  approval before adding a committed `tests/fixtures/` directory.
- Recorded the human's guidance that most fixtures are liabilities in `docs/HUMAN_GUIDANCE.md` and
  the canonical policy. Fixture use now requires an actual product dependency on shared setup,
  file shape, or loader behavior; test-author convenience is insufficient.
- Gave the canonical fixture rules a dedicated uppercase `FIXTURE POLICY` heading so the human's
  case-sensitive `grep 'FIXTURE' docs/PYTEST_STYLE.md` sharing workflow finds it directly.
- Audited the pre-split pytest guide against `docs/PYTEST_STYLE.md` and
  `docs/PYTEST_AUTHORING_GUIDE.md`, then restored substantive authoring rules lost during the split:
  brittle-test criteria, external-data rationale, test structure, command variants, hygiene
  discovery layers and helper contracts, report lifecycle, and scaffold guard ownership.

### Decisions and Failures

- Coding agents overproduce fixtures when guidance merely describes when fixtures can be useful.
  Restored the July 5 inline-first wording from `docs/CHANGELOG-2026-08a.md`: fixtures remain
  available for three named durable cases, rather than becoming a default abstraction.
- Recorded that splitting testing guidance changes document ownership, not policy; substantive
  requirements move to their owning document instead of being discarded during condensation.

### Developer Tests and Notes

- Verified Markdown whitespace, links, and repository diff hygiene after the documentation-only
  restoration.

## 2026-09-11

### Behavior or Interface Changes

- Required every canonical source copied through a whole-file overwrite route, across all file
  extensions, to carry the exact propagation warning in a native header comment. Required
  shebangs remain first; noexist and partial-ownership buckets remain outside this rule.
- Reframed the shipped testing guidance around fewer, stronger permanent tests. Permanent tests
  now earn their place by protecting intentionally stable behavior worth preserving without
  unnecessarily constraining future design.
- Established the ignored `tests/_temp/` subtree for temporary tests, reproductions, and one-time
  checks. Pytest-suitable `test_*.py` files participate in the normal `pytest tests/` run while
  heavier checks run explicitly; plan closeout promotes tests that deserve lasting protection and
  removes the rest.
- Added repo-wide principles to ground requirements and gates in actual needs and to give every new
  blocking CI, build, release, or repository-wide behavior gate an actionable failure plan.
- Kept `tests/_temp/` local to the template checkout during propagation. The universal tests walker
  now skips `_temp*` subtrees even though pytest continues to collect suitable local Python tests.
- Clarified that permanent tests follow native language and framework conventions. Rust keeps
  durable unit tests inline under `#[cfg(test)]`, durable integration tests in crate-level `tests/`,
  and temporary verification in the repository-root `tests/_temp/` workspace.

### Fixes and Maintenance

- Replaced the shipped pytest footer convention with the visible, exact propagation warning at the
  top of every overwrite-shipped test, and added the same header to other overwrite-shipped code,
  configuration, developer utilities, and test helpers.
- Reorganized testing documentation by decision point: `docs/PYTEST_STYLE.md` owns permanent-test
  policy, `docs/PYTEST_AUTHORING_GUIDE.md` owns pytest construction and hygiene mechanics,
  `docs/E2E_TESTS.md` owns permanent whole-system testing, and `tests/TESTS_README.md` remains the
  noexist test map and command reference.
- Preserved concise permanence, temporary-test, and removal guidance in the Python, Rust, Swift,
  TypeScript, Playwright, E2E, pytest-authoring, test-folder, and template-meta documents so each is
  independently useful when skimmed. Retained short checklists at the relevant decision points.
- Added the exact whole-file overwrite disclaimer to all eleven overwrite-shipped Markdown sources
  under the Rust, Swift, TypeScript, and website overlays. Noexist seeds, header-merge documents,
  and template-only files retain their distinct ownership signals.
- Added the universal `/tests/_temp/` ignore rule to `templates/gitignore.universal` and the
  template repository's rendered `.gitignore`, and documented its collection and propagation
  behavior in the meta gitignore guide.
- Removed the universal pytest authoring guide's link to the template-meta changelog, resolving the
  previously recorded link-bucket isolation failure while preserving the changelog instruction.
- Corrected both Python `assert` location summaries to include `tests/_temp/`, and refined the
  permanent-test and E2E checklist wording to emphasize stable behavior and temporary-first proof.
- Distinguished pytest-suitable temporary tests from heavier temporary checks. Only suitable
  `test_*.py` files join the pytest fast lane; service, network, and other whole-system checks run
  explicitly from `tests/_temp/`.

### Decisions and Failures

- Treated `overwrite_files`, `devel_files`, and `test_files` as one whole-file ownership class for
  warning purposes. Kept the format's native comment syntax instead of imposing one wrapper on
  Markdown, Python, shell, JavaScript, and configuration sources.
- Kept short, important policy repetition in satellite test guides because skim-reading agents
  benefit from seeing the decision rule where they act. Canonical documents remain linked for
  rationale and implementation details.
- Retained the overwrite-disclaimer sweep after an external review suggested pulling it back; the
  human explicitly included that ownership audit in this work.
- Extended the existing propagation scratch-path regression test rather than adding a policy-text
  inventory test. The stable behavior under protection is that ignored temporary tests never ship
  to consumer repositories.

### Developer Tests and Notes

- The plan-derived vendored-header gate resolves all 76 current canonical sources across
  `overwrite_files`, `devel_files`, and `test_files`; every source carries the exact warning within
  its first five lines. The initial sweep added headers to 54 previously unmarked sources.
- `templates/typescript/.prettierrc` parses as YAML to its unchanged option mapping after receiving
  its `#` header. `bash -n` passes for every changed shipped shell script.
- `source source_me.sh && python3 -m pytest tests/ -q` passes all 2,417 tests, and the focused
  vendored-header, pyflakes, whitespace, ASCII, shebang, source-size, and Bash-size suite passes all
  918 cases.
- A disposable `tests/_temp/test_testing_policy_cleanup.py` was ignored by Git and collected through
  the normal `pytest tests/` path. Its two checks passed: temporary pytest collection worked, every
  whole-file overwrite Markdown source carried the exact disclaimer, and every noexist Markdown
  seed omitted it. The disposable test and empty folder were removed before closeout.
- The focused documentation, hygiene, link-bucket, propagation-source, folder-routing, inheritance,
  and overlay suite passes all 836 cases.
- `source source_me.sh && python3 -m pytest tests/ -q` passes all 2,417 tests.
- `git diff --check` is clean.

## 2026-09-07

### Behavior or Interface Changes

- Made `docs/REPO_STYLE.md` the canonical four-way script-placement policy: optional standalone
  domain utilities use `tools/`, repository engineering uses `devel/`, primary workflows and
  reusable behavior use the application, and thin application delegates may use an optional local
  `launchers/` directory.
- Defined a standalone tool by its independence from repository-local packages rather than by a
  one-file limit. A self-contained tool directory may own its helpers and use standard-library
  modules plus declared installed dependencies.

### Fixes and Maintenance

- Corrected the GitHub About source-paragraph limit from 250 to 350 Python characters in the
  canonical repository policy and the vendored README gate, including its test name and failure
  message.
- Rewrote the vendored `tools/TOOLS_README.md` and `devel/DEVEL_README.md` as concise,
  audience-specific explanations of the canonical policy. Removed the historical tools migration
  material and aging devel inventory while preserving current Graphify and release instructions.
- Extended the existing support-directory gate to recognize `launchers/`, reject standalone-tool
  imports of root repository packages and packages grouped under `packages/`, allow tool-local
  helpers, and preserve devel's flat sibling-helper model.
- Updated the `source_me.sh` import-path example, test guidance, and consumer/template ledgers to
  distinguish application launchers from standalone tools and local launcher convention from
  propagation policy.
- An independent six-pass audit found and fixed two policy gaps: the gate now recognizes
  Python-bearing namespace-package directories without `__init__.py`, and template documentation
  no longer directs standalone tools to root-level helper packages.

### Decisions and Failures

- Kept current command paths and propagation routing unchanged. The policy does not create a
  `launchers/` directory; repositories add one only for a concrete launcher use case.
- Kept permanent coverage focused on import boundaries. README wording, current command inventory,
  and migration-state checks remain outside pytest.
- An optional full-suite audit exposed the committed universal-to-meta link from
  `docs/PYTEST_AUTHORING_GUIDE.md` to `docs/CHANGELOG.md`. The source guide, isolation test, and
  propagation routing are unchanged by this work, so the unrelated documentation repair remains
  outside this script-placement change.

### Developer Tests and Notes

- `source source_me.sh && python3 -m pytest tests/test_readme_first_paragraph.py
  tests/test_pytest_hygiene.py tests/test_function_typing.py tests/test_ascii_compliance.py
  tests/test_whitespace.py tests/test_vendored_headers.py tests/test_markdown_links.py -q` passes
  all 585 focused cases.
- `source source_me.sh && python3 -m pytest tests/test_support_dirs_not_imported.py
  tests/test_import_requirements.py -q` passes all 261 focused cases.
- The related pyflakes, typing, pytest-hygiene, source-limit, ASCII, whitespace, indentation,
  vendored-header, and guidance-format gates pass all 939 cases.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q` passes all 56 cases,
  and `git diff --check` is clean.
- A one-time working-tree inventory confirms that every changed path is modified in place,
  propagation manifests are unchanged, and no `launchers/` directory exists.
- A one-time direct invocation of a disposable `tools/report/main.py` importing
  `helpers.formatter` from the same self-contained tool directory printed the expected result under
  the normal sourced Python command. The temporary tree was removed; no inventory-shaped pytest
  was added.
- The optional full suite reached 2,416 passes and the one pre-existing link-bucket failure recorded
  above; the requested focused, Markdown-link, and diff gates are green.

## 2026-09-06

### Additions and New Features

- Added `tests/test_bash_script_line_limit.py`, a fast hygiene gate for nonignored `.sh` files.
  It requires fewer than 100 physical lines and 8000 characters and reports how to simplify the
  script or move substantial logic to Python under the existing 1000-line source-file limit.
- Added `docs/PYTEST_AUTHORING_GUIDE.md`, a concise companion to `docs/PYTEST_STYLE.md`. It covers
  shared pytest implementation conventions, focused/full verification, and maintenance records.
  `meta/docs/PYTEST_META_GUIDE.md` holds template-only propagation, vendoring, and meta-test
  coverage.

### Fixes and Maintenance

- Simplified the four existing scripts that exceeded the new Bash budget while retaining their
  commands and cleanup scopes: `devel/clean_build.sh`, `devel/dist_clean.sh`,
  `templates/typescript/check_codebase.sh`, and
  `templates/typescript/noexist/run_playwright_tests.sh`.

### Decisions and Failures

- The new Bash line-limit counts a final unterminated physical line and uses a separate character
  ceiling, so collapsing substantial logic onto a few long lines does not bypass the policy.

### Developer Tests and Notes

- Classified the initial Bash-script syntax and size inventory as one-time rebuild evidence. The
  permanent suite retains only the deterministic real-file hygiene gate and its report.

## 2026-09-05

### Additions and New Features

- Added a dedicated development-requirements propagation bucket. It seeds missing files, migrates
  marker-free consumers, compares parseable requirements by canonical package name, refreshes the
  universal block, and preserves repository-specific dependencies, comments, and pip directives.
- Reworked `devel/graphify_docs_lib.py` to generate a compact community-level SVG directly from
  `graph.json`, plus repository-group, major-community, and graph-observation prose in
  `docs/GRAPHIFY.md`.

### Behavior or Interface Changes

- Made `--svg` independent of the mutually exclusive Graphify lifecycle modes. Bare `--svg`
  publishes an existing graph; `--fresh --svg` and `--update --svg` build and publish in one run.
- The Markdown link checker now includes nonignored untracked regular files as sources and targets
  while their creation age is strictly under 24 hours. It captures one current time per scan, uses
  macOS birth time with a ctime fallback, and continues to reject ignored or older paths.

### Fixes and Maintenance

- Re-verified the completed support-directory work without changing it: the repository survey and
  reciprocal `tools/` and `devel/` documentation are present, and the focused import-boundary gate
  passes all 136 cases.
- Updated propagation routing, source resolution coverage, meta-leak checks, reset E2E inventory,
  maintainer guidance, and bucket documentation for the new requirements policy.
- Regenerated only this repository's Graphify artifacts after an incremental map update. The page
  now reflects the new requirements synchronizer and places its SVG immediately below the title.

### Removals and Deprecations

- Removed the matplotlib SVG cleanup stage, its focused tests, and the `lxml` development
  requirement from this template. Future propagation leaves consumer-owned legacy copies intact.
- Removed all six completed entries from `meta/docs/TODO.md`.

### Decisions and Failures

- The committed Graphify illustration shows only the largest twelve communities. Circle size
  represents membership and line weight represents intercommunity relationships; readable names
  and source detail belong in Markdown rather than a per-community SVG legend.
- Left other repositories' existing Graphify artifacts unchanged. Their compact pages regenerate
  the next time their maintainers run `--svg`.
- Requirements synchronization is defined and tested in this template; applying it to another
  repository remains an explicit maintainer operation.

### Developer Tests and Notes

- `source source_me.sh && python3 -m pytest tests/ -q` passes all 2,377 permanent fast-lane tests.
- Focused requirements synchronization cases verify marker-free migration, managed precedence,
  local-content order, later updates, malformed-marker refusal, and missing-file seeding.
- A one-time render at 1600 by 900 pixels confirmed the unlabeled community figure remains clear.
  The prior 405 KB cleaned full-graph SVG is now a 3.3 KB self-contained SVG; no byte-size gate was
  added.

## 2026-09-04

### Additions and New Features

- Added `devel/graphify_prune_tests.py`, removing Rust `#[cfg(test)]` symbols from `graph.json`
  between extraction and clustering. Spans come from tree-sitter rather than a brace scan, which
  would have to reason about strings, comments, and nested modules to be correct.
- Added `devel/graphify_docs_lib.py` and a `--page` mode writing `docs/GRAPHIFY.md`: a Mermaid
  community diagram GitHub renders natively, a size and language summary, a community table, and
  the most-connected symbols per area.
- Added `devel/graphify_clean_svg.py`, turning Graphify's 1.9 MB SVG export into a 435 KB
  committable figure by dropping per-symbol labels, collecting unreferenced definitions, and
  rounding coordinates.
- Added `devel/graphify_context_lib.py`, holding the Graphify artifact loaders and orientation
  formatting that `devel/graphify_map_repo.py` previously carried inline. The script was at 839 of
  its 1000-line budget, so the split ran before any feature work rather than after it. This follows
  the existing `changelog_lib.py` sibling-helper pattern and ships by folder with no manifest entry.
- Manager context now reports architectural hubs and map size. The `gods` field was already being
  validated by the analysis-sidecar loader and then discarded, so the hub data was parsed and thrown
  away on every run.
- Added `--reflect`, aggregating outcomes saved with `graphify save-result` into
  `graphify-out/reflections/LESSONS.md`. Manager context points at that file when it exists but
  never regenerates it, so building or printing context cannot rewrite reflections as a side effect.
- Added `--global`, merging a repository into the shared cross-repository graph and tagging it with
  the repository directory name. Useful only for a repo family, which is what this template seeds.
- Added `--deep` for aggressive inferred-edge semantic extraction, and `--force-shrink` so an update
  can write a smaller graph after code was deleted.
- Added a Graphify section to `devel/DEVEL_README.md` so downstream repositories learn that
  `graphify-out/MANAGER_CONTEXT.md` exists and that targeted queries beat a broad repository sweep.

### Behavior or Interface Changes

- Minimized `devel/graphify_map_repo.py` to automatic update, `--fresh`, `--update`, `--context`,
  and `--svg`. The SVG action writes only cleaned `docs/GRAPHIFY_map.svg`; Graphify's full export
  remains generated under `graphify-out/`. `--ollama` remains available with `--fresh` when the
  Claude allowance is exhausted.
- Cross-area connectors now drop symbols spanning more than a quarter of the map, with a floor of
  three communities so small maps still report connectors. A `Timestamp` type joining 34 of about 40
  communities was being presented as a navigational bridge, which it is not.
- Notable relationships and architectural hubs now exclude test scaffolding and uninformative call
  targets. The whole surprises list is scanned before truncation, because filtering otherwise
  empties the section on a test-heavy repository.
- Incremental builds pass `--missing-only` when relabeling. A full relabel previously re-paid for an
  LLM call per community whenever `--update --include-docs` ran.
- Fresh builds now report same-endpoint edge collapse after benchmarking, and context mode warns
  when non-code changes are pending. Both are advisory and neither can fail a build or suppress
  orientation.

### Fixes and Maintenance

- The six-pass audit restored every visible major-area name in `docs/GRAPHIFY_map.svg` by moving
  legend glyph definitions to root SVG definitions before deleting node-label groups. The cleaner
  now rejects unresolved local references, and the committed figure was rendered at 1600 px and
  640 px to confirm all 26 group labels remain visible.
- Graphify page generation now treats malformed optional SVG exports as non-fatal, sanitizes LLM
  community names before placing them in Mermaid or Markdown, and writes detail sections for every
  community as planned. Architectural hubs now include their source paths.
- Removed the thin SVG file-round-trip pytest and strengthened the retained-label test around the
  actual cross-group glyph-reference failure that broke the rendered legend.
- A disposable real-Cargo run found that fresh, unclustered Graphify output stores relationships
  under `edges`, not the post-clustering `links` field used by the initial fixture. The pruner and
  its fixtures now follow the real extraction schema.
- A fresh build in a Cargo repository now extracts with `--no-cluster`, prunes Rust test symbols,
  then runs `cluster-only`, so community detection and hub ranking never see the test suite. The
  gate is `Cargo.toml`, so repositories without Rust run the original pipeline unchanged, and the
  run reports how many nodes and links were removed. Incremental updates deliberately do not
  prune: re-clustering renumbers communities and would strand the stored labels.
- `tests/meta/test_graphify_map_repo.py` now places `devel/` on `sys.path` itself. That test
  propagates to consumer repositories but `pytest.ini` does not, so the sibling import added by the
  module split would have failed downstream where nothing puts `devel/` on the path.
- Rotated `docs/CHANGELOG.md` past its 800-line threshold, moving 2026-08-10 through 2026-08-31
  into `docs/CHANGELOG-2026-08b.md` and keeping the two most recent day blocks active.
- Removed the temporary Cargo argv and lifecycle probes after the disposable real-Cargo rebuild
  proved the sequence end to end. The permanent pruning module now tests Rust span recognition and
  graph surgery, while `tests/meta/test_graphify_map_repo.py` is back to 829 lines.
- Applied the permanent-test checklist to all Graphify additions and removed 31 rebuild-only cases:
  exact CLI plumbing, output-count snapshots, file-wrapper round trips, duplicate predicates, and
  geometry-attribute inventories. Their useful implementation evidence remains in this changelog;
  only fast, deterministic tests of durable behavior remain in pytest.
- Refined maintainer guidance so adaptability comes from clear boundaries, stable domain concepts,
  and replaceable components, while speculative mechanisms wait for a concrete requirement or
  likely failure mode.

### Removals and Deprecations

- Removed the wrapper's `--reflect`, `--page`, `--force-shrink`, `--deep`, `--global`, and
  `--include-docs` controls. Their narrow upstream behaviors do not belong in the recurring
  repository-mapping workflow.

### Decisions and Failures

- Rejected `docs/USAGE.md` as the home for Graphify guidance. Files under `docs/` propagate by
  overwrite, so creating one in the template would have replaced every consumer's own usage
  document on the next sync. `AGENTS.md` and `README.md` were rejected for the opposite reason:
  the first ships only when absent, the second never ships.
- Rejected `graphify claude install` and `graphify codex install`. They write into `CLAUDE.md` and
  `AGENTS.md` and install a PreToolUse hook, contending with the merge and noexist routing those
  files already have.
- Rejected shelling out to `graphify check-update`. It only tests for a `needs_update` flag file and
  always exits zero, so a subprocess would have cost a process and broken context mode's documented
  promise to print orientation without running Graphify. The flag is read directly instead.
- Recorded `docs/GRAPHIFY.md` and `docs/GRAPHIFY_map.svg` as files that never transfer between
  repositories, using exact entries rather than a filename pattern. Link-bucket classification
  reads only the exact list, and the page must classify as non-shared so it may link to the
  `devel/` tooling that generates it. Rejected creating the page under a shared docs path, which
  would have overwritten each repository's page with a map of the wrong codebase.
- Rejected extracting Mermaid from Graphify's call-flow HTML. Its diagrams pin a dark theme and
  use HTML labels, which renders wrong in GitHub light mode and risks sanitization, and scraping
  generated markup would couple this repository to a presentation layer that changes often. The
  diagram is generated from `graph.json` instead.
- Measured, then kept, the unreferenced-definition pass in the SVG cleaner even though it removes
  nothing from current output: matplotlib nests each glyph definition inside the text group that
  uses it, so dropping the labels already collects them structurally. Kept because it is cheap and
  covers definitions emitted at the document root; it is exercised by unit tests, not by real
  output. Measured 42 definition blocks before label removal, 40 of them nested, 5 remaining after.
- Confirmed matplotlib is not a Graphify dependency and added none. It is only needed by
  `graphify export svg`, so a machine without it gets the page with no figure.
- Confirmed but did not fix an upstream defect: Graphify's Rust extractor indexes
  `#[cfg(test)] mod tests` contents as production symbols, because `walk()` in
  `graphify/extractors/rust.py` reaches `mod_item` through its generic fallback and never inspects
  an `attribute_item`. Graphify's own source is not modified; the prune step removes the symbols
  from this repository's copy of `graph.json` between two of Graphify's documented commands. An
  upstream fix would still be better, since it would reach every Graphify user and every build
  path, including the incremental updates that deliberately do not prune here.

### Developer Tests and Notes

- `source source_me.sh && python3 -m pytest tests/ -q` passes 2359 permanent fast-lane tests. The 60
  retained Graphify cases cover connector spread, test-symbol filtering, hub rendering, advisory
  staleness, Rust span pruning, safe page rendering, and SVG reference integrity across four
  focused modules.
- A disposable clone of `peptidyle-learning-engine` completed the real Rust pipeline: Graphify
  extracted 10,501 nodes and 31,376 edges; pruning removed 720 test nodes and 1,849 incident edges;
  and `cluster-only` completed with 9,781 nodes and 27,082 edges. The named denied-membership test
  symbol was absent from both `graph.json` and the generated orientation afterward.
- Measured on this repository: the figure cleaner reports 1934 KB to 435 KB, 452 node labels
  removed, 26 community labels kept.
- Two existing tests changed meaning rather than being loosened. The bridge test now asserts the god
  node never takes the connector slot instead of asserting it is absent entirely, since hubs are now
  rendered. The connector-bounding test now builds a map large enough that a ten-community connector
  is still a real bridge, since otherwise the spread filter rejects it before the display bound is
  reached.

## 2026-09-02

### Additions and New Features

- Added a commented `tests/source_file_line_limit_overrides.txt` seed through universal noexist
  propagation, giving new consumers exact-path instructions while preserving existing approvals.
- Added support-directory and root-script-budget pytest gates, plus `tools/TOOLS_README.md` and
  reciprocal devel guidance. Extracted the Gitignore and reset-finish owners into focused modules.
- Added a complete `~/nsh` tools/devel usage survey covering 109 Git roots and defining the
  user-utility, developer-command, application-CLI, and importable-package boundaries.
- Recorded the settled audience and input/output placement rule in the design-decision ledger.

### Behavior or Interface Changes

- The source-file line-limit gate now automatically excludes Markdown beneath any
  `docs/active_plans/` or `docs/archive/` tree while retaining coverage for other source types.
- Rendered `.gitignore` now places canonical propagated blocks before the consumer-owned LOCAL
  block, preserving the LOCAL body while replacing stale recognized managed blocks.
- Reset removes consumer `pytest.ini` and uses one default-Yes finish decision for stage, commit,
  and push; config keeps push off by default and reports explicit publication terminal outcomes.

### Fixes and Maintenance

- Preserved consumer `.gitignore` rules beneath the previous full LOCAL heading and moved that
  section after canonical managed blocks, including repositories where it appeared in the middle.
- The final six-pass audit aligned archive wording, propagation-ledger navigation, support-tool
  examples, positive changelog guidance, and the noexist-routing test description.
- Rephrased source-line-limit guidance around the required exact-path format and individual
  approvals, following the repository's positive-prompting policy.
- Restored the shipped human-guidance and design-decision files to neutral consumer seeds and moved
  starter-template-specific records into `meta/docs/`.
- Removed the reset `pytest.ini` leak while retaining that template-owned configuration for the
  non-shipping meta suite; trusted file-path loaders and a template-owned propagation changelog
  writer remove the remaining support-directory import drift.
- Removed real Git repository, commit, remote, and push implementation checks from permanent
  pytest; the excluded reset E2E harness owns those workflows and now covers every publication
  terminal outcome.
- Removed unused `repolib.files` and `repolib.reset` compatibility facades and the dead
  `replace_managed_block` helper; callers now use the focused owning modules directly.
- Review corrections preserved heading-like LOCAL comments, qualified no-finish stage wording,
  restored source-line budgets, and removed Bandit temporary-literal findings.
- Six-pass audit corrections refreshed the reset quick start, Gitignore ownership and rendering
  documentation, plan and survey timing, and Python comment structure.
- The final audit documented universal `tools/` routing and exact deprecated-path cleanup, aligned
  live pytest commands with the repository environment, and corrected a legacy LOCAL docstring.

### Decisions and Failures

- Recorded adaptability and good-enough stopping points as complementary template-maintainer design
  guidance; the Gitignore transition uses a focused compatibility rule for observed prior banners.
- Keep the disk-budget `du` check in the base pytest lane; it measures the checkout and the
  vendored file is restored after deletion. Rejected a footer-comment gate as comment-only policy.
- Use exact source-line-limit overrides for individually approved external files; universal
  planning/archive exclusions belong in the gate.
- The audit remains active in `meta/docs/active_plans/audit/`; completed planning artifacts move to
  `docs/archive/` under the repository convention. Its current survey corrects stale historical
  no-runtime-import claims; consumer repairs remain with their maintainers.
- Support-directory imports unconditionally reject `tools`, `devel`, and `tests` package roots;
  documented flat devel and test helpers remain allowed, while tools scripts never import tool
  siblings. The root-script budget counts tracked `.py` and `.sh` files plus executable-shebang
  launchers; five or six report and seven or more fail.
- The six-pass audit left coordinated design work open for destination-first deprecated-path
  migration and duplicated propagation-changelog parsing.
- The usage survey classifies Graphify and TypeScript dependency refresh as developer commands,
  keeps HTML-to-PDF as a user tool, and identifies nine identical stale devel PDF copies for
  propagation cleanup.
- Keep one native application, library, or tool-helper package in a named root-level folder. Use
  `packages/` as the grouping layer when a repository contains multiple native products or
  packages.
- Moved the universal Graphify launcher and typed TypeScript dependency refresh into `devel/`.
  Propagation now retires their former `tools/` paths and the obsolete duplicate
  `devel/html_to_pdf.mjs` path.

### Developer Tests and Notes

- `source source_me.sh && pytest tests/ -q` passes all 2,225 permanent fast-lane tests.
- Accepted focused gates cover Gitignore rendering, reset configuration and interview outcomes,
  support-directory imports, root-script thresholds, documentation hygiene, typing, and pyflakes.
- A disposable current-candidate clone passed the complete LOCAL reset E2E matrix, including a
  synthetic bare-origin publication check and a local push-failure outcome; no remote host was
  contacted. A disposable TypeScript consumer confirmed moved-path cleanup and a clean second
  propagation pass. Diff checks passed.
