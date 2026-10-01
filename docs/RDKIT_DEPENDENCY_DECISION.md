# RDKit native canvas dependency

The Rust converter will use an optional, runtime-loaded RDKit drawing shim for Canvas-source
molecule images. The main workspace has no link-time RDKit dependency, so normal package and
table-only conversion builds remain available without RDKit.

## Release build and runtime sources

Release engineering builds the target shim with RDKit headers and a compiler:

```bash
brew install rdkit
# or, in a dedicated conda environment:
conda install conda-forge::librdkit
# Debian 13 x86_64 release build:
sudo apt-get install librdkit-dev
```

Runtime installs the release-provided shim and only its matching shared library:

```bash
# Debian 13 x86_64 runtime:
sudo apt-get install librdkit1t64
export QTI_RDKIT_SHIM=/absolute/path/to/libqti_rdkit_shim.so
```

The release-specific shim is compiled against the selected RDKit ABI and installed beside the
application's optional native assets. At first canvas conversion, `qti-molecule` loads the shim;
when unavailable it returns `MoleculeError::RdkitUnavailable` with this install guidance. A bank
with no molecule canvas therefore remains convertible without the optional dependency.

Debian 13's official amd64 package set supplies `librdkit-dev 202503.1-4` and
`librdkit1t64 202503.1-4`. Its libraries use `/usr/lib/x86_64-linux-gnu`, so release builds pass
that directory as `RDKIT_LIBRARY_DIR`; because Cairo's header is in
`/usr/include/cairo`, Debian builds also pass `CAIRO_PREFIX=/usr`. The Linux ABI proof cross-compiled the x86_64 shim and Rust
tests with a native arm64 Rust 1.98.1 compiler and Debian amd64 sysroot, then executed those ELF
x86-64 artifacts in a Debian `--arch amd64` runtime container. With `librdkit1t64 202503.1-4`, its
three option/error tests and all 58 public-API corpus renders passed. The runtime was Podman x86
emulation on an arm64 host, so this records an amd64 runtime proof rather than native-hardware CI.

The Homebrew 2026.03.6 installation measured during the WP-M1 spike occupied 166 MiB, including
70 MiB of shared libraries. This is an optional local install footprint, not a Rust binary
dependency. The dedicated shim was 40 KiB before stripping.

The binding returns PNG bytes directly from `MolDraw2DCairo`. It supports dimensions, legends,
`explicitMethyl`, atom and bond highlights, RGB highlight colors, and peptide-bond SMARTS. Its
small C ABI is owned by `qti-molecule`: Rust retains the loaded library while using any symbol,
copies a returned byte buffer, then invokes the shim's matching free function. The loader requires
the ABI v1 version symbol before resolving drawing functions and returns `ShimAbiMismatch` for an
incompatible installed shim.

See [the detailed spike record](active_plans/decisions/rdkit_canvas_options.md) for corpus evidence,
the rejected MinimalLib path, and reproducible commands. Platform availability comes from the
[Homebrew RDKit formula](https://formulae.brew.sh/formula/rdkit) and
[conda-forge librdkit](https://anaconda.org/conda-forge/librdkit).
