# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## Rust port

- Faster conversion is important for the Rust library to justify its existence; demonstrate
  performance benefits on practical workloads.

- Chromium must always run headlessly. Prefer sensible fixed internal behavior; add configuration
  only for demonstrated caller needs. When in doubt, use the simpler shared design.

- Use Chromium for HTML-to-image. Python table rendering is not a visual target: it also
  needed improvements and remains poor. Prioritize readable tables and preserved content.

- HTML-to-image is a moving target; do not maintain our own HTML renderer. Use an
  established rendering engine. Existing sugar-library PNG/SVG exports should replace HTML sugars.
- The 2026-10-01 gallery review rejects two real rendering failures, excludes four malformed
  HTML inputs, and excludes two obsolete HTML sugars. Exact IDs are recorded in the progress ledger.

- Prefer the vendored `devel/bump_version.py -cA`. `26.09` and `26.9` mean the same version.
  Focus review on correctness, maintainability, validation, and delivery; avoid bikeshedding.

- The goal is to work like the Python package, not reproduce every exact nuance of the
  parent package. Judge parity by useful workflows, content, grading, media, interoperability,
  and readability; accept harmless implementation and presentation differences.

- We can implement our own code as well. Consider a small local implementation alongside
  dependencies when it makes the overall design simpler.

- I prefer to use the latest versions of things. Prefer current dependency releases and
  adapt to their APIs when practical, while retaining reproducible locked builds.

- Apply KISS aggressively: prefer the smallest coherent design for actual requirements and
  known failure modes. Complexity must earn its place.
- Treat tests as liabilities as well as assets. Use the `docs/PYTEST_STYLE.md` checklist;
  keep rebuild proof in ignored `tests/_temp/`, and when in doubt remove the permanent test.
- New behavior gates need a concrete failure plan. Avoid arbitrary thresholds, unnecessary
  byte/pixel equivalence, and exhaustive matrices without a product requirement.

- Implement the plan in `docs/active_plans/majestic-shimmying-tiger.md` to make a feature parity
  version of qti-package-maker in Rust.
- Have an independent agent run the readme-docs skill so we have something on GitHub.
- Use the rust-code-expert skill; it might be helpful. Other Rust books are available in
  `~/Documents/teaching/MARKDOWN_BOOKS/rust-code/`.
- The hierarchical folder design for the Python qti-package-maker has been really good.
  Preserve that organization in the Rust port.
- Rust is different than Python, so the folder design will require some translation, but there
  is something to be learned from it.

- 2026-09-30: Prefer `devel/make_release.py` for manual releases instead of a GitHub release
  workflow; remove the Rust workflow and retain local validation.

- 2026-09-30: Avoid overly strict requirements such as byte or pixel equivalence and arbitrary
  timing cutoffs. Make sure plan gates and requirements are grounded in reality.

- 2026-09-30: Uploads must have valid XML and valid package structure. Keep this practical
  requirement alongside correct grading and media references; distinguish local package
  validation from an actual LMS import result.
