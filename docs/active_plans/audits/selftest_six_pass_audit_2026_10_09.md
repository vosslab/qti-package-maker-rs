# Selftest six-pass audit

Date: 2026-10-09. All six fresh independent reviewers completed: Plan, Test, Style, Docs, Legacy,
and Comment. No blocker, high, or medium findings remain. Four low-severity findings were accepted;
two were corrected and two naming items remain open.

## Findings and disposition

| Severity | Finding | Evidence and impact | Disposition |
| --- | --- | --- | --- |
| Low | Package instructions duplicate transient tool versions | `docs/WASM_PACKAGE.md` disagreed with the package/release requirement and recorded runtime. It could suggest an unnecessary downgrade. | Fixed: setup docs reference manifest requirements rather than duplicating version numbers. |
| Low | ORDER browser assertion depends on the initial reversed layout | `packages/qti-wasm/tests/selftest.spec.ts`, in the interactive-controls test, named the expected first and second rows after a move. A valid initial-order change could fail it. | Fixed: assert the relative result of moving the last row up once, regardless of initial ordering. |
| Low | Verification report uses uppercase filename | `docs/active_plans/reports/SELFTEST_REGRESSION_VERIFICATION.md` conflicts with the active-plan snake_case rule in `docs/REPO_STYLE.md`. | Open naming cleanup: rename to `selftest_regression_verification.md` and update its links. |
| Low | Python-parity config uses a hyphen | `packages/qti-wasm/playwright.python-parity.config.ts` conflicts with the non-Markdown underscore naming rule. | Open naming cleanup: rename to `playwright_python_parity.config.ts` and update the npm script. |

The naming changes were left for commit preparation to preserve the existing index state. The
repository requires `git mv` for renames, and the verification report was untracked at audit time.
Neither naming issue affects runtime behavior or the reported checks.

The Test reviewer also questioned manual script replay. The coordinator retained replay and the
single-move behavior check because repeated initialization without duplicate listeners is an
explicit user requirement. The incidental initial-layout assumption was removed. No permanent
test was added. Current-Python browser tests remain an explicit transitional lane with a documented
retirement point, not the ordinary Rust/browser correctness suite.

## Pass results

- Plan: no findings after clarification. The reviewer withdrew an initial process concern after
  confirming that conversation approval satisfies the rule; no stored plan file is required.
- Test: the low-severity ORDER assertion finding above. Other changed tests protect meaningful
  identity, grading, host-hook, or archive-safety behavior. No new permanent test proposed.
- Style: no findings.
- Docs: the Node wording finding above. Documentation claims matched retained evidence.
- Legacy: no findings; historical repair modules have no remaining active references, and current
  Python remains isolated from ordinary verification.
- Comment: the two naming findings above; changed comments/docstrings were accurate and ASCII-safe.

The review boundary was the working-tree change relative to
`faf1161474e1ccd73f208e87a82e0d4f3d7b2d86`, plus the then-untracked verification report. Reviewers
read the repository/language/test rules and raw changed source. A local source hash receipt is
retained at `tests/_temp/selftest_audit_source_receipt.json`. No reviewer edited source or the index.

## Verification and limits

The implementation's retained evidence records 325 Rust tests, strict Clippy/formatting, 1,810
Python hygiene checks, nine Node tests, 30 browser tests, and three current-Python browser tests.
Those were inspected evidence, not fresh full-suite audit reruns. The Docs reviewer freshly ran
the Markdown-link checks: 79 passed.

After the low-risk cleanup, strict TypeScript passed and the affected interactive-controls test
passed in Chromium, Firefox, and WebKit (three tests). The first cleanup attempt compared changing
position labels as well as answers; the corrected assertion compares answer text. Markdown-link
checks passed again (80), as did shell syntax and diff whitespace checks. The audit did not rebuild
production artifacts or rerun full Rust/Node/browser suites. The handoff tarball and its hashes
remain those in the [verification report](../reports/SELFTEST_REGRESSION_VERIFICATION.md).

After the six reviews, the user clarified that transient versions should stay out of permanent
setup docs and that ungrounded version gates should not reject working software. The coordinator
updated README/install/package instructions to refer to manifests, removed release-script Rust,
Python, C++, and Node version checks, and retained version reporting for diagnosis. The Linux image
now derives its compiler tag from the workspace requirement instead of selecting an older compiler.
These follow-up script edits passed shell syntax checks; the container was not pulled or executed.
The six independent passes predate this narrow user-directed follow-up. Build, shim, and behavior
tests remain the compatibility checks. Cargo/package manifest requirements were not changed.

The actual website package-adoption and A-B-A reroll/persistence journey remains external acceptance.
Shipping Safari, Node 24, Linux, and native RDKit were not freshly exercised. Broader current-Python
migration findings remain visible and classified in the verification report; this audit does not
claim universal Python/Rust equivalence or release/deployment acceptance.
