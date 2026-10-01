# Install

Build the native command-line tools from this source checkout. No published installer or release
artifact is certified yet.

## Requirements

- Rust 1.98.1 or later with Cargo.
- macOS or Linux source-build environment. The current native development target is macOS arm64.
- Python is not required to build or run either production CLI binary.

## Build from source

From the repository root, create optimized native binaries:

```bash
cargo build --locked --release -p qti-cli --bins
```

Cargo writes these binaries into `target/release/`:

- `bbq-converter` converts BBQ question files.
- `qti-package-maker` inspects engines, item kinds, and completed packages.

## Verify the build

```bash
./target/release/bbq-converter --help
./target/release/qti-package-maker engines
```

The first command prints conversion options. The second reports the registered reader and writer
capabilities. These commands run without Python.

## Optional RDKit canvas support

Ordinary builds and conversions without static RDKit canvases need no RDKit SDK, Python, compiler,
or shared library. A conversion using `--html-to-image` loads the optional shim only when it needs
to render an RDKit canvas. Set the shim path for the target platform:

```bash
# macOS
export QTI_RDKIT_SHIM=/absolute/path/to/libqti_rdkit_shim.dylib

# Linux
export QTI_RDKIT_SHIM=/absolute/path/to/libqti_rdkit_shim.so
```

Release engineering builds the shim separately from Cargo. The supported source-build commands,
runtime package names, ABI contract, and failure behavior are in
[../crates/qti-molecule/README.md](../crates/qti-molecule/README.md) and
[RDKIT_DEPENDENCY_DECISION.md](RDKIT_DEPENDENCY_DECISION.md).

The recorded evidence includes a macOS arm64 corpus run and Debian amd64 shim and corpus execution
under Podman x86 emulation. The Debian result proves that ABI/runtime combination under emulation;
it does not establish native x86 hardware or completed release certification.

## Development-only oracle tools

`cargo xtask` contains corpus, oracle, parity, and benchmark commands. They use the pinned Python
reference through `source_me.sh` and are development or release-gate tooling, not a production
runtime dependency. See [PARITY.md](PARITY.md) and
[../refactor_progress.md](../refactor_progress.md).

## Manual release preparation

Run the local release checks against the pinned Python reference checkout before preparing a
source release. The native conversion checks additionally need the installed RDKit shim.
The check script runs both deterministic fixtures and the complete harvested corpus;
keep `output_tables/corpus` available for this validation.

```bash
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo \
RUST_RELEASE_PYTHON_QTI="/path/to/qti-package-maker" \
bash devel/rust_release_check.sh
source source_me.sh && python3 devel/make_release.py --dry-run
```

`RUST_RELEASE_PYTHON_QTI` must be a Git checkout that contains the pinned Python
reference commit; the generated `output_tables/oracle_snapshot/` copy is immutable
oracle data, not a Git source checkout.

Set `RUST_RELEASE_HTML_TO_IMAGE=1` and `QTI_RDKIT_SHIM` to include native conversion
comparisons. This optional lane currently exits nonzero on the raw HTML differences
documented in [PARITY.md](PARITY.md), including frozen Python defects. Review findings
against source content, grading, media, and package integrity before deciding whether a
product fix is needed. The classified corpus evidence does not make that command pass;
new or changed findings still need investigation.

To run the complete Linux amd64 check locally under Podman, including the native RDKit lane, use:

```bash
bash devel/run_linux_release_check.sh
```

The helper uses an immutable Rust 1.98.1 Debian 13 image, clones committed HEAD and the local
pinned Python Git commit into ignored output_tables/, copies the harvested corpus, builds the Debian
RDKit shim, and runs the same full local gate. On an arm64 macOS host this is amd64 emulation
evidence, not a native Linux-hardware receipt. If the emulated Rust executable cannot start, run
this script on a native Linux host; a failed emulator startup is not a release pass.

The release helper prints a notes-drafting prompt when no notes file is supplied. Supply
`--notes-file <path>` to preview a release with prepared notes, then use `--write` to build source
archives from committed HEAD. It prints tagging and publication commands for manual execution.
It does not run the Rust validation checks or publish a release itself.

## Known gaps

- Complete release validation from the intended committed source before preparing a release with
  `devel/make_release.py`. Working-tree builds on macOS arm64 and Linux arm64 pass; the Linux
  x86_64 release binaries are cross-built and pass three-format table conversion and package
  checks under amd64 emulation.
- Record the user's visual decision on the native table gallery. Corpus conversion, package
  integrity, and the comparison against Python's conversion time have passed.
