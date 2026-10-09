# Install

Build the native command-line tools from this source checkout. No published installer or release
artifact is certified yet.

## Requirements

- Rust 1.98.1 or later with Cargo.
- macOS or Linux source-build environment. The current native development target is macOS arm64.
- Python is not required to build or run either production CLI binary.
- Node, npm, wasm-pack, and a Wasm Rust target are not native CLI prerequisites.
- A local Chromium-compatible browser is required only for `--html-to-image` tables. Put it on
  `PATH`, or set `QTI_CHROMIUM` to its executable path.

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

## HTML-to-image tables

Table conversion always uses headless Chromium controlled directly by the Rust binary. Discovery
prefers `chrome-headless-shell` or `chromium-headless-shell` on `PATH`, then the newest headless shell
in the standard Playwright cache, then a full browser launched headlessly. It does
not require a Node.js or Python production runtime. Use `QTI_CHROMIUM` when the browser is not
discoverable on `PATH`:

```bash
export QTI_CHROMIUM=/absolute/path/to/chromium
./target/release/bbq-converter --input questions.txt --qti21 --html-to-image
```

The converter starts Chromium only for a selected input that contains tables. Static RDKit canvases
remain optional and follow the setup below. See [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) for the
rendering and acceptance contract.

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

## Browser package setup

Build the separate local browser/Node package with Node 24 and the Rust
`wasm32-unknown-unknown` target. `npm ci` installs the package-local build and test tools,
including wasm-pack. See [WASM_PACKAGE.md](WASM_PACKAGE.md) for the complete commands and
browser installation step. These tools are development dependencies for the browser package.

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

Set `RUST_RELEASE_WASM=1` to run the browser package lane with Node 24. It checks portable
crates for `wasm32-unknown-unknown`, then runs `npm ci`, build, strict TypeScript checks,
Node host/native-Wasm parity tests, and Chromium/Firefox/WebKit tests in `packages/qti-wasm`. Install the Playwright
browsers first as described in [WASM_PACKAGE.md](WASM_PACKAGE.md). Missing prerequisites and
failed checks stop the selected lane; install or correct the reported requirement and rerun.
Leaving the variable unset runs the existing native release checks.

To run the complete Linux check locally under Podman, including Chromium and native RDKit, use:

```bash
bash devel/run_linux_release_check.sh
```

The helper uses an immutable multi-architecture Rust 1.98.1 Debian 13 image, clones committed HEAD
and the local pinned Python Git commit into ignored output_tables/, copies the harvested corpus,
installs Chromium, builds the Debian RDKit shim, and runs the same full local gate. It also installs
the Python reference's comparison dependencies, including current lxml and Playwright; Debian's
lxml 5.3 cannot import the pinned reference's annotations. These are development dependencies in
the disposable container, not Rust application requirements. Each run records the installed Python
package versions and both Chromium versions in `prerequisites.txt`, so comparison-environment
changes can be diagnosed while continuing to use current dependencies. Browser and
validation commands run as a non-root user with Chromium's sandbox enabled. It uses the Podman
host's architecture: arm64 on an Apple Silicon VM and amd64 on an amd64 Linux host. Run on an
amd64 Linux host for amd64 browser validation; Chromium could not start under this Mac's amd64
emulator. A failed emulator startup is not a release pass.

The release helper prints a notes-drafting prompt when no notes file is supplied. Supply
`--notes-file <path>` to preview a release with prepared notes, then use `--write` to build source
archives from committed HEAD. It prints tagging and publication commands for manual execution.
It does not run the Rust validation checks or publish a release itself.

## Known gaps

- Complete release validation from the intended committed source before preparing a release with
  `devel/make_release.py`. Chromium-based working-tree builds pass on macOS arm64 and Linux arm64;
  Linux x86_64 release binaries cross-compile successfully. Linux arm64 also passes three-format
  rendering and package checks for both reported failures and the script/redirect fixtures.
- Native Linux amd64 Chromium runtime validation remains open. Earlier amd64 rendering receipts
  used the retired custom renderer and do not cover Chromium.
- The Chromium gallery renders all 290 tables, and all 721 refreshed corpus ZIPs pass integrity
  and public-content/grading comparison. Full Chrome for Testing 153.0.8010.12 can delay the first
  screenshot by about ten seconds even in headless mode. Automatic discovery now prefers the
  dedicated headless shell, which avoided that delay in focused checks. The earlier full-browser
  comparison is superseded for automatic shell selection by
  [WEBSITE_EXPORT_BENCHMARK.md](WEBSITE_EXPORT_BENCHMARK.md). The historical full-browser
  CLI batches were slower than the retained Python baseline. See the historical
  [benchmark report](active_plans/reports/html_to_image_native_benchmark.md).
