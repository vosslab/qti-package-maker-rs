# Code architecture

## Overview

`qti-package-maker-rs` is a Rust 2024 workspace that reads BBQ question banks, represents seven
validated assessment-item kinds, optionally converts supported HTML tables and static molecule
canvases to PNG media, and writes assessment packages. It also inspects completed packages and
provides parity, corpus, benchmark, and gallery tooling. [Cargo.toml](../Cargo.toml) defines the
eight workspace members and the Rust 1.98.1 minimum version. The native default member set
excludes `qti-wasm`; the browser package is built explicitly.

Each crate root is a small public facade. It re-exports stable types and functions while focused
implementation modules remain private where possible.

## Major components

| Component | Responsibility | Public boundary |
| --- | --- | --- |
| [crates/qti-core/Cargo.toml](../crates/qti-core/Cargo.toml) | Validated items, ordered banks, CRCs, fingerprints, media, manifests, and ZIP assembly. | [crates/qti-core/src/lib.rs](../crates/qti-core/src/lib.rs) exports the item, bank, media, manifest, and ZIP contracts. |
| [crates/qti-engines/Cargo.toml](../crates/qti-engines/Cargo.toml) | Portable format readers/writers, one registry, typed byte artifacts, and conversion orchestration. | [crates/qti-engines/src/traits.rs](../crates/qti-engines/src/traits.rs) and [crates/qti-engines/src/registry.rs](../crates/qti-engines/src/registry.rs). |
| [crates/qti-native/Cargo.toml](../crates/qti-native/Cargo.toml) | Filesystem input, lazy directory assets, atomic persistence, PLE ownership, local context, and native rendering. | [crates/qti-native/src/lib.rs](../crates/qti-native/src/lib.rs) exports host services. |
| [crates/qti-wasm/Cargo.toml](../crates/qti-wasm/Cargo.toml) | Owned typed JavaScript transport over the shared engine and integrity API. | [crates/qti-wasm/src/lib.rs](../crates/qti-wasm/src/lib.rs) and [WASM_PACKAGE.md](WASM_PACKAGE.md). |
| [crates/qti-molecule/Cargo.toml](../crates/qti-molecule/Cargo.toml) | Optional runtime RDKit C-ABI loading and molecule-canvas PNG rendering. | [crates/qti-molecule/src/lib.rs](../crates/qti-molecule/src/lib.rs) exports `CanvasSource`, `RdkitRenderer`, and typed errors. |
| [crates/qti-integrity/Cargo.toml](../crates/qti-integrity/Cargo.toml) | Independent finished-package inspection without ZIP extraction. | [crates/qti-integrity/src/lib.rs](../crates/qti-integrity/src/lib.rs) exports package checks and violations. |
| [crates/qti-cli/Cargo.toml](../crates/qti-cli/Cargo.toml) | Native command parsing, conversion fan-out, reporting, and inspection commands. | [crates/qti-cli/src/app.rs](../crates/qti-cli/src/app.rs) owns application flow. |
| [xtask/Cargo.toml](../xtask/Cargo.toml) | Development-only corpus, parity, oracle, benchmark, and gallery commands. | [xtask/src/main.rs](../xtask/src/main.rs) dispatches the subcommands. |

### Core data and media

[crates/qti-core/src/item.rs](../crates/qti-core/src/item.rs) defines exhaustive Rust enums for
item bodies. [crates/qti-core/src/bank.rs](../crates/qti-core/src/bank.rs) owns ordered items and
validation; its `bank/` children own provider-based media collection, bank merging, and focused
tests. [crates/qti-core/src/media/mod.rs](../crates/qti-core/src/media/mod.rs) and its child
modules resolve source media, apply format policy, name package members, rewrite HTML, and assemble
package media. `AssetSource` reads exact authored references; `MemoryAssets` owns validated named
bytes. `MediaAsset` owns resolved bytes, with no host file path. Metadata-only inspection supports
reference/placeholder writers without reading local payloads. Writers return structured warnings.
Resolved media snapshots and `NamedFile` use `Arc<[u8]>` so dependencies and PLE question files
share immutable payloads. Rust output-boundary methods explicitly copy to `Vec<u8>` when needed;
the JavaScript boundary returns independent owned arrays.

### Engines and package contracts

Each format module below [crates/qti-engines/src/lib.rs](../crates/qti-engines/src/lib.rs)
implements a reader, a writer, or both. The Blackboard export ZIP writer keeps its public
integration point in `write.rs`, with package assembly, item XML, and HTML normalization in its
`write/` children. Blackboard QTI 2.1 keeps writer behavior and item XML separate; `fragment.rs`
owns shared fragment and XML escaping helpers so item rendering has a one-way dependency. `mod.rs`
is the small public facade. The text2qti module similarly keeps
parser, reader, writer, media, and tests in separate files while `mod.rs` provides the registry
constructors. The registry is the single available-engine list and
describes all four readers and eleven writers, their supported kinds, media policies, naming, and
native-render eligibility. `ReadInput` carries borrowed named bytes or entries. `WriteContext`
carries explicit output name, document metadata, and seed. `Writer::write_package` returns a
`WriteOutcome` containing optional owned `WriteArtifact` bytes and warnings.

The PLE Native JSON engine maps each validated item to one of seven response kinds; its source
model is in [source.rs](../crates/qti-engines/src/ple_native_json/source.rs). The public
`export_bank` function returns per-question
JSON, item identity, associated file bytes, and warnings for direct library consumption. The
writer uses that export to assemble a logical directory of question JSON and `media/` bytes.
Native persistence stages and publishes it with an ownership manifest. QPM validates
its mapping and lossless transport; PLE's decoder owns its private count, size, and length limits.
For PLE, media collection runs over mapped display fields only and uses the shared `qti-core::media`
scan, resolution, naming, policy, and HTML rewrite functions. Whole-bank media traversal would
inspect FIB accepted-answer literals and could change grading or report false missing files.

[crates/qti-native/src/html_to_image/mod.rs](../crates/qti-native/src/html_to_image/mod.rs)
selects supported fragments, statically parses permitted canvas scripts, renders canvases before
tables, assigns item-scoped names, and uses a content-hash cache. The CLI invokes this conversion
once before sending the converted bank to eligible selected writers.

### Table and molecule rendering

```text
HTML fragment with embedded media
  -> shared Chromium session
  -> browser HTML/CSS layout
  -> document-bounds PNG capture
```

Chromium owns HTML/CSS layout. The adapter serializes access to one lazily created page;
conversion caching and package fan-out remain in Rust. Embedded Atkinson fonts and licenses
remain under `crates/qti-raster/fonts/`; the custom rasterizer crate has been removed.
[HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) documents rendering and error behavior.

[crates/qti-molecule/src/renderer.rs](../crates/qti-molecule/src/renderer.rs) loads an explicitly
named shim with `libloading`, verifies ABI version 1, validates canvas dimensions and colors before
FFI, copies returned bytes into Rust ownership, and uses paired shim deallocators. The install and
platform contract is [RDKIT_DEPENDENCY_DECISION.md](RDKIT_DEPENDENCY_DECISION.md).

## Primary data flow

```text
Native paths -> qti-native input/DirectoryAssets -> shared read_bank
Browser bytes -> qti-wasm owned transport       -> shared read_bank
  -> qti-core ItemBank plus recovered MemoryAssets
  -> native-only optional rendering -> bank plus generated MemoryAssets
  -> shared AssetOverlay and write_bank
  -> WriteArtifact files/companions/directory entries
  -> native persist_artifact or browser/Node byte handoff
  -> shared qti-integrity byte/entry inspection when requested
```

`bbq-converter` parses arguments, selects registry entries, reads the BBQ bank, applies any item
limit, runs optional conversion, and calls selected writers in order. `qti-package-maker` lists
engines and item types or delegates finished-package inspection to `qti-integrity`.

Portable `convert` composes `read_bank` (read then trim) and `write_bank` once. Native fan-out
reuses those phases with one rendering pre-pass. `AssetOverlay`, owned by `qti-engines`, serves
recovered/generated memory first and the caller's provider only for absent sources. Providers
and read/render outcomes own bytes; item banks own no temporary directories.

`qti-native` exports `read_source`, `read_package_entries`, `check_package_path`, `DirectoryAssets`,
context resolution, and `persist_artifact`. The canonical input root confines native local media
reads, including authored absolute paths and symlinks. Host persistence validates names again
before joining output roots. PLE replacement verifies the prior manifest and staged bytes.
`resolve_write_context(output_name, title)` resolves local date and seed once and constructs a
validated shared context; native writer fan-out reuses those metadata values.

The package under `packages/qti-wasm` wraps Rust-generated `tsify` declarations and Wasm loading.
Its explicit initialization, registry inventory, conversion, and `checkPackage` API use owned
`Uint8Array` input/output copies. No parsing or writer implementation is duplicated in TypeScript.

## Testing and verification

The independent package checker in `crates/qti-integrity/src/checker.rs` dispatches checks over
safe in-memory package entries. Its child modules own XML parsing, manifest and grading checks,
media references, Blackboard export rules, identifiers, and image dimensions.

`xtask/src/parity.rs` coordinates differential comparisons; input selection, process execution,
receipt handling, and adjacent comparison work live in focused parity modules. The transitional Python
oracle adapter in `xtask/support/parity_oracle.py` retains command parsing and delegates writer,
QTI 1.2, and projection behavior to support modules. Active migration checks resolve current source
and record its identity; historical receipts retain their original provenance. Independent tests
own the lasting Rust behavior contract. See [SELFTEST_COMPATIBILITY.md](SELFTEST_COMPATIBILITY.md).

Unit tests live with their owning Rust modules. Process contracts live in
[crates/qti-cli/tests/cli_contract.rs](../crates/qti-cli/tests/cli_contract.rs), browser acceptance
in [packages/qti-wasm/tests/browser.spec.ts](../packages/qti-wasm/tests/browser.spec.ts), and
native-shim coverage in [crates/qti-molecule/tests/corpus.rs](../crates/qti-molecule/tests/corpus.rs).
Repository policy tests under [tests/TESTS_README.md](../tests/TESTS_README.md) enforce local
documentation, source-size, and tooling rules.

[RUST_STYLE.md](RUST_STYLE.md) records the workspace gates: `cargo fmt --check`, `cargo check`,
`cargo test`, and `cargo clippy -- -D warnings`. `cargo xtask` dispatches development evidence;
[PARITY.md](PARITY.md) states the current parity authority and completed evidence.
Shared native/Wasm verification and measurements are recorded in
[shared_engine_delivery.md](active_plans/reports/shared_engine_delivery.md).

## Extension points

- Add a new item shape in [crates/qti-core/src/item.rs](../crates/qti-core/src/item.rs), validate
  it in [crates/qti-core/src/validate.rs](../crates/qti-core/src/validate.rs), then extend its
  readers, writers, fingerprint, and parity fixture.
- Add an engine in a responsibility-named module under
  [crates/qti-engines/src/lib.rs](../crates/qti-engines/src/lib.rs), implement its trait, and
  register it in [crates/qti-engines/src/registry.rs](../crates/qti-engines/src/registry.rs).
- Keep HTML/CSS layout in Chromium. Change selection, asset preparation, or screenshot capture
  in the owning `html_to_image` module when a demonstrated conversion problem requires it.
- Add CLI behavior in [crates/qti-cli/src/app.rs](../crates/qti-cli/src/app.rs) with a process
  assertion in [crates/qti-cli/tests/cli_contract.rs](../crates/qti-cli/tests/cli_contract.rs).
- Add reproducible development evidence as an `xtask` subcommand instead of a production-crate
  dependency.

## Validation boundaries

Implementation verification covers all formats, the refreshed Chromium corpus, and platform
builds. [PARITY.md](PARITY.md) distinguishes semantic agreement from incidental serialization
and frozen-reference defects. Chromium can delay its first screenshot; see the
[benchmark report](active_plans/reports/html_to_image_native_benchmark.md).

Real LMS imports and native Linux amd64 Chromium execution remain external validation.
Manual release preparation operates on human-committed source, as described in [INSTALL.md](INSTALL.md).
