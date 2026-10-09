# File structure

## Top-level layout

```text
.
+- .cargo/                         Cargo aliases and local configuration
+- crates/                          Workspace library and application crates
+- devel/                           Maintainer checks and release helpers
+- docs/                            Durable documentation and active plans
+- packages/qti-wasm/               Local TypeScript/Wasm host package
+- tests/                           Repository-policy tests and small fixtures
+- xtask/                           Development-only Cargo subcommands
+- Cargo.toml                       Workspace members and shared metadata
+- Cargo.lock                       Exact tested dependency resolution
+- VERSION                          Workspace release version
+- README.md                        Project landing page and current status
+- source_me.sh                     Python development environment setup
+- refactor_progress.md             Rust-port milestone ledger
+- pip_requirements-dev.txt         Pinned Python oracle development dependencies
+- LICENSE.LGPL-3.0                 Project license
`- AGENTS.md                        Repository operating guidance
```

[Cargo.toml](../Cargo.toml) is the source of workspace membership, edition, Rust-version floor,
license, repository metadata, and shared dependencies. Each package inherits that metadata in its
own `Cargo.toml`. [Cargo.lock](../Cargo.lock) records the exact dependency graph used for testing.

## Workspace crates

```text
crates/
+- qti-core/                        Validated domain model and package primitives
|  +- src/item.rs                   Seven assessment item shapes
|  +- src/bank.rs                   Ordered items and validation facade
|  +- src/bank/                     Media collection, bank merge, and bank tests
|  +- src/media/                    Media policy, resolution, naming, rewrite, packaging
|  +- src/manifest.rs               QTI manifest generation
|  +- src/zip.rs                    Deterministic ZIP assembly
|  `- src/lib.rs                    Public core facade
+- qti-engines/                     Registered import and export formats
|  +- src/traits.rs                 Reader, writer, hook, and outcome contracts
|  +- src/registry.rs               Registered engine metadata and factories
|  +- src/conversion.rs             Shared read/write/convert and AssetOverlay
|  +- src/blackboard_export_zip/    Blackboard export reader and writer
|  |  +- write.rs                   Blackboard export writer facade
|  |  `- write/                     ZIP assembly, item XML, and HTML normalization
|  +- src/blackboard_qti_v2_1/      Writer facade, item XML, fragments, and tests
|  +- src/text2qti/                Parser, reader, writer, media, and tests
|  +- src/ple_native_json/          PLE source model, mapping, scan, export, writer
|  +- src/<format>/                 One module for each other format engine
|  `- src/lib.rs                    Portable engine facade
+- qti-integrity/                   Independent completed-package checker
|  +- src/input.rs                  Bounded byte ZIP and entry-map validation
|  +- src/input/zip_directory.rs    ZIP directory preflight before decoding
|  +- src/checker.rs                XML, manifest, media, and format-check dispatcher
|  +- src/checker/                  Blackboard, grading, identifiers, manifest, media, XML, image checks
|  `- src/checker_tests.rs          Focused checker regressions
+- qti-raster/fonts/                Bundled font assets and licenses (renderer removed)
+- qti-render/                      Portable rendering contract
|  +- src/render_plan.rs            Selection, dependency planning, PNG validation, finalization
|  +- src/canvas_script.rs          Bounded static canvas parser
|  +- src/canvas_source.rs          CanvasSource and source-owned RDKit options/query
|  +- src/selectors.rs              Eligible display fragments
|  +- src/naming.rs                 Item-scoped generated media names
|  `- src/wrapper.rs                Shared static capture document
+- qti-native/                      Native host services
|  +- src/assets.rs                 Canonical-root lazy DirectoryAssets
|  +- src/input.rs                  Bounded filesystem loading and package checks
|  +- src/persistence.rs            File/companion persistence dispatcher
|  +- src/ple_output.rs             PLE staging and publication facade
|  +- src/ple_output/               Ownership verification and native regressions
|  +- src/metadata.rs               Once-resolved native document context
|  +- src/html_to_image/            Native Chromium/Rayon/RDKit execution and cache
+- qti-wasm/                        Typed Wasm adapter
|  +- src/transport.rs              Rust-authoritative tsify request/result types
|  +- src/adapter.rs                Pure owned transport validation/dispatch
|  +- src/boundary.rs               JavaScript preflight and wasm-bindgen exports
|  +- src/render_transport.rs       Typed render jobs and completions
|  +- src/render_adapter.rs         Stateless planning and final conversion
|  `- src/diagnostics.rs            Structured error and warning transport
+- qti-molecule/                    Optional RDKit canvas renderer
|  +- src/renderer.rs               Safe runtime shim loader
|  +- native/                       Versioned C ABI shim sources and build helper
|  `- tests/                        Canvas corpus and shim integration coverage
`- qti-cli/                         Native command-line boundaries
   +- src/app.rs                    Parsing, dispatch, conversion, reporting
   +- src/bin/                      Thin bbq-converter and qti-package-maker binaries
   `- tests/cli_contract.rs         Process-level CLI behavior checks
```

The literal `<format>` placeholder in the tree represents format-specific directories under
[crates/qti-engines/src/lib.rs](../crates/qti-engines/src/lib.rs), including BBQ text upload, Canvas
QTI 1.2, Blackboard QTI 2.1, Blackboard export ZIP, exam YAML, standalone self-test,
human-readable HTML, Moodle Aiken, Okla CHRST BQGen, text2qti, and PLE Native JSON.

## Development and verification

```text
packages/qti-wasm/
+- src/index.ts                    Explicit Wasm initialization and generated exports
+- examples/                       Node, browser, and worker host examples
+- scripts/                        Build, local serving, and measurements
+- tests/                          Node parity and maintained browser acceptance
+- package.json                    Private package commands and exports
+- package-lock.json               Tested npm dependency resolution
+- tsconfig*.json                  Strict main/worker TypeScript configurations
`- playwright.config.ts            Chromium, Firefox, and WebKit acceptance

xtask/
+- src/main.rs                      Development-command dispatcher
+- src/crc_corpus.rs                CRC agreement corpus command
+- src/oracle_crosscheck.rs         Integrity-oracle comparison command
+- src/parity.rs                    Differential export-parity orchestration
+- src/parity_inputs.rs             Parity input selection and validation
+- src/parity_process.rs            Process invocation and output comparison
+- src/parity_receipts.rs           Media and order receipt comparison
+- src/table_corpus.rs              Table and canvas corpus harvesting
+- src/table_bench.rs               Native conversion benchmark command
+- src/table_gallery.rs             Native and Python review gallery command
`- support/                         Python helpers used only by xtask commands
   +- parity_oracle.py              Oracle CLI adapter and dispatch
   +- parity_oracle_writer.py        Registered Python writer calls and receipts
   +- parity_oracle_qti12.py         QTI 1.2 XML projection and repairs
   `- parity_oracle_projection.py    YAML, Aiken, self-test, and HTML projections

devel/
+- rust_release_check.sh            Local Rust release-gate runner
+- check_rust_release_metadata.py   Workspace metadata verification
+- clean_build.sh                   Build-directory cleanup helper
`- make_release.py                  Release preparation helper

tests/
+- fixtures/bb_export_slice.zip     Permanent Blackboard export fixture
+- test_*.py                        Repository-policy checks
+- TESTS_README.md                  Test-policy guidance
`- _temp/                           Ignored one-time evidence and generated probes
```

[xtask/src/main.rs](../xtask/src/main.rs) is development tooling, not production conversion code.
[devel/rust_release_check.sh](../devel/rust_release_check.sh) defines local validation.
[devel/make_release.py](../devel/make_release.py) prepares manual source releases. [PARITY.md](PARITY.md) records the parity authority.

## Generated artifacts

[.gitignore](../.gitignore) excludes Cargo `target/` directories, temporary test evidence under
`tests/_temp/`, and `output*` directories. In a developer checkout, `output_tables/gallery/`
contains generated table and canvas corpora,
benchmark receipts, and timestamped native/reference gallery runs. They are review evidence and
are regenerated by `cargo xtask table-corpus`, `cargo xtask table-bench`, and
`cargo xtask table-gallery`; they are not source artifacts.

Rust build products are written to `target/` by Cargo. Python helper caches can appear below
`xtask/` and `tests/_temp/`; they are also ignored.
The Wasm package regenerates `generated/` and `dist/`; generated declarations and runtime bytes
are build artifacts, while Rust transport types and the small TypeScript wrapper are authored source.

## Documentation map

- [README.md](../README.md) introduces the workspace and its certification state.
- [INSTALL.md](INSTALL.md) explains source builds and the optional RDKit shim.
- [USAGE.md](USAGE.md) documents native command-line interfaces and output contracts.
- [WASM_PACKAGE.md](WASM_PACKAGE.md) documents byte ownership, package loading, and host examples.
- [RUST_STYLE.md](RUST_STYLE.md) defines Rust and Cargo conventions.
- [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) defines Chromium rendering and the static-canvas contract.
- [PARITY.md](PARITY.md) records completed parity evidence and open scope.
- [DEPENDENCY_DECISIONS.md](DEPENDENCY_DECISIONS.md) and
  [RDKIT_DEPENDENCY_DECISION.md](RDKIT_DEPENDENCY_DECISION.md) record dependency choices.
- [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) describes component boundaries and data flow.
- [archive/rust_port_plan.md](archive/rust_port_plan.md) is the completed
  Rust-port plan; [refactor_progress.md](../refactor_progress.md) records milestone state.

## Where to add new work

- Put production Rust in the crate that owns its data or side effect. Keep `src/lib.rs` and
  `src/bin/` as small facades or adapters; create a responsibility-named module for behavior.
- Put Rust unit tests beside their owner and integration contracts in the owning crate's `tests/`
  directory. Put repository-policy tests under [tests/TESTS_README.md](../tests/TESTS_README.md).
- Put development evidence commands under [xtask/src/main.rs](../xtask/src/main.rs) and Python
  support under [xtask/support/parity_oracle.py](../xtask/support/parity_oracle.py).
- Put durable reference documents in `docs/`, in-flight work in `docs/active_plans/`, and
  completed plans in `docs/archive/`, following [REPO_STYLE.md](REPO_STYLE.md).
