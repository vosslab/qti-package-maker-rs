# starter_repo_template
Canonical bootstrap scaffolding for new Python repositories: ready-to-use repo policy docs, Python style rules, licensing boundaries, and pytest lint checks so projects start consistent before any project-specific code is added.

Only `README.md` and `docs/CHANGELOG.md` are intentionally repository-specific; every other file is designed to remain generic for downstream template users.

## Documentation

- [meta/docs/REPO_TYPE.md](meta/docs/REPO_TYPE.md): Repository type marker format, available
  types, inheritance, and multi-type behavior.
- [meta/docs/LICENSE_POLICY.md](meta/docs/LICENSE_POLICY.md): Canonical license filenames,
  complete-text sources, GitHub detection behavior, legacy propagation, and release verification.
- [docs/REPO_STYLE.md](docs/REPO_STYLE.md): Repository structure, naming, versioning, dependency manifest, and licensing conventions.
- [docs/PYTHON_STYLE.md](docs/PYTHON_STYLE.md): Python implementation rules for formatting, structure, imports, argparse, and testing.
- [docs/PYTEST_STYLE.md](docs/PYTEST_STYLE.md): Policy and checklist for deciding whether a
  permanent test should exist.
- [docs/PYTEST_AUTHORING_GUIDE.md](docs/PYTEST_AUTHORING_GUIDE.md): Construction conventions for
  permanent pytest tests.
- [tests/TESTS_README.md](tests/TESTS_README.md): Test layout, commands, and temporary-test
  lifecycle.
- [docs/E2E_TESTS.md](docs/E2E_TESTS.md): Permanent non-browser and browser whole-system tests.
- [templates/website/docs/PLAYWRIGHT_TEST_STYLE.md](templates/website/docs/PLAYWRIGHT_TEST_STYLE.md):
  Permanent browser-test checklist and authoring guidance.
- [templates/website/docs/PLAYWRIGHT_USAGE.md](templates/website/docs/PLAYWRIGHT_USAGE.md):
  Playwright installation and execution.
- [docs/MARKDOWN_STYLE.md](docs/MARKDOWN_STYLE.md): Markdown writing and formatting conventions for repository documentation.
- [docs/AUTHORS.md](docs/AUTHORS.md): Canonical authorship and attribution metadata for template maintenance.
- [docs/CHANGELOG.md](docs/CHANGELOG.md): Repository-specific history of updates to this template.

## Template layout

File location is the primary routing determinant. Files under `docs/`, `tests/`, `devel/`, and `tools/` ship universally to every consumer repo. Files under `templates/<type>/` (e.g., `templates/typescript/`, `templates/python/`, `templates/rust/`) ship only to repos of that type. Root-level files ship only when listed in `ROOT_PROPAGATE_ALLOWLIST`. Template-only tooling lives under `meta/tools/`; it never propagates and is removed by `reset_repo.py` at consumer bootstrap. Propagation manifests live in `meta/propagation/manifests.yaml` (template-only; never ships to consumers).

## Quick start

Bootstrap a fresh clone (sets project type + licenses, installs canonical files):

```bash
source source_me.sh && python3 reset_repo.py
```

The interview asks for repo types, licenses, PyPI packaging, then one finish decision covering
staging, commit, and push.
Repo types accept names, comma-separated lists, or letter runs (`python`, `python,rust`, or `pr`).
`--dry-run` previews the reset; `--config <file>` loads answers from JSON.

Preview and then update an existing consumer repository:

```bash
source source_me.sh && python3 propagate_style_guides.py -n -R ../consumer-repo
source source_me.sh && python3 propagate_style_guides.py -R ../consumer-repo
```

Propagation includes the conservative legacy-license migration described in
[meta/docs/LICENSE_POLICY.md](meta/docs/LICENSE_POLICY.md#legacy-migration-during-propagation).
A successful non-dry-run that changes repository files also records the maintenance in the
consumer's `docs/CHANGELOG.md`; previews and no-op runs leave that changelog unchanged.

Run the fast test suite:

```bash
source source_me.sh && pytest tests/
```

Non-browser end-to-end tests live under `tests/e2e/` per [docs/E2E_TESTS.md](docs/E2E_TESTS.md) when present; this repo does not currently ship any. Each runner is self-contained -- invoke them individually with `bash tests/e2e/e2e_<name>.sh`.

Run browser-driven Playwright tests (see [templates/website/docs/PLAYWRIGHT_USAGE.md](templates/website/docs/PLAYWRIGHT_USAGE.md)):

```bash
node tests/playwright/test_example.mjs
```
