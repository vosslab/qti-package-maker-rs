# QPM namespace migration

QPM abbreviates QTI Package Maker. Rust/Wasm and project-owned integration identifiers use
QPM; established full names, Python APIs, semantic names such as `qti_packer`, and serialized
format markers remain unchanged. QTI terminology remains for the interoperability standard,
format identifiers, XML schemas, and interoperability tools. This migration includes the
existing CSS-validation work without changing its strict policy.

| Surface | Current name |
| --- | --- |
| Cargo crates and Rust imports | `qpm-*`, `qpm_*` |
| Wasm package | `packages/qpm-wasm`, `@vosslab/qpm-wasm` |
| Generated bindings | `qpm_wasm.js`, `qpm_wasm_bg.wasm` |
| Inspection executable | `qpm` |
| Conversion executable | `bbq-converter` |
| Python package/distribution and interface | `qti_package_maker`, `QTIPackageInterface` |
| HTML/CSS/DOM hooks | `qpm-*`, `qpm` JavaScript properties |
| Native RDKit shim | `libqpm_rdkit_shim`, `qpm_rdkit_*` symbols |
| Cargo cache | `qpm-rust-cargo` |

`blackboard_qti_v2_1`, `canvas_qti_v1_2`, `text2qti`, `QtiVersion`, QTI XML namespaces,
Standard-specific comparison helper names were preserved during renaming; those migration
helpers were subsequently retired in the separate test-infrastructure cleanup. Repository URLs and physical sibling checkout paths remain accurate; renaming local
packages does not rename a GitHub repository or publish a PyPI release. The Python package,
distribution, and `QTIPackageInterface` API retain their established names. Migrated Rust/Wasm,
CLI, DOM, and native ABI identifiers have no compatibility aliases.

## Explicit configuration

- Native rendering discovers Chromium and the installed/development RDKit shim normally.
  Nonstandard paths are `bbq-converter --html-to-image --chromium PATH --rdkit-shim PATH`.
- Library callers pass paths directly to renderer constructors.
- Build the shim with `crates/qpm-molecule/native/build_shim.sh`; SDK/output overrides are
  explicit arguments listed by `--help`.
- Release checks select `--native-renderers` and `--wasm`. The Linux helper accepts
  `--image` and `--smoke-only`.
- The retained table benchmark accepts `--python-qpm PATH`. Corpus subprocesses receive import paths
  through standard `PYTHONPATH` rather than a custom environment channel.
- BPW and PLE vendor helpers accept `--qpm-root PATH`.

Standard OS and toolchain settings such as PATH, HOME, CARGO_HOME, CARGO_TARGET_DIR, PYTHONPATH,
CXX, and CXXFLAGS retain their normal meaning. Removed custom settings have no renamed aliases.

## Consumer handoff

BPW and PLE vendor the rebuilt package using their existing `devel/vendor_qpm_wasm.py` helpers.
Their loader paths, generated wrappers, Wasm files, and hash receipts change together. BPW's
topic indexes and self-test/progress selectors use the new hooks. BP and other Python consumers
continue importing `qti_package_maker` through the established Python API. No site or package was published as part of this local migration.

## Verification

These results record the completed migration before temporary parity tooling was retired.

- Established Python imports and `QTIPackageInterface` work; Python QPM's full suite passes
  3,893 tests. Rust repository Python checks pass 1,940 tests.
- The rebuilt Wasm package passes 13 Node/native parity tests, 42 browser tests, and three
  Python-to-delivered-Wasm parity tests across Chromium, Firefox, and WebKit.
- BP checkout-path and bptools tests pass 165 checks. Filesystem helpers use the real
  `qti-package-maker` checkout, whose importable child package is `qti_package_maker`.
- BPW and PLE each verify all eight vendored artifact hashes. BPW's real Canvas/Blackboard
  download checks pass; PLE's production bundle builds.
- The pre-existing Rust ZIP size-limit test remains a separate failure reproduced before
  this migration. The remaining 340 native tests pass when that one test is explicitly skipped;
  three optional native-shim tests pass in their separate renderer check. Strict Clippy and
  the native release build pass.
- The broader BPW smoke lane still exposes three known invalid source CSS cases (`spacing`,
  a unitless nonzero `height`, and a hex color missing `#`). QPM rejects them as required;
  BP source generators own their correction.

This is local build and integration evidence; no package, site, or remote repository was published.
