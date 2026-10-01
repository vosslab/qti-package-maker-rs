# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## Rust port

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
