# File structure

## Top-level layout

```text
.
+- .cargo/                         Cargo aliases and local configuration
+- crates/                          Workspace library and application crates
+- devel/                           Maintainer checks and release helpers
+- docs/                            Durable documentation and active plans
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
|  +- src/bank.rs                   Ordered banks and media-base ownership
|  +- src/media/                    Media policy, resolution, naming, rewrite, packaging
|  +- src/manifest.rs               QTI manifest generation
|  +- src/zip.rs                    Deterministic ZIP assembly
|  `- src/lib.rs                    Public core facade
+- qti-engines/                     Registered import and export formats
|  +- src/traits.rs                 Reader, writer, hook, and outcome contracts
|  +- src/registry.rs               Registered engine metadata and factories
|  +- src/html_to_image/            Selection, static canvas parsing, conversion, cache
|  +- src/blackboard_export_zip/    Blackboard export reader and writer
|  +- src/<format>/                 One module for each other format engine
|  `- tests/browser_proof.rs        Standalone self-test browser evidence
+- qti-integrity/                   Independent completed-package checker
|  +- src/input.rs                  Bounded ZIP and directory input handling
|  +- src/checker.rs                XML, manifest, media, and format checks
|  `- src/checker_tests.rs          Focused checker regressions
+- qti-raster/fonts/                Bundled font assets and licenses (renderer removed)
|  +- src/subset.rs                 Parser and typed unsupported-feature errors
|  +- src/style.rs                  Styles, defaults, and CSS values
|  +- src/table_layout/             Grid and collapsed-border layout helpers
|  +- src/inline_layout.rs          Text, inline, and decorated-box layout
|  +- src/render_assembly.rs        Display-list assembly
|  +- src/paint.rs                  Raster painting and PNG encoding
|  `- fonts/                        Embedded Atkinson font files and licenses
+- qti-molecule/                    Optional RDKit canvas renderer
|  +- src/source.rs                 Validated CanvasSource model
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
human-readable HTML, Moodle Aiken, Okla CHRST BQGen, and text2qti.

## Development and verification

```text
xtask/
+- src/main.rs                      Development-command dispatcher
+- src/crc_corpus.rs                CRC agreement corpus command
+- src/oracle_crosscheck.rs         Integrity-oracle comparison command
+- src/parity.rs                    Differential export-parity command
+- src/table_corpus.rs              Table and canvas corpus harvesting
+- src/table_bench.rs               Native conversion benchmark command
+- src/table_gallery.rs             Native and Python review gallery command
`- support/                         Python helpers used only by xtask commands

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

## Documentation map

- [README.md](../README.md) introduces the workspace and its certification state.
- [INSTALL.md](INSTALL.md) explains source builds and the optional RDKit shim.
- [USAGE.md](USAGE.md) documents native command-line interfaces and output contracts.
- [RUST_STYLE.md](RUST_STYLE.md) defines Rust and Cargo conventions.
- [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) defines the supported native-rendering subset.
- [PARITY.md](PARITY.md) records completed parity evidence and open scope.
- [DEPENDENCY_DECISIONS.md](DEPENDENCY_DECISIONS.md) and
  [RDKIT_DEPENDENCY_DECISION.md](RDKIT_DEPENDENCY_DECISION.md) record dependency choices.
- [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) describes component boundaries and data flow.
- [active_plans/active/rust_port_plan.md](active_plans/active/rust_port_plan.md) is the active
  Rust-port plan; [refactor_progress.md](../refactor_progress.md) records milestone state.

## Where to add new work

- Put production Rust in the crate that owns its data or side effect. Keep `src/lib.rs` and
  `src/bin/` as small facades or adapters; create a responsibility-named module for behavior.
- Put Rust unit tests beside their owner and integration contracts in the owning crate's `tests/`
  directory. Put repository-policy tests under [tests/TESTS_README.md](../tests/TESTS_README.md).
- Put development evidence commands under [xtask/src/main.rs](../xtask/src/main.rs) and Python
  support under [xtask/support/parity_oracle.py](../xtask/support/parity_oracle.py).
- Put durable reference documents directly in [RUST_STYLE.md](RUST_STYLE.md). Put in-flight plans,
  decisions, reports, and workstreams under
  [active_plans/active/rust_port_plan.md](active_plans/active/rust_port_plan.md), as described by
  [REPO_STYLE.md](REPO_STYLE.md).
