# Tools and devel usage survey

## Scope

This survey examined every Git root beneath `/Users/vosslab/nsh` on 2026-09-02.
Filesystem discovery found 109 repositories: 72 first-party or top-level working repositories and
37 embedded repositories. The embedded set consists of prior-art clones under `OTHER_REPOS/` plus
`py-movie-media-manager/tinyMediaManager`; their layouts are evidence, not targets for template
policy or cleanup.

For every first-party repository, `git ls-files -- tools devel` supplied the tracked inventory.
The survey inspected filenames, script headers and docstrings, repository documentation, repeated
template files, and representative call sites. It classifies purpose and audience, using the
existing folder name as evidence rather than as the deciding criterion.

The first-party inventory contains 1,093 tracked support-directory entries:

| Location | Repositories | Tracked entries | Code files |
| --- | ---: | ---: | ---: |
| `tools/` | 44 | 303 | 285 |
| `devel/` | 71 | 790 | 731 |

Forty-four repositories use both directories, 27 use only `devel/`, and
`secure-agent-playbook-generic` tracks neither. No first-party repository uses only `tools/`.

## Settled distinction

Classify a command by the work it performs and the person it serves:

| Destination | Audience and purpose | Typical examples |
| --- | --- | --- |
| Application CLI or package | A normal, supported product workflow | Run the application, serve the site, export through the product API |
| `tools/` | A regular user performs an optional domain task | Convert a document, validate a playlist, transform user data, calculate a result |
| `devel/` | A maintainer engineers the repository itself | Git, changelog, version, release, dependency refresh, build, source generation, lint, benchmark, capture, diagnostic |
| Importable package | Several commands or tests reuse behavior | Parsers, geometry, render engines, shared runners, serializers |

Use the input and output as the tie-breaker. A command that consumes user data and produces a
domain result belongs in `tools/`. A command that consumes the checkout, source tree, tests,
package manifests, Git state, or internal fixtures and produces repository state or engineering
evidence belongs in `devel/`.

Treat folder placement as an entry-point choice, and put reusable behavior in an importable
package.

Place one native application, library, or helper package in a named folder at the repository root.
Use `packages/` when multiple native products or packages benefit from a shared grouping layer.

## Systemic findings

### Graphify belongs in devel

Seventeen first-party repositories track Graphify launchers under `tools/`: 15 Python launchers
and two legacy shell launchers. The current Python command installs or upgrades Graphify, selects
Claude CLI or Ollama, reads repository source, creates `graphify-out/`, benchmarks extraction, and
writes agent-manager context. Its input and output are repository-engineering artifacts, and its
audience is highly technical maintainers and coding agents.

The canonical launcher moved from `tools/graphify_map_repo.py` to
`devel/graphify_map_repo.py`. Propagation retires both historical tool paths.

### Dependency refresh belongs in devel

Twenty-one TypeScript repositories track `tools/sync_typescript_package_pins.py`. It queries npm,
rewrites dependency manifests, and prints lockfile and audit follow-up commands. This is dependency
maintenance, so the typed template now ships it at `devel/sync_typescript_package_pins.py` and
retires the former tool path.

### HTML-to-PDF belongs in tools

Twenty-one TypeScript repositories track `tools/html_to_pdf.mjs`. It accepts a file or URL and
produces a PDF, which is a direct user task. Nine of those repositories also track the identical
file at `devel/html_to_pdf.mjs`; every duplicate has the same SHA-256 digest as the canonical tool.
The canonical `tools/` location remains, and propagation retires the stale `devel/` path.

### Standard developer family is aligned

The repeated changelog, version, release, cleanup, setup, and packaging scripts are consistently
under `devel/`. `commit_changelog.py` appears in all 71 repositories with tracked `devel/` content;
the main version/changelog/cleanup family appears in 59-61 repositories. This is strong evidence
that `devel/` already functions as the maintainer lifecycle surface.

## Repository verdicts

The 27 devel-only repositories are aligned: `3VEE/3vee-server`, `3VEE/vossvolvox-cpp`,
`3VEE/vossvolvox-rust`, `LEGO/brick-collection`, `LEGO/goldberg-hexagon-measure`,
`LEGO/hilbert-curve-brick`, `LEGO/sorting-brick-strips`, `PROBLEMS/adapt-test-suite`,
`PROBLEMS/crazy-problem-extractor`, `PROBLEMS/webwork-pg-renderer`, `SWIFT/AppCloser-macos`,
`SWIFT/SwiftlyCodeEdit`, `SWIFT/airplay2tv`, `SWIFT/dockutil`,
`automated-radio-disc-jockey`, `battery-control`, `book-to-markdown`,
`codex-cli-account-switcher`, `codex-podman-container`, `codex-rules`, `easy-screenshot`, `energy`,
`m4b-merge-nvl`, `movie-slide-maker`, `play-hdhomerun-tui`, `py-movie-media-manager`, and
`slide-deck-pipeline`. Their repository-specific developer files are builds, setup helpers,
benchmarks, probes, release support, or engineering evidence.

The mixed repositories need the following placement:

| Repository | Verdict |
| --- | --- |
| `3VEE/vossvolvox-pages` | Keep HTML-to-PDF in tools; move grid benchmark, README capture, and dependency refresh to devel. |
| `LEGO/brick-inventory-label-maker` | Keep HTML-to-PDF in tools; move dependency refresh to devel. |
| `PROBLEMS/ADAPT-WeBWorK-Handbook` | Keep author-facing textbook conversions in tools; move capture, link checking, lint, extraction, and repository repair to devel. |
| `PROBLEMS/biology-problems-website` | Move all current tools to devel; they build, audit, dump, count, or reset repository-owned site data. |
| `PROBLEMS/biology-problems` | Keep direct question/SVG transformation utilities in tools; move changelog, audit, index, import, count, cleanup, and Graphify commands to devel. |
| `PROBLEMS/peptidyle-learning-engine` | Keep HTML-to-PDF in tools; move Graphify and dependency refresh to devel. |
| `PROBLEMS/pgml-opl-training-set` | Move renderer lint, batch lint, warning repair, and analysis commands to devel. |
| `PROBLEMS/qti-package-maker` | Keep BBQ and XML conversion utilities in tools; move Graphify and repository merge commands to devel. |
| `SWIFT/swift-usb-imager` | Move bundle, signing, notarization, and authorization probes to devel. |
| `TYPESCRIPT/andes-virus-outbreak-game` | Keep HTML-to-PDF in tools; move benchmark, capture, typecheck stub, and dependency refresh to devel. |
| `TYPESCRIPT/attack-on-cancer` | Keep HTML-to-PDF in tools; move builds, visual catalogs, captures, Graphify, and dependency refresh to devel. |
| `TYPESCRIPT/cancer-cell-mule` | Move repository-type detection to devel. |
| `TYPESCRIPT/cancer-clicker-ng` | Keep HTML-to-PDF in tools; move balancing, builds, captures, render verification, Graphify, typecheck, and dependency refresh to devel. |
| `TYPESCRIPT/concept-map-maker` | Keep contrast calculation and HTML-to-PDF in tools; move CSS policy checking and dependency refresh to devel. |
| `TYPESCRIPT/glycolysis-okimon` | Keep HTML-to-PDF in tools; move dependency refresh to devel. |
| `TYPESCRIPT/glycolysis-race-to-pyruvate` | Keep HTML-to-PDF in tools; move dependency refresh to devel. |
| `TYPESCRIPT/human-chemo-drug-simulation` | Keep HTML-to-PDF in tools; move dependency refresh to devel and retire the duplicate devel PDF command. |
| `TYPESCRIPT/image-gen-interface` | Keep HTML-to-PDF in tools; move Graphify and dependency refresh to devel. |
| `TYPESCRIPT/mule-game` | Keep HTML-to-PDF in tools; move balance reports, version labels, icon generation, and dependency refresh to devel. |
| `TYPESCRIPT/ncaa-school-find-game` | Keep HTML-to-PDF in tools; move dependency refresh to devel and retire the duplicate devel PDF command. |
| `TYPESCRIPT/pseudo-code-mapper` | Keep contrast calculation and HTML-to-PDF in tools; move CSS policy checking and dependency refresh to devel. |
| `TYPESCRIPT/science-choose-adventure` | Keep HTML-to-PDF in tools; move dependency refresh to devel and retire the duplicate devel PDF command. |
| `TYPESCRIPT/sports-life-game` | Keep HTML-to-PDF in tools; move asset extraction, simulations, dependency refresh, and game-balance helpers to devel. |
| `TYPESCRIPT/stem-lesson-quiz-game` | Keep HTML-to-PDF and intentional user data conversions in tools; move build/export, typecheck, content extraction, and dependency refresh to devel. |
| `TYPESCRIPT/super-bowling` | Keep HTML-to-PDF in tools; move dependency refresh to devel. Its extensive capture, measurement, probe, and verification suite is already correctly in devel. |
| `TYPESCRIPT/virtual-lab-protocol-simulation` | Keep explicit user input/output converters and editors in tools; move fixtures, harnesses, render reviews, diagnostics, censuses, smoke commands, Graphify, and dependency refresh to devel. Move reusable `svg_normalizer` behavior into an importable package. |
| `TYPESCRIPT/wnba-mystery-player-hunt` | Keep HTML-to-PDF in tools; move roster building, candidate fetching, difficulty simulation, and dependency refresh to devel. |
| `bkchem-oasa` | Keep intentional chemistry conversions and interactive domain utilities in tools; move calibration, conformance, license, build, capture, measurement, and test commands to devel. Move reusable `measurelib` behavior into the application package. |
| `claude-code-permissions-hook` | Move both enforcement and command-decision inspection scripts to devel. |
| `course-finder-grid` | Keep grid building, schedule reporting, and scheduler launch in tools; these are direct operator workflows. |
| `emwy-video-editor` | Move all six experiment, analysis, and model-export scripts to devel. |
| `exam-formatting-tools` | Move image-choice measurement to devel; it is implementation calibration. |
| `ferrum-chemical-forge` | Move Graphify to devel. |
| `iptv-filters` | Keep playlist validation in tools; it is a direct user utility. Move reusable validation behavior into the package as already identified by the import audit. |
| `junk-drawer` | Move Graphify to devel. |
| `local-llm-wrapper` | Move Graphify to devel. |
| `marp-slides` | Keep slide import/export commands in tools; move Graphify to devel. Move shared converter behavior into `marp_lib/` so tool scripts remain entry points. |
| `populous-python-nvl` | Keep an intentional standalone map viewer in tools; move smoke, screenshot, measurement, and diagnostics to devel. Move reusable headless behavior into `populous_game/`. |
| `protein-image-grader` | Keep archive-copy and image-hash utilities in tools; move Graphify to devel. |
| `screenshot-ai-renamer-macos` | Move Graphify to devel. |
| `starter-repo-template` | Move Graphify to devel and preserve the canonical distinction in propagated documentation. |
| `syllabus` | Move Graphify and table-layout calibration to devel. |
| `track-runner-virtual-dolly-cam` | Move Graphify, benchmarks, generated-help refresh, and mode-doc refresh to devel. Move reusable mode-doc behavior into `track_runner/`. |
| `vosslab-skills` | Keep the generic HTML-to-PDF command in tools; move index, manifest, plugin, sidecar, loaded-skill, plan, Graphify, and dependency-maintenance commands to devel. |

## Embedded repositories

The 37 embedded Git roots were inventoried but remain outside first-party cleanup. Thirty-five have
no tracked top-level `tools/` or `devel/` content relevant to this policy. The embedded
`ferrum-chemical-forge/OTHER_REPOS/bkchem-oasa` mirrors the first-party BKChem support layout, and
`play-hdhomerun-tui/OTHER_REPOS/textual` contains upstream tools. Their owners define their layout.

## Migration order

1. Publish the audience-and-purpose distinction in the canonical template docs.
2. Move Graphify and TypeScript dependency refresh to `devel/` in the template. Complete.
3. Keep HTML-to-PDF in `tools/` and retire its stale `devel/` path. Complete in template policy.
4. Propagate those three systemic corrections before repo-specific moves.
5. Apply repo-specific moves under each repository's own rules and tests.
6. Move reusable support-directory modules into importable packages before relocating their thin
   entry points.

This sequence fixes the source of future placement first, then lets consumer cleanup converge while
keeping each application refactor independently scoped.
