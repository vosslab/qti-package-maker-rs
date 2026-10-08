# Code architecture

## Overview

`qti-package-maker-rs` is a Rust 2024 workspace that reads BBQ question banks, represents seven
validated assessment-item kinds, optionally converts supported HTML tables and static molecule
canvases to PNG media, and writes assessment packages. It also inspects completed packages and
provides parity, corpus, benchmark, and gallery tooling. [Cargo.toml](../Cargo.toml) defines the
six workspace members and the Rust 1.98.1 minimum version.

Each crate root is a small public facade. It re-exports stable types and functions while focused
implementation modules remain private where possible.

## Major components

| Component | Responsibility | Public boundary |
| --- | --- | --- |
| [crates/qti-core/Cargo.toml](../crates/qti-core/Cargo.toml) | Validated items, ordered banks, CRCs, fingerprints, media, manifests, and ZIP assembly. | [crates/qti-core/src/lib.rs](../crates/qti-core/src/lib.rs) exports the item, bank, media, manifest, and ZIP contracts. |
| [crates/qti-engines/Cargo.toml](../crates/qti-engines/Cargo.toml) | Format readers and writers, engine registry, typed outcomes, and HTML-to-image conversion. | [crates/qti-engines/src/traits.rs](../crates/qti-engines/src/traits.rs) and [crates/qti-engines/src/registry.rs](../crates/qti-engines/src/registry.rs). |
| [crates/qti-molecule/Cargo.toml](../crates/qti-molecule/Cargo.toml) | Optional runtime RDKit C-ABI loading and molecule-canvas PNG rendering. | [crates/qti-molecule/src/lib.rs](../crates/qti-molecule/src/lib.rs) exports `CanvasSource`, `RdkitRenderer`, and typed errors. |
| [crates/qti-integrity/Cargo.toml](../crates/qti-integrity/Cargo.toml) | Independent finished-package inspection without ZIP extraction. | [crates/qti-integrity/src/lib.rs](../crates/qti-integrity/src/lib.rs) exports package checks and violations. |
| [crates/qti-cli/Cargo.toml](../crates/qti-cli/Cargo.toml) | Native command parsing, conversion fan-out, reporting, and inspection commands. | [crates/qti-cli/src/app.rs](../crates/qti-cli/src/app.rs) owns application flow. |
| [xtask/Cargo.toml](../xtask/Cargo.toml) | Development-only corpus, parity, oracle, benchmark, and gallery commands. | [xtask/src/main.rs](../xtask/src/main.rs) dispatches the subcommands. |

### Core data and media

[crates/qti-core/src/item.rs](../crates/qti-core/src/item.rs) defines exhaustive Rust enums for
item bodies. [crates/qti-core/src/bank.rs](../crates/qti-core/src/bank.rs) keeps ordered banks and
media-base ownership. [crates/qti-core/src/media/mod.rs](../crates/qti-core/src/media/mod.rs) and
its child modules resolve source media, apply format policy, name package members, rewrite HTML,
and assemble package media. Writers return structured media warnings through their trait outcome.

### Engines and package contracts

Each format module below [crates/qti-engines/src/lib.rs](../crates/qti-engines/src/lib.rs)
implements a reader, a writer, or both. The registry is the single available-engine list and
describes read, write, and media-policy capabilities. `EngineOptions` carries document metadata
and `html_to_image`; `Writer::save_package` returns `WriteOutcome`, including writers that
intentionally complete without producing an artifact.

The PLE Native JSON engine maps each validated item to one of seven response kinds; its source
model is in [source.rs](../crates/qti-engines/src/ple_native_json/source.rs). The public
`export_bank` function returns per-question
JSON, item identity, associated file bytes, and warnings for direct library consumption. The
writer uses that export to stage a directory of question JSON and `media/` files. QPM validates
its mapping and lossless transport; PLE's decoder owns its private count, size, and length limits.
For PLE, media collection runs over mapped display fields only and uses the shared `qti-core::media`
scan, resolution, naming, policy, and HTML rewrite functions. Whole-bank media traversal would
inspect FIB accepted-answer literals and could change grading or report false missing files.

[crates/qti-engines/src/html_to_image/mod.rs](../crates/qti-engines/src/html_to_image/mod.rs)
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
BBQ input
  -> qti-engines BBQ reader
  -> qti-core ItemBank
  -> optional html_to_image conversion
       -> qti-molecule canvas PNGs
       -> Chromium table PNGs
       -> rewritten item media
  -> selected qti-engines writers
  -> package files, text outputs, or a PLE JSON/media directory
  -> optional qti-integrity inspection
```

`bbq-converter` parses arguments, selects registry entries, reads the BBQ bank, applies any item
limit, runs optional conversion, and calls selected writers in order. `qti-package-maker` lists
engines and item types or delegates finished-package inspection to `qti-integrity`.

## Testing and verification

Unit tests live with their owning Rust modules. Process contracts live in
[crates/qti-cli/tests/cli_contract.rs](../crates/qti-cli/tests/cli_contract.rs), self-test browser
evidence in [crates/qti-engines/tests/browser_proof.rs](../crates/qti-engines/tests/browser_proof.rs),
and native-shim coverage in [crates/qti-molecule/tests/corpus.rs](../crates/qti-molecule/tests/corpus.rs).
Repository policy tests under [tests/TESTS_README.md](../tests/TESTS_README.md) enforce local
documentation, source-size, and tooling rules.

[RUST_STYLE.md](RUST_STYLE.md) records the workspace gates: `cargo fmt --check`, `cargo check`,
`cargo test`, and `cargo clippy -- -D warnings`. `cargo xtask` dispatches development evidence;
[PARITY.md](PARITY.md) states the current parity authority and completed evidence.

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
