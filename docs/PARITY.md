# Parity record

This record applies the authority order in the active Rust-port plan: observed
LMS behavior, documented format requirements, Python runtime behavior at the
pinned revision, then advisory signals. Plain exports pass the full differential corpus;
converted exports have the classified semantic evidence described below. The raw HTML
comparison still reports the documented differences.

## M13 differential harness

`cargo xtask parity --fixtures` compares actual exports through two explicit oracle lanes:
the seven formats exposed by the frozen Python CLI and three registered-only writers invoked
through its package interface. A current, self-built CLI agrees across 56 per-engine question-kind
projections, including Canvas scoring behavior. Media and full-corpus cases remain under
verification in that fixture receipt; the later corpus evidence is summarized below.

### Current corpus evidence

The full plain-export run `tests/_temp/parity_6191` completes 1,709 input banks across all
ten writer formats with zero findings. The subsequent image-only whitespace correction
does not affect plain exports.

The current release replay at `tests/_temp/native_html_replay_current` exports all 537
projected HTML-to-image inputs successfully. Every package passes integrity without errors
or warnings. Public grading agrees for 178 Blackboard, 178 Canvas, and 181 QTI 2.1 pairs,
including the five source-bound repaired Python references described below. Presentation
review preserves authored text, image ownership/order, styling attributes, response settings,
and grading; it does not require identical paragraph wrappers or pixels.

The raw HTML differential report contains incidental paragraph/serialization differences
and frozen-reference defects. Its findings were classified in the replay's
`presentation_review.md`, with per-field source hashes and comparison receipts. This is a
classified semantic result, not a claim that the unchanged raw harness reports zero findings.

Observed defects and deliberate differences:

- The native table alt-text routine now preserves whitespace before filtering to ASCII.
  The current corpus replay resolves the previously fused labels.
- Frozen Python drops text following converted tables, including question wording, formula
  steps, and `AND` between diagrams. Rust preserves the authored source text. The reviewed
  native fields match source text outside rendered tables/canvases.
- Rust removes redundant nested multiple-choice display labels. Differences such as `A.` in
  image alt text also change visual-choice hashes, while choice positions and grading agree.
- Frozen Canvas MULTI_FIB exports contain literal answers where scoring requires declared
  choice identifiers. For the two converted cases, rerendering the source reproduces the
  full frozen item structures exactly. Replacing only those references with their uniquely
  matching declared identifiers makes grading agree; repaired references pass integrity.
- Frozen conversion loses the local-media base directory for the horse-image source. The
  previously verified development-only media-staging repair produces reference packages
  that agree with native output in all three formats. The repaired references pass integrity.

These checks remain implementation evidence. No production workaround copies Python's
content loss, and no permanent test requires incidental XML serialization. Real LMS imports
remain external validation and are not implied by these local results.

The frozen Canvas MULTI_FIB writer creates response identifiers that its scoring expressions do not
declare. Preserve its two `qti12-dangling-varequal` findings as a defect receipt. Rust must pass
integrity and preserve the choice-based blank interactions and intended accepted answers;
unresolved identifiers must fail
the comparison projection. See the [binding ruling](active_plans/decisions/engine_trait_ruling.md).

## M6 package-integrity oracle

`qti-integrity::check_package(path) -> Vec<Violation>` reads either a ZIP or
an extracted package tree without extracting ZIP members. It is deliberately
independent of `qti-core` and the writer crates. The cross-language authority
is Python `package_integrity.py` at
`55e5f368777f7809fe2e91b5d070caf6df0cb581`.

| Check or finding codes | Authority and provenance | Implemented evidence |
| --- | --- | --- |
| `missing-manifest`, manifest resource/file/dependency resolution, duplicate or missing resource identifiers | IMS Content Packaging relationships; `FormatRequirement` | `checker.rs` builds a resource map and resolves every declared reference. |
| `qti12-dangling-varequal`, `qti21-dangling-correct-response` | QTI answer references must name choices declared in the same item; `FormatRequirement` | Namespace-resolved QTI 1.2 and 2.1 parsing, including renamed XML prefixes. |
| `dangling-image-source`, `unmanifested-item` | The referenced item and entry must be present; `FormatRequirement` | Decoded, query/fragment-free URI resolution handles inherited `xml:base` and the IMS filebase form. |
| `broken-media-trace` | Blackboard import failure reproduced by the Python checker; `BlackboardImportFailure` | The checker follows an item resource through its complete manifest dependency closure to readable package bytes. |
| `unreadable-raster` | Python package-integrity behavior; `FormatRequirement` | PNG, GIF, and JPEG headers are probed in `checker/image_dimensions.rs`. |
| `invisible-raster` | Python-parity smoke signal only; `PythonParityAdvisory` and advisory severity | A raster at or below 5 pixels on either side is reported without elevating it above recorded LMS or format evidence. |
| `unsafe-identifier`, `missing-score-outcome` | Blackboard import failures recorded by the Python project; `BlackboardImportFailure` | Identifier and QTI 2.1 `SCORE` outcome checks operate only on namespace-resolved QTI documents. |
| `dangling-bb-file`, `orphaned-xid-token`, `orphaned-xid-resource-link`, `orphaned-cs-parent`, `missing-lom-sidecar` | Blackboard export cross-reference invariants; `FormatRequirement` | Blackboard namespace attributes, `.dat` resources, CSResourceLinks, csfiles binaries, and LOM sidecars are checked as one graph. |
| `invalid-xml`, `unsafe-entry-path`, `duplicate-entry`, `symlink-entry`, `package-size-limit`, `package-input` | Safe inspection boundary; `SafeInputHandling` | DTDs, malformed/trailing XML, unsafe paths, duplicate ZIP members, symlinks, and bounded input are rejected before format checks. |

The format rows use the named constructs in IMS Content Packaging 1.1.4
(`resource`, `file`, and `dependency`) and IMS QTI 1.2/2.1 (`varequal`,
`responseDeclaration`/`correctResponse`, and `outcomeDeclaration`). The two
`BlackboardImportFailure` rows are the Python checker's regression canaries for
the Blackboard import incidents: unsafe IDs and absent `SCORE`, plus a media
resource that cannot be reached through its manifest dependency graph. They
are retained at the pinned Python revision and independently exercised by the
M6 corpus.

QTI semantic elements are URI-qualified at every traversal. `bb:file` is also
accepted only when its namespace URI is
`http://www.blackboard.com/content-packaging/`. The nested Blackboard `.dat`
records and IMS CP resource graph retain the Python checker's local-name
semantics because the committed export fixture uses unqualified records and
the pinned Python oracle does not distinguish a foreign prefix there. This is
a recorded tier-3 parity choice, not a new format claim; a contrary LMS result
or specification survey must be escalated before tightening it.

The pinned Python checker explicitly treats QTI 2.1 correct-response values as
whitespace-separated membership tokens. The M6 regression covers gap, order,
and match interactions, including an inverted directed pair, and confirms that
text-entry, extended-text, and numeric declarations are skipped when no
choice-bearing interaction is bound. Directed source/target role validation is
therefore not an M6 finding: adding it would diverge from the tier-3 Python
authority and requires an architect ruling backed by a format or LMS source.

### Safe-input limits

Both ZIP and extracted-tree inputs enforce at most 10,000 entries, 32 MiB per
entry, and 256 MiB total logical bytes. ZIP members stay in memory and are
never extracted. Directory traversal rejects symlinks; ZIP member names reject
absolute paths, parent traversal, and backslashes.

These are untrusted-input inspection limits, not package-production rules.
`input.rs` has a regression that drives both a ZIP and an extracted directory
past the entry limit; per-entry and total-byte checks share the same constants
on both paths.

### Architect ruling: unused raster members

The standard result intentionally has no `orphaned-packaged-image` finding.
An unused ZIP raster has no item-media reference for an LMS to resolve, and
the pinned Python implementation does not require every raster member to have
a manifest `<file>` declaration. The decision is recorded in
[`m6_orphaned_raster_ruling.md`](active_plans/decisions/m6_orphaned_raster_ruling.md).
The oracle still checks dimensions for every raster and fully traces every
referenced image.

### M6 automated exit evidence

| Exit criterion | Artifact and result |
| --- | --- |
| Blackboard fixture is clean | `tests/fixtures/bb_export_slice.zip` is included in the real-package portion of `cargo xtask oracle-crosscheck`; it produced an empty Python and Rust violation list. |
| Referenced-media trace is complete | `checker_tests.rs` covers a clean extracted package, missing media, nested-resource isolation, dependency closure, percent decoding, query/fragment removal, and inherited `xml:base`. |
| Negative corpus and four canaries reject their defect classes | `xtask/support/oracle_crosscheck.py` constructs 13 single-category corruptions plus `canary_qti12`, `canary_qti21`, `canary_parent`, and `canary_manifest_media`; its separate `unused_raster` package is a required clean parity case under the architect ruling. Focused crate tests add parser and resource-boundary regressions. |
| Independent cross-language agreement | `cargo xtask oracle-crosscheck` completed with 23 packages agreeing with the pinned Python oracle, with no Rust-only findings filtered. |

Focused crate verification after the final split:

```text
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo fmt -p qti-integrity --check
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo test -p qti-integrity
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo clippy -p qti-integrity -- -D warnings
```

All three commands passed; the package test includes 26 unit tests and its
doctest target. The checker implementation is split into focused
`checker.rs`, `checker_tests.rs`, and `checker/image_dimensions.rs` to remain
within the repository source-file limit.

## M13 status

M13 is **incomplete**. The M6 oracle agreement proves the integrity checker
matches the pinned Python checker on its independent corpora; it does not prove
all reader/writer formats have semantic parity, item-fingerprint round trips,
or a clean full-workspace integration gate. Those remain M13 acceptance work.
