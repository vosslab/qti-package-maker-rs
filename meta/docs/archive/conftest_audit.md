# conftest.py audit across ~/nsh

Regenerated 2026-07-03. Scope: first-party `tests/conftest.py` under `~/nsh`.
Excludes OLD_CODE, venvs, site-packages, node_modules, vendored `external/`,
`cinemagoer/`. 52 conftest files reviewed.

This audit reconstructs a prior review that was lost. It maps each repo's
conftest against the current propagation model: a single managed
`collect_ignore` block plus a repo-local `REPO_HYGIENE_FILTERS` dict, with
repo-root sys.path injection via `file_utils.get_repo_root()` (git rev-parse).

## Axes tracked

| Axis | Canonical (current) | Drift seen |
| --- | --- | --- |
| collect_ignore | `["e2e", "playwright"]` managed block | absent (empty file); `+ "meta/e2e"`; GUI repo uses none |
| REPO_HYGIENE_FILTERS | present, `{}` unless needed | absent entirely; only 1 repo populated |
| repo-root sys.path | `file_utils.get_repo_root()` (git) | `git_file_utils`, raw subprocess, `__file__`-derived, custom walk, none |
| hygiene fixtures | dropped (moved to helpers) | legacy `--no-ascii-fix` + `skip_repo_hygiene` + `ascii_fix_enabled` cluster retained |
| OPTIONAL_HELPERS_MENU | present, all-commented | absent in older/stale files |

## Group 1: empty conftest (7) -- HIGHEST DRIFT

Zero bytes. No `collect_ignore`, so if any of these grows a `tests/e2e/` or
`tests/playwright/` tree, those slow tests get collected by `pytest tests/`.

- `3VEE/3vee-server`
- `3VEE/vossvolvox-rust`
- `homebrew-top-new`
- `PROBLEMS/vosslab-webwork-pg-set`
- `PROBLEMS/webwork-pg-renderer`
- `rust-dns-benchmark`
- `vosslab-podcast`

Fix: propagate at least the managed `collect_ignore` + empty
`REPO_HYGIENE_FILTERS` block, or confirm each is intentionally test-free.

## Group 2: legacy hygiene-fixture style (4) -- STALE

Carry the old `repo_root` fixture + `pytest_addoption("--no-ascii-fix")` +
`skip_repo_hygiene` + `ascii_fix_enabled` cluster. NO `collect_ignore`, NO
`REPO_HYGIENE_FILTERS`, NO OPTIONAL_HELPERS_MENU. Repo root is
`__file__`-derived, which violates REPO_STYLE.md (git rev-parse rule). Some do
not even insert repo root onto sys.path.

- `3VEE/vossvolvox`
- `AppCloser-macos`
- `llm-file-rename-n-sort` (REPO_ROOT computed, never inserted to sys.path)
- `PROBLEMS/ADAPT_WeBWorK_Handbook` (same, never inserted)

Fix: re-propagate to current model; drop the ascii-fix/skip fixtures if the
vendored tests no longer read them.

## Group 3: mixed -- hygiene fixtures BUT with collect_ignore (7)

Have the managed `collect_ignore` block yet still keep the legacy hygiene
fixture cluster. Partial migration.

- `automated-radio-disc-jockey` (also registers `slow` marker via pytest_configure)
- `automated-radio-disc-jockey/local-llm-wrapper` (nested pkg; no collect_ignore, fixtures only)
- `local-llm-wrapper`
- `brick-collection`
- `energy` (re-exports `git_file_utils.HYGIENE_SKIP_DIRS`)
- `emwy-video-editor` (sets PYTHONPATH for subprocesses)
- `protein-image-grader` (also has REPO_HYGIENE_FILTERS + menu -- furthest along in this group)

## Group 4: current canonical model (many)

`collect_ignore` + `REPO_HYGIENE_FILTERS = {}` + OPTIONAL_HELPERS_MENU. sys.path
via `file_utils.get_repo_root()` where present.

- `airplay2tv` (collect_ignore adds `meta/e2e`)
- `brickarchitect-label-converter`
- `hilbert-curve-brick`
- `claude-code-permissions-hook`
- `starter-repo-template` (source of truth)
- `TYPESCRIPT/stem-lesson-quiz-game`
- `TYPESCRIPT/virtual-lab-protocol-simulation`
- `TYPESCRIPT/sports-life-game`
- `TYPESCRIPT/ncaa-school-find-game` (collect_ignore adds `meta/e2e`)
- `usb-cable-rater`
- `vosslab-skills`
- `track-runner-virtual-dolly-cam` (no menu; `__file__`-derived root)

Note: several in this group inject sys.path via raw `os.path`/`__file__` or
`git_file_utils` rather than the canonical `file_utils.get_repo_root()`.

## Group 5: only the managed block (4) -- thin but correct

`collect_ignore` present, nothing else (or minimal sys.path).

- `PROBLEMS/PG_v2.17` (5 lines, block only)
- `scientific-skills` (5 lines, block only)
- `TYPESCRIPT/brick-inventory-label-maker` (5 lines, block only)
- `PROBLEMS/pgml-opl-training-set` (no collect_ignore; only `__file__` sys.path)

## Group 6: the one populated hygiene filter (1)

- `TYPESCRIPT/concept-map-maker`: `REPO_HYGIENE_FILTERS = {"all": ["vendor/**"]}`
  to exclude Font Awesome woff2/CSS from ASCII + lint scans. This is the
  reference example of a legitimate Layer 2 exclusion.

## Group 7: bespoke / multi-package (rest)

Custom sys.path or fixtures that legitimately deviate:

- `bkchem-oasa` (root): custom `_find_repo_root` walking for `AGENTS.md` +
  `docs/REPO_STYLE.md` instead of git rev-parse. Deviates from REPO_STYLE.
- `bkchem-oasa/packages/bkchem-app`: Tk singleton bootstrap fixture.
- `bkchem-oasa/packages/bkchem-qt.app`: PySide6 offscreen QApplication +
  MainWindow fixtures; visual-hold env vars; NO collect_ignore (GUI suite).
- `bkchem-oasa/packages/oasa`: `--save` option; package sys.path.
- `emwy-video-editor/emwy_tools`: multi-subpackage sys.path fan-out.
- `PROBLEMS/biology-problems`: MPLCONFIGDIR set by hand; `stub_bptools`
  fixture; injects `~/nsh/qti_package_maker` onto sys.path.
- `PROBLEMS/biology-problems-website`: adds `tools/` to sys.path.
- `PROBLEMS/qti-package-maker`: rich sample-item/BBQ fixtures + `tmp_cwd`.
- `populous-python-nvl` (+ `tests/parity`): headless SDL env for pygame;
  seeded `rng` fixture.
- `apple-foundation-models-2`: sets PYTHONUNBUFFERED/PYTHONDONTWRITEBYTECODE
  and Apple-Intelligence availability skip; `session` fixture.
- `slide-deck-pipeline`: minimal `__file__` sys.path, no collect_ignore.

## Cross-cutting findings

1. Root-detection is not uniform. Canonical is `file_utils.get_repo_root()`
   (git rev-parse). In the wild: `git_file_utils.get_repo_root()`, raw
   `subprocess` git, `__file__`-derived `os.path`, and one AGENTS.md/REPO_STYLE
   marker walk (`bkchem-oasa`). REPO_STYLE.md mandates git rev-parse.
2. Two helper module names coexist: `file_utils` and `git_file_utils`. Worth
   settling on one name repo-wide.
3. `apple-foundation-models-2` and `biology-problems` set env vars
   (PYTHONUNBUFFERED, PYTHONDONTWRITEBYTECODE, MPLCONFIGDIR) directly in
   conftest. The OPTIONAL_HELPERS_MENU explicitly says the first two belong in
   `source_me.sh`, and MPLCONFIGDIR has a ready-made Recipe 1. Migrate.
4. Only `concept-map-maker` populates `REPO_HYGIENE_FILTERS`; every other
   populated-style file leaves it `{}`. Registry adoption is healthy.
5. `collect_ignore` has a `meta/e2e` variant in `airplay2tv` and
   `ncaa-school-find-game` only. Confirm whether that third entry should be in
   the managed block for all repos or stay repo-specific.
6. 7 empty + 4 stale = 11 repos not on the current propagation model. These are
   the priority targets for a propagation sweep.

## Recommended next steps

- Run the propagation sweep against Groups 1-3 to bring 11 repos current.
- Normalize root detection to `file_utils.get_repo_root()` where a bespoke
  reason does not exist.
- Move env-var setup out of conftest into `source_me.sh` / Recipe 1.
- Decide the `meta/e2e` collect_ignore entry's scope (managed vs per-repo).
