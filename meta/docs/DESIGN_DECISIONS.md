# Design decisions

Durable decisions for the starter repository template. Consumer-facing conventions remain in
[docs/REPO_STYLE.md](../../docs/REPO_STYLE.md); this ledger records how this repository implements
and propagates them.

## Testing and hygiene

### Disk-budget guards follow storage scope

**Decision.** Ship the checkout and machine-wide Podman disk-budget guards universally in the base
pytest lane. Ship the repository-local `target/` guard through the Rust overlay.

**Why.** Responsible continuous development includes stewardship of finite hard-drive space.
Routine builds can accumulate enough generated data to fill a developer volume; one observed Rust
cache reached 130 GiB. Optional or E2E-only guards do not protect the development loop that creates
the data.

**Consequence.** Propagation restores all three mandatory vendored guards after deletion. Podman
storage is checked from every consumer because it is machine-wide; `target/` is checked only in
Rust consumers because it is repository-local Rust output.

**Owner.** [docs/PYTEST_STYLE.md](../../docs/PYTEST_STYLE.md),
[tests/test_podman_disk_budget.py](../../tests/test_podman_disk_budget.py), and
[templates/rust/tests/test_target_disk_budget.py](../../templates/rust/tests/test_target_disk_budget.py).

### Planning and archive Markdown has no source-code line budget

**Decision.** Exclude `.md` files beneath any `docs/active_plans/` or `docs/archive/` tree from the
source-file line-limit gate. Keep other source types in those trees covered.

**Why.** Plans and archives preserve working context and history; their useful size follows the work
they document rather than the maintainability budget for executable or current reference source.
An exact local override list gives every exceptional source file an individual approval record.

**Consequence.** The universal test applies the directory-category rule automatically wherever the
named `docs/` tree is nested. Exact-path overrides remain reserved for exceptional external sources,
while other hygiene checks continue to inspect these Markdown files. Universal noexist propagation
seeds a commented override file for new consumers and preserves each established consumer's file.

**Owner.** [tests/test_source_file_line_limit.py](../../tests/test_source_file_line_limit.py) and
[docs/REPO_STYLE.md](../../docs/REPO_STYLE.md).

## Repository structure

### PyPI publishing separates operational boundaries

**Decision.** Keep `submit_to_pypi.py` as the publishing coordinator. Put project metadata and
version files in `pypi_project.py`, Git and release preconditions in `pypi_release.py`, and build,
upload, and installation verification in `pypi_distribution.py`.

**Why.** The former entry script mixed local metadata, repository mutation, credentialed network
operations, artifact lifecycle, and orchestration. These responsibilities have different failure
modes and security boundaries and can change independently.

**Consequence.** The coordinator preserves release-step ordering and production confirmation while
the focused modules own their inputs and side effects. Every module remains in the PyPI overlay and
ships automatically by location without manifest registration.

**Owner.** [templates/pypi/devel/submit_to_pypi.py](../../templates/pypi/devel/submit_to_pypi.py),
[templates/pypi/devel/pypi_project.py](../../templates/pypi/devel/pypi_project.py),
[templates/pypi/devel/pypi_release.py](../../templates/pypi/devel/pypi_release.py), and
[templates/pypi/devel/pypi_distribution.py](../../templates/pypi/devel/pypi_distribution.py).

### Placement policy ships without launcher routing

**Decision.** Propagate the canonical classifier, audience-specific support READMEs, and import
gate through their existing universal routes. Keep `launchers/` as an optional consumer-local
convention with no dedicated propagation route.

**Why.** Every consumer needs the same distinction between standalone user utilities, repository
engineering, and application behavior, but a policy should not create a directory that a repository
does not need.

**Consequence.** Root `tools/` and `devel/` keep their current location-based routing. The template
does not create or register `launchers/`; a consumer adds it only for a concrete thin delegate.

**Owner.** [PROPAGATION_RULES.md](PROPAGATION_RULES.md),
[docs/REPO_STYLE.md](../../docs/REPO_STYLE.md), and
[tests/test_support_dirs_not_imported.py](../../tests/test_support_dirs_not_imported.py).

### One native package stays at root

**Decision.** Place a repository's only native application or library package in a named folder at
the repository root. Use `packages/` to group multiple native applications or libraries.

**Why.** A root-level package makes one code owner immediately visible. The additional `packages/`
layer earns its place when it separates multiple independently named code owners.

**Consequence.** Application code uses the named root package when the repository has one and gives
each package a distinct home under `packages/` when it has several. A standalone tool keeps its
helpers inside its own self-contained directory instead of creating a root helper package.

**Owner.** [docs/REPO_STYLE.md](../../docs/REPO_STYLE.md) and the repository's package manifests.

## Propagation

### Consumer ledgers seed only instructions

**Decision.** Keep the propagated `docs/HUMAN_GUIDANCE.md` seed to its managed instructions and
keep `docs/DESIGN_DECISIONS.md` to its managed instructions plus the blank decision template.

**Why.** Receiving repositories need fresh ledgers for their own guidance and decisions. Shared
policy belongs in the relevant propagated `docs/*_STYLE.md` file, while template-specific records
belong under `meta/docs/` and must not preload consumer history.

**Consequence.** Adding a shared rule changes its authoritative style document rather than adding a
seed entry. Propagation continues to refresh only each ledger's marked header and preserves every
consumer-owned entry below it.

**Owner.** [HEADER_BUCKET_SPEC.md](HEADER_BUCKET_SPEC.md) and
[docs/REPO_STYLE.md](../../docs/REPO_STYLE.md#human-guidance-and-design-decisions).

### GitHub Pages is a child of TypeScript

**Decision.** Define `githubpages -> typescript -> website` in the repository-type inheritance
graph. Keep `build_github_pages.sh`, `deploy-pages.yml`, `run_playwright_tests.sh`, and
`run_web_server.sh` in the `templates/githubpages/` overlay rather than the generic TypeScript
overlay.

**Why.** Every Pages project uses TypeScript conventions, but not every TypeScript repository uses
the template's single-page esbuild bundle, preview server, deployment workflow, or build-aware
Playwright runner. Shipping those front doors to complex TypeScript repositories gives them
commands whose assumptions do not fit their build.

**Consequence.** A Pages consumer declares `REPO_TYPE=githubpages` and inherits TypeScript and
website content automatically. A generic `REPO_TYPE=typescript` consumer omits the four Pages
files. Its noexist `package.json` uses Playwright directly and does not advertise missing build or
serve scripts. Existing consumers require an explicit marker choice because the template cannot
infer whether their previously copied Pages files remain intentional.

**Owner.** [REPO_TYPE.md](REPO_TYPE.md) and
[build_github_pages.sh](../../templates/githubpages/noexist/build_github_pages.sh).

### Repo-owned Prettier ignores live in `.prettierignore.local`

**Decision.** Put repository-specific Prettier exclusions in the consumer-owned
`.prettierignore.local` file and pass it with `.gitignore` and `.prettierignore` to every Prettier
check and write command.

**Why.** A separate noexist file keeps shared and repository-owned exclusions distinct, mirrors
the established `eslint.config.local.js` convention, and uses Prettier's repeated `--ignore-path`
interface without adding a propagation merge format.

**Consequence.** Propagation ships `.prettierignore.local` once and never overwrites it. Existing
consumers update their owned `package.json` format script manually; the vendored aggregate check
updates automatically.

**Owner.** [templates/typescript/check_codebase.sh](../../templates/typescript/check_codebase.sh)

### Whole-file overwrite sources declare their ownership

**Decision.** Put the exact message, "This file is vendored. Local changes can and will be
overwritten by propagation.", in a native header comment on every source copied through the
whole-file overwrite, devel, or test bucket. Keep a required shebang first.

**Why.** A warning at the top is visible before a consumer edits a centrally maintained file, and
the same ownership rule applies regardless of the file's extension or propagation route.

**Consequence.** Noexist seeds and partial-ownership buckets omit the whole-file warning. A
plan-derived meta test checks every currently resolved canonical overwrite source across every
repository type.

**Owner.** [PROPAGATION_RULES.md](PROPAGATION_RULES.md#classification-criterion) and
[tests/meta/test_vendored_docs.py](../../tests/meta/test_vendored_docs.py).

### Development requirements have split ownership

**Decision.** Propagate `pip_requirements-dev.txt` through a dedicated bucket that owns a marked
universal package block and preserves the repository-specific local block.

**Why.** NOEXIST freezes shared dependencies at repository creation, while OVERWRITE would erase
packages each repository needs. Requirement-aware merging can refresh shared tools without
confusing comments, directives, extras, or version constraints with package identity.

**Consequence.** Marker-free consumers migrate automatically. Canonical package names identify
universal duplicates, the template specification wins during migration, and ambiguous ownership
markers stop the write.

**Owner.** [repolib/requirements_sync.py](../../repolib/requirements_sync.py) and
[REQUIREMENTS_BUCKET_SPEC.md](REQUIREMENTS_BUCKET_SPEC.md).

### Gitignore migration preserves local content

**Decision.** Recognize every shipped LOCAL-section banner and relocate its consumer-owned body
after the canonical managed blocks without changing the body's line order.

**Why.** Earlier propagation placed the LOCAL section before or between managed blocks. A canonical
rebuild can distinguish those rules from obsolete managed content only by recognizing the banner
that assigned their ownership.

**Consequence.** The parser retains compatibility with the previous full LOCAL heading, the legacy
short heading, and the older divided banner. Repositories converge to the current trailing LOCAL
layout on their next propagation.

**Owner.** [repolib/gitignore.py](../../repolib/gitignore.py) and
[GITIGNORE_SYSTEM.md](GITIGNORE_SYSTEM.md).

### Propagation records consumer maintenance

**Decision.** A successful, non-dry-run single-repository propagation that changes files adds one
canonical maintenance entry to the consumer's active changelog. No-op, dry-run, and failed runs
leave the changelog unchanged.

**Why.** Propagated maintenance belongs in the repository history, while unchanged or unsuccessful
runs have no maintenance event to record.

**Consequence.** The propagation writer preserves canonical changelog structure and remains
idempotent. Its entry stays compatible with the consumer's changelog query, rotation, and commit
tools even if implementation ownership changes later.

**Owner.** [propagate_style_guides.py](../../propagate_style_guides.py) and
[repolib/propagation_changelog.py](../../repolib/propagation_changelog.py).
