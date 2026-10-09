# Shared engine six-pass audit

## Scope and method

The requested audit used six fresh independent reviewers: Plan, Test, Style, Docs, Legacy,
and Comment. All six completed. Review covered the current shared native/Wasm conversion
change, including staged additions, against the original user plan, repository rules,
[shared contracts](../../archive/shared_engine_contracts.md), and
[delivery evidence](../reports/shared_engine_delivery.md).

The working-tree comparison spans 158 files and includes PLE and source-split work that
predated the shared-engine implementation. Findings from that earlier work are identified
separately below. The audit preserves the existing Git index and unrelated edits.

## Findings within the shared-engine change

| Severity | Finding and impact | Smallest useful correction | Status |
| --- | --- | --- | --- |
| Medium | Public Wasm DTOs/exports and the new asset-provider APIs lack required Rustdoc. Their defaults, byte ownership, validation, and root boundaries are difficult to discover from the API. | Document the public contract in `qti-wasm/src/{transport,boundary}.rs`, `qti-core/src/media/assets.rs`, and `qti-native/src/assets.rs`. | Fixed |
| Medium | `qti-native/tests/browser_proof.rs` is an ignored one-time HTML generator, not a browser assertion. Its fixed temporary-directory cleanup and lack of assertions do not earn permanent test status. | Remove the generator after retaining historical evidence, and correct its current architecture/file-map references. | Fixed |
| Low | `qti-core/src/bank/tests.rs` pins `Arc::strong_count(payload)` to 161, constraining incidental reference bookkeeping. | Keep the provider-read, canonical-asset, payload-identity, and byte checks; remove the exact reference-count assertion. | Fixed |
| Low | `qti-native/Cargo.toml` directly declares unused `markup5ever`. The engines crate separately owns its actual use. | Remove only the native dependency entry and corresponding lockfile edge. | Fixed |
| Low | The Node example documentation omits `ASSET_NAME=PATH`, needed for nested authored image references. | Explain bare-path and explicit-name forms and include a nested-name example. | Fixed |
| Low | The browser documentation claims directory selection, while the example uses a multiple-file picker and editable logical names. | Describe the existing picker and name editor accurately. | Fixed |
| Low | The archived contract says Wasm uses "fixed adapter values," conflicting with its documented UTC-today date default. | State that the host resolves the date once per conversion. | Fixed |

The Plan reviewer initially interpreted the clock wording as an architectural problem.
After comparing the original instruction, that finding was withdrawn: the user explicitly
allows adapters to resolve clock values once and pass explicit context to shared code.
The remaining issue is documentation, with no API or behavior change required.

## Separate pre-existing findings

These low-severity findings remain outside this cleanup's shared-engine boundary:

- [Integrity Cargo.toml](../../../crates/qti-integrity/Cargo.toml#L13) has an unused direct
  `sha2` dependency. The same unused
  entry already exists at `HEAD`; it was not introduced by this migration.
- Permanent [parity code](../../../xtask/src/parity.rs#L1) retains `M13`/`M6` milestone labels
  in comments, help, or diagnostics.
  The labels predate the shared-engine work. Durable feature names would be clearer.
- The initial split [Python oracle modules](../../../xtask/support/parity_oracle_projection.py#L12)
  omit the function separators required by
  `docs/PYTHON_STYLE.md`.
- [COHESIVE_SOURCE_SPLITS.md](../active/COHESIVE_SOURCE_SPLITS.md) uses uppercase naming where the
  active-plan convention calls for snake_case. Renaming that earlier plan would require an
  index-writing move and is not part of this cleanup.

## Pass completion and test policy

| Pass | Result |
| --- | --- |
| Plan | No implementation or architectural finding; one archived-wording correction. |
| Test | Remove the one-time generator and weaken the incidental allocation assertion. |
| Style | One filename finding in the earlier source-split plan. |
| Docs | Two example-usage documentation corrections. |
| Legacy | One unused native dependency and one separate pre-existing integrity dependency. |
| Comment | Missing public API docs and two earlier parity/source-split conventions. |

Every pass reported a concrete finding; none was omitted or replaced. No permanent test was
proposed. The Test pass classified the other changed core, engine, integrity, persistence,
rendering, transport, type-contract, Node, native/Wasm parity, and browser tests as durable
contract coverage. The process-based parity and browser suites are explicit delivery checks,
not part of fast Python hygiene.

## Verification evidence

Before cleanup, fresh SHA-256 checks found no drift from the prior acceptance receipts across
168 native source files, 160 package source files, and nine packaged artifacts. Those receipts
record 323 passing Rust tests, five existing ignored tests, native rendering, 1,699 zero-divergence
corpus comparisons, eight Node tests, 15 browser tests, and a clean installed package consumer.
Those are earlier runtime results, not independently rerun by the six audit reviewers.

Fresh audit checks before cleanup passed:

- `cargo fmt --check`.
- `git diff HEAD --check`.
- `bash -n devel/rust_release_check.sh`.
- `source source_me.sh && python3 -m pytest tests/`: 1,821 passed in 6.02 seconds, including
  disk budgets. The higher count than the earlier 1,581 reflects the current tracked-file set.

Post-cleanup verification passes:

- Strict Clippy across the workspace and all targets.
- Rust workspace/all-target tests: 323 passed, zero failures, four existing optional tests ignored.
  Removing the ignored one-time generator accounts for the change from five ignored tests.
- Release Wasm package build and strict TypeScript checks.
- Eight Node host/native-Wasm parity tests on Node 24.21.0.
- Fifteen Chromium, Firefox, and WebKit tests, including workers and downloads.
- A fresh installed npm tarball: runtime conversion/integrity and positive/negative strict
  TypeScript checks. All nine packed files match the distributed and installed bytes.

The rebuilt Wasm SHA-256 remains
`fb3bf13c35e4ae1ec62b69e7a069f06fbb76df627e1bc5f0024002a32910b52f`, identical to the prior
accepted binary. The documented tarball is 1,516,463 bytes with SHA-256
`bd400342eadcf79108aba74e46e4275f1798211a42f416fc1805560620979973`.
Logs and `artifact_identity.json` are retained under
`tests/_temp/shared_engine_audit_2026_10_08/`. Earlier full-corpus, native rendering, and timing
results remain prior-snapshot evidence; those long-running/native rendering lanes were not rerun
for this documentation, dependency, and test-only cleanup.

Before rebuilding, the prior debug CLI and xtask binaries were preserved with matching hashes,
then Cargo removed only regenerable workspace dev-profile artifacts. Dependency caches, release
outputs, and prior evidence were retained. The current `target/` is 6,399,916 KiB, below the
10,485,760 KiB budget. The lockfile change is verified to be only the removed native dependency
edge; no dependency versions changed during cleanup.

Final formatting, diff-whitespace, and shell-syntax checks pass. All six local links in this
audit report resolve and its text is ASCII. The final full Python hygiene run passes 1,819 checks
in 5.20 seconds, including disk budgets; the two fewer file checks reflect the deleted generator.
The earlier source hashes describe their frozen acceptance snapshot, not subsequently edited
comments, manifests, or tests.

## Residual boundaries

This is a focused six-pass code audit, not an LMS-import certification or publication review.
Reviewers used current source and recorded runtime evidence; fresh checks are labeled above.
No blocker or high-severity issue was found. Broader architectural changes and new permanent
tests were neither proposed for implementation nor added by this audit.
The original six reviewers did not perform a second independent audit after cleanup; the
coordinator inspected the bounded corrections and ran the fresh validation recorded above.
