# Design decisions

### Graphify normalization and overwrite protection

**Decision.** Keep the reclustering correction upstream in Graphify. The tested
candidate compares independently normalized disk and candidate graphs, preserving
canonical node IDs and remapped links.

**Why.** Cargo workspace aliases legitimately reconcile with AST package nodes;
raw JSON counts falsely report data loss, while stale link endpoints drop dependencies.

**Consequence.** Unexpected canonical-node loss still refuses overwrite in the candidate.
This converter does not force writes, maintain a dependency fork, or patch packages at runtime.
The released Graphify 0.9.80 still has the fault pending an upstream correction.

**Owner.** Graphify `build_from_json`, `export.to_json`, and `cluster-only`/`label`;
[GRAPHIFY_CLUSTER_NORMALIZATION.md](GRAPHIFY_CLUSTER_NORMALIZATION.md).

- 2026-09-30: QTI 2.1 serialization decodes HTML named entities using the existing
  `markup5ever` table before preserving XML escaping. Raw HTML-only entities such as
  `&alpha;` cannot remain in assessment XML; serialization preserves source item identity.

<!-- VENDORED HEADER: START -->
Record each durable decision about how this code and repository are shaped, once it is settled, with
the reasoning a later reader needs. Guidance Neil Voss states belongs in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), dated history in `docs/CHANGELOG.md`, open discussion in
`docs/active_plans/decisions/`. [PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

Write each decision as a level-three heading with these four fields. `Owner` names the
authoritative code or contract document, rather than a person.

```markdown
### <decision title>

**Decision.** <the durable direction>

**Why.** <the reason it was chosen>

**Consequence.** <the constraint a future change preserves>

**Owner.** <the authoritative code or contract doc>
```

### Hierarchical Rust modules

**Decision.** Preserve the Python package's separation of core helpers, per-format engines,
readers, writers, and interaction controls in the Rust crate and module hierarchy.

**Why.** The user finds the hierarchy effective for navigation and responsibility boundaries.
Rust translates the implementation through owned data, exhaustive enums, and object-safe traits.

**Consequence.** Each engine owns its format-specific code under its own directory. Shared
media, validation, archive, and integrity contracts keep one authoritative implementation.

**Owner.** [rust_port_plan.md](archive/rust_port_plan.md).

### Shared temporary media ownership

**Decision.** Banks share an owned temporary media directory with `Arc<TempDir>` when merged
or cloned; equal-path merges retain an owning handle if either side has one.

**Why.** A merged bank must resolve images after its source banks drop. Python's non-owning
merge can retain a path after its owner removes the directory.

**Consequence.** The last owned handle removes temporary files. Caller-supplied paths remain
untouched. Different base paths still fail explicitly.

**Owner.** [bank.rs](../crates/qti-core/src/bank.rs).

### Merge position parity

**Decision.** Merge preserves Python's source item numbers, including right-side replacement
of a duplicate CRC. Explicit `renumber_items()` assigns contiguous positions when needed.

**Why.** The architect verified Python merge does not renumber. No LMS behavior or package
integrity invariant proves a different result, so Python behavior controls under parity tier 3.

**Consequence.** Merging banks is not an implicit renumbering operation. Ordering remains the
`IndexMap` insertion order, independently of stored numbers.

**Owner.** [bank.rs](../crates/qti-core/src/bank.rs).

### Blackboard pool source-preservation metadata

**Decision.** The Blackboard pool writer records `min_answers_required` and
`allow_all_correct` for MA, plus the authored numeric tolerance and
`tolerance_message` flag for NUM, in four `bbmd_qti_package_maker_*` children
of `itemmetadata`. The Blackboard pool reader parses those names strictly and
uses the established source defaults when they are absent.

**Why.** The pinned Python MA and NUM writers encode only the Blackboard
response and bounds, losing these model fields on a writer-reader round trip.
They remain fingerprint-relevant source data. The extension preserves that
data without changing the interoperable response or scoring XML.

**Consequence.** These are application-private round-trip metadata, not
Blackboard grading options. Writer documentation and parity receipts must say
that Blackboard grades from the ordinary response/resprocessing payload; they
must not claim an LMS consumes the four fields. A malformed present value is a
typed reader failure for that item, while an absent field retains the frozen
reader default. Future Blackboard compatibility evidence may replace this
carrier, but must retain the same Rust source round-trip.

**Owner.** [blackboard_export_zip](../crates/qti-engines/src/blackboard_export_zip).

### Chromium HTML-to-image rendering

**Decision.** Render valid HTML tables through a local Chromium browser controlled directly from
Rust. `QTI_CHROMIUM` optionally supplies the browser executable. Start it lazily and reuse it for
the conversion; static RDKit canvases and sugar-library PNG/SVG outputs remain image paths.

**Why.** Browser HTML/CSS support is the product requirement. The custom renderer rejected a gel
band's `box-shadow` and omitted colors and circles from a pathway figure. Maintaining a partial
browser engine would add recurring compatibility work without improving the instructor workflow.

**Consequence.** Readable source-faithful Chromium output is the visual authority. The Python
PNG is diagnostic evidence only, and wrapper fonts or viewport defaults are implementation
choices rather than a pixel-parity contract. Chromium is a conditional local runtime dependency
for table conversion; the production binary has no Python or Node.js runtime dependency.

**Owner.** [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md).

### Headless browser selection

**Decision.** Always launch Chromium headlessly. Prefer a dedicated headless shell on `PATH`
or in the standard installed Playwright cache before discovering a full browser. Retain the
existing executable-path setting for nonstandard installations; add no rendering-mode options.

**Why.** The same pathway export took a median 13.10 seconds with full Chromium and 2.91 seconds
with headless shell. Executable discovery owns this internal performance decision.

**Consequence.** Installed shells work automatically without Python or Node at runtime. A full
browser remains usable in headless mode when no shell is installed. Chromium's sandbox stays enabled.

**Owner.** [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md).

- 2026-09-30: Release preparation uses the existing manual `devel/make_release.py`. Rust
  checks remain local in `devel/rust_release_check.sh`; Linux/macOS validation evidence is
  required before release. No GitHub Actions release workflow is part of this design.

- 2026-09-30: Blackboard MA parity treats the frozen writer's empty `varequal` predicates
  whose `respident` names a response label as a bounded legacy grammar. Resolve each to
  exactly one item-local label, represented by response and label ordinals, preserving
  attributes and branch order. A nonempty predicate instead requires a declared response
  interaction and typed value. Reject unknown, duplicate, colliding, and cross-form
  references. This preserves the pinned package form without inferring answer meaning
  or accepting arbitrary undeclared references.
