# Design decisions

### Library dependency features

**Decision.** Enable Scraper's HTML parse diagnostics without its standalone CLI, and ZIP's
`deflate-flate2-zlib-rs` backend without the additional Zopfli encoder.

**Why.** Conversion uses Scraper as a library and ZIP's default ordinary Deflate compression
level. The extra CLI and compression features add unused dependencies.

**Consequence.** Preserve existing HTML diagnostics, archive input support, and output compression.
Declare direct dependencies only in the crates and build kinds that use them.

**Owner.** [Cargo.toml](../Cargo.toml) and the workspace member manifests.

### Graphify normalization and overwrite protection

**Decision.** Keep the reclustering correction upstream in Graphify. The tested
candidate compares independently normalized disk and candidate graphs, preserving
canonical node IDs and remapped links.

**Why.** Cargo workspace aliases legitimately reconcile with AST package nodes;
raw JSON counts falsely report data loss, while stale link endpoints drop dependencies.

**Consequence.** Unexpected canonical-node loss still refuses overwrite in the candidate.
This converter does not force writes, maintain a dependency fork, or patch packages at runtime.
The released Graphify 0.9.80 still has the fault pending an upstream correction.

**Owner.** Graphify `build_from_json`, `export.to_json`, and `cluster-only`/`label`.
The cited Graphify evidence artifact is unavailable in this checkout.

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

### Selftest consumer compatibility

**Decision.** Generated selftests preserve CRC-based DOM identifiers and route grading through
the public per-question function. Initialization preserves host wrappers and supports remounting.
Current Python comparisons are an explicit migration lane; independent Rust and browser tests
own the lasting behavior contract.

**Why.** The website observes public grading hooks to record individual question completion.
Frozen-source comparisons and answer-data projections missed behavior that broke that contract.
Keeping Python installed forever would defeat the intended transition to Rust authority.

**Consequence.** Test emitted HTML through actual controls and host wrappers. Preserve useful
consumer behavior, not obsolete interfaces or accidental details. Add no compatibility layer for
earlier Rust mistakes. Keep historical
comparison provenance, but use current source for migration checks. Retire Python tooling when
independent coverage and resolved migration findings make it unnecessary.

**Owner.** [SELFTEST_COMPATIBILITY.md](SELFTEST_COMPATIBILITY.md).

### PLE Native JSON authority and export

**Decision.** QPM maps its seven validated item kinds to PLE Native JSON without changing answer
meaning or adding presentation permutations. `ple_native_json::export_bank` supplies per-question
JSON and associated file bytes; the CLI writes `ple-<content_name>/` with `item_NNNNN.json` and
`media/`. PLE owns import binding, asset IDs, grading behavior, and student presentation.

**Why.** QPM is the content and grading source. PLE has a private, unversioned source contract and
needs both a library handoff and a reviewable file transport. The `ple-` prefix follows the
existing concise output names.

**Consequence.** QPM validates its mapping, media references, and known lossy conversions, while
PLE's decoder owns its count, size, and length limits. No QPM shuffle, ZIP carrier, HOTSPOT, or
new CLI option is introduced.

**Owner.** [ple_native_json/source.rs](../crates/qti-engines/src/ple_native_json/source.rs) and
[ple_native_json_writer_plan.md](active_plans/active/ple_native_json_writer_plan.md).

### PLE text answers and URL inventory

**Decision.** FIB and MULTI_FIB use PLE `normalized` matching. Their required `maxLength` uses
PLE's 16,384-character ceiling because QPM has no authored response length. QPM scans represented
display HTML for external resources: it rejects scripts, protocol-relative and non-HTTPS URLs,
deduplicates repeated same-kind URLs, and rejects one URL used in different resource roles.

**Why.** `normalized` preserves case-insensitive answers with incidental whitespace ignored.
PLE requires each external URL to be unique independent of kind, so a single URL used as both a
link and image cannot be inventoried without losing one role. Keeping only the first kind would
also change PLE's image-resource predicate.

**Consequence.** The response length shown in PLE is a platform ceiling, not an authored QPM
limit. Multi-role URL reuse fails explicitly until PLE can record each role; QPM does not silently
drop an image or link role. See the
[PLE issues report](active_plans/reports/ple_issues_from_native_json_writer.md).

**Owner.** [mapping.rs](../crates/qti-engines/src/ple_native_json/mapping.rs),
[scan.rs](../crates/qti-engines/src/ple_native_json/scan.rs), and PLE's private decoder contract.

### PLE display-only media collection

**Decision.** Collect and rewrite PLE media only in the mapped source document's display HTML
fields. Reuse `qti-core::media` scanning, resolution, collision-safe naming, policy, and HTML
rewrite functions; do not use whole-bank `collect_assets` or `rewrite_item_media` for this engine.

**Why.** A temporary media-boundary reproduction found that whole-bank traversal also scans FIB
and MULTI_FIB accepted-answer literals. An answer containing image-like text then causes a false
missing-file error or is rewritten, changing accepted grading input. The PLE output has distinct
display fields, so scanning them directly preserves answer meaning.

**Consequence.** Shared media naming and policy stay authoritative in `qti-core`; native path
confinement belongs to `qti-native`. PLE export packages only images referenced by represented
display HTML. Do not
create a surrogate bank or alter the source item to work around this boundary.

**Owner.** [ple_native_json/media.rs](../crates/qti-engines/src/ple_native_json/media.rs) and
[qti-core media](../crates/qti-core/src/media/mod.rs).

### PLE media fidelity and directory ownership

**Decision.** Before export, verify that each local image URL as an HTML viewer interprets it
names exactly one associated file for that question. Reject a mismatch with item context. The
CLI directory includes a hidden `.qpm-ple-native-json` manifest recording the engine marker and
SHA-256 hashes of every generated JSON and media file. Replace a prior output only when this
manifest and the complete prior file set still match; an empty destination is accepted.

**Why.** The shared media scanner can treat `a&amp;b.png` as a literal filename even though HTML
resolves the attribute to `a&b.png`, allowing a valid-looking package with the wrong image. A
schema-valid `item_00001.json` is not proof that QPM owns a user's existing directory; a prior
schema-only ownership check could overwrite it.

**Consequence.** QPM rejects media it cannot package faithfully and preserves unrelated or edited
directories unchanged. The manifest belongs only to directory transport, not the in-memory
per-question file list. PLE reads the JSON and associated files; it does not consume the marker.

**Owner.** [ple_native_json/media.rs](../crates/qti-engines/src/ple_native_json/media.rs) and
[qti-native/src/ple_output.rs](../crates/qti-native/src/ple_output.rs).

### PLE staged output integrity

**Decision.** Verify the staged directory against exact manifest paths and SHA-256 hashes before
publishing it. Keep the exported in-memory file names unchanged.

**Why.** On a case-insensitive filesystem, distinct generated paths such as `media/A.png` and
`media/a.png` can alias one physical file during staging. The pre-fix regression reproduced this
failure; its filesystem-aware version passes after staged-tree verification.

**Consequence.** An aliased or changed staged file is rejected before it can replace an existing
directory. The ownership manifest proves both the expected file set and bytes at publication.

**Owner.** [qti-native/src/ple_output.rs](../crates/qti-native/src/ple_output.rs) and
[ple_native_json_writer_plan.md](active_plans/active/ple_native_json_writer_plan.md) (D8).

### One Rust conversion engine for CLI and Wasm

**Decision.** Build the same Rust parsing, validation, media, and writer implementation for the
native CLI and `wasm32-unknown-unknown`. Generate TypeScript bindings from Rust with `tsify`; expose
an explicit typed API for initialization, format enumeration, conversion, and package checking.

**Why.** A single conversion implementation keeps native and browser behavior aligned and avoids a
second JavaScript conversion path.

**Consequence.** Keep portable bank, media, ZIP, converter, registry, and integrity behavior in the
existing core, engine, and integrity crates. Put filesystem, persistence, rendering, environment,
and time services in `qti-native`; keep `qti-wasm` as the typed host adapter with owned
`Uint8Array` inputs and outputs. D1-D9 establish byte inputs, logical output names, explicit
context, media-source ownership, lazy reads, safe archive handling, and shared orchestration.
Native and Wasm adapters use the same four readers and eleven writers.

**Owner.** [shared_engine_contracts.md](archive/shared_engine_contracts.md).

### Explicit context and registry metadata

**Decision.** The registry owns supported kinds, default and content-derived output names, and
native-render eligibility. Writers use the supplied logical output name literally. Callers resolve
date and shuffle seed once. Preserve the existing fixed ZIP package titles (D10).

**Why.** One format inventory and explicit invocation values prevent target-dependent conversion
behavior and accidental metadata changes during the native/Wasm split.

**Consequence.** Exam YAML uses context title/date; ZIP titles keep their established values.
Explicit browser date/title/seed make repeated calls reproducible. The browser adapter defaults
to UTC today, title `Exam`, seed zero, and the registry default output name.

**Owner.** [registry.rs](../crates/qti-engines/src/registry.rs),
[traits.rs](../crates/qti-engines/src/traits.rs), and
[adapter.rs](../crates/qti-wasm/src/adapter.rs).

### Shared asset overlay ownership

**Decision.** `qti-engines::AssetOverlay` serves recovered or rendered memory bytes first and
consults the caller's provider only when that source is absent (D11).

**Why.** Native rendering and portable conversion need the same composition behavior and one
authoritative owner.

**Consequence.** Recovered bytes win over supplied duplicates. Provider failures propagate;
composition introduces no broad error fallback. Native callers import this shared type.

**Owner.** [conversion.rs](../crates/qti-engines/src/conversion.rs).

### Logical names and native confinement

**Decision.** Portable file and memory asset names are relative POSIX names with no trailing
slash, traversal, empty components, backslash, colon, or control characters. Directory markers
have a separate ZIP contract. Conflicting memory bytes at one exact key fail. Native media reads
are confined to the canonical input root, including authored absolute paths and symlinks.

**Why.** Byte transports need unambiguous names; a native root makes file authorization explicit
without adding host paths to portable models.

**Consequence.** Reference/placeholder writers inspect metadata without payload reads. Payload
writers request only emitted media. Banks whose media sits outside the input directory must
move under a common root and update their references. Spaces and UTF-8 names remain supported.

**Owner.** [assets.rs](../crates/qti-core/src/media/assets.rs),
[zip.rs](../crates/qti-core/src/zip.rs), and
[assets.rs](../crates/qti-native/src/assets.rs).

### Hierarchical Rust modules

**Decision.** Preserve the Python package's separation of core helpers, per-format engines,
readers, writers, and interaction controls in the Rust crate and module hierarchy.

**Why.** The user finds the hierarchy effective for navigation and responsibility boundaries.
Rust translates the implementation through owned data, exhaustive enums, and object-safe traits.

**Consequence.** Each engine owns its format-specific code under its own directory. Shared
media, validation, archive, and integrity contracts keep one authoritative implementation.

**Owner.** [rust_port_plan.md](archive/rust_port_plan.md).

### Provider-owned media bytes

**Decision.** Item banks own validated items; asset providers and reader outcomes own media
bytes. Blackboard recovery returns `MemoryAssets`; banks carry no media directory or keepalive.

**Why.** Portable models must remain valid independently of filesystem and temporary-directory
lifetime. Media provenance and payload access belong to an explicit provider.

**Consequence.** Bank cloning and merging operate on item data. The caller retains its provider
for the conversion; recovered reader bytes remain owned by the read outcome. This replaces the
earlier `Arc<TempDir>` bank ownership design.

**Owner.** [bank.rs](../crates/qti-core/src/bank.rs),
[assets.rs](../crates/qti-core/src/media/assets.rs), and
[traits.rs](../crates/qti-engines/src/traits.rs).

### ZIP fallback allocation bounds

**Decision.** Before constructing zip-rs, scan all plausible EOCD and associated ZIP64 candidates
and enforce original member-count and extensible-record bounds (D14). Index candidates once
to avoid repeated prefix scans (D19).
Keep selected-directory raw-name, duplicate, and entry validation.

**Why.** The decoder can fall back from a malformed final directory to an earlier directory.
Preflighting only the final record permits oversized metadata allocation before a later error.

**Consequence.** Both regular ZIP32/ZIP64 packages and prefix bytes remain supported. An unusual
input embedding a plausible overlimit archive directory can be conservatively rejected because
the decoder may select it through fallback. Candidate indexes grow with input candidates; decoder
metadata stays bounded before allocation. ZIP32/ZIP64 regressions cover the bypass, and dense
ZIP64 candidate regressions protect the indexed scan from returning to quadratic prefix work.

**Owner.** [zip_directory.rs](../crates/qti-integrity/src/input/zip_directory.rs).

### Shared immutable resolved payloads

**Decision.** Store resolved `MediaAsset` and `NamedFile` bytes as `Arc<[u8]>` (D23). Use
`shared_bytes()` and `from_shared` for dependency and per-question file fan-out. Keep the provider
Cow contract and `EntryMap` byte vectors unchanged. Explicit output-boundary methods return copies.

**Why.** Vector-bearing media records introduced one full image allocation per cloned dependency;
a 160-item, 1 MiB-image probe reproduced 161 image-sized allocations. Immutable shared ownership
retains one resolved payload while each dependency keeps its own metadata.

**Consequence.** Rust media/file clones share their byte allocation; PLE associated files preserve
exact payloads. `read_bytes()` and `into_parts()` still provide independent vectors when required,
and generated JavaScript input/output arrays retain their owned-copy contract.

**Owner.** [resolve.rs](../crates/qti-core/src/media/resolve.rs),
[zip.rs](../crates/qti-core/src/zip.rs), and
[media.rs](../crates/qti-engines/src/ple_native_json/media.rs).

### Native missing-source classification

**Decision.** Map native `NotFound` media reads to `MediaError::MissingAsset` with authored source
spelling; preserve `AssetRead` for provider failures (D24).

**Why.** The shared provider contract distinguishes an absent source from a failed read.

**Consequence.** Missing sources retain consistent portable diagnostics; permission and other
provider errors propagate without broad fallback.

**Owner.** [assets.rs](../crates/qti-native/src/assets.rs).

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
