# Shared conversion engine baseline

Measured 2026-10-08 on macOS arm64 before the shared-engine refactor. The initial
workspace already contains uncommitted PLE and cohesive-source-split changes.
This baseline preserves those changes; the Git commit alone is not its source identity.

## Preserved evidence

All generated evidence lives in ignored `tests/_temp/wasm_baseline/`:

- `source.tar` and `source/`: tracked and untracked non-ignored source snapshot.
- `initial_status.txt`, `initial_worktree.diff`, and `Cargo.lock`: initial state.
- `bbq-converter-debug`: executable that existed before this run.
- `bbq-converter-release`: optimized executable built from the frozen source snapshot.
- `fixtures/`: deterministic parity fixtures, including all seven question kinds,
  writer projections, relative images, basename collisions, and tables.
- `workspace-test.log`, `release-build.log`, and `xtask-build.log`: build/test receipts.
- `rdkit/`, `rdkit-build.log`, `rdkit-tests.log`, and `rdkit-corpus.log`: native rendering proof.
- `canvas_corpus/`: all 58 harvested native canvas records used in the baseline.
- `timing.py`, `timing_before_native/receipt.json`, and `timing-before-native.log`:
  repeatable commands, input hashes, durations, exit codes, diagnostics, and artifacts.
- `parity-fixtures.log` and `parity-native-fixtures.log`: existing oracle harness receipts.

Git HEAD is `50deac3baa5d83781217399c83a82a6f0e152ba1`.
Snapshot SHA-256 is `0d6aff6d238f8c05c882b6f34b7569e9e1105e2be1451b07d2235f8f07d71eb5`.
Optimized binary SHA-256 is
`084a5bd58a7f32bb8133e08c57d517d9d02ec18432500790851d1643219d0395`.

The snapshot's oracle directory is a link to the immutable local Python snapshot
at commit `55e5f368777f7809fe2e91b5d070caf6df0cb581`. The harness runs with
`GIT_WORK_TREE` pointing at the preserved source, so its required current-workspace
CLI build cannot race with ongoing production edits.

## Native timing

Three fresh subprocesses per workload, optimized build, warm OS caches, no conversion
cache carried between processes. Times include process startup and output persistence.
These small representative fixtures establish reproducible before/after measurements;
they do not measure the complete website build or all production workloads.

| Workload | Median seconds | Output evidence |
| --- | ---: | --- |
| 500 MC questions to Canvas ZIP | 0.5934 | 10,080 bytes; ZIP CRCs pass |
| 500 MC questions to PLE directory | 0.6156 | 149,464 bytes; 501 files |
| Repeated/colliding image paths to PLE | 0.00550 | 830 bytes; 4 files |
| One rendered table to Blackboard ZIP | 0.1590 | 3,962 bytes; 1 PNG; ZIP CRCs pass |

All twelve measured exports succeed. Output sizes describe the first repetition.
Clock-dependent metadata may change subsequent sizes.

The first sandboxed Chromium attempt fails with macOS
`MachPortRendezvousServer ... Permission denied (1100)`. The approved unsandboxed
rerun succeeds. Failed attempts remain in `timing_before/` and are excluded from
the successful timing table above.

To repeat against the final optimized executable, source the project environment,
set `QTI_RDKIT_SHIM` to the preserved `rdkit/libqti_rdkit_shim.dylib`, and run:

```bash
source source_me.sh
python3 tests/_temp/wasm_baseline/timing.py \
  --binary target/release/bbq-converter --label after_native
```

Chromium execution on this host requires the same approved unsandboxed context.

## Validation receipt

`cargo test --workspace --locked` passes: 282 tests, zero failures, five ignored tests.
The optional native RDKit shim builds against the installed SDK. The existing ignored
shim lane passes all three tests, including precise malformed-SMILES and highlight-index
errors. The ignored native corpus lane passes and renders all 58 harvested canvases.
These runs use the frozen snapshot, not the refactored workspace.

The existing parity fixture lane passes with zero divergences: 59 input banks;
seven frozen-CLI formats plus three registered-only adapter formats agree with the
pinned Python oracle. The native HTML-to-image fixture lane also passes with zero
divergences: one table bank across all three packaging ZIP writers. These are the
existing parity harness's structural checks, not browser/Wasm or visual acceptance.

Exact commands (the snapshot variants also pass `--manifest-path` to Cargo or run
the preserved `xtask` binary with `GIT_DIR` and `GIT_WORK_TREE` set as described above):

```bash
cargo test --workspace --locked
cargo build -p qti-cli --bin bbq-converter --release --locked
cargo test -p qti-molecule --test rdkit_shim --locked -- --ignored
cargo test -p qti-molecule --test corpus --locked -- --ignored
source source_me.sh
cargo run --locked -p xtask -- parity --fixtures
cargo run --locked -p xtask -- parity --fixtures --html-to-image
```

The native lanes set `QTI_RDKIT_SHIM`; the corpus lane also sets `QTI_CANVAS_CORPUS`.

## Tool availability

| Component | Verified version or status |
| --- | --- |
| Rust / Cargo | 1.98.1 / 1.98.1 |
| Installed Rust targets | `aarch64-apple-darwin`, `wasm32-unknown-unknown` |
| Node / npm | 26.10.0 / 11.19.1; Node 24 is not the default installed runtime |
| wasm-pack | Absent from PATH at baseline |
| Python | 3.12.15 through `source source_me.sh` |
| Chrome headless shell | Google Chrome for Testing 156.0.8078.4 |
| Firefox application | 157.0.1 |
| Safari application | 27.0 |
| Python RDKit | 2026.03.6 |
| Native Homebrew RDKit SDK | 2026.09.1 |

Firefox and Safari versions here establish installed applications only; no Wasm
browser acceptance is claimed. Playwright Chromium/headless-shell caches exist;
Playwright Firefox and WebKit caches are absent at baseline.
