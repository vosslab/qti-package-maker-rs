## 2026-08-31

### Behavior or Interface Changes

- `docs/MARKDOWN_STYLE.md` now adopts GitHub Flavored Markdown as its syntax baseline and links its
  official online specification.
- The high-level guide restores pipe tables as the preferred form for tabular content, includes a
  simple table example, and adds accessibility boundaries for captions and complex headers.

### Decisions and Failures

- Kept the propagated `docs/HUMAN_GUIDANCE.md` and `docs/DESIGN_DECISIONS.md` seeds unchanged and
  recorded the starter-template-specific Markdown direction in `meta/docs/HUMAN_GUIDANCE.md`.
- GitHub Flavored Markdown replaces core CommonMark because it specifies pipe tables while
  retaining CommonMark as its foundation and matching the repository's GitHub rendering target.
- GFM has no caption or complex header-association syntax. Those tables require semantic HTML or an
  appropriate publishing pipeline; the guide links W3C accessible-table guidance rather than
  duplicating it.

### Developer Tests and Notes

- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py
  tests/test_ascii_compliance.py tests/test_whitespace.py tests/test_source_file_line_limit.py -q`
  passes all 535 focused documentation and hygiene tests.
- `pandoc --from=gfm --to=html docs/MARKDOWN_STYLE.md -o /dev/null` parses and renders the guide
  successfully with Pandoc's GFM reader.
- `git diff --check` passes.

## 2026-08-30

### Behavior or Interface Changes

- Graphify manager context now says `Graph mapped at <local time>` instead of attributing the map
  to Graphify's `built_at_commit`. The wrapper maps uncommitted and untracked working-tree code, so
  a commit hash was incomplete provenance and confused coders about the graph's actual inputs.
- Root `tools/` is now retained through `reset_repo.py` cleanup, so the universally routed
  `tools/graphify_map_repo.py` survives bootstrap in TypeScript and every other repository type.

### Decisions and Failures

- The timestamp comes from the primary generated graph artifact rather than the current clock. It
  therefore advances when Graphify rebuilds the map and remains stable when `--context` only reads
  existing artifacts.
- Corrected the documentation owner during review: this repository's Graphify preference and
  rationale live in `meta/docs/HUMAN_GUIDANCE.md`. Putting them in the root guidance seeds would
  copy starter-template-specific content into every new consumer created through the HEADER bucket.
- Ordinary propagation already routed root `tools/` universally; a TypeScript dry run correctly
  planned `tools/graphify_map_repo.py`. The loss occurred later because reset cleanup deleted the
  template clone's tracked `tools/` directory after propagation. The routing test also checked only
  Python, allowing its universal claim to remain narrower than its name.
- Permanent-test review retained the fixed-input manager-context format contract and the pure,
  offline universal-routing contract. It removed a test that merely compared the timestamp helper
  with `Path.stat().st_mtime` and removed redundant timestamp assertions from the report-fallback
  test; those implementation details did not earn additional permanent coverage.
- The existing clone-based reset harness remains in the explicit E2E lane because full repository
  bootstrap is durable behavior. No Graphify-specific E2E assertion was added; its engine-derived
  expected-file check already covers universal tools after the reset fix is committed.

### Developer Tests and Notes

- Permanent tests: `source source_me.sh && python3 -m pytest
  tests/meta/test_graphify_map_repo.py -q` passes all 30 focused Graphify tests.
- `source source_me.sh && python3 -m pytest tests/meta/test_repolib_folder_convention.py
  tests/meta/test_no_meta_leaks.py tests/meta/test_no_meta_content_leaks.py -q` passes all 63 focused
  propagation tests; `source source_me.sh && python3 -m pytest -q` passes all 1,988 tests.
- One-time implementation checks: a disposable current-worktree TypeScript reset retained the
  tracked `tools/graphify_map_repo.py`, and a TypeScript dry run planned the same universal tool.
- One-time implementation check: a real default update rebuilt the current working-tree graph with
  433 nodes and 616 edges, wrote `graphify-out/MANAGER_CONTEXT.md`, and rendered
  `Graph mapped at 9:47 PM CDT Aug 30 2026` with no commit attribution.

## 2026-08-27

### Additions and New Features

- Added the HEADER propagation bucket: a consumer-owned file is seeded whole when absent, and on
  every later sync only its marked vendored region is refreshed while the repository's own entries
  stay byte-for-byte intact. `repolib/header_sync.py` holds the helper, `header_files` in
  `meta/propagation/manifests.yaml` names its members, and `meta/docs/HEADER_BUCKET_SPEC.md` is the
  specification. Routing runs after the MERGE step and removes a path from the overwrite and noexist
  buckets, since seed-plus-refresh subsumes both.
- Seeded `docs/HUMAN_GUIDANCE.md` and `docs/DESIGN_DECISIONS.md` through that bucket. The first
  holds guidance the human states, in his own words at one to three lines per bullet; the second
  holds settled decisions about how the code and repository are shaped, written as `Decision`,
  `Why`, `Consequence`, and `Owner` fields.
- `docs/REPO_STYLE.md` now carries the authoritative statement of that split, the vendored headers
  restate it in three lines, and `AGENTS.md` points at both files in one line. Making REPO_STYLE the
  single source keeps the same rule from drifting independently in three places.

### Behavior or Interface Changes

- `propagate_style_guides.py` now adds a `Fixes and Maintenance` entry to the target repository's
  `docs/CHANGELOG.md` after a successful, non-dry-run propagation makes real changes. No-op,
  dry-run, and failed propagation runs leave the changelog alone. The writer is part of
  `devel/changelog_lib.py`, inserts into an existing current-day category or creates a canonical new
  day block, and refuses unsafe rewrites when date headings are invalid or duplicated.
- An ambiguous vendored-marker structure (unpaired, duplicated, or reversed markers) is reported as
  an error and leaves the file untouched, rather than guessing which region to rewrite.

### Fixes and Maintenance

- Follow-up six-pass review documented the consumer-changelog side effect in `README.md`, removed
  dead index state from the changelog insertion helper, and added intent comments around its
  section-boundary preservation logic.
- Stopped `.gitignore` from reporting two changes on every propagation when a repository-owned
  unanchored rule canonicalized to the same rule in a propagated block. Deduplication now retains
  the propagated canonical copy and removes the equivalent local alias, so later runs are byte-stable
  and do not trigger a spurious changelog entry.
- Fixed a false positive in the shipped prose rule found by review: a blank line no longer closes a
  bullet, so a bullet written with a blank-line-separated continuation is read as one bullet instead
  of a stray prose paragraph. The bug would have failed CI in every consumer repo the first time
  someone wrote ordinary Markdown, without any code change on their side.
- Guarded the marker strings the shipped tests carry. They cannot import `repolib`, which never
  propagates, so both keep their own copies; a new template-meta case asserts those copies equal
  `repolib.header_sync`'s constants. Drift there would be silent rather than loud, because both
  tests discover the files they check by matching the markers: a stale copy yields an empty file
  list and a green run that checked nothing.
- Replaced the stale "one of four policy categories" count in `meta/docs/PROPAGATION_RULES.md` with
  a count-free sentence and added the HEADER bullet, plus a `HEADER_FILES` entry in the manifest
  section beside `MERGE_FILES`. Dropping the count rather than bumping it to five keeps the next
  bucket from making the same line stale again.
- Restored the import-group comments and `#===` function separators in
  `tests/meta/test_repolib_header_sync.py`, the one file in the change that had drifted from the
  convention its siblings follow.
- Reworded a docstring that referenced "the milestone gates", planning scaffolding that does not
  exist in the repo, to describe the test names as `-k` selectors instead.
- Added `header_files` to the bucket tuples in `tests/meta/test_no_meta_leaks.py` and
  `tests/meta/test_no_meta_content_leaks.py`. Both enumerate buckets explicitly, so the new bucket
  was invisible to them: they passed without ever inspecting it, which made the META guard on
  `header_files` look verified when nothing had exercised it. Checked that the guard now fires by
  routing a META path into the bucket and watching the run fail; that shows the check is not
  vacuous, which is a weaker claim than the bucket being correct.
- Added HEADER cases to `tests/meta/test_repolib_plan_precedence.py`: one proving a HEADER path
  leaves the overwrite and noexist buckets, one proving META still wins over HEADER.
- Restored "or approves for preservation here" and "close paraphrase" to the vendored header in
  `docs/HUMAN_GUIDANCE.md`. An earlier pass shortened the header and dropped both, which quietly
  narrowed the provenance test the wording exists to state; `docs/REPO_STYLE.md` had kept the full
  phrase, so the two had diverged.

### Decisions and Failures

- Surveyed the eleven local `docs/HUMAN_GUIDANCE.md` files before designing anything: 8 to 495
  lines, 1,300 lines total. The largest files are large because of agent-authored sections
  (`Security design decisions`, `Dependency versions`, `Generated artifacts`) carrying external
  legal citations, while the healthy files run 2.0 to 3.0 lines per bullet. That measurement is
  where the one-to-three-line entry target comes from.
- `peptidyle-learning-engine` already had a 483-line `docs/DESIGN_DECISIONS.md` and still leaked
  agent decisions into its human-guidance file. A parallel file alone therefore does not stop the
  leak, which is why the routing rule ships as refreshable text rather than as a seed-once stub.
  Its `Decision` / `Why` / `Consequence` / `Owner` entry shape was adopted as the seeded skeleton.
- Reversed the earlier rejection of HTML-comment fences recorded in `meta/docs/MERGE_BUCKET_SPEC.md`.
  That rejection was specifically about consumers needing a one-time hand edit; every surveyed file
  carries a level-one heading on line 1, so a marker-free file gets its header inserted at that
  anchor automatically and no repository is asked to prepare a file. The anchor keys on the heading
  line rather than its text, because one surveyed file opens with an unrelated title.
- Stated the file's ownership as three parts (region, separator, content) after an earlier draft
  claimed both that content outside the region is preserved byte-for-byte and that adjacent blank
  lines are normalized. Naming the separator as its own part removes the contradiction.
- Entry threshold for `docs/DESIGN_DECISIONS.md`: material earns an entry by becoming settled
  rationale, not by arriving from a reviewer. Without that threshold the new file would accumulate
  review transcripts and reproduce the problem it was created to fix.

### Developer Tests and Notes

- Added focused tests that parse the generated consumer entry in strict mode through
  `devel/changelog_lib.py`, verify canonical category placement, reproduce the local-versus-managed
  `.gitignore` churn, prove its first run moves the canonical rule into the managed block, and prove
  a second real CLI run changes neither `.gitignore` nor the generated changelog.
- Added two shipped hygiene tests, split by concern. `tests/test_vendored_headers.py` checks the
  header region in any file carrying the markers: it discovers by marker rather than by filename, so
  a file added to the HEADER bucket later is covered with no edit, and a doc that quotes the markers
  inside a fence stays out of scope. `tests/test_guidance_doc_format.py` checks entry shape in the
  two guidance docs, and uses formatting to keep the human file honest. Both follow the standard
  hygiene shape (`discover_files` with a `test_key`, a `collect_report` autouse fixture, and a
  `report_*.txt`), so a repository can scope either through `REPO_HYGIENE_FILTERS` while bringing
  existing files into compliance.
- Rules, each set from a corpus measurement rather than a guess: the vendored header region is
  present, paired, and non-empty; `HUMAN_GUIDANCE.md` bullets stay within three lines; entries under
  a section are bullets rather than prose paragraphs; and every `DESIGN_DECISIONS.md` entry carries
  its `Decision` and `Why` fields. Ordered lists and table rows count as structure, and prose above
  the first section heading is left alone.
- A HUMAN_GUIDANCE failure closes with the question the formatting rules stand in for: "are we sure
  this guidance came from the human, and not from an agent or an LLM reviewer? Long prose usually
  means it did not." The rules are a proxy for provenance, so the failure says so rather than
  leaving an agent to treat it as a formatting chore. It rides on the last violation line so the
  reported violation count stays accurate.
- Added the tie-break rule in all three places the split is stated: rearrange aggressively, and when
  an entry's origin is uncertain move it to `docs/DESIGN_DECISIONS.md`. The asymmetry is the reason
  a default is safe to state at all: a design decision filed as human guidance misrepresents who
  decided it, while the reverse only files a preference one document away.
- Prose share is what makes the bullet rule a usable honesty signal: across the local corpus, files
  that kept the human's terse statements run 0 to 7 percent prose (vosslab-skills 0, vossvolvox 2.2,
  peptidyle 2.4, syllabus 5.6, bkchem 7.1), while files an agent expanded run 19 to 100 percent
  (super-bowling 18.8, SwiftlyCodeEdit 73.1, mule-game 91.3, virtual-lab 100). The two shapes are
  cleanly separated, so the rule catches narration without judging content.
- Left section names unenforced. They legitimately differ per repository (`Audience and content`,
  `Product identity`, `Device and viewport priorities`), so a required-name list would fight real
  content instead of catching agent narration.
- Validated the decision-entry rule against the only real example: peptidyle's 483-line
  `docs/DESIGN_DECISIONS.md` passes with zero violations, so the required fields describe a shape
  that already works at scale.
- `tests/meta/test_repolib_header_sync.py` also checks every `header_files` manifest entry against
  its template source, and that syncing a stub onto a copy of itself reports `unchanged`. A
  manifest entry whose source lost its markers would otherwise error in every consumer at once.
- `tests/meta/test_repolib_header_sync.py` covers replacement and insertion with distinctive
  multiline content above and below the region, idempotence, each ambiguous marker structure, the
  no-heading fallback, and a synthetic unrelated file exercising the helper against content that
  shares nothing with the two documentation files.
- Added `tests/meta/test_guidance_format_rules.py`, adversarial-input tests for the vendored
  guidance-format parsing helpers. The shipped rules run against real, varied Markdown in consumer
  repos while the template holds near-empty stubs, so exercising them end-to-end here proved almost
  nothing; that gap is how the blank-line continuation bug survived. The helpers are identical in
  every repo that receives them, so the edge cases live once in template-meta.
- `tests/meta/e2e/e2e_header_bucket.py` runs the real CLI against disposable consumers built from
  captured file shapes rather than copies of live repositories, so the harness runs anywhere. It
  checks refresh, seeding, consumer-content preservation, idempotence across two runs, and the
  ambiguous-marker refusal.
- Put the helper in its own module because `repolib/files.py` sits at 976 lines against the
  1000-line source gate.

## 2026-08-26

### Behavior or Interface Changes

- Propagation now migrates recognized legacy root licenses to the current `LICENSE.<SPDX>` system.
  It handles prior typed Markdown names, observed underscore-era names, grep-detectable generic
  `LICENSE`/`LICENSE.md` bodies, and safe generic symlink aliases. Complete or customized bodies
  are preserved; only fingerprinted old summaries receive the complete local catalog text.
  Conflicting, custom, and ambiguous legal files are warned about and left untouched.
- Changed reset-installed license names from `LICENSE.<SPDX>.md` to real plain-text
  `LICENSE.<SPDX>` files so the license identifier remains visible while the current Licensee
  filename matcher can recognize the file. Release preparation now requires this convention,
  rejects unsupported names, executable files, and symlinks, and verifies every license in
  multi-license source archives. Fresh resets also seed a README scope map for the selected
  code and documentation licenses.
- Added `-D`/`--include-docs` to fresh and explicit update Graphify builds. It includes
  nonignored document, paper, and image inputs through the selected Claude CLI or Ollama
  backend while ordinary fresh builds remain code-only.
- `--update --include-docs` now runs Graphify's incremental semantic extraction without
  `--force`, refreshes all community labels, validates artifacts, and regenerates manager
  context. When no graph exists, it falls back to the complete fresh semantic lifecycle.
- Pinned Claude CLI semantic extraction to Sonnet and retained the configured Ollama model for
  local semantic extraction.

### Fixes and Maintenance

- Surveyed 92 root license candidates across local `~/nsh` Git repositories before defining the
  migration. The observed set included 36 typed Markdown files, 26 recognized underscore-era
  names, 28 generic files or links, 16 known abbreviated template bodies, and 5 unrecognized or
  custom-license files. Added focused migration coverage for full-text replacement, customized
  body preservation, generic body detection, symlink cleanup, conflicts, and dry-run behavior,
  plus a disposable real-CLI propagation E2E.
- Six-pass audit fixes restricted body replacement to exact known summary fingerprints, required
  completion markers before promoting any other recognized generic or typed body, retained generic
  aliases when canonical-content conflicts preserve their legacy targets, and replaced fixed-depth
  test-root derivation with the repository's Git-root conventions.
- Replaced all eight abbreviated license summaries in `LICENSES/` with complete publisher legal
  text, stored as ASCII. Added `meta/docs/LICENSE_POLICY.md` as the canonical policy for root
  filenames, multi-license scope, GitHub detection, upstream body sources, reset behavior, and
  release validation; it also classifies the supplied action, community discussion, and gist as
  supporting rather than normative sources.
- Promoted root `/node_modules/` and `/.env` ignores to the universal propagation template and
  the template repository's own `.gitignore` because Node tooling and secret-bearing local
  environment files cross repository types. Removed the redundant TypeScript-only
  `node_modules/` entry.
- Defined `meta/propagation/deprecated_gitignore.txt` as the canonical negative-rule list and
  added both `__pycache__/` spellings so propagation keeps Python cache directories visible for
  explicit cleanup. Removed the stale `_bundle.js` template entry that contradicted the existing
  negative rule. Exact-line cleanup now leaves one trailing newline instead of adding an extra
  blank line, covered by one generic behavior test.
- Added the anchored and unanchored `.claude` and `.codex` directory spellings to the negative
  rule list so repository-local agent configuration is always visible. Included the observed
  `.codex/agents/` spelling used by one surveyed repository.
- Clarified `.gitignore` ownership in rendered files. Repository managers now get an explicit
  `LOCAL REPOSITORY RULES` heading labeled `ADD CUSTOM IGNORES HERE`, while propagated universal
  and active type headings state `LOCAL EDITS OVERWRITTEN`. Legacy `LOCAL` headings are renamed
  in place so ignore-rule ordering stays unchanged.
- Added a separate exact-replacement policy for canonical `.gitignore` spellings. Propagation now
  converts `node_modules/` to `/node_modules/` and `OTHER_REPOS/` to `/OTHER_REPOS/` before
  deduplication. Replacement is distinct from the negative list because the rule remains present
  in a narrower root-scoped form rather than being deleted. Deduplication now leaves one trailing
  newline when it collapses a converted alias instead of adding a trailing blank line.
- Restored the focused Graphify command-selection suite after the semantic-scope parameters
  changed, and made extraction and benchmark phase labels describe semantic maps accurately.
- Kept `.graphifyignore` authoritative for repository-specific scope. The new option includes
  semantic inputs that each repository has not excluded.
- Six-pass audit cleanup removed two permanent tests that only asserted policy lists were
  nonempty, corrected the exact-replacement test description, wrapped the changed propagation
  call, and clarified that one-off repository scans exclude paths containing `/OLD`.
- Synchronized the two existing universal local-directory rules across every eligible repository
  root. `/OTHER_REPOS/` and `/LOCAL_ONLY/` now each appear exactly once inside the managed
  `UNIVERSAL` section rather than depending on repository-local adoption.
- Defined one generated-output directory policy in `docs/REPO_STYLE.md`: use stable root
  `output/` or `output_<purpose>/` names under the universal `/output*/` rule, while leaving
  legitimate nested paths such as `tests/output/` visible to Git.
- Added the observed redundant `output/` and `output_smoke/` spellings and the broader unanchored
  `output*/` family to the canonical negative-rule list. Propagation now removes those aliases
  while retaining the root-scoped universal rule.
- Added `meta/docs/GITIGNORE_SYSTEM.md` as the canonical reference for `.gitignore` source
  ownership, pattern semantics, managed/local rendering, exact replacements, negative cleanup,
  output-family policy, maintainer workflow, and validation. `HUMAN_GUIDANCE.md` now routes to it,
  and `PROPAGATION_RULES.md` retains only the integration summary instead of a parallel policy.

### Decisions and Failures

- Kept the universal gitignore block small after reviewing the complete low-frequency survey
  tail. OS/editor cruft, generic backup and log names, and repository-specific data/media paths
  remain local instead of consuming space in every repository.
- Retained `blob-report/` in the TypeScript block. No surveyed repository currently selects
  Playwright's blob reporter, but it is a legitimate optional Playwright artifact rather than a
  universal path or a forbidden rule.
- Kept heading text out of permanent configuration-snapshot tests. Existing merge and idempotence
  tests cover the durable writer behavior; exact labels and the direct multi-repository migration
  remain one-time acceptance checks.
- Kept exact-replacement canonicalization limited to the two requested directory spellings and
  did not infer replacements for `.env`, build, or generated paths. Output aliases moved to
  negative cleanup only after the tracked-file survey established the root-only output contract;
  the universal block supplies the retained `/output*/` rule.
- Kept connected Claude CLI and Ollama semantic extraction outside permanent pytest. The
  permanent tests verify deterministic command and model selection; a real extraction remains
  one-time acceptance evidence because it invokes an LLM and produces model-derived artifacts.
- Two independent reviewers recommended a permanent `process_repo()` integration test for the
  complete gitignore transformation sequence. Kept the existing generic helper tests and
  one-time multi-repository acceptance evidence instead, following the preference to omit
  permanent tests when their added value is uncertain.
- Kept tool-specific output names separate from the general output-directory family.
  `/graphify-out/` remains universal, while `*.out`, logs, and tool-mandated directories such as
  `/out/` remain exact local or tool-owned rules rather than aliases for `/output*/`.

### Developer Tests and Notes

- Direct validation of the new 233-line `meta/docs/GITIGNORE_SYSTEM.md` passed its
  ASCII/ISO-8859-1, whitespace, final-newline, and local-link checks. The focused policy and
  tracked-file style selection passes: 502 tests.
- The ordinary pre-staging suite passed 1,819 tests and reported only the two expected tracked-link
  failures from existing meta docs to the new untracked document. A disposable info-only Git index
  then included the new document without changing the real index or object database; the complete
  intended-tree suite passes: 1,826 tests. `git diff --check` passes.
- One-time output-rule survey reproduced the supplied Mac Studio census before alignment:
  13 `.gitignore` files used unanchored `output*/`, four used `/output*/`, and five used
  `/graphify-out/`. Tracked-file inspection found legitimate nested output paths in
  `track-runner-virtual-dolly-cam/tests/output/` and
  `ferrum-chemical-forge/packages/ferrum-chem-qt.app/tests/output_smoke/`, confirming that the
  universal output rule must remain root-scoped.
- Direct `git check-ignore --no-index` probes confirm root `output_smoke/` and `graphify-out/`
  paths are ignored, while `tests/output/` and `out/` remain visible for tracked content or an
  explicit local policy.
- Permanent focused propagation-policy, Markdown-link, ASCII, line-limit, and whitespace tests
  pass: 545 tests. The complete `source source_me.sh && python3 -m pytest tests/ -q` suite passes:
  1,821 tests. `git diff --check` passes.
- One-time implementation survey: inspected 76 live `.gitignore` files under
  `/Users/vosslab/nsh` at depth three after excluding paths containing
  `/OTHER_REPOS` or `/OLD`: 70 are tracked repository-root files and six are untracked
  `.pytest_cache` internals. Root `node_modules/` rules span 27 repositories and several
  non-TypeScript types; eight repository roots carried forbidden `__pycache__/` rules. No root
  `.env` file is tracked, two repositories ignore `.claude/`, one ignores `.codex/agents/`, and
  no repository configures Playwright's optional blob reporter. Direct `git check-ignore`
  probes confirmed the template repository now ignores root `node_modules/` and `.env`.
- Permanent tests: focused exact-removal behavior, gitignore-routing, and multi-type coverage
  pass: 45 tests. The complete `source source_me.sh && python3 -m pytest tests/ -q` suite passes:
  1,819 tests. The style, typing, indentation, ASCII, line-limit, and Markdown-link selection
  passes: 632 tests.
- One-time final checks: `git diff --check` passes; direct policy inspection confirms every
  requested cache and agent-directory spelling is present in the negative list and absent from
  positive gitignore templates.
- One-time heading migration: updated 70 of 71 validated Git roots under `/Users/vosslab/nsh`,
  excluding paths containing `/OTHER_REPOS` or `/OLD`. The template root was already current.
  Verification confirmed every root has one repository-owned section, every active managed block
  has its ownership label, the ordered non-comment ignore rules did not change, root
  `node_modules` and `.env` remain ignored by Git, and a second dry run proposed zero changes.
- Permanent tests after the heading change: the focused merge, deprecation, typing, indentation,
  whitespace, and lint selections pass: 504 tests. The complete
  `source source_me.sh && python3 -m pytest tests/ -q` suite passes: 1,819 tests.
- One-time exact-spelling migration: converted 25 `node_modules/` lines and 38 `OTHER_REPOS/`
  lines across 46 of 71 validated Git roots. Verification found zero broader source spellings,
  exactly one canonical target per relevant repository, the expected ordered rule sequence with
  target-only deduplication, preserved Git ignore behavior, and zero changes on a second dry run.
- Permanent tests after exact replacement: the focused policy, merge, typing, indentation,
  whitespace, and lint selections pass: 505 tests. The complete
  `source source_me.sh && python3 -m pytest tests/ -q` suite passes: 1,820 tests.
- Permanent tests after the six-pass audit cleanup: the focused policy, merge, typing,
  indentation, whitespace, lint, and Markdown-link selection passes: 546 tests. The complete
  `source source_me.sh && python3 -m pytest tests/ -q` suite passes: 1,818 tests.
- One-time universal-rule migration: validated 71 Git roots beneath `/Users/vosslab/nsh` after
  excluding paths containing `/OTHER_REPOS` or `/OLD`; changed 57 `.gitignore` files and left 14
  already compliant. Added 17 missing `/OTHER_REPOS/` rules and 55 missing `/LOCAL_ONLY/` rules,
  moved or deduplicated four existing rules, and initialized 13 missing `UNIVERSAL` headings.
  Verification preserved every unrelated line and its order, confirmed Git ignores both root
  directories in every repository, and produced zero changes on a second dry run.
- Permanent tests after the universal-rule migration: the focused propagation, policy,
  whitespace, and Markdown-link selection passes: 273 tests. The complete
  `source source_me.sh && python3 -m pytest tests/ -q` suite passes: 1,818 tests.
- Permanent tests: `source source_me.sh && python3 -m pytest
  tests/meta/test_graphify_map_repo.py -q` passed: 30 tests.
- Permanent style and wrapper tests: `source source_me.sh && python3 -m pytest
  tests/meta/test_graphify_map_repo.py tests/test_pyflakes_code_lint.py
  tests/test_function_typing.py tests/test_indentation.py tests/test_shebangs.py
  tests/test_ascii_compliance.py tests/test_source_file_line_limit.py
  tests/test_markdown_links.py -q` passed: 837 tests.
- Permanent tests: `source source_me.sh && python3 -m pytest tests/ -q` passed: 1,818 tests.
- One-time checks: CLI help rendered the documented `--include-docs` scope, and invoking
  `--include-docs` without an explicit `--fresh` or `--update` exited 2 with the documented
  validation error. Inspection of installed Graphify 0.9.50 confirmed that incremental
  `extract` uses its manifest and cache, and stops before community labeling and reports.

## 2026-08-25

### Behavior or Interface Changes

- `devel/rotate_changelog.py` now begins automatic rotation only after the active changelog
  exceeds 800 lines. It partitions older day blocks into 800-900-line target archives, keeps
  every archive strictly below 1000 lines, and refuses an oversized day block before writing.
- Added a vendored-file notice to every root `docs/*.md` file that propagation overwrites in
  consumer repositories. Changelogs and noexist documentation remain consumer-owned.

### Developer Tests and Notes

- `source source_me.sh && python3 -m pytest tests/meta/test_rotate_changelog.py
  tests/test_source_file_line_limit.py` passed: 157 tests. The existing archive files contain
  610, 895, and 914 lines, respectively.
- `source source_me.sh && python3 -m pytest tests/` passed: 1,792 tests.
- `source source_me.sh && python3 -m pytest tests/meta/test_vendored_docs.py
  tests/test_markdown_links.py tests/test_ascii_compliance.py` passed: 204 tests.

## 2026-08-24

### Behavior or Interface Changes

- Preserved prominent phase labels in the Python Graphify tool for graph extraction or update,
  community labeling, and benchmarking.
- Replaced `tools/graphify_map_repo.sh` with the executable Python 3.12 tool
  `tools/graphify_map_repo.py`. It automatically extracts a missing graph or runs the real
  `graphify update .` path for an existing graph. Fresh extraction labels and benchmarks before
  manager-context generation; ordinary updates regenerate manager context immediately.
- Restored explicit Graphify lifecycle controls as `-F`/`--fresh`, `-U`/`--update`, and
  `-C`/`--context` while retaining automatic fresh-or-update selection with no flag. Update mode
  prominently announces and performs a fresh extraction when no graph exists; context prints CLI
  help before the first map exists. Expanded `-h`/`--help` with the complete pipeline,
  requirements, output location, and runnable examples.
- Changed Graphify community labeling to use `claude-cli` by default without an API key. Added
  `-O`/`--ollama` as the explicit local-backend override, retaining the configured model and the
  required Ollama package extra.
- Set fresh Claude CLI labeling to pass `--model=sonnet` explicitly. Graphify community naming now
  uses Sonnet independently of the interactive Claude default, while the Ollama override retains
  its configured local model.
- Fresh builds upgrade `graphifyy[ollama,sql,terraform]`; Ollama-selected fresh builds also pull
  the configured model. Update and context modes perform no package or model setup.
- Made the Graphify pip upgrade quiet and disabled pip's already-unusable local cache. Fresh builds
  no longer print the complete satisfied-dependency inventory or cache-permission warning, while
  installation errors remain visible.
- Replaced the redundant automatic `label --missing-only` update phase with a true fast path:
  existing graphs now run only `graphify update .` before manager-context regeneration. Full
  semantic labeling remains part of fresh extraction instead of a separate relabel lifecycle.
- Limited package upgrades, full labeling, and benchmarking to fresh extraction.
- Replaced generic artifact and policy output with repository-specific manager orientation derived
  from `graphify-out/graph.json`. Context now names the repository, map size, primary domain
  subsystems, highly connected code with source locations, cross-subsystem bridges, and copyable
  queries grounded in the active map. It omits `Corpus Check`, `.graphifyignore` exclusions, and
  generated-file hygiene; the complete Graphify diagnostics remain in `GRAPH_REPORT.md`.
- Capped each cross-area connector at eight displayed community names and appended `and N more` for
  the remainder. This bounds manager-context size without semantically filtering or reordering
  Graphify's connector evidence.
- Strengthened the `Prompt positively` repository principle: lead with the desired action or tool,
  omit irrelevant unwanted actions, and reserve explicit prohibitions for necessary safety or
  correctness boundaries.

### Removals and Deprecations

- Removed the shell tool's positional mode syntax. The Python tool exposes the same lifecycle
  choices as mutually exclusive flags.
- Removed the intermediate `-R`/`--relabel` mode. Fresh extraction is the single intentional route
  for full semantic labeling because labeling already dominates the fresh-build cost.
- Added `meta/propagation/deprecated_paths.txt` so propagation removes the retired
  `tools/graphify_map_repo.sh` path from consumer repositories after shipping the Python tool.

### Decisions and Failures

- The first sandboxed Graphify extraction failed with `Operation not permitted` when Graphify
  started its AST workers. The same command completed outside the sandbox; this is an execution
  permission requirement, not a repository parsing failure.
- The `attack-on-cancer` trial has no Graphify ignore policy. Its final orientation correctly
  warned that generated graph files are visible to Git instead of silently presenting a clean
  repository state.
- Graphify 0.9.49 source confirms that `update` replaces stale names with hub-derived labels and
  that `label --missing-only` treats those names as present. The old incremental label phase could
  not improve them, but still repeated clustering, analysis, and report/JSON/HTML generation.
- Benchmark traverses the graph to measure token reduction but does not improve agent-facing graph
  data. It is therefore outside the routine update path.
- A 19,334-node fresh run inherited the interactive Opus default and exhausted the shared Claude
  session allowance while labeling 711 communities. Graphify continued with fallback names for
  failed batches, so the next intentional fresh build should run after the allowance resets with
  the explicit Sonnet model.
- Sonnet is the conservative default for fresh community labeling because semantic label quality
  matters more than the incremental Haiku savings on an occasional fresh build. Haiku remains a
  one-time comparison candidate on a representative large repository.

### Developer Tests and Notes

- Added focused behavior tests for fresh/update command selection, generated artifact inventory,
  concise orientation output, universal tool routing, and traversal-safe deprecated-path cleanup.
- Exercised real fresh and update lifecycles in this template and `attack-on-cancer`, plus real
  updates in `peptidyle-learning-engine` and `ferrum-chemical-forge`. Final maps ranged from 371
  nodes in this template to 19,047 nodes in Ferrum; every run produced the required report and
  graph artifacts and reached the concise orientation.
- An earlier pre-mode validation snapshot passed all 1,751 collected tests plus focused pyflakes,
  typing, indentation, shebang, ASCII, import-requirement, Bandit, source-size, CLI-help,
  rejected-mode, and `git diff --check` validation.
- After the repository-specific context revision, all 18 focused Graphify behavior cases, direct
  `--context` runs against this template and `attack-on-cancer`, `git diff --check`, and the
  complete 1,778-test suite pass.
- A fresh six-pass independent audit of the mode and help revision identified and corrected the
  incomplete-output context boundary, stale documentation wording, and missing alias/fallback
  coverage before final validation.
- The default Claude CLI update path completed against this repository in 1.9 seconds. Graphify
  found no topology changes, the pre-update and post-update label and analysis sidecars had
  identical hashes, and the workflow regenerated reports, benchmarked, and wrote manager context.
- All 27 focused Graphify behavior tests and the complete 1,787-test suite pass after adding the
  Claude CLI default, Ollama override, and missing-only update lifecycle.
- A connected quiet-mode update completed in 2.2 seconds. Its package phase printed only the
  prominent phase heading before continuing to Graphify update, with no dependency inventory or
  cache warning.
- The final connected fast update completed in 0.6 seconds and ran only `graphify update .` before
  regenerating `MANAGER_CONTEXT.md`. It performed no pip, label-backend, labeling, or benchmark
  phase. Fresh extraction remains the intentional route for replacing degraded community labels.
- A permanent-test policy audit retained the offline command-selection, explicit-mode, fresh-label,
  and Ollama behavior cases, and removed two redundant parser-default checks. The connected runs,
  timing, pip-output probes, installed-source inspection, and help/context executions remain
  one-time implementation evidence instead of permanent pytest cases. All 27 focused lifecycle
  tests and the complete 1,787-test suite pass after the audit.
- After removing the intermediate relabel mode and bounding connector output, all 26 focused
  Graphify behavior tests and the complete 1,786-test suite pass. The direct help check exposes
  only fresh, update, context, and the Ollama backend override.
- After explicitly selecting Sonnet for fresh Claude CLI labels, all 26 focused Graphify behavior
  tests and the complete 1,786-test suite pass. The direct help check identifies Sonnet as the
  Claude label model while retaining the Ollama override.

## 2026-08-21

### Fixes and Maintenance

- Added a vendored-file header to every propagated `test_*.py` file. It warns that local changes
  can and will be overwritten, without identifying or linking an upstream source location.

## 2026-08-20

### Additions and New Features

- Added a universal `.graphifyignore` seed that excludes `tests/`, `devel/`, `tools/`, and
  `docs/` from Graphify repository maps. It propagates to every repo only when absent,
  preserving repository-specific additions after bootstrap.

### Fixes and Maintenance

- Added `/graphify-out/` to `templates/gitignore.universal`, the canonical source for the
  propagation-managed `UNIVERSAL` `.gitignore` block. This preserves Graphify output ignores
  across future propagation runs.
- Added the `pytestqt` to `pytest-qt` import-distribution alias to
  `tests/test_import_requirements.py`.
- Added canonical import-distribution aliases for `applefoundationmodels`, `bricklink`,
  `exiftool`, `graphify`, `markdown_it`, `material`, `screencapturekit`, and `skimage`.
- Added the canonical `graphifyy[ollama,sql,terraform]` PyPI development requirement. This installs
  Graphify's complete base dependency set plus the Ollama backend used by
  `tools/graphify_map_repo.sh`, the `tree-sitter-sql` parser needed for authored database schemas,
  and the `tree-sitter-hcl` parser needed for Terraform repositories, without unrelated optional
  integrations. This keeps the dependency inventory explicit under ASVS 15.1.2 and 15.2.4.
- Corrected the stale propagation test that still classified root `tools/` as template metadata.
  Root tools are universal consumer tools under the current location-based routing policy.

### Developer Tests and Notes

- Confirmed from the `graphifyy` 0.9.48 wheel metadata that the package requires Python 3.10+,
  its `ollama` extra adds the `openai` client, its `sql` extra adds `tree-sitter-sql`, and its
  `terraform` extra adds `tree-sitter-hcl`.
- Confirmed the native Graphify ignore matcher excludes all four universal paths and the
  propagation plan routes `.graphifyignore` to `noexist_files` for every declared repo type.
- The complete pytest suite passes with 1723 tests. Direct ASCII checks and `git diff --check`
  also pass for the changed files.

## 2026-08-19

### Behavior or Interface Changes

- Updated `tools/graphify_map_repo.sh context` output to be strictly Graphify-focused.
  The context now defines what Graphify is, the key commands for query/explain/affected/path,
  and a manager delegation template that uses Graphify evidence to minimize prompt/context
  size when assigning subagent tasks.

## 2026-08-12

### Behavior or Interface Changes

- `templates/rust/docs/RUST_STYLE.md`: defined a security-focused Cargo version policy. New direct
  dependencies use the manager-selected repository form: wildcard `*` for every stable version,
  or `>=LATEST` for an explicit security floor with newer versions eligible. Dependency refreshes
  advance major, minor, and patch components. The guide treats application repositories as the
  normal case, keeps `Cargo.lock` as their exact tested graph, records the future crates.io wildcard
  constraint, and reserves exact requirements for documented temporary constraints.
- Set the Rust toolchain policy to the latest stable compiler with the current manifest floor
  written as `rust-version = "1.97.1"`, matching the installed
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Documented that Cargo requires a bare version, so
  `rust-version = ">=1.97.1"` is invalid, and that this MSRV field does not update dependencies.

### Fixes and Maintenance

- Reformatted the Rust examples in `templates/rust/docs/RUST_STYLE.md` with rustfmt's four-space
  indentation instead of tabs, aligning the examples with the guide's own formatting rule.
- Rephrased Rust directives around the desired implementation behavior: explicit imports, safe
  Rust boundaries, concrete library errors, lifetime elision, and behavior-focused tests.
- Baked the repository's source-file ceiling into `templates/rust/docs/RUST_STYLE.md`: use 999
  physical lines as the inclusive maximum and keep generic crate roots, module roots, and test
  indexes as concise routing stubs for descriptively named implementation files.
- Applied the six-pass audit corrections: added a canonical Rust filename map, identified
  `docs.rs` as the hosted documentation service, directed reusable binary behavior into focused
  library modules, reserved generic Cargo filenames for thin entry stubs, made `Cargo.toml` the
  direct source for the selected dependency form, and expressed the quick-start and public-
  documentation rules as positive implementation guidance.
- Condensed the new toolchain and dependency policy to its executable contract: current compiler,
  latest stable direct dependencies, the manager-selected `*` or `>=LATEST` form, lockfile refresh,
  and Cargo gates.
- Aligned the guide with the `rust-code-expert` reference workflow. Existing-project work now names
  the owning crate and module, callers, features, target triple, error contract, and value flow;
  greenfield work starts with domain types plus one success and error test. The completion baseline
  now includes `cargo fmt --check`, `cargo check`, `cargo test`, and Clippy, followed by the matching
  CLI, Tokio, unsafe/FFI, PyO3, or performance oracle.
- Completed a whole-document Rust review against the skill references and current official sources.
  Corrected modern module paths and the private-module `pub use` example; narrowed Python-style
  carryover to repo philosophies; made `Result`, panic, library/application errors, and CLI testing
  precise; added enum/trait/generic selection and explicit async task ownership; updated unsafe for
  2024 extern blocks and unsafe attributes; and added narrow FFI adapters with links to the focused
  Python and WebAssembly guides at their relevant boundaries.
- Split Python binding guidance into `templates/rust/docs/RUST_PYO3_STYLE.md` so
  `RUST_STYLE.md` stays focused on Rust. The new guide owns PyO3 boundary architecture, extension
  and embedding shapes, `cdylib`/`rlib`, current maturin linking, ABI selection, interpreter-bound
  values, domain-error to Python-exception translation, task ownership, and Python integration
  proof. The Rust guide retains the general FFI contract and links to the focused guide.
- Added `templates/rust/docs/RUST_WASM_STYLE.md` for the browser and WASI boundary. It covers
  target and tool selection, a Rust-core/thin-export architecture, `wasm-bindgen` API design,
  JavaScript ownership and error translation, browser and WASI validation, size and performance
  measurement, and current project-owned wasm-bindgen references. `RUST_STYLE.md` links exactly
  twice to each focused guide: at the general FFI boundary and at the foreign-caller proof point.
- Rotated `docs/CHANGELOG.md` after it crossed 1000 lines. Kept the two newest day blocks active
  and moved the 2026-08-07 through 2026-06-30 blocks into `docs/CHANGELOG-2026-08a.md`.

### Developer Tests and Notes

- Confirmed the documented compiler floor against
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- After the six-pass audit corrections, Markdown links, ASCII compliance, whitespace, and the
  source-file line limit pass: 501 targeted tests. The complete `pytest tests/` suite passes with
  1717 tests.

## 2026-08-10

### Additions and New Features

- Added the universal `tests/test_source_file_line_limit.py` hygiene gate. It scans Git-tracked
  authored source files through `file_utils.discover_files`, accepts 999 physical lines, fails at
  1000, and writes the standard complete violation report. Scope includes common programming,
  build, query, template, and authored-document formats (including `.md`) plus conventional names
  such as `Makefile` and `Dockerfile`; generic `.txt`, data, config, generated, notebook, and binary
  formats remain outside the gate.
- Added the optional manager-owned `tests/source_file_line_limit_overrides.txt` contract for exact
  paths to tracked sources outside local control, such as a downloaded normative specification.
  Blank lines and full-line comments are accepted; globs and paths escaping the repo are rejected.
  The propagation manifest marks the file as template-meta so one repo's approvals never ship to
  another repo.

### Behavior or Interface Changes

- `devel/bump_version.py patch` now prepares the next patch release. It treats repo versions such
  as `26.08` and Cargo versions such as `26.8.0` as the same release, previews the affected files,
  and uses plain `Current version` and `Next version` labels.
- Shortened source file line-limit report entries to `path: N lines`; the report header carries
  the shared policy context once.
- Promoted PyPI packaging from the file-presence-driven `templates/python/_pypi/` conditional
  overlay to the real `pypi` repo type under `templates/pypi/`. The inheritance declaration
  `pypi: python` gives packages the complete Python rule set plus publishing-specific files.
  Declaring `python` selects Python tooling; declaring `pypi` adds publishing files.
  Legacy reset configs using `project_type: python` with `pypi: true` normalize to the canonical
  `pypi` marker.

### Fixes and Maintenance

- Split repository classification from repository style: `meta/docs/REPO_TYPE.md` now owns
  marker format, names, inheritance, and multi-type behavior, while `docs/REPO_STYLE.md`
  contains no type-marker contract. Updated human-guidance and propagation references.
- Recorded plan-gate guidance: ground exactness and performance requirements in measured product
  contracts, separate one-time implementation probes from permanent tests, and apply the
  permanent-test checklist before adding suite coverage.
- `tests/test_shebangs.py` now treats exact `#!perl` lines in `.conf` files as WeBWorK
  configuration markers rather than executable shebangs, covered with inline `tmp_path` input.
- Added template-local pytest import paths so `pytest tests/` resolves the template's helper
  modules without adding `PYTHONPATH` or custom commands to the downstream `source_me.sh` seed.
- Removed the fragile reset self-propagation pytest; the whole reset workflow remains in the
  clone-based E2E runner.
- Removed the unused `repolib.files.safe_walk` helper and corrected release-routing documentation.
- Condensed the source-file-size rules to the boundary, scope owner, and override path.
- Reduced `reset_repo.py` to an executable CLI stub. Filesystem mutation and orchestration now
  live in `repolib/reset.py`, while interview and JSON-answer resolution live in
  `repolib/reset_answers.py`.
- Split propagation planning from file mutation: `repolib/files.py` retains file and merge
  operations, and `repolib/plan.py` owns plan construction, typed overlays, and source buckets.
- Split the version command into the small `devel/bump_version.py` CLI,
  `devel/version_lib.py` for shared version behavior, and `devel/version_files.py` for repository
  discovery and updates. `make_release.py` and the PyPI publisher now use the same version library.
- Split PyPI authentication/repository resolution and console/subprocess helpers into
  `pypi_auth.py` and `pypi_support.py`. All three files propagate together into a package repo's
  `devel/` directory and use normal sibling imports; template tests provide those source overlay
  paths through a test-only `PYTHONPATH`.
- Extracted shared overlay routing into `repolib/plan.py`.
- Extracted reset cleanup and completion phases into focused helpers.
- Clarified reset answer parsing and aligned the extracted helpers with repository style.

### Decisions and Failures

- Untracked `local-only/` reference books need no path exception because hygiene discovery already
  uses `git ls-files`. The source selector deliberately does not exclude that directory, preventing
  it from becoming a loophole for tracked oversized code.

### Developer Tests and Notes

- Added an explicit boundary case proving that 999 lines passes and 1000 lines fails.
- Added an inline `tmp_path` behavior case proving the optional override list loads an exact path
  while ignoring comments and blank lines.
- The boundary cases pass, and direct pyflakes validation is clean. The complete new gate reports
  four existing source debts instead of weakening the rule: `devel/bump_version.py` (1329),
  `repolib/files.py` (1332), `reset_repo.py` (1126), and
  `templates/python/_pypi/devel/submit_to_pypi.py` (1349).
- Resolved all four debts above without overrides. The largest replacement module is now
  `templates/pypi/devel/submit_to_pypi.py` at 988 lines. The complete pytest suite passes with
  1727 tests and 2 environment-dependent skips; the standalone real-Git release E2E also passes.
- Exported the staged index into an isolated temporary Git repository and completed a live `pypi`
  reset. The canonical marker, Python inheritance, PyPI support trio, shared version modules, and
  source-release helper were present afterward; template/repolib/reset infrastructure was removed.
