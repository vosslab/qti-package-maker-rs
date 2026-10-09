# Shared engine delivery

## Current acceptance state

The shared native/Wasm implementation passes its final Rust/Python gates, native fixture parity,
full native corpus/table parity, actual native rendering regressions, generated package tests,
three-browser acceptance, and a
clean installed npm consumer. Fresh specification and quality reviews accept the D23/D24 payload
and error-taxonomy corrections. Native before/after measurements are recorded below.
The listed acceptance results are a prior snapshot. The cleanup audit is recorded in
[shared_engine_six_pass_audit_2026_10_08.md](../audits/shared_engine_six_pass_audit_2026_10_08.md);
rerun bounded validation before treating the snapshot as current.

The exact API and execution ledger live in
[shared_engine_contracts.md](../../archive/shared_engine_contracts.md) and
[shared_engine_execution.md](../../archive/shared_engine_execution.md). No npm publication, site
deployment, source release, or LMS-import certification is part of this work.

## Source and artifact identity

[shared_engine_baseline.md](shared_engine_baseline.md) preserves the pre-refactor source snapshot,
optimized native binary, fixtures, pinned Python oracle, tests, rendering, and timings. Its
282 passing tests/five ignored and 59-bank parity are baseline evidence. The snapshot includes
pre-existing uncommitted PLE/source-split work; a Git commit alone does not identify those bytes.
The disposable extracted source was removed before Python hygiene checks; the snapshot tar,
binaries, fixtures, logs, and hashes remain available for reconstruction.

Final local package evidence is `tests/_temp/wasm_acceptance/package_final/receipt.json`, with
`browser.json`, `host_parity.log`, `consumer_runtime.log`, `consumer_types.log`, and `measure.log`.
The receipt bound 160 source files and all nine packed npm artifacts when recorded. Generated,
dist, tarball, and installed consumer bytes agreed then, and the packed LGPL license matched the
root license exactly. The cleanup changes mean these hashes are prior acceptance evidence, not
claims about the current files.

| Artifact | SHA-256 |
| --- | --- |
| Generated Wasm | `fb3bf13c35e4ae1ec62b69e7a069f06fbb76df627e1bc5f0024002a32910b52f` |
| Packed npm tarball | `37587ee809d09a59a2e3de240e68058982c218fda0d4b14bca534757a427de9c` |
| Debug CLI used for final native fixture/correction receipts | `0ebafa3c4529743891707af36873e88c1ed2f224a18985bdc5200521432e9c05` |
| Optimized CLI used for final native timings | `9fe222e4d3708db6bf58d329f8a6b6b378b104a45b34e48d5aeda628ea8d1502` |

The tarball is
`packages/qti-wasm/_verification/d23_final/vosslab-qti-wasm-0.1.0.tgz`.
Earlier `_verification/` root receipts and pre-D23 package runs describe superseded artifacts.
Native evidence is `tests/_temp/wasm_acceptance/final_native/receipt.json` (status `passed`);
its directory contains 168 source hashes, three binary identities, final gate logs, and timing
receipts. `final_pytest_after_artifacts.log` records all 1,581 Python tests passing after
the completed records were archived and links updated.
These ignored local evidence trees are unavailable in a fresh clone; reproduce the commands
below to generate current receipts.

## Implementation boundaries

- Four readers and eleven writers use shared portable Rust modules and one registry.
- `qti-core` owns item banks, provider-based media, logical names, and byte ZIP assembly.
- `qti-engines` owns read/write/convert orchestration and recovered-first `AssetOverlay`.
- `qti-integrity` owns safe ZIP-byte and entry validation, independent of the item model.
- `qti-native` owns bounded filesystem input, DirectoryAssets, native rendering, context, and
  persistence, including PLE staged ownership checks.
- `qti-wasm` owns typed requests/results and JavaScript defaults. Rust `tsify` and wasm-bindgen
  generate declarations; TypeScript owns loading and host examples.

Native media payload reads, including absolute references and symlinks, stay inside the input root.
Browser inputs use exact validated relative names. Writers use explicit output names literally;
registry naming preserves CLI content-derived filenames. Browser omission defaults to UTC today,
`Exam`, and seed zero. Explicit context makes comparisons reproducible.

Resolved media snapshots and named Rust files share immutable `Arc<[u8]>` payloads; dependencies
and PLE per-question files avoid image-sized clone allocations. Output-boundary vectors and
JavaScript arrays are independent copies. Missing native media is `MissingAsset`; other provider
failures retain `AssetRead`.

## Acceptance evidence

| Gate | Prior acceptance evidence | Recorded state |
| --- | --- | --- |
| Rust workspace | 323 active tests pass; five existing optional tests ignored; strict formatting and Clippy pass | Passed |
| Python hygiene | 1,581 tests pass, including disk-budget checks | Passed |
| Native fixture parity | 59 banks, zero divergences, CLI identity-bound receipt | Passed |
| Native runtime | Three shim tests, all 58 canvases, and four-format correction receipt | Passed |
| Native full corpus/table parity | 180 BBQ inputs yield 1,699 projected banks; zero divergences. One rendered table bank agrees across three ZIP formats | Passed |
| Native comparable timings | All 12 exports succeed with matching fixture hashes and output checks; seven alternating collision pairs preserve exact bytes | Passed |
| Generated package | Release build and strict TypeScript pass on Node 24.21.0 | Passed |
| Node host/native-Wasm parity | Eight tests: five host and three parity | Passed |
| Browser runtime | 15 tests; zero skipped, flaky, or unexpected results | Passed |
| Actual packed consumer | Clean installed tarball initializes, lists 11 writers, converts/checks MC; positive/negative strict TypeScript passes | Passed |
| Independent reviews | Corrections specification, browser specification, and D23/D24 specification/quality accepted | Passed for reviewed scopes |
| Final integration | Composition is clean; 168 native source/three binary and 160 package source/nine artifact hashes match independently | Composition passed |

Native correction evidence is
`tests/_temp/wasm_acceptance/final_native/native_corrections/correction_receipt.json`.
It covers all four rendering formats, skipped unsupported items, FIB grading literals, source
numbering, source-image handling, and persistence preflight. Fixture parity is recorded in
`final_parity_fixtures.log`; the 23-package Python integrity oracle and native shim/corpus lanes
have separate logs under the same final native directory. Full corpus parity is in
`final_parity_corpus.log`; rendered-table parity is in `final_parity_native_fixtures.log`.

The final Node parity corpus covers all eleven writers, all seven item kinds including unsupported
outcomes, all four readers, decoded logical ZIP entries, media across six payload formats, grading,
warning/error provenance, and a 1 MiB/512-item input. The browser suite covers worker conversion,
ZIP checking, PLE question/media downloads, editable relative names with colliding basenames,
file/companion ZIP envelopes, malformed UTF-8, and invalid asset names. Node host tests and the
clean installed consumer separately verify unknown nested request fields, owned-array independence,
and recovery after malformed proxy input through the generated API.

The current corpus scope differs from historical 181-input/1,709-bank receipts; those counts do not
describe this final run. Package native/Wasm parity independently covers every reader/writer.

The final native gate removes only stale incremental build caches and uses `CARGO_INCREMENTAL=0`.
Final native hygiene records `target/` at 10,418,260 KiB, below the 10,485,760 KiB (10 GiB) gate.
Cleanup and final mandatory Python verification are recorded separately from production behavior
checks.

## Package size and timing

Measured on macOS arm64 with Node 24.21.0, wasm-pack 0.15.0, and Playwright 1.64.0:

| Component | Raw bytes | Gzip bytes |
| --- | ---: | ---: |
| Wasm | 3,791,201 | 1,504,245 |
| Generated loader | 19,786 | 4,551 |
| TypeScript wrapper JavaScript | 307 | 220 |
| Runtime total | 3,811,294 | 1,509,016 |

The nine-file npm tarball is 1,515,551 bytes, with 3,826,724 unpacked bytes. `npm run measure`
records component sizes, runtime version, platform, and architecture. These values describe the
final artifact above; they are not download measurements from a published CDN.

Browser smoke timing observations use one MC question with a PNG companion. Initialization is
recorded once per worker realm.
Each conversion duration measures the worker API call, excluding message transfer, UI, and
download work. Each value is one acceptance-run observation, not a repeated throughput benchmark.

| Browser version | Initialize ms | Blackboard ZIP ms | PLE ms |
| --- | ---: | ---: | ---: |
| Chromium 156.0.8078.4 | 10.6 | 25 | 8 |
| Firefox 157.0 | 22 | 7 | 5 |
| WebKit 27.2 | 12 | 17 | 6 |

Native measurements use three fresh optimized subprocesses per workload, warm OS caches, and no
cross-process conversion cache. All 12 final exports succeed with unchanged input hashes. PLE
authored bytes and ownership hashes agree; ZIP CRC, XML, item, and PNG counts agree. Measurements
and output checks are in `final_native/timing_comparison.json` and
`final_native/timing_after_native/receipt.json` beneath the acceptance tree.

| Native workload | Before seconds | After seconds | Change |
| --- | ---: | ---: | ---: |
| 500 MC to Canvas | 0.593403 | 0.592276 | -0.19% |
| 500 MC to PLE | 0.615606 | 0.620629 | +0.82% |
| Colliding images to PLE | 0.005499 | 0.005904 | +0.405 ms / +7.36% |
| Rendered table to Blackboard | 0.159042 | 0.149624 | -5.92% |

The small collision increase does not reproduce in seven alternating before/after pairs on the
current host: medians are 0.005882750 and 0.005809917 seconds (-1.24%). All 14 follow-up outputs,
including ownership markers, are byte-identical. `final_native/collision_confirmation.json`
retains the control. These local observations do not establish a broad speed improvement or
regression.

D19 optimized dense-candidate ZIP probes run at 2,500, 5,000, 10,000, and 20,000 candidates. ZIP32
medians are 0.1135, 0.2271, 0.4633, and 0.9346 ms; ZIP64 medians are 0.5314, 1.0498, 2.0958,
and 4.2181 ms. Largest inputs are 440,022 and 1,960,022 bytes. Earlier repeated prefix scans take
1105.59 ms and 7.51 seconds at 20,000 candidates. The indexed correction removes that measured
quadratic work while preserving D14 rejection. Log: `/private/tmp/qti_integrity_d19_bench.log`.
These are ZIP-input probes, not ordinary conversion timings; no arbitrary timing cutoff is a gate.

D14 regressions reject a malformed final directory that makes zip-rs select an earlier
10,001-entry directory before decoder metadata allocation. Unusual inputs embedding plausible
overlimit archive directories can be conservatively rejected because fallback may select them.

## Local reproduction

Use [INSTALL.md](../../INSTALL.md) and [WASM_PACKAGE.md](../../WASM_PACKAGE.md) for native/package
setup and commands. [rust_release_check.sh](../../../devel/rust_release_check.sh) remains a local
manual release workflow. `RUST_RELEASE_WASM=1` includes portable target checks, npm installation,
package build, strict typecheck, combined Node host/parity tests, and three-browser acceptance.
Selected missing or failed prerequisites stop the lane; correct the reported requirement and rerun.

Browser sandbox approval on this development host concerns execution permission; it adds no
product setting. Native Chromium rendering and package downloads are separate acceptance lanes.
The historical raw Python rendering parity differences remain documented in
[PARITY.md](../../PARITY.md), and LMS imports/release publication remain external validation.
