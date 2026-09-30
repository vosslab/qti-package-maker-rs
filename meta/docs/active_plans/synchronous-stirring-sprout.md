# Plan: Clear the starter-repo-template TODO backlog

## Context

[meta/docs/TODO.md](../TODO.md) holds six unrelated backlog
items that share one root cause: the template's contracts are enforced by convention rather than by
code, so agents working in this repo and its consumers guess wrong. Concretely, today:

- `.gitignore` renders UNIVERSAL, then the consumer-owned LOCAL block, then typed blocks. That
  middle position is mechanical, not designed (`repolib/files.py:801-899` writes UNIVERSAL, calls
  `ensure_gitignore_local_section()` at `repolib/files.py:508-539`, then appends typed blocks). No
  test asserts block order, so nothing holds the layout in place.
- `pytest.ini` survives `reset_repo.py`. It sits in the `meta_files` propagation bucket
  (`meta/propagation/manifests.yaml:113`), which only blocks shipping *to* consumers; the reset
  deletion list `clean_template_files()` (`repolib/reset.py:740-759`) never removes it, and
  `TEMPLATE_META_PATHS` (`tests/meta/e2e/e2e_reset_routing.py:84-90`) does not check it. Its content
  is five `pythonpath` entries, three of which (`meta/tools`, `templates/pypi/devel`, and `devel`)
  exist only for the template's own meta suite.
- Reset asks separate stage and commit questions, both defaulting to No
  (`repolib/reset_answers.py:245-261`), and never pushes (`complete_reset()`,
  `repolib/reset.py:783-828`). Agents routinely stop with a half-finished bootstrap, which is the
  specific failure this plan is asked to remove.
- `tests/test_checkout_disk_budget.py` shells out to `du` and so reads as an E2E candidate to every
  fresh agent. It is vendored (footer line 33) and returns after deletion, wasting the argument
  every time.
- `tools/` has no README and no import gate. An initial targeted survey under `~/nsh/` found four
  plan-named test importers: `populous-python-nvl` (`tests/test_click_hits_visible_tile.py:20`),
  `iptv-filters` (`tests/test_validate_m3u.py:10`), `marp-slides`
  (`tests/test_pptx_to_marp.py:16`), `track-runner-virtual-dolly-cam` (`tests/ui/test_keymap.py:13`).
  This sample establishes known violations; WP-8a owns the complete
  static survey, including runtime imports. `devel/DEVEL_README.md:15-17` already states the
  equivalent rule for `devel/`; `tools/` has no counterpart.
- Nothing caps root-level script clutter. The template itself is clean (3 root `.py`/`.sh` files,
  2 with the exec bit), which makes now the right time to install the gate.

Intended outcome: each item becomes an enforced, documented contract, verified by tests that run
offline with no human in the loop.

## Objectives

- Render `.gitignore` with every propagated block first and the consumer-owned block last, under a
  banner that cannot be mistaken for vendored content.
- Delete `pytest.ini` during reset and prove the reset consumer's suite still resolves its imports.
- Make `reset_repo.py` finish a bootstrap end to end -- stage, commit, and push the reset consumer
  repository -- behind one default-Yes confirmation, with partial completion reported unmistakably.
- Document `tests/test_checkout_disk_budget.py` as a deliberate base-lane pytest that returns after
  deletion, and record its exemption where the style rule lives.
- Ship `tools/TOOLS_README.md` and a pytest enforcing the import contract: no `tools.*`, `devel.*`,
  or `tests.*` package-root imports anywhere; no sibling imports inside `tools/`; documented flat
  sibling helpers allowed inside `devel/`; flat test helpers allowed inside `tests/` only.
- Cap root scripts: fail at 7 or more, report a warning band at 5 or 6.
- Leave every milestone completable by a manager and subagents with no network and no human input.

## Design philosophy

Fix the design, not the symptom (`docs/REPO_STYLE.md` core philosophies). Every item here has an
existing workaround that relies on an agent remembering a rule; each milestone replaces the
reminder with an executable contract plus the doc that owns it. The trade-off accepted: several
milestones add a vendored test that will fail in consumer repos on their next propagation. That is
intended -- a failing gate in a repo with misplaced code is the point -- and the migration recipe
ships in the same commit as the gate.

Two repositories, two Git policies. Implementation agents leave **this** repository's working tree
staged and uncommitted for the human. `reset_repo.py` is a product whose job is to finish
bootstrapping a **different** repository -- the reset consumer -- and staging, committing, and
pushing that consumer is the deliverable behavior, not an agent commit. Every mention of commit or
push below refers to the consumer unless it names this template.

Rejected alternative: a single "template hardening" milestone touching all six areas. It hides six
independent design decisions behind one review and one rollback boundary.

Contract versus implementation: each work package's acceptance criteria state the behavior that must
hold. Named files, helpers, and scaffolds are the recommended route derived from existing repository
evidence; a doer who finds a better-fitting implementation may use it, provided the stated behavior
and the repository style guides still hold.

The support-directory boundary is a settled architectural rule, not a question the audit decides:
`tools/` and `devel/` hold standalone entry-point scripts, `tests/` holds tests and test-only
support, and none of the three is an importable library namespace. Reusable code belongs in a proper
package namespace, and a `tools/` script should be a thin entry point that imports from there. The
enforced contract is deliberately asymmetric: package-root imports of all three are banned
everywhere; `tools/` scripts stand alone with no sibling imports; `devel/` keeps a documented flat
sibling-helper exception for its vendored tooling; and `tests/` may share flat helpers internally
because `tests/conftest.py` and pytest already arrange that directory. The gate stops there. A
non-test file doing `import file_utils` needs no rule -- `conftest.py` shapes only pytest execution,
so that script has no `tests/` on its path and fails by itself.
This repository's reach stops at detection -- it ships a gate that flags violations so consumer
maintainers see them; the remediation happens in each consumer repository, by its maintainer.

- Evidence strategy for uncertain methods: two questions stay open at dispatch and are answered by
  read-only investigation inside their own work packages -- what the root-script counting rule
  should include (WP-11a), and whether the import gate is already green on this template (WP-10a).
  Each names its decision rule below.

## Scope

- Reorder and re-banner the rendered `.gitignore` block sequence, with order and idempotency tests.
- Remove `pytest.ini` during reset; extend the reset E2E harness to assert its absence and to prove
  the reset consumer's test imports still resolve.
- Replace the reset stage and commit prompts with one default-Yes finish prompt covering stage,
  commit, and push of the reset consumer; add a `push` config key; add remote preflight; define
  explicit terminal states so a partial finish is machine-distinguishable.
- Build an offline synthetic-remote E2E case (local bare repository as `origin`) proving push works.
- Add a module docstring and style-guide exemption for `tests/test_checkout_disk_budget.py`.
- Write the `tools/` misuse audit to `meta/docs/active_plans/audit/`.
- Author `tools/TOOLS_README.md`, cross-linked with `devel/DEVEL_README.md`.
- Add `tests/test_support_dirs_not_imported.py` and `tests/test_root_script_budget.py`.
- Update `meta/docs/GITIGNORE_SYSTEM.md`, `docs/PYTEST_STYLE.md`, `docs/REPO_STYLE.md`,
  `tests/TESTS_README.md`, `meta/docs/HUMAN_GUIDANCE.md`, and `docs/CHANGELOG.md`.

## Non-goals

- Do not commit or push **this** repository. Agents stage changes and update `docs/CHANGELOG.md`;
  the human commits. This is separate from the consumer-facing reset behavior being built.
- Do not modify consumer repos. The audit and the shipped gate report violations; fixing
  `populous-python-nvl`, `iptv-filters`, `marp-slides`, and `track-runner-virtual-dolly-cam` is
  their maintainers' work, in their repositories.
- Do not run propagation against a real consumer repository. Propagation verification uses a
  disposable clone under `/tmp` only.
- Do not push anything to GitHub, from this repository or from an E2E clone. Push verification uses
  a local bare repository as `origin`.
- Do not run the REMOTE mode of `tests/meta/e2e/e2e_reset_routing.py`; it needs pushed history.
- Do not add reset automation flags beyond the `push` config key.
- Do not relax the fast-lane pytest budget for anything other than the one named disk-budget
  exemption.
- Do not add a permanent test for the vendored footer comment. Investigation found the string
  `# Vendored pytest file. Local changes can and will be overwritten.` appears nowhere in
  `repolib/` or `meta/`: it is explanatory prose, not propagation metadata, so a gate on it would
  test a comment rather than behavior.

## Current state summary

| Area | Owning code | Owning doc | Gate today |
| --- | --- | --- | --- |
| Gitignore render | `repolib/files.py:508,543,801` | `meta/docs/GITIGNORE_SYSTEM.md` | content and idempotency only, no order |
| Reset cleanup | `repolib/reset.py:740-759` | `meta/docs/HUMAN_GUIDANCE.md` | `TEMPLATE_META_PATHS` list, no `pytest.ini` |
| Reset finish | `repolib/reset.py:783-828`, `repolib/reset_answers.py:245-261` | `meta/docs/HUMAN_GUIDANCE.md` | `assert_commit_state`, `assert_stage_state` |
| Disk budget test | `tests/test_checkout_disk_budget.py` | none | none |
| tools/ boundary | none | none (devel/ has one) | none |
| Root clutter | none | none | none |

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1, M2 / WS-A gitignore | `repolib/files.py`, `tests/meta/test_repolib_multi_type.py`, `meta/docs/GITIGNORE_SYSTEM.md`, `.gitignore` | render order + idempotency |
| M3-M6 / WS-B reset | `repolib/reset.py`, `repolib/reset_answers.py`, `tests/meta/test_reset_config.py`, `tests/meta/e2e/e2e_reset_routing.py` | bootstrap completion contract |
| M7 / WS-C test policy | `tests/test_checkout_disk_budget.py`, `docs/PYTEST_STYLE.md` | fast-lane policy |
| M8-M10 / WS-D support-dir boundary | `meta/docs/active_plans/audit/`, `tools/TOOLS_README.md`, `devel/DEVEL_README.md`, `tests/test_support_dirs_not_imported.py` | folder-purpose contract |
| M11 / WS-E root budget | `tests/test_root_script_budget.py`, `docs/REPO_STYLE.md` | repo-root hygiene |
| M12 / WS-F close-out | `tests/TESTS_README.md`, `meta/docs/HUMAN_GUIDANCE.md`, `docs/CHANGELOG.md` | shared-doc single owner |

WS-A, WS-B, WS-C, WS-D, and WS-E touch disjoint source files and may run concurrently, with two
shared documents given single owners so the claim holds: WS-F owns every edit to
`tests/TESTS_README.md`, and WS-E owns every edit to `docs/REPO_STYLE.md`. WS-D hands WS-E its one
support-directory sentence rather than editing that file itself; WS-E lands both text blocks in one
pass, so M10's README reference and M11's policy section never race.

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Gitignore order | Move LOCAL block last in the rendered sequence | Vendored blocks read first, consumer block last |
| M2 | Gitignore banner and docs | Loud LOCAL banner, doc + root file refresh | Ownership is obvious on sight |
| M3 | Reset removes pytest.ini | Add the `git_rm` and the E2E assertions | No `pytest.ini` in a fresh consumer, imports still resolve |
| M4 | Reset finish prompt | One default-Yes prompt, `push` config key | Answer layer knows about push |
| M5 | Reset finish execution | Preflight, push, explicit terminal states | Partial finishes are unmistakable |
| M6 | Reset push E2E | Local bare remote fixture case | Push proven offline, no human |
| M7 | Disk budget doc | Module docstring + `docs/PYTEST_STYLE.md` exemption | The argument stops recurring |
| M8 | tools/ violation audit | Which repos violate the rule, and what must move | Violations recorded, migration direction named |
| M9 | TOOLS_README | Ship `tools/TOOLS_README.md`, cross-link devel | `tools/` states its own rule |
| M10 | Import-boundary pytest | Ban module-root imports, plus siblings in `tools/`/`devel/` | Rule is enforced, not advisory |
| M11 | Root script budget | Fail at 7+, report at 5-6 | Root clutter capped |
| M12 | Close-out | Shared docs, changelog, integration gate | Working tree complete and green |

### Milestone: M1 gitignore render order

- Depends on: none.
- Deliverables: `.gitignore` rendering emits UNIVERSAL, then each typed block, then LOCAL last; an
  existing mid-file LOCAL section is relocated to the end with its body lines intact; order and
  relocation tests.
- Workstreams: WS-A.
- Entry criteria: none.
- Exit criteria: `source source_me.sh && pytest tests/meta/test_repolib_multi_type.py tests/meta/test_repolib_helpers.py
  tests/meta/test_load_deprecation_lists.py -q` passes; a second render of the same file is
  byte-identical.
- Parallel-plan ready: yes.

### Milestone: M2 gitignore banner and documentation

- Depends on: M1 (the banner ships with the block that moved).
- Deliverables: the approved LOCAL banner; `meta/docs/GITIGNORE_SYSTEM.md` rendered-ownership and
  processing-pipeline sections updated to state the new order; this repository's root `.gitignore`
  re-rendered to match.
- Workstreams: WS-A.
- Entry criteria: M1 exit criteria met.
- Exit criteria: `git check-ignore -v --no-index output_smoke/probe.txt graphify-out/probe.json`
  resolves the same owning rules as before the change; `source source_me.sh && pytest tests/test_markdown_links.py
  tests/test_ascii_compliance.py -q` passes.
- Parallel-plan ready: yes.

### Milestone: M3 reset removes pytest.ini

- Depends on: none.
- Deliverables: reset deletes root `pytest.ini` from the consumer; the reset E2E harness asserts its
  absence and asserts the consumer's test suite still resolves imports without it.
- Workstreams: WS-B.
- Entry criteria: none.
- Exit criteria: `bash tests/meta/e2e/run_all.sh` passes in LOCAL mode including the new assertions.
- Parallel-plan ready: yes.

### Milestone: M4 reset finish prompt and config key

- Depends on: M3 (same files; keeps the reset lane serial).
- Deliverables: one default-Yes prompt replaces the two default-No prompts and sets stage, commit,
  and push together; `ResetAnswers` and the `--config` reader gain `push`; the plan summary prints
  push intent; unit coverage in `tests/meta/test_reset_config.py`.
- Workstreams: WS-B.
- Entry criteria: M3 merged into the lane branch.
- Exit criteria: `source source_me.sh && pytest tests/meta/test_reset_config.py -q` passes, covering the default-Yes path,
  the explicit-no path, and `push` absent from a config file.
- Parallel-plan ready: no -- serial edits to `repolib/reset*.py` shared with M5.

### Milestone: M5 reset finish execution and terminal states

- Depends on: M4 (consumes the `push` answer).
- Deliverables: remote presence resolved during preflight, before any mutation; push of the reset
  consumer performed when requested; four explicit, distinguishable terminal outcomes reported
  (published, publication not requested, requested with no remote, push attempted and failed).
- Workstreams: WS-B.
- Entry criteria: M4 exit criteria met.
- Exit criteria: the excluded reset E2E harness covers every outcome using disposable `/tmp` Git
  repositories; no reachable answer combination lands in an outcome that misdescribes it; every
  blocking precondition runs before mutation, and no post-mutation condition aborts the run.
- Parallel-plan ready: no -- shares `repolib/reset.py` with M4.

### Milestone: M6 reset push E2E with a synthetic remote

- Depends on: M5.
- Deliverables: an E2E case that stands up a local bare repository as `origin`, runs reset with push
  requested, and proves the reset commit reached that remote.
- Workstreams: WS-B.
- Entry criteria: M5 exit criteria met.
- Exit criteria: `bash tests/meta/e2e/run_all.sh` passes offline; the case contacts no network host
  and leaves no artifacts behind.
- Parallel-plan ready: yes.

### Milestone: M7 disk budget test documentation

- Depends on: none.
- Deliverables: a module docstring in `tests/test_checkout_disk_budget.py` stating that the test is
  a deliberate base-lane pytest run by `source source_me.sh && pytest tests/`, why the `du`
  subprocess is accepted, and that the file is vendored -- deleting it locally only removes it until
  the next propagation restores it; a named exemption in `docs/PYTEST_STYLE.md` citing the file so
  future reviews stop relitigating it.
- Workstreams: WS-C.
- Entry criteria: none.
- Exit criteria: `source source_me.sh && pytest tests/test_checkout_disk_budget.py tests/test_markdown_links.py
  tests/test_ascii_compliance.py tests/test_function_typing.py -q` passes.
- Parallel-plan ready: yes.

### Milestone: M8 tools/ violation audit

- Depends on: none.
- Deliverables: an audit report under `meta/docs/active_plans/audit/` naming which surveyed repos
  violate the settled no-import rule, with the importing `file:line` and, for each violation, which
  module would have to move into an importable package for the consumer maintainer to resolve it.
- Workstreams: WS-D.
- Entry criteria: none.
- Exit criteria: every verdict carries a citation; each of the four known violators has a named
  migration direction; no repository other than this one is modified.
- Parallel-plan ready: yes.

### Milestone: M9 TOOLS_README and cross-references

- Depends on: M8 (the audit's violation shapes become the README's migration section).
- Deliverables: `tools/TOOLS_README.md` stating the folder's purpose, the no-import rule covering
  `tools/`, `devel/`, and `tests/`, and where reusable code belongs instead; a reciprocal link in
  `devel/DEVEL_README.md`; one support-directory sentence handed to WS-E for the
  `docs/REPO_STYLE.md` repository-structure section.
- Workstreams: WS-D.
- Entry criteria: M8 exit criteria met.
- Exit criteria: `source source_me.sh && pytest tests/test_markdown_links.py tests/test_ascii_compliance.py
  tests/test_whitespace.py -q` passes; a propagation dry run against a disposable `/tmp` clone plans
  `tools/TOOLS_README.md` for every repo type.
- Parallel-plan ready: yes.

### Milestone: M10 import-boundary pytest

- Depends on: M9 (the failure message points at `tools/TOOLS_README.md`).
- Deliverables: a vendored pytest enforcing the import contract -- no `tools.*`, `devel.*`, or
  `tests.*` package-root imports anywhere; no sibling imports inside `tools/`; documented flat
  sibling helpers allowed inside `devel/`; flat test-helper imports allowed inside `tests/`.
- Workstreams: WS-D.
- Entry criteria: M9 exit criteria met.
- Exit criteria: the gate is green on this template; synthetic `tmp_path` trees prove each rule --
  `import tools.foo`, `from devel import bar`, and `from tests.helpers import baz` fail R1 wherever
  the importer sits; a `tools/a.py` containing `import b` fails R2 when `tools/b.py` exists and
  passes when it does not; a `devel/` script importing a flat sibling helper passes R3; a test
  importing `file_utils` passes R4. The four audited consumer
  repos are named in the README
  migration section and left unmodified. Detection is the whole deliverable -- remediation belongs
  to each consumer's maintainer.
- Parallel-plan ready: yes.

### Milestone: M11 root script budget

- Depends on: none.
- Deliverables: a root-script-budget policy section in `docs/REPO_STYLE.md` stating the counting
  rule and both thresholds; a vendored pytest enforcing them.
- Workstreams: WS-E.
- Entry criteria: none.
- Exit criteria: the gate is green on this template; synthetic roots below, inside, and above the
  band produce silence, a report, and a failure respectively.
- Parallel-plan ready: yes.

### Milestone: M12 close-out

- Depends on: M2, M6, M7, M10, M11.
- Deliverables: `tests/TESTS_README.md` updated once; `meta/docs/HUMAN_GUIDANCE.md` records the
  settled decisions; `docs/CHANGELOG.md` gains one dated block in the documented subsection order;
  everything staged.
- Workstreams: WS-F.
- Entry criteria: every other milestone's exit criteria met.
- Exit criteria: the integration gate below passes and the working tree is staged. Milestone
  completion is defined as implementation complete plus gates green; the human's commit of this
  repository happens after the plan, outside its completion semantics.
- Parallel-plan ready: no -- single-owner documentation consolidation.

## Workstream breakdown

### Workstream: WS-A gitignore rendering

- Goal: vendored-first, LOCAL-last rendering with a banner and an order test.
- Owner: one `coder`, reviewed by one `reviewer`.
- Work packages: WP-1a, WP-1b, WP-2a.
- Needs: nothing from other workstreams.
- Provides: the rendered order that `meta/docs/GITIGNORE_SYSTEM.md` documents.
- Review boundary, when modifying the repository: `repolib/files.py` plus its meta tests.

### Workstream: WS-B reset completion

- Goal: reset deletes `pytest.ini` and finishes the consumer repository behind one prompt.
- Owner: one `expert_coder` (serial lane), reviewed by one `reviewer`.
- Work packages: WP-3a, WP-4a, WP-5a, WP-6a.
- Needs: nothing from other workstreams.
- Provides: the bootstrap contract recorded in `meta/docs/HUMAN_GUIDANCE.md` by WS-F.
- Review boundary, when modifying the repository: `repolib/reset*.py` plus the reset E2E harness.

### Workstream: WS-C fast-lane test policy

- Goal: the disk-budget test defends itself in prose that agents read before deleting it.
- Owner: one `tester`.
- Work packages: WP-7a.
- Needs: nothing.
- Provides: the `docs/PYTEST_STYLE.md` exemption text WS-F cites.
- Review boundary, when modifying the repository: `tests/test_checkout_disk_budget.py` and
  `docs/PYTEST_STYLE.md`.

### Workstream: WS-D support-directory boundary

- Goal: `tools/`, `devel/`, and `tests/` state and enforce their purpose as non-importable
  directories.
- Owner: one `reviewer` for the audit (WP-8a), then one `coder` for WP-9a and WP-10a.
- Work packages: WP-8a, WP-9a, WP-10a.
- Needs: nothing.
- Provides: `tools/TOOLS_README.md`, the import gate, and one support-directory sentence handed to
  WS-E for `docs/REPO_STYLE.md`.
- Review boundary, when modifying the repository: `tools/`, `devel/DEVEL_README.md`, and the new
  test. WS-D does not edit `docs/REPO_STYLE.md`.

### Workstream: WS-E root budget

- Goal: cap root scripts with a documented policy and a gate.
- Owner: one `tester`.
- Work packages: WP-11a.
- Needs: nothing.
- Provides: the policy section WS-F cites.
- Review boundary, when modifying the repository: `docs/REPO_STYLE.md` and the new test.

### Workstream: WS-F close-out

- Goal: one owner for every shared document.
- Owner: one `integrator`.
- Work packages: WP-12a.
- Needs: completion notes from WS-A through WS-E.
- Provides: staged changes and the changelog block.
- Review boundary, when modifying the repository: shared docs only.

## Work packages

### Work package: WP-1a relocate the LOCAL block to the end

- Owner: WS-A coder.
- Touch points: `repolib/files.py:508-539,801-899` (recommended site:
  `ensure_gitignore_local_section` plus `merge_gitignore_blocks`).
- Depends on: none.
- Acceptance criteria (behavior): after rendering, the LOCAL section is the last section in the
  file, every propagated block precedes it, and typed blocks keep `expand_marker_types()` order.
  Inputs with LOCAL at the top, in the middle, or absent all converge to that layout. A legacy
  `# === LOCAL ===` header is renamed and relocated in the same pass. Local body lines survive
  verbatim and in their original relative order. A second render of the result is byte-identical.
- Evidence or review, when useful: `reviewer` confirms no consumer LOCAL body line is dropped.
- Obvious follow-ons: WP-1b.

### Work package: WP-1b order and idempotency tests

- Owner: WS-A coder.
- Touch points: `tests/meta/test_repolib_multi_type.py:121-210` (existing block tests live here).
- Depends on: WP-1a.
- Acceptance criteria (behavior): tests fail if LOCAL precedes any propagated block, if relocation
  loses or reorders local body lines, or if a repeat render differs. Expectations derive from live
  constants (`repolib.gitignore.managed_gitignore_header`, `repolib.model.expand_marker_types`) rather
  than hardcoded block-name lists.
- Obvious follow-ons: WP-2a.

### Work package: WP-2a banner, doc, and root refresh

- Owner: WS-A coder.
- Touch points: `repolib/gitignore.py` LOCAL header constants, `meta/docs/GITIGNORE_SYSTEM.md:62-102`,
  root `.gitignore`.
- Depends on: WP-1b.
- Acceptance criteria (behavior): the LOCAL banner is visually distinct from the propagated headers
  and names where to add rules and that the section is preserved; the approved form is a rule line
  above and below, `ADD YOUR CUSTOM IGNORES BELOW`, and the preservation notice. The doc's rendered
  example matches what the renderer emits. The root `.gitignore` is re-rendered and
  `git check-ignore -v --no-index` reports the same owning rule for every previously covered path.
- Obvious follow-ons: hand the doc delta to WS-F.

### Work package: WP-3a delete pytest.ini during reset

- Owner: WS-B expert_coder.
- Touch points: `repolib/reset.py` and `tests/meta/e2e/e2e_reset_routing.py`.
- Depends on: none.
- Acceptance criteria (behavior): a reset consumer has no root `pytest.ini`, and its test imports
  still resolve without one. The permanent E2E assertion is a collection-only run inside the clone
  (`source source_me.sh && pytest tests/ --collect-only -q` succeeding with a non-empty collection), which is what actually
  proves import resolution; the retained template-owned `pytest.ini` supplies paths only the
  non-shipping `tests/meta/` suite needs.
  Collection does not execute function-body imports, so the one-time probe below is what rules out
  a deferred-import dependency on the removed `pythonpath` entries; if it finds one, the permanent
  assertion widens to a run that executes the affected tests.
- Evidence or review, when useful: run the clone's full `source source_me.sh && pytest tests/ -q` once during
  implementation as a one-time probe and record the result in the patch report; keep only the
  collection-only assertion permanently unless the full run catches something the probe reveals.
- Obvious follow-ons: WP-4a.

### Work package: WP-4a single finish prompt plus push config key

- Owner: WS-B expert_coder.
- Touch points: `repolib/reset_answers.py:245-301,350-391`, `repolib/reset.py:575-603`,
  `tests/meta/test_reset_config.py`.
- Depends on: WP-3a.
- Acceptance criteria (behavior): one interactive question, defaulting to Yes, decides stage,
  commit, and push together; an explicit no leaves all three off. A `--config` run reads an optional
  `push` key defaulting to false, so existing E2E configs keep today's behavior and no automated run
  contacts a network by accident. The pre-mutation summary states whether the run intends to push.
- Evidence or review, when useful: the interactive default and the config default differ
  deliberately -- the human answering the prompt has stated intent to finish the bootstrap, while an
  unattended config run must never publish something nobody asked for. Record that asymmetry in the
  patch report so it reaches `meta/docs/HUMAN_GUIDANCE.md`.
- Obvious follow-ons: WP-5a.

### Work package: WP-5a finish execution with explicit terminal states

- Owner: WS-B expert_coder.
- Touch points: `repolib/reset.py:57-110,783-828`.
- Depends on: WP-4a.
- Acceptance criteria (behavior): every blocking precondition, remote availability included, is
  evaluated and announced during preflight, before the first file mutation. Once mutation begins,
  the run carries on to a terminal outcome: an operational failure such as a rejected push is
  reported there, not turned into a mid-flight abort, and completed reset work is never rolled back.
  Every reachable
  answer combination -- interactive Yes, interactive No, and a config run with `push` true or false
  -- ends in exactly one reported outcome, each distinguishable by both its printed final line and
  its process exit status:
  - reset complete and published (push requested, remote present, push succeeded);
  - reset complete, publication not requested (the interactive No path, or a config run with `push`
    false); a normal success, not a degraded one, and the line still states which of stage and
    commit ran;
  - reset complete, publication requested but no remote configured;
  - reset complete, push attempted and failed -- the failure text and the exact manual push command
    are printed, and the run does not report plain success.

  The consumer's files, staging, and commit are never rolled back by a push failure.
- Evidence or review, when useful: `reviewer` enumerates the reachable answer combinations against
  the outcome list and verifies each maps to exactly one outcome that describes it accurately, that
  a caller can tell them apart without parsing prose, and that a push failure cannot be mistaken for
  full success.
- Obvious follow-ons: WP-6a.

### Work package: WP-6a synthetic-remote E2E case

- Owner: WS-B expert_coder.
- Touch points: `tests/meta/e2e/e2e_reset_routing.py`.
- Depends on: WP-5a.
- Acceptance criteria (behavior): a case creates a local bare repository, wires it as the clone's
  `origin`, runs reset with push requested, and proves the reset commit is present in that bare
  repository. A second case covers the no-remote outcome. Both are offline, self-cleaning, and
  follow the harness's existing disposable-`/tmp` convention.
- Obvious follow-ons: hand the bootstrap contract to WS-F.

### Work package: WP-7a document the disk budget test

- Owner: WS-C tester.
- Touch points: `tests/test_checkout_disk_budget.py:1-10`, `docs/PYTEST_STYLE.md` runtime-budget
  section.
- Depends on: none.
- Acceptance criteria (behavior): an agent reading the top of the file learns the base-lane run
  command, why the `du` subprocess is accepted here, and that the file is vendored and returns after
  deletion. The style guide names this file as the one standing exemption to its no-subprocess rule,
  so the E2E-migration argument has a documented answer. The existing footer line stays last.
- Obvious follow-ons: hand the policy delta to WS-F.

### Work package: WP-8a tools/ misuse audit report

- Owner: WS-D reviewer.
- Touch points: `meta/docs/active_plans/audit/tools_folder_misuse_survey.md` (new; snake_case per
  the active-plans convention).
- Depends on: none.
- Acceptance criteria (behavior): every surveyed repo gets a verdict with a `file:line` citation,
  covering `tools.*`, `devel.*`, and `tests.*` importers alike.
  For each violation, the report names the imported module and the migration direction its
  maintainer would take -- move the reusable code into an importable package namespace (for example
  a `repo_lib/`-style package) and leave a thin entry-point script behind in `tools/`. It distinguishes
  the four plan-named test importers from the broader current result, including runtime imports
  found by the complete static survey. It records the existing in-repo alternative for a test that
  must exercise a script:
  `protein-image-grader/tests/test_copy_archive_images.py:8` loads the tool by file path rather than
  importing `tools.*`. It changes no repository other than this one.
- Evidence or review, when useful: the boundary itself is settled and is not up for audit-driven
  revision. The audit's job is detection plus migration direction, so a violation that looks like a
  deliberate reusable API is still a violation -- report it as code that belongs in a package.
- Obvious follow-ons: WP-9a.

### Work package: WP-9a author TOOLS_README and cross-links

- Owner: WS-D coder.
- Touch points: `tools/TOOLS_README.md` (new), `devel/DEVEL_README.md:1-17`. The
  `docs/REPO_STYLE.md` sentence is drafted here and handed to WS-E, which owns that file.
- Depends on: WP-8a.
- Acceptance criteria (behavior): a reader landing in `tools/` learns that the folder holds
  standalone entry-point scripts, that reusable modules belong in an importable package namespace
  instead, that nothing imports `tools`, `devel`, or `tests` as a package root -- including files
  inside those directories -- that a `tools/` script does not import its siblings either, and how a
  repo that already
  does should migrate -- move the reusable code into a package and leave a thin script behind, or,
  for a test exercising a script, load it by file path as the named in-repo precedent does. The
  README states plainly that this gate reports violations for the consumer's maintainer to fix, and
  that a test needing test-only helpers keeps them importable the way this repo already does -- as
  same-directory modules such as `file_utils`, not as `tests.something`. The document
  mirrors `devel/DEVEL_README.md`'s shape and run-command convention, links to it, and is linked
  back from it. `docs/REPO_STYLE.md` distinguishes `tools/` from `devel/` in one sentence. Link,
  ASCII, and whitespace gates pass.
- Obvious follow-ons: WP-10a.

### Work package: WP-10a import-boundary gate

- Owner: WS-D coder.
- Touch points: `tests/test_support_dirs_not_imported.py` (new); recommended scaffold:
  `tests/test_import_dot.py` for the harness shape and `tests/test_import_requirements.py`
  (`ImportCollector`, `collect_import_roots`) for root-module extraction.
- Depends on: WP-9a.
- Acceptance criteria (behavior): the gate enforces four rules.

  R1 -- package-root imports of `tools`, `devel`, or `tests` fail everywhere, at any submodule depth
  and in every form: `import tests`, `import tests.file_utils`, `from tests import file_utils`,
  `from tests.file_utils import something`, and the same shapes for the other two roots. Where the
  importer lives is irrelevant; a `tools/` script reaching for `tests.file_utils` is precisely what
  this catches.

  R2 -- a `tools/` script may not import a sibling `tools/` module. Sibling identity is decided from
  the actual contents of `tools/` in the repository under test, never from a hardcoded module list:
  a bare `import b` inside `tools/a.py` is a violation when `tools/b.py` exists, and is ignored when
  it resolves to a stdlib module, a declared dependency, or a package elsewhere in the repo. The
  same holds for `from b import c`. Shared reusable code belongs in a package.

  R3 -- a `devel/` script may import a flat sibling helper such as `changelog_lib`, `version_lib`,
  or `version_files`. This is the documented internal-helper exception for vendored development
  tooling, and it relaxes the sibling rule only, never R1.

  R4 -- a test may import a flat test helper such as `file_utils`, which is how `tests/conftest.py`
  and pytest already arrange that directory. This stays green.

  No rule is needed for a non-test file importing a flat test helper: `tests/conftest.py` shapes
  only pytest execution, so a standalone `tools/foo.py` doing `import file_utils` has no `tests/` on
  its path and already fails on its own. The gate does not carry machinery for a case the runtime
  rejects.

  The failure message names the rule, the importing file, the imported module, and
  `tools/TOOLS_README.md` for the migration direction. Reporting uses the standard violation
  harness; repo-specific exclusions route through `REPO_HYGIENE_FILTERS` rather than an inline list
  in a vendored file.
- Evidence or review, when useful: run the gate against this template before committing it. R1 is
  expected green (no package-root import of the three exists here). R3 is what keeps the propagated
  devel tooling green -- `commit_changelog.py`, `query_changelog.py`, and `rotate_changelog.py`
  import `changelog_lib`, and `bump_version.py` imports `version_lib` and `version_files`. R4 keeps
  every vendored test green through its bare `import file_utils`. If anything unexpected fails,
  report the finding and pause; do not widen a filter to make it green.
- Obvious follow-ons: hand the boundary decision to WS-F.

### Work package: WP-11a root script budget policy and gate

- Owner: WS-E tester, who also owns every `docs/REPO_STYLE.md` edit in this plan and lands WS-D's
  handed-over support-directory sentence in the same pass.
- Touch points: `docs/REPO_STYLE.md` (new policy section plus WS-D's sentence),
  `tests/test_root_script_budget.py` (new);
  recommended scaffold: `tests/test_shebangs.py`, which already enumerates tracked files and reads
  the executable bit.
- Depends on: none.
- Acceptance criteria (behavior): the policy section states the counting rule and both thresholds in
  prose, so the test enforces documented policy rather than a bare number. A root with 7 or more
  counted files fails; 5 or 6 passes while writing a report naming the counted files; 4 or fewer
  passes silently and leaves no report. Thresholds are exercised with synthetic roots, not with this
  repository's current count, so a legitimate root addition does not rewrite the test.
- Evidence or review, when useful: the settled part of the rule is not reopened -- every tracked
  root `.py` and `.sh` file counts, `source_me.sh` included. The open part is only the third
  category, "executable files": read the repository roots under `~/nsh/` and decide whether an
  exec-bit file that is neither `.py` nor `.sh` should count as a script (for example by requiring a
  shebang, so a compiled launcher or a data file with a stray exec bit does not inflate the count).
  Record the chosen rule and its rationale in the policy section and note the alternative considered
  in the patch report.
- Obvious follow-ons: hand the policy delta to WS-F.

### Work package: WP-12a shared documentation and staging

- Owner: WS-F integrator.
- Touch points: `tests/TESTS_README.md`, `meta/docs/HUMAN_GUIDANCE.md`, `docs/CHANGELOG.md`.
- Depends on: WP-2a, WP-6a, WP-7a, WP-10a, WP-11a.
- Acceptance criteria (behavior): `tests/TESTS_README.md` is edited exactly once and names the
  disk-budget exemption and the two new gates. `meta/docs/HUMAN_GUIDANCE.md` records the settled
  decisions under existing headings: LOCAL block renders last, reset finishes the consumer behind
  one default-Yes prompt with config defaulting push off, the disk-budget exemption, the
  support-directory import boundary covering `tools/`, `devel/`, and `tests/`, and the root budget
  rule. The changelog block uses the documented subsection order and
  includes the decisions and failures encountered. Everything is staged; nothing in this repository
  is committed.
- Obvious follow-ons: none; the human commits.

## Acceptance criteria and gates

- Per-patch gate: the milestone's own exit criteria -- the focused test files it names -- plus
  `git diff --check`. The full base suite is not a per-patch requirement; it runs once at the
  integration gate, which keeps parallel lanes fast. A doer runs the full suite early only when a
  focused run surfaces something that looks cross-cutting.
- Integration gate: `source source_me.sh && pytest tests/ -q` passes with no new failures
  against the baseline recorded at plan start; `bash tests/meta/e2e/run_all.sh` passes in LOCAL
  mode; a propagation dry run against a disposable clone plans `tools/TOOLS_README.md` and the two
  new tests, and proposes no second change when re-run.
- Propagation-target rule: the dry-run target is a fresh `git clone` of this template into a `/tmp`
  path created by the doer for that check and removed afterward, matching the existing
  `e2e_reset_routing.py` convention. A path outside `/tmp` is never a valid target for these gates.
- Independent review gate: a `reviewer` subagent reads the WS-A, WS-B, WS-D, and WS-E diffs against
  `docs/PYTHON_STYLE.md` (tabs, typing, no try/except, no `dict.get` fallbacks) and
  `docs/PYTEST_STYLE.md` (the fragile-test checklist) before WS-F runs. A finding blocks only when
  it cites a concrete violation of a named repository contract or a stated acceptance criterion that
  does not hold; everything else is advice the manager weighs. The reviewer does not halt
  integration over style preference, and final judgment stays with the manager and the human.

Failure semantics: a failing per-patch gate blocks that milestone only. A failing integration gate
blocks M12. A gate that fails for a pre-existing reason is recorded in the changelog's decisions
subsection with the evidence, not silenced.

## Test and verification strategy

- Fast lane: every new gate is a deterministic, offline pytest using `tmp_path` and inline inputs,
  following the canonical hygiene module shape in `docs/PYTEST_STYLE.md`.
- Synthetic transitions replace human steps: a `tmp_path` Git repository stands in for a consumer
  when testing finish outcomes, and a local bare repository stands in for GitHub in the E2E case.
- Reset behavior is verified through `--config` JSON answer files, the documented non-interactive
  interface, so no prompt ever waits on a human.
- Threshold tests use synthetic roots on both sides of each boundary rather than asserting this
  repository's current count.
- Import-resolution evidence for the `pytest.ini` removal is a collection-only run in the reset
  clone; the full suite runs once as an implementation probe rather than as permanent suite weight.
- The one behavior that cannot be tested offline -- pushing to GitHub -- is verified against a local
  bare remote and declared a non-goal beyond that.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Removing root `pytest.ini` breaks consumer import paths | Reset produces a repo whose suite cannot run | Collection fails inside the reset clone | WS-B | WP-3a asserts in-clone collection and runs the full suite once as a probe before acceptance |
| Gitignore relocation drops a consumer's local rules | Silent loss of repo-specific ignores | Local body lines missing after re-render | WS-A | WP-1b asserts verbatim body preservation plus idempotency |
| A failed push reads as a completed publish | Consumer believed published when it is not | Push requested, remote rejects | WS-B | WP-5a defines four distinguishable terminal outcomes with distinct exit status |
| Automated reset run pushes unexpectedly | Unreviewed history on a remote | E2E or subagent run against a repo with a real origin | WS-B | Config `push` defaults false; preflight announces remote before mutation |
| Import gate fails four consumer repos on next propagation | Consumer suites go red | Maintainer propagates | WS-D | Intended detection; the migration direction ships in `tools/TOOLS_README.md` and the audit names each repo and module |
| A lane tries to fix a consumer repo | Out-of-scope changes in repos nobody opened | Doer reads the gate failure as work to do | WS-D | Non-goals and WP-8a state detection-only; remediation belongs to each consumer's maintainer |
| Root budget counts non-scripts | False failures in a consumer | Exec bit on a data file or launcher | WS-E | WP-11a validates the counting rule against real repo roots before finalizing it |
| Two lanes edit `tests/TESTS_README.md` | Late merge conflict | Concurrent doc edits | WS-F | WS-F is the sole owner; other lanes hand it deltas |
| Reset lane serialization stalls the plan | M6 arrives last | WS-B falls behind | WS-B | WS-B is the longest lane and starts first; the other four lanes are independent |

## Documentation close-out requirements

- Active plan / progress tracker: this plan file; the audit report lands in
  `meta/docs/active_plans/audit/`.
- docs/CHANGELOG.md entry: one dated block using the documented subsection order -- new gates and
  README under additions, gitignore order and reset finish behavior under behavior or interface
  changes, the `pytest.ini` leak under fixes and maintenance, and the settled decisions (LOCAL block
  last, finish behind one prompt with config push off, disk-budget exemption, support-directory
  import boundary, root thresholds, and the rejected vendored-footer gate) under decisions and
  failures.
- Archive / closure notes: when the plan completes, move the audit report to `meta/docs/archive/`
  with `git mv` if the maintainer keeps that convention; leave it in place otherwise.

## Patch plan and reporting format

- Patch 1: WS-A (M1, M2).
- Patch 2: WS-B (M3-M6).
- Patch 3: WS-C (M7).
- Patch 4: WS-D (M8-M10).
- Patch 5: WS-E (M11).
- Patch 6: remaining repository-required work -- WS-F (M12), staged for the human.

Each patch report states: milestone IDs, files touched, the exact gate commands run, their pass or
fail result, and any decision recorded for the changelog.

## Resolved decisions

- Reset stages, commits, and pushes the reset consumer after one default-Yes confirmation. This is
  settled maintainer direction: agents repeatedly abandon resets half-finished, and finishing the
  bootstrap is the script's job. Review attention belongs on whether the implementation fulfills it,
  not on whether to do it.
- Implementation agents still leave this template's own changes staged and uncommitted. The two
  policies apply to two different repositories.
- No permanent test guards the vendored footer comment. The string is prose, absent from `repolib/`
  and `meta/`, so a gate on it would test a comment rather than behavior; `docs/PYTEST_STYLE.md`
  prefers a missing test to a fragile one.
- Root-script thresholds (fail at 7 or more, report at 5 or 6) are maintainer decisions and are not
  reopened; only the counting rule is investigated.
- `tools/` and `devel/` are script locations and `tests/` is a test location; none of the three is
  an importable library namespace. Reusable code belongs in a package namespace with a thin
  entry-point script in `tools/`. The module-root rule is unconditional and binds files inside those
  directories too: a `tools/` script may not import `tests.file_utils`.
- `tools/` scripts are self-contained: no sibling imports from their own directory. Shared reusable
  code moves into a package instead.
- `devel/` keeps a documented flat sibling-helper exception (`changelog_lib`, `version_lib`,
  `version_files`) for its vendored development tooling. It relaxes the sibling rule only, never the
  package-root ban.
- `tests/` may share flat helpers internally, which is how `tests/conftest.py` already arranges the
  directory: `import file_utils` is correct, `import tests.file_utils` is not.
- No gate is added for a non-test file importing a flat test helper. `tests/conftest.py` shapes only
  pytest execution, so `import file_utils` from a standalone `tools/` script already fails on its
  own; the runtime enforces that boundary without help.
- The gate this repo ships detects and reports violations; each consumer's maintainer owns the
  remediation.

## Open questions and decisions needed

- Manager/subagent decision procedure -- root-script counting rule:
  - Decision owner or dedicated class: WS-E `tester` (WP-11a).
  - Evidence and decision rule: the `.py` and `.sh` half of the rule is settled and stays as is --
    every tracked root `.py` and `.sh` file counts, `source_me.sh` included. Open only: whether a
    tracked root file that carries the executable bit but is neither `.py` nor `.sh` should count.
    Apply the candidate rule to the repository roots under `~/nsh/`; if it counts something that is
    not script clutter, narrow that third category (for example by requiring a shebang, so a
    compiled launcher or a data file with a stray exec bit is excluded) and record the rationale in
    the policy section.
- Manager/subagent decision procedure -- import gate baseline:
  - Decision owner or dedicated class: WS-D `coder` (WP-10a).
  - Evidence and decision rule: run the gate against this template before committing it; a failure
    here is reported, not filtered away.
- Non-blocking follow-up: the four flagged repos are remediated by their own maintainers, in their
  own repositories, after propagation surfaces the gate. This plan neither fixes them nor tracks
  that work.
- Non-blocking follow-up: whether `docs/PYTEST_STYLE.md` should gain a general "documented
  exemption" mechanism instead of naming one file. Not needed until a second exemption appears.
