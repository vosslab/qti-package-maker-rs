# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## Source organization

- 2026-10-08: When source files approach a size cap, split them into focused modules and preserve
  useful comments and behavior; do not squeeze comments just to meet the cap.

## Rust port

- 2026-10-08: I want the native CLI, WebAssembly package, and generated TypeScript API to use the
  same Rust parsing, validation, and media-writer implementation. Keep the native CLI free of Node
  prerequisites.
- 2026-10-07: For the PLE handoff, QPM supplies canonical question content and correct-answer
  information. PLE owns student presentation and any shuffling; QPM adds no permutations.
- 2026-10-07: Ordinary FIB answers ignore capitalization and incidental whitespace. `DNA`,
  `dNa`, and `DnA` should match; PLE may simplify its matching model separately.
- 2026-10-07: Treat the Native JSON handoff files as task inputs and focus on the writer. Leave
  their Git disposition alone except for moves required by implementation.
- 2026-10-07: Record PLE problems in a QPM-discovered PLE issues document, then continue the
  writer. The issues document is separate from the QPM implementation specification.
- 2026-10-07: Reuse QPM's table-to-image and media pipeline for assets it already produces.
  Record unsupported inline images as a separate QPM capability gap.
- 2026-10-07: Complete this plan with manager and subagents. Use independent agent assessments
  and automated tests in place of human gates; prefer more, smaller milestones.

- Preserve Graphify's shrinkage safety check. Compare equivalent graph representations
  so legitimate normalization does not trigger it; verify a completely fresh mapping run.
- Avoid maintaining patches to third-party code or wrapper hacks around upstream faults;
  future upstream fixes should not create conflicts or break our workflow.

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

- 2026-10-09: I like to keep dependencies down to reduce the supply chain attack surface.

- 2026-10-09: Preserve Python selftest behavior in Rust, including stable question CRCs and
  independent completion for each variant. This repository owns generated HTML and grading hooks;
  the website manager owns reroll selection and persisted progress.
- 2026-10-09: Rust QPM should be up to date with current Python QPM during migration. Turn important
  behaviors into independent Rust tests, then retire Python from active workflows. Rust QPM is
  intended to become the sole maintained, authoritative implementation.
- 2026-10-09: Keep configuration simple. Add options only for demonstrated needs; prefer fixed
  shared behavior for internal choices and separate commands for genuinely different tasks.
- 2026-10-09: I am the only one using this code; I do not want to preserve legacy behavior.
- 2026-10-09: Prefer newer Cargo package versions; never pin old versions.
- 2026-10-09: Do not put transient dependency/toolchain versions in permanent setup docs.
  Refer to the package manifests; keep exact tested versions in dated verification evidence.
- 2026-10-09: Avoid version-based gates without a demonstrated compatibility need. A different
  tool version alone should not stop working software; use meaningful build and behavior checks.

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
