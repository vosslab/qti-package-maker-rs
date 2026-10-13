# qpm-molecule native shim

`qpm-molecule` renders a static `CanvasSource` through an optional native RDKit shim. Cargo never
builds or links RDKit: a table-only converter build therefore has no RDKit compiler, header, or
shared-library requirement.

Release engineering builds the versioned ABI v1 shim separately:

```bash
bash crates/qpm-molecule/native/build_shim.sh
```

The script discovers the standard Homebrew SDK locations. Pass an explicit SDK location only when
the installation is nonstandard:

```bash
bash crates/qpm-molecule/native/build_shim.sh --rdkit-prefix /opt/homebrew
```

It writes the shim to `crates/qpm-molecule/dist` by default, where native tests and local QPM
commands discover it. Release packages place the same library beside the executable. A conversion
using another packaged location passes `--rdkit-shim /absolute/path/to/libqpm_rdkit_shim.dylib`.

For release engineering on Linux x86_64, Debian 13 provides headers and link libraries as
`librdkit-dev` (candidate `202503.1-4`). Its multiarch library path must be named explicitly:

```bash
sudo apt-get install librdkit-dev libcairo2-dev g++
bash crates/qpm-molecule/native/build_shim.sh \
  --rdkit-prefix /usr \
  --rdkit-library-dir /usr/lib/x86_64-linux-gnu \
  --cairo-prefix /usr
```

Application runtime installs the shipped target shim and `librdkit1t64`; it needs no headers,
C++ compiler, Cargo, or Python:

```bash
sudo apt-get install librdkit1t64
bbq-converter --html-to-image --rdkit-shim /absolute/path/to/libqpm_rdkit_shim.so ...
```

`--rdkit-library-dir` defaults to the SDK's `lib` directory and selects Debian's normal x86_64
multiarch directory when the prefix is `/usr`. The macOS arm64 spike passed with Homebrew RDKit 2026.03.6. Linux x86_64 remains
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
`qpm_rdkit_shim_abi_version()` result to equal ABI v1; a mismatch returns a typed error.
