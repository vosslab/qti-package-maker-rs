# Tests

This folder holds permanent test lanes plus an ignored temporary workspace. Prefer fewer, stronger
permanent tests that protect behavior worth preserving. Use `tests/_temp/` for implementation proof,
then promote or remove each check before completing the plan. When in doubt, remove the test.

## Layout

```text
tests/
  test_*.py       permanent fast pytest tests
  test_*.mjs      permanent Node tests, when used
  _temp/          ignored temporary tests and one-time checks
  conftest.py     pytest configuration
  e2e/            permanent non-browser whole-system tests
  playwright/     permanent browser-driven tests
```

`tests/_temp/` is not a fourth test tier. Pytest-suitable temporary tests named `test_*.py`
participate in the normal `pytest tests/` run. Run heavier temporary checks and other formats
explicitly with their appropriate tool. `tests/conftest.py` excludes only `e2e` and `playwright`
from pytest collection.

## Run tests

- Fast pytest: `source source_me.sh && python3 -m pytest tests/`
- Focused pytest: `source source_me.sh && python3 -m pytest tests/test_<name>.py -q`
- Node: `node --test tests/test_<name>.mjs`
- Non-browser E2E: `bash tests/e2e/e2e_<name>.sh` or
  `source source_me.sh && python3 tests/e2e/e2e_<name>.py`
- Browser: use the repository's Playwright runner or explicit Playwright command.

The fast pytest command always runs the mandatory checkout and Podman disk-budget guards. Rust
repositories also run the mandatory `target/` disk-budget guard. These checks make responsible use
of the developer volume part of continuous development; do not move them to E2E, skip them, or
delete their vendored files.

## Plan closeout checklist

Before completing a plan, review its files under `tests/_temp/`:

- [ ] Promote tests whose behavior deserves lasting protection into a permanent lane.
- [ ] Remove checks that only proved implementation, debugging, migration, or rollout work.
- [ ] Confirm no plan-specific temporary checks remain.

The completion review is the cleanup gate. It has a direct failure response: classify every
remaining plan-specific check, then promote or remove it.

## Guidance

- [../docs/PYTEST_STYLE.md](../docs/PYTEST_STYLE.md) decides whether a permanent test should exist.
- [../docs/PYTEST_AUTHORING_GUIDE.md](../docs/PYTEST_AUTHORING_GUIDE.md) explains pytest
  construction and shared hygiene helpers.
- [../docs/E2E_TESTS.md](../docs/E2E_TESTS.md) explains permanent whole-system tests.
