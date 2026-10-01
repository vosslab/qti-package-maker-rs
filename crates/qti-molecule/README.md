# qti-molecule native shim

`qti-molecule` renders a static `CanvasSource` through an optional native RDKit shim. Cargo never
builds or links RDKit: a table-only converter build therefore has no RDKit compiler, header, or
shared-library requirement.

Release engineering builds the versioned ABI v1 shim separately:

```bash
RDKIT_PREFIX="$(brew --prefix rdkit)" \
BOOST_PREFIX="$(brew --prefix boost)" \
CAIRO_PREFIX="$(brew --prefix cairo)" \
bash crates/qti-molecule/native/build_shim.sh
```

Install the resulting `libqti_rdkit_shim.dylib` beside the application native assets, then provide
its absolute path at runtime:

```bash
export QTI_RDKIT_SHIM=/absolute/path/to/libqti_rdkit_shim.dylib
```

For release engineering on Linux x86_64, Debian 13 provides headers and link libraries as
`librdkit-dev` (candidate `202503.1-4`). Its multiarch library path must be named explicitly:

```bash
sudo apt-get install librdkit-dev libcairo2-dev g++
RDKIT_PREFIX=/usr \
RDKIT_LIBRARY_DIR=/usr/lib/x86_64-linux-gnu \
CAIRO_PREFIX=/usr \
bash crates/qti-molecule/native/build_shim.sh
```

Application runtime installs the shipped target shim and `librdkit1t64`; it needs no headers,
C++ compiler, Cargo, or Python:

```bash
sudo apt-get install librdkit1t64
export QTI_RDKIT_SHIM=/absolute/path/to/libqti_rdkit_shim.so
```

`RDKIT_LIBRARY_DIR` defaults to `$RDKIT_PREFIX/lib`; set it for distributions that use a multiarch
library directory. The macOS arm64 spike passed with Homebrew RDKit 2026.03.6. Linux x86_64 remains
a release gate until this exact `.so` build passes the 58-canvas corpus under Rust 1.98.1.

### Linux x86_64 release-gate evidence (2026-09-30)

The rootless Podman machine is Linux arm64. A native arm64 Rust 1.98.1 toolchain, Debian's
`g++-x86-64-linux-gnu`, and an extracted Debian amd64 RDKit/Cairo sysroot built an x86_64 shim and
x86_64 Rust test executables. `file` identified every artifact as ELF x86-64. A separate Debian 13
`--arch amd64` runtime container reported `x86_64`, installed `librdkit1t64 202503.1-4`, and passed
three optional-shim tests covering successful drawing and all four native errors, plus the public
58-canvas corpus test (2.72 seconds). This proves the
Debian amd64 ABI and runtime under Podman's x86 emulation; it is not a claim of native x86 hardware.

An all-emulated Rust compiler was also attempted first and `rustc --version` exited 139. The split
build avoids that host-emulation failure while retaining actual x86_64 execution of the shim and
tests.

The ABI accepts only caller-owned bounded strings, fixed-width values, and caller-owned arrays. It
returns allocated PNG/error buffers only with paired free functions. Rust retains the dynamic
library while invoking its copied function pointers, copies PNG bytes before release, and maps
status codes to `MoleculeError` variants. Before loading drawing symbols, Rust requires the shim's
`qti_rdkit_shim_abi_version()` result to equal ABI v1; a mismatch returns a typed error.
