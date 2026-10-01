# RDKit canvas binding spike

Date: 2026-09-30

## Corpus surveyed before the binding comparison

`output_tables/corpus/manifest.json` records 205 generators, 176 generated BBQ files, and
342 deduplicated fragments. Fifty-eight fragments are `CanvasSource` records. The canvas set uses
six size pairs: 27 at 480x320, 12 at 480x512, 10 at 320x240, four at 260x200, four at 512x256,
and one at 540x320. Every canvas requests `explicitMethyl`; twelve request an atom highlight,
the peptide-bond SMARTS, bright-green RGB `(0, 1, 0)`, and a nonempty legend. The remaining
46 have no highlights; 27 use an empty legend and 14 omit it.

The source generators are overwhelmingly under `PUBCHEM`; peptide sequence and charge generators
produce the twelve highlighted canvases. This is a real feature surface, so M18 requires native
RDKit rather than a placeholder image path.

## Candidate comparison

| Candidate | Evidence | Result | Install and runtime story |
| --- | --- | --- | --- |
| (a) MinimalLib C API through `librdkitcffi` | Homebrew RDKit 2026.03.6 has no `librdkitcffi`, `MinimalLib.h`, or CFFI draw entry point. Its `libRDKithc` exports only the historical C helper symbols. | Does not pass on the actual macOS arm64 installation. It cannot be selected without a separately packaged MinimalLib build. | A pinned custom RDKit build would add a second, unproven distribution channel. |
| (b) Small C ABI shim over `MolDraw2DCairo`, runtime-loaded by Rust | `crates/qti-molecule/native/build_shim.sh` compiles the ABI v1 shim against Homebrew RDKit; its Rust 2024 runner dynamically loads the shim, renders all 58 records, writes a valid 480x512 PNG, and reproduces all four error classes. Two production-shim runs produced SHA-256 `00fae78b868474b87abbba727b8a67319bfc54f26fc17a4de4e2ede56e5207e9` for the worked example. | Passes the required feature surface on macOS arm64. The shim has direct API support for declared width/height, legend, `explicitMethyl`, atom and bond index highlighting, RGB highlight maps, and peptide SMARTS matching. | Homebrew's `rdkit` bottle supports Apple Silicon and Linux x86_64; conda-forge's `librdkit` package also publishes `osx-arm64` and `linux-64`. The installed Homebrew tree is 166 MiB, with 70 MiB under `lib`; the shim itself is 40 KiB. The production binary loads the shim only when a canvas is converted. |
| (c) Python RDKit subprocess | The development oracle rendered all 58 corpus records with the reference Python implementation. | A useful parity floor only. It violates the native offline runtime requirement and is not a production fallback. | Requires a Python runtime plus RDKit. |

## Decision

Select candidate (b): ship a small, versioned C ABI shim over RDKit's stable C++ drawing APIs;
`qti-molecule` will load that shim with `libloading` at canvas-conversion time. The workspace must
compile and run table-only conversions with no RDKit installed. The runtime search and diagnostic
will name the required shim and documented package source.

The Rust-side API owns all UTF-8 and index buffers for the duration of the FFI call. It keeps the
`Library` owner alive for at least as long as every loaded symbol and output-buffer destructor, and
copies returned bytes before calling the shim's matching free function. These are the FFI boundary
invariants for ASVS 1.4.1 and 1.4.3. `CanvasSource` remains the validation boundary: dimensional
limits and finite RGB components in the inclusive `0..=1` range are verified by the public Rust
renderer before FFI, while the shim verifies atom and bond ranges against the parsed molecule and
reports a typed error.

The spike used C++23 because the installed RDKit 2026.03.6 headers declare constexpr virtual
destructors. M18 implementation must pin the supported RDKit API range and compile the distributable
shim for each release target; application users do not need C++ headers or a compiler at runtime.

## Reproduction

```bash
bash tests/_temp/rdkit_spike/build_and_run.sh
```

The spike is intentionally ignored under `tests/_temp/`; it has no production dependency edge.
The checked command reports `rendered 58 CanvasSource records and reproduced four typed errors`.
The separate Python oracle ran with `source source_me.sh && python3` and the reference
`render_canvas_png` function over the same 58 JSON records.

Sources: [Homebrew RDKit formula](https://formulae.brew.sh/formula/rdkit) documents bottles for
Apple Silicon and Linux x86_64. [conda-forge librdkit](https://anaconda.org/conda-forge/librdkit)
documents `osx-arm64` and `linux-64` native library packages.

## Linux x86_64 release-gate proof

A rootless Linux arm64 Podman machine used a native arm64 Rust 1.98.1 compiler with Debian's
`g++-x86-64-linux-gnu` and extracted amd64 RDKit/Cairo sysroot to cross-compile the shim and test
executables. All three artifacts were ELF x86-64. A separate Debian 13 `--arch amd64` runtime
container reported `x86_64`, installed `librdkit1t64 202503.1-4`, and passed both optional-shim
tests plus `native_renderer_renders_every_harvested_canvas` across all 58 records in 2.51 seconds.
This is an actual amd64 RDKit runtime test under Podman x86 emulation, with its host platform
explicitly recorded. The initial all-emulated Rust compiler did exit 139 at `rustc --version`; the
split build avoids that host-emulation defect.
