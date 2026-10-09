# Selftest parity repair audit

All six fresh independent passes completed: Plan, Test, Style, Docs, Legacy, and Comment.
The boundary covers the HTML selftest writer and CSS, its direct random-library dependency,
changed browser tests, and parity documentation. Unrelated portable rendering changes are excluded.
No production correctness defect was identified. The audit found test and maintenance issues.

## Findings and disposition

- Medium, fixed: the desktop embedded-container check compared overflow only with the full
  viewport, so content could escape the narrow question and still pass. The existing scientific
  content test now checks the question container's own scroll width against its client width.
- Medium, proposed: permanent checks do not detect a return to constant MATCH/ORDER ordering.
  The existing Rust token-pairing test could cover a small fixed seed set, reproducibility, and
  more than one arrangement, without requiring exact permutations. This is a coverage gap,
  not evidence that the implemented shuffle fails. No permanent test was added in this audit.
- Low, fixed: the browser test pinned reduced-motion transform mechanics and the hidden feedback
  heading's geometry. Those implementation-proof assertions were removed; compact layout,
  themes, assignment colors, content preservation, grading, and Reset checks remain.
- Low, fixed: MATCH slots received flex/scroll declarations that later rules overrode. Those
  declarations now apply only to the choice control, with no intended presentation change.
- Low, fixed: removed a stale transform-origin comment and clarified that concurrently mounted
  questions have distinct CRC-keyed hooks. The synthetic duplicate-instance probe is not a
  demonstrated BPW defect and is not grounds for architectural changes.

The proposed seed check satisfies the meaningful-behavior, plausible-regression, and nonduplicate
coverage criteria in [PYTEST_STYLE.md](../../PYTEST_STYLE.md): constant ordering was a demonstrated
regression, and exact permutations are not a requirement. Failure would mean investigating and
restoring seed-dependent presentation while preserving identity. It remains a proposal for human
approval, not an added release gate or a new test suite.

## Review disagreements

Plan questioned the restored interaction styling as unrequested enhancement. Inspection of the
Python reference's `html_selftest/html_functions.py` shows the same button hover/pressed/disabled
states, reduced-motion rules, and 44px minimum. Its system-dark rule followed by explicit body-theme
selectors also establishes the restored theme precedence. Keep these restorations; their exact
dimensions and implementation are not new public requirements. The associated brittle test
assertions were nevertheless removed as the Test pass recommended.

Comment and Docs disagreed about the provenance sentence for BPW progress scope. The sentence
was unnecessary to the design decision and was removed. Nothing was added to human guidance.

The Test pass said the temporary seed proof was gone; the coordinator confirmed that
`output_selftest_parity/verify.mjs` and `checks.json` still retain it. That historical proof does
not close the permanent coverage gap, and does not justify adding tests without approval.

## Pass results

- Plan: interaction-scope concern resolved by Python source evidence; no architectural finding.
- Test: two coverage findings and brittle implementation assertions, as described above.
- Style: no findings.
- Docs: no findings; package hash and contents confirmed.
- Legacy: redundant CSS declarations corrected.
- Comment: misleading comments corrected and disputed provenance sentence removed.

## Verification and limits

The prior handoff records 333 Rust tests, 12 Node tests, 21 focused browser checks, and 1,855
hygiene checks. These remain historical evidence, not fresh full-suite audit runs.
The coordinator confirmed the delivered tarball still has the recorded SHA-256 in
[qpm_selftest_handoff_2026_10_09.md](../reports/qpm_selftest_handoff_2026_10_09.md).

Fresh post-cleanup validation: the WASM package build and TypeScript checks passed; all 21
selftest browser checks passed across Chromium, Firefox, and WebKit against current native and
rebuilt WASM output. Markdown links and diff whitespace checks passed. Full Rust, Node, and
repository hygiene suites were not rerun for this CSS/test/documentation cleanup.

No new package replaces the delivered
tarball during this audit. That artifact predates only the audit's redundant CSS/comment cleanup;
no grading, Reset, CRC, answer order, or intentional renderer behavior changed. BPW's reported
integration checks are external evidence, not checks rerun by these reviewers.

Authored scientific-table contrast remains a separate source-content issue. The audit does not
review biology generator changes, website CSS/lifecycle, or the unrelated portable renderer.
The original package remains a disclosed local build from an uncommitted checkout.
