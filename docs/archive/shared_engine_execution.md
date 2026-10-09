# Shared engine execution

## Purpose and authority

Deliver one Rust conversion implementation for the native CLI and a browser WebAssembly package
with generated TypeScript bindings. The reviewed API and boundary contract is
[shared_engine_contracts.md](shared_engine_contracts.md); this ledger tracks execution and evidence.
The user-adopted plan remains the source of scope. Native rendering stays available without Node
prerequisites. The browser target is `wasm32-unknown-unknown`, with no WASI or publication step.

## Adopted architecture

| Area | Responsibility |
| --- | --- |
| `qti-core` | Portable bank, media providers/bytes, logical names, and ZIP assembly |
| `qti-engines` | One registry, all four readers and eleven writers, shared orchestration/overlay |
| `qti-integrity` | Independent bounded ZIP-byte and package-entry checks |
| `qti-native` | Filesystem, persistence, rendering, local context, and environment services |
| `qti-wasm` | Typed owned transport, JavaScript adapter, and generated declarations |
| `packages/qti-wasm` | Explicit initialization, strict TypeScript, host examples, acceptance |

Rust owns format conversion; TypeScript owns loading and host integration. Shared writers return
owned byte artifacts and consume explicit context. Asset bytes belong to providers and read/render
outcomes; banks carry no temporary-directory ownership.

## Work ledger

Completed on 2026-10-08. Final evidence is summarized in
[shared_engine_delivery.md](../active_plans/reports/shared_engine_delivery.md).

| Work area | Owner | Status | Handoff and evidence |
| --- | --- | --- | --- |
| Preserved native baseline | Baseline agent | Complete | Source/binary hashes, 282 tests/five ignored, 59-bank parity, three shim tests, 58 canvases, and before timings in [shared_engine_baseline.md](../active_plans/reports/shared_engine_baseline.md). |
| Portable contracts | Contract specialist and manager | Complete | Exact D1-D24 signatures and approved corrections reconcile with frozen code in [shared_engine_contracts.md](shared_engine_contracts.md). |
| Portable core | Core and payload owners | Complete | Native/Wasm compilation, shared-payload identity and exact-byte regressions, and fresh D23 specification/quality accepted; final workspace gates pass. |
| Safe integrity input | Integrity owners | Complete | D14/D19 correction passes 36 tests and native/Wasm strict Clippy; fresh specification accepts bounded decoder allocation and indexed candidate scanning. |
| All-format engine migration | Engine owners | Complete | Four readers/eleven writers dispatch from one registry; final generated native/Wasm parity covers kinds, artifacts, media, warnings/errors, grading, and large input. |
| Native host and callers | Native/caller owners | Complete | D16-D18/D24 corrections accepted; final 59-bank parity passes with zero divergences and CLI identity binding; three shim tests, 58 canvases, and four-format correction receipt pass. |
| Typed Wasm adapter | Adapter owners | Complete | Generated typed API, owned arrays, D15 logicalName, D22 nested request validation, and explicit defaults pass final host/parity tests and strict target gates. |
| Package and host acceptance | Package owners | Complete | Node 24.21.0; build/typecheck; eight host/parity and 15 browser tests; actual installed nine-file tarball runtime and positive/negative strict types pass. Final 160-source/nine-artifact hashes match. |
| Documentation and release lane | Documentation owner | Complete | Architecture, Arc payload ownership, migration, generated API, local release lane, and final receipts documented; bash syntax and 334 focused docs/policy checks pass. Contracts and this completed ledger are archived. |
| Final integration and metrics | Manager and fresh reviewers | Complete | Final workspace 323 active tests/five optional ignored, formatting, strict Clippy, and Python 1,581 pass. Composition and native 168-source/three-binary plus package 160-source/nine-artifact identities pass. Current 180 BBQ inputs yield 1,699 projected banks with zero divergences; rendered-table parity passes. All 12 comparable timing exports pass payload checks; the collision increase does not reproduce in seven alternating pairs. |

## Decisions and propagation

- D1-D9: exact portable model, reader/writer, registry, media, archive, and host contracts in the
  contract document. Core, engines, native, callers, and adapter implement the same signatures.
- D10: existing fixed ZIP titles remain; explicit title/date continues to affect exam YAML.
- D11: `qti-engines::AssetOverlay` is the shared recovered-first owner used by conversion and native.
- D13: registry `output_name_for_content` owns all eleven native filename mappings; callers delegate.
- D14: inspect all plausible EOCD/associated ZIP64 candidates before constructing
  zip-rs. Safe-input bounds may reject an unusual input embedding a plausible overlimit archive.
- D15: retain outer logical input/output names separately from authored media/member source in
  Wasm error provenance; focused regression and final generated-call verification pass.
- D16: file artifacts and companions use per-file atomic persistence; all target types and conflicts
  are preflighted before mutation. PLE retains staged whole-directory publication; native correction
  and final native verification pass.
- D17-D18: native rendering receives supported kinds explicitly, reads source images only inside
  selected rendered display-table fragments, returns generated assets only, and uses the fallible
  core rewrite map to preserve source identity, kind, order, and numbering. Correction and fresh
  specification/runtime acceptance pass.
- D19: index EOCD/ZIP64 candidates once before decoder construction. Preserve D14 rejection
  behavior while removing quadratic repeated prefix scans; 36 focused tests, four-size dense
  candidate timing probes, and fresh specification acceptance pass.
- D20: browser examples expose relative companion names and preserve directory outputs through
  a ZIP download envelope; shared File/Directory artifact types remain unchanged. Corrected UI
  passes all 15 browser checks.
- D21: `npm test` includes both host and native/Wasm parity tests. `test:parity` remains a focused
  convenience command. The release lane runs the combined test command once.
- D22: the JavaScript boundary rejects unknown request fields before tsify deserialization, matching
  native serde's contract. Browser specification accepts corrected source and runtime; final
  generated artifact after payload corrections passes final host/browser acceptance.
- D23: shared Rust payloads use canonical reference-counted ownership to avoid per-item media
  cloning during native fan-out; JavaScript inputs/outputs retain independent owned copies.
  Implementation, allocation-identity regression, and fresh specification/quality pass.
- D24: native missing-source reads use `MissingAsset`, preserving `AssetRead` for provider failures;
  correction, taxonomy regression, and fresh review pass.

A decision is closed only after affected owners restate it, apply it, and verify its consequence.
Specification findings remain attached to the relevant row until correction and re-review land.
Do not replace required behavior with a smaller test lane or a documentation-only exception.

## Completion gates

- Workspace formatting, tests, strict Clippy, and Python hygiene pass on the final source tree.
- Same core, engine, and integrity sources compile natively and for `wasm32-unknown-unknown`.
- Native CLI filenames, warnings, package contents, rendering, and PLE ownership remain verified.
- BBQ plus image and PLE conversions pass through generated Node and browser calls.
- All four readers and eleven writers have native/Wasm parity evidence with explicit context.
- Node 24, Chromium, Firefox, WebKit, worker conversion, and downloads pass acceptance.
- Bundle size and conversion timing use recorded commands, source/input hashes, and runtime versions.
- Fresh specification, quality, and final integration reviews accept the final bytes and evidence.

Local release checks remain manual. `RUST_RELEASE_WASM=1` selects package/browser acceptance;
`RUST_RELEASE_HTML_TO_IMAGE=1` selects the native rendering parity lane. Selected missing or failed
prerequisites fail clearly and require correction plus rerun. No release or npm publication is implied.
