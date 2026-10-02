# Test lanes

This folder holds durable repository checks and an ignored temporary workspace. Rust tests also
live beside their owning crate. Prefer fewer, stronger permanent tests that protect behavior worth
preserving. Use `tests/_temp/` for implementation proof, then promote or remove each check before
closing a workstream.

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

Rust test ownership follows the workspace layout:

```text
crates/*/src/       focused unit tests in #[cfg(test)] modules
crates/*/tests/     crate integration and process-contract tests
xtask/src/          development-tool unit tests
tests/              repository policy and Python development checks
tests/_temp/        ignored receipts, probes, and one-time investigations
```

## Three test classes

The Rust-port plan uses three classes with different purposes and retention rules.

| Class | Location and cadence | What belongs there |
| --- | --- | --- |
| Permanent contract | `cargo test` and repository pytest lanes | validation, CRC, readers/writers, integrity, CLI process contracts, layout geometry, parser rules, conversion ordering and typed failures |
| Tooling evidence | explicit `cargo xtask` commands and CI environments that provide their inputs | corpus harvest, Python-oracle parity, benchmark runs, package checks, and gallery generation |
| One-time proof | ignored `tests/_temp/` artifacts | browser receipts, spike output, baseline measurements, and a reviewer-facing gallery until its decision is recorded |

Do not promote a screenshot, timing receipt, or debugging probe merely because it passed once.
Promote a regression that protects a stable public or safety contract. Do not describe tooling or
one-time proof as a substitute for the permanent contract lane.

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

For Rust work, run focused crate tests while iterating, then use the workspace gates appropriate to
the change:

```bash
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo fmt --check
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo test --workspace --all-targets
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo clippy --workspace --all-targets -- -D warnings
```

`cargo xtask --help` lists the corpus, benchmark, gallery, integrity, and parity commands. These
require their documented local oracle or renderer inputs; production Rust binaries do not require
Python or a browser at runtime.

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
- [../docs/PARITY.md](../docs/PARITY.md) records completed parity evidence and remaining gates.
- [../docs/archive/rust_port_plan.md](../docs/archive/rust_port_plan.md)
  defines the Rust-port acceptance criteria.
