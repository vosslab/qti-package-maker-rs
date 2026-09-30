# Support-directory package-import survey

## Scope and method

This static survey was completed before in-repo remediation on 2026-09-02. The
candidate set is every repository directly under `/Users/vosslab/nsh/` with a top-level `tools/`
directory: 18 repositories at the time of the scan. For each candidate, the
scan examined every `*.py` file outside `.git/` for ordinary Python `import`
and `from ... import` statements whose package root is `tools`, `devel`, or
`tests`. It does not claim to detect dynamically constructed imports.

"Clean" means that this static scan found no such package-root statement; it
does not mean that the repository has no Python source or cannot perform a
dynamic import. Each clean verdict includes a source anchor in the scanned
repository and its Python-file count. `secure-agent-playbook-generic` is a
candidate because its `tools/` directory exists, though that directory has no
Python source.

The settled boundary is applied as written: `tools/`, `devel/`, and `tests/`
are not library package namespaces. A violation remains a violation even if it
currently provides a useful API. Its maintainer should move reusable behavior
into the named importable package and retain a thin standalone entry point.

## Scan-time verdicts

| Repository | Static verdict and file:line evidence | Modules and migration direction |
| --- | --- | --- |
| `bkchem-oasa` | **Violation:** `tests/test_render_beta_sheets.py:12` imports `tools.render_beta_sheets`. | Move reusable `render_beta_sheets` behavior into the installed `oasa` package (for example `packages/oasa/oasa/render_lib/`); keep `tools/render_beta_sheets.py` thin. |
| `claude-code-permissions-hook` | **Clean:** 30 Python files scanned; no `tools.*`, `devel.*`, or `tests.*` import. Scope anchor: `tools/run_command_decisions.py:1`. | None. |
| `course-finder-grid` | **Clean:** 72 Python files scanned; no package-root support-directory import. Scope anchor: `tools/email_schedule_report.py:1`. | None. |
| `emwy-video-editor` | **Clean:** 128 Python files scanned; no package-root support-directory import. Scope anchor: `tools/export_yolo_onnx.py:1`. | None. |
| `exam-formatting-tools` | **Clean:** 49 Python files scanned; no package-root support-directory import. Scope anchor: `tools/measure_image_choices.py:1`. | None. |
| `ferrum-chemical-forge` | **Clean:** 466 Python files scanned; no package-root support-directory import. Scope anchor: `tools/graphify_map_repo.py:1`. | None. |
| `iptv-filters` | **Violation:** `tests/test_validate_m3u.py:10` imports `tools.validate_m3u`. | Move reusable `validate_m3u` behavior into `iptv_filters/`; keep `tools/validate_m3u.py` thin. |
| `junk-drawer` | **Clean:** 304 Python files scanned; no package-root support-directory import. Scope anchor: `tools/graphify_map_repo.py:1`. | None. |
| `local-llm-wrapper` | **Clean:** 60 Python files scanned; no package-root support-directory import. Scope anchor: `tools/graphify_map_repo.py:1`. | None. |
| `marp-slides` | **Violation:** tests import `tools.odp_to_marp` (`tests/test_odp_to_marp.py:12`), `tools.pptx_to_marp` (`tests/test_odp_to_marp.py:13`, `tests/test_pptx_to_marp.py:16`); runtime scripts import `tools.odp_visibility` and `tools.pptx_to_marp` (`tools/odp_to_marp.py:21-22`) and `tools.odp_to_marp` (`tools/odp_visibility.py:145`). | Move `odp_to_marp`, `odp_visibility`, and `pptx_to_marp` reusable behavior into `marp_lib/`; keep the three `tools/` files as thin entry points. |
| `populous-python-nvl` | **Violation:** tests import `tools.headless_runner` at `tests/test_click_hits_visible_tile.py:20`, `tests/test_canvas_layout.py:17`, `tests/test_debug_layout_overlay_matches_transform.py:19`, and `tests/test_effect_sfx_toggle.py:8`; runtime source also imports it at `populous_game/cli.py:244` and smoke entry points such as `tools/smoke/debug_layout.py:31`. | Move reusable `headless_runner` behavior into `populous_game/`; keep `tools/headless_runner.py` and smoke scripts thin. |
| `protein-image-grader` | **Clean:** 96 Python files scanned; no package-root support-directory import. The direct script-test precedent is `tests/test_copy_archive_images.py:8-15`, which builds a file-path module specification for `tools/copy_archive_images.py` rather than importing `tools.*`. | None. Use this file-path loading pattern when a test must exercise a standalone script without making `tools/` a package. |
| `screenshot-ai-renamer-macos` | **Clean:** 54 Python files scanned; no package-root support-directory import. Scope anchor: `tools/graphify_map_repo.py:1`. | None. |
| `secure-agent-playbook-generic` | **Clean:** 11 Python files scanned; no package-root support-directory import. `tools/` contains no Python file; scope anchor: `install.py:1`. | None. |
| `starter-repo-template` | **Remediated after scan:** test imports included `tools.graphify_map_repo` (`tests/meta/test_graphify_map_repo.py:12`) and `devel.bump_version` (`tests/meta/test_bump_version.py:9`), `devel.changelog_lib` (`tests/meta/e2e/e2e_header_bucket.py:39`, `tests/meta/test_propagate_cli.py:10`), and `devel.flatten_broken_md_links` (`tests/meta/test_flatten_broken_md_links.py:8`); runtime source imported `devel.changelog_lib` at `propagate_style_guides.py:10`. | Reusable behavior moved behind package or trusted file-path boundaries; the current support-directory gate passes. |
| `syllabus` | **Clean:** 53 Python files scanned; no package-root support-directory import. Scope anchor: `tools/graphify_map_repo.py:1`. | None. |
| `track-runner-virtual-dolly-cam` | **Violation:** `tests/ui/test_keymap.py:13` imports `tools.refresh_mode_docs`. | Move reusable `refresh_mode_docs` behavior into the existing `track_runner/` package; keep `tools/refresh_mode_docs.py` thin. |
| `vosslab-skills` | **Clean:** 84 Python files scanned; no package-root support-directory import. Scope anchor: `tools/build_agents_index.py:1`. | None. |

## Required plan findings and historical drift

The four original plan-named `tools.*` importers remain present and are test
files:

- `populous-python-nvl/tests/test_click_hits_visible_tile.py:20` imports
  `tools.headless_runner`.
- `iptv-filters/tests/test_validate_m3u.py:10` imports `tools.validate_m3u`.
- `marp-slides/tests/test_pptx_to_marp.py:16` imports `tools.pptx_to_marp`.
- `track-runner-virtual-dolly-cam/tests/ui/test_keymap.py:13` imports
  `tools.refresh_mode_docs`.

Fresh evidence does **not** support the plan's historical statement that no
runtime code imports `tools/`. Runtime imports now exist in
`populous-python-nvl/populous_game/cli.py:244`; they also occur between
`marp-slides` tool scripts (`tools/odp_to_marp.py:21-22` and
`tools/odp_visibility.py:145`) and in `populous-python-nvl` smoke scripts (for
example `tools/smoke/debug_layout.py:31`). The report therefore distinguishes
the four plan-named *test* importers from the broader current result.

No static `tests.*` package-root import was found in any of the 18 candidates.
At scan time, the only static `devel.*` findings were the four `starter-repo-template`
uses listed in its row; those uses were subsequently remediated. These statements cover ordinary
import syntax only.

## Follow-up ownership

This is detection and migration direction, not a consumer-repository repair.
Each consumer maintainer owns its move. WP-9a should use the migration pattern
here in `tools/TOOLS_README.md`; WP-10a should install the import gate only
after treating these current violations as intentional detection targets.
