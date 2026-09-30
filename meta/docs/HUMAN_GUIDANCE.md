# Human guidance

Durable preferences and stable decisions for agents working in this repo.
Keep entries current. Move outdated entries to `docs/CHANGELOG.md`.

See [docs/REPO_STYLE.md](../../docs/REPO_STYLE.md) for repo-wide conventions.

## Decision priority

- Optimize repository tools for the human maintainer's real workflow and stated preferences.
- Address the highest-impact risks first. Spend review and validation time on decisions that
  affect correctness, maintainability, delivery, or the maintainer's routine use.
- Apply the `Focus on important issues` philosophy directly. Finish consequential work before
  considering generalization, speculative flexibility, or low-impact polish.
- Prefer adaptability over speculative edge-case handling. Use clear boundaries, stable domain
  concepts, and replaceable components so unexpected cases can be handled later without redesign.
- Address concrete requirements and likely failure modes now. Add mechanisms, policies, and tests
  when supported by an actual need rather than a hypothetical future case.
- Treat a good-enough system as a valid stopping point when pursuing perfection would slow useful
  progress without a proportionate long-term benefit.

## Documentation ownership

- Give every whole-file overwrite source, across all file extensions, the exact header message,
  "This file is vendored. Local changes can and will be overwritten by propagation." Use the
  format's native comment syntax and place it after a required shebang. Keep it out of noexist
  seeds, partial-ownership buckets, and template-only files whose local content persists.
- Keep the propagated `docs/HUMAN_GUIDANCE.md` and `docs/DESIGN_DECISIONS.md` seeds minimal.
  Record starter-template-specific guidance in this file instead of shipping it to consumer repos.
- Keep one native application or library package in a named root-level folder. Use a `packages/`
  grouping layer when multiple native applications or libraries need separation.
- Keep `docs/MARKDOWN_STYLE.md` high-level. Use GitHub Flavored Markdown as its syntax baseline and
  link the official online specification instead of restating fine-grained parsing rules.
- Support simple pipe tables in Markdown. Direct captions and complex accessible header
  relationships to semantic HTML or an appropriate publishing pipeline.
- [REPO_TYPE.md](REPO_TYPE.md) owns marker format, names, inheritance, and
  multi-type behavior.
- Keep generic TypeScript repositories independent of the template's GitHub Pages build.
  Pages build, deployment, preview-server, and build-aware Playwright scripts belong to the
  `githubpages` child type, which inherits `typescript`.
- [GITIGNORE_SYSTEM.md](GITIGNORE_SYSTEM.md) owns `.gitignore` sources, rendered ownership,
  canonicalization, cleanup, pattern semantics, and validation.
- [LICENSE_POLICY.md](LICENSE_POLICY.md) owns license filenames, canonical body sources,
  GitHub detection guidance, reset installation, legacy propagation, and release verification.
- [docs/REPO_STYLE.md](../../docs/REPO_STYLE.md) owns repository conventions.
  Keep type-marker rules out of it; the shared `REPO_` prefix does not make the
  two documents interchangeable.

## Hygiene discovery ownership

- Top-level hygiene tests select tracked files through
  [tests/file_utils.py](../../tests/file_utils.py) `discover_files`; they do not
  maintain local discovery or exclusion-prefix copies.
- Universal skip directories belong in [tests/file_utils.py](../../tests/file_utils.py).
  Repo-specific exclusions belong in [tests/conftest.py](../../tests/conftest.py)
  `REPO_HYGIENE_FILTERS`.
- The line-limit gate keeps only its exact-path manager approval list because
  that policy is specific to the gate.
- Automatically exclude Markdown under any `docs/active_plans/` or `docs/archive/` tree from the
  source-code line limit while retaining the limit for other authored source files.
- Use one exact repo-relative path for each source-line-limit override. Encode universal folder
  categories in the gate and approve exceptional files individually.
- Seed `tests/source_file_line_limit_overrides.txt` through universal noexist propagation so new
  consumers receive its instructions and established consumers retain their approvals.

## Plan and test gates

- Treat a source-size advisory as a prompt to make the best cohesive responsibility split, not as
  a target line count. Trimming just below the threshold pushes the problem to the next editor.
- Review implementation plans against [docs/REPO_STYLE.md](../../docs/REPO_STYLE.md),
  [docs/PYTEST_STYLE.md](../../docs/PYTEST_STYLE.md),
  [tests/TESTS_README.md](../../tests/TESTS_README.md),
  [devel/DEVEL_README.md](../../devel/DEVEL_README.md), and relevant language guides.
- Prefer fewer, stronger permanent tests. Protect intentionally stable behavior worth preserving,
  not incidental implementation. When in doubt, remove the test.
- Ground acceptance gates in product behavior, repository policy, security, demonstrated failure,
  or measured evidence. Use exactness and thresholds when the actual contract requires them.
- Give each new blocking gate a failure plan that names the failure and the decision, correction,
  or recovery that follows.
- Put implementation probes and one-time checks in the ignored `tests/_temp/` subtree. Let
  pytest-suitable `test_*.py` files run with pytest, run heavier checks explicitly, then promote or
  remove them before handoff.

## Shebang semantics

- WeBWorK `.conf` files use an exact first-line `#!perl` configuration marker.
  Shebang hygiene treats that marker as data, not an OS interpreter directive.

## Graphify orientation

- Keep the Graphify manager and subagent orientation short and repository-specific.
- Publish a compact community-level Graphify SVG without per-symbol labels or a legend; keep the
  repository groups, community names, and observations readable in `docs/GRAPHIFY.md`.
- Let `--svg` publish an existing map alone or compose with `--fresh` and `--update` so graph data
  and documentation can advance in one command.
- Ship `devel/graphify_map_repo.py` to every repository type and retain it during new-repository
  reset; all repositories need the same maintainer-facing Graphify navigation entry point.
- Report when the graph was mapped rather than Graphify's `built_at_commit`. The map includes
  uncommitted and untracked working-tree code, so commit attribution confuses coders.
- Use the primary generated graph artifact's local modification time as mapping provenance. It
  advances on rebuild and remains stable when `--context` only reads the existing map.
- Derive manager context from `graphify-out/graph.json`: identify the repository, primary
  subsystems, highly connected code with source paths, cross-subsystem bridges, and targeted
  starting queries.
- Keep `Corpus Check`, ignore/exclusion policy, generated-file hygiene, and generic artifact
  descriptions out of manager context. Preserve Graphify's full diagnostics in `GRAPH_REPORT.md`.
- Bound each cross-area connector to eight displayed community names and summarize the remainder
  as `and N more`. Preserve Graphify's deterministic ordering instead of semantically filtering
  connector communities.
- Treat Graphify as structural navigation. Verify conclusions against current source,
  configuration, tests, and runtime behavior.
- The propagated Python tool automatically extracts a missing graph or updates an existing graph.
  Ordinary updates run only `graphify update .` before regenerating manager context. Graphify may
  assign deterministic hub names to changed communities; use a fresh build when those names have
  degraded enough to warrant full Claude CLI or Ollama labeling.
- Fresh extraction upgrades `graphifyy[ollama,sql,terraform]`, fully labels every community, and
  benchmarks. Keep the pip phase concise by suppressing satisfied-dependency inventory and unusable
  cache warnings while leaving installation failures visible.
- Fresh Claude CLI labeling explicitly selects Sonnet so Graphify does not inherit the interactive
  Claude model. This keeps high-volume community naming separate from an Opus coding session.
- Keep Sonnet as the maintained label-quality default. Treat Haiku as a one-time representative
  quality comparison before changing the default for additional allowance savings.
- Ordinary updates perform no package upgrade, labeling pass, or benchmark. Fresh builds include
  all three operations, including Graphify's benchmark.
- Use `-F`/`--fresh` to force extraction and `-U`/`--update` to update with a fresh-extraction
  fallback. Use `-C`/`--context` to print existing-map orientation without running Graphify or
  either label backend; before the first graph exists, context prints the CLI help.

## Propagation routing model

- File location is the primary routing determinant. Agents use location first;
  per-file overrides only when location cannot express the rule.
- During the Gitignore layout transition, recognize the previous LOCAL banner and move its
  consumer-owned body after the managed blocks while preserving every line in order.
- Record a `devel/changelog_lib.py`-compatible changelog entry for each real single-repository
  propagation maintenance change; idempotent `.gitignore` normalization leaves it unchanged.
- Every file under `docs/`, `tests/`, `devel/`, and `tools/` ships universally to all
  consumer repos (overwrite bucket by default).
- Keep `launchers/` as an optional consumer-local convention outside universal propagation
  routing. Create it only in repositories with a concrete launcher.
- Every file under `templates/<type>/` ships to consumer repos of that type,
  at its consumer-relative path (e.g. `templates/python/foo.py` ships as `foo.py`).
- `docs/PYTHON_STYLE.md` ships to all repo types. It is a universal doc.
- `pip_requirements-dev.txt` ships through its dedicated managed/local requirements bucket. The
  universal package block refreshes while repository-specific dependencies remain local.
- `pip_requirements.txt` is a Python-only noexist seed at
  `templates/python/noexist/pip_requirements.txt`.
- `.graphifyignore` ships universally as a noexist seed. Its shared defaults exclude
  `tests/`, `devel/`, `tools/`, and `docs/`; each consumer may add local exclusions afterward.
- Every propagation run invokes the conservative root-license migration in
  `repolib/license_migration.py`. It uses exact known names and grep-like body markers, preserves
  customized text and conflicts, and obtains full replacements only from the local `LICENSES/`
  catalog. Keep its detailed contract in `LICENSE_POLICY.md`, not in parallel guidance here.

## ROUTING_OVERRIDES holds only exclude_repos

- `ROUTING_OVERRIDES` in `meta/propagation/manifests.yaml` holds only one
  exception: `exclude_repos` for `docs/CLAUDE_HOOK_USAGE_GUIDE.md` (blocks
  the mirror from shipping back to its source repo `claude-code-permissions-hook`).
- Do not add `language`, `bucket`, or `requires_repo_file` fields. Those were
  removed when location-based routing replaced per-file gates.
- When a new language-specific file is needed, put it under the correct
  `templates/<type>/` folder rather than adding a `ROUTING_OVERRIDES` entry.

## Conditional overlays (_folder convention)

- An underscore folder under `templates/<type>/` (e.g. `templates/python/_ci/`)
  is a conditional overlay. The base walk skips it; a `conditional_overlays`
  manifest rule enables it per consumer.
- Conditional overlay rules live in `meta/propagation/manifests.yaml` under
  `conditional_overlays: <type>: <overlay_name>: {when, path, description}`.
- The only supported `when` verb is `has_file`: the overlay ships when the named
  file exists at the consumer repo root.
- The mechanism currently has no live overlays; `conditional_overlays` is empty.
- Prefer conditional overlays over `requires_repo_file` in `ROUTING_OVERRIDES`.

## PyPI child type

- `pypi` is a real child repo type of `python`, declared by `pypi: python` in
  `repo_type_inherits`.
- PyPI-only files live under `templates/pypi/` and ship through the `pypi` type.
- The legacy reset answer `project_type: python` plus `pypi: true` is normalized
  to the canonical `pypi` marker.

## Manifests single source of truth

- All propagation manifests live in `meta/propagation/manifests.yaml`.
- `repolib/manifests.py:load_manifests()` reads the YAML at import time with
  `yaml.safe_load` and returns the correct Python types.
- `repolib/model.py` assigns loaded values to its module-level public names.
- Edit `meta/propagation/manifests.yaml` to change any manifest. Do not add
  inline literals back to `repolib/model.py`.

## reset_repo.py design

- `reset_repo.py` is the bootstrap entry point for new consumer repos.
- Selected licenses install as real root files named `LICENSE.<SPDX>` from complete plain-text
  catalog bodies with the same names. Do not add rendering extensions or license aliases.
- Interactive interview is the human default: it asks project type, license, PyPI intent, then one
  default-Yes finish question for stage, commit, and push together.
- CLI surface is minimal: `-h`, `--dry-run`, and `--config <file>`. The `--force` and
  `--yes` flags were removed; `--force` had no use case and `--yes` is replaced by
  `--config` for non-interactive runs.
- `--config <file>` is the testing/reproducibility interface: a JSON answer file drives a
  non-interactive reset for E2E and subagent testing. Required keys are `project_type` and
  `code_license`; optional defaults are `docs_license` (CC-BY-4.0), `pypi` (false), `stage` (true),
  `commit` (false), and `push` (false). Short aliases are accepted for required keys.
- Folder-name guard: reset refuses to run when the repo root basename is exactly
  `starter-repo-template`. This protects the template development checkout. Guard is
  folder name only; no remote or origin inspection (remote-slug detection is fragile
  for freshly cloned consumer repos that have not yet renamed their remote).
- Running outside a git repository exits with a clear message instead of a raw
  subprocess traceback.
- Do not add automation flags for decisions the user makes once at repo creation.

## Enforced template contracts

- Render propagated `.gitignore` blocks first and the consumer-owned LOCAL block last; preserve its
  body verbatim when rebuilding managed content.

- Keep checkout, machine-wide Podman, and Rust `target/` disk-budget checks mandatory in the base
  pytest lane. Continuous development must use hard-drive space responsibly, and propagation
  restores the vendored checks after deletion.
- Treat `tools/`, `devel/`, `tests/`, and `launchers/` as support locations rather than
  repository-level import packages. Keep standalone tools independent of repository-local packages,
  while allowing a self-contained tool directory to own helpers and a launcher to delegate into the
  application. Documented flat devel and test helpers remain allowed.
- Count tracked root `.py` and `.sh` scripts, plus executable-shebang launchers of other types.
  Five or six write a report; seven or more fail the root-script budget.

## Tools and developer scripts

- Use `devel/` for highly technical developers and repository or Git work such as versioning.
- Use `tools/` for optional standalone utilities that regular users can run for a direct domain
  task.
- Keep one native application or library package in a named root-level folder. Use a `packages/`
  grouping layer when the repository contains multiple native applications or libraries.
- Survey all repositories under `~/nsh` before applying systemic placement changes so the template
  fixes the shared source of drift first.

## E2E harness design

- `tests/meta/e2e/e2e_license_migration.py` creates a disposable Git consumer and runs the real
  propagation CLI to verify typed-name migration, complete catalog replacement, and generic
  symlink removal together.
- `tests/meta/e2e/e2e_reset_routing.py` clones the template into consumer-named `/tmp` dirs
  (e.g. `/tmp/my_project_python/`) so each test case is isolated and ephemeral.
  Template-meta: lives under `tests/meta/e2e/`; never propagates to consumers; removed by reset.
- LOCAL mode (default): offline, clones committed local history only. Uncommitted
  working-tree changes are not exercised; commit before running LOCAL if you need the
  harness to see them.
- REMOTE mode (opt-in via `remote` argument): GitHub HTTPS clone (read-only); exercises
  what a consumer receives from origin/main. New code must be pushed to origin/main by the
  human first; REMOTE clones whatever is already there.
- Each case uses an ephemeral per-case JSON config; verified against the live
  propagation engine (oracle) plus reset-specific anchor checks.
- `tests/meta/e2e/run_all.sh` iterates all `e2e_*` scripts under `tests/meta/e2e/`
  and reports pass/fail; offline only (LOCAL mode). Also template-meta.

## Tests follow live config

- Tests assert on propagation engine behavior using synthetic repo trees; they
  do not duplicate manifest constants inline.
- Preferred pattern: call `repolib.manifests.load_manifests()` or inspect
  `repolib.model.*` constants rather than hardcoding expected sets.
- Routing assertions use `compute_propagation_plan` on synthetic repo trees so
  they reflect the live config automatically when manifests change.

## Test fixture policy

- Use inline setup first. For fixture cases, see
  [docs/PYTEST_AUTHORING_GUIDE.md](../../docs/PYTEST_AUTHORING_GUIDE.md).

## Prefer rule-based routing over per-file customization

- The goal is zero per-file routing entries in `ROUTING_OVERRIDES`.
- When a file needs special handling, exhaust location-based options first:
  move it to the correct folder or create a `_folder` conditional overlay.
- Only fall back to `ROUTING_OVERRIDES` for exceptions that cannot be expressed
  by directory placement (currently: `exclude_repos` only).
