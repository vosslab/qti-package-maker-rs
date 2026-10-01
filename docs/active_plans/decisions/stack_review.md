# M16 raster and M18 RDKit stack review

## Scope and evidence

This independent review covers WP-T3 and WP-M1 only. It inspected the ignored spike sources,
their decision records, the six raster gallery PNGs, and the locally installed Homebrew RDKit.
The installed `rdkit` is 2026.03.6 under `/opt/homebrew/opt/rdkit`; its Cellar footprint is 166 MiB,
of which `lib/` is 70 MiB. The measured unstripped shim is 40 KiB. `otool -L` confirms the shim
loads Homebrew's `libRDKitMolDraw2D`, Depictor, SubstructMatch, SmilesParse, GraphMol,
RDGeometryLib, DataStructs, RDGeneral, and Cairo libraries through its recorded rpaths.

## M16 ruling: accept the stack, reject the claimed milestone exit

**Accepted stack decision:**

- `cosmic-text` for the bundled Atkinson variable-font shaping, wrapping, and glyph rasterization;
- `tiny-skia` for anti-aliased paths, rounded fills, dashed strokes, and PNG output; and
- `cssparser` solely for CSS tokenization beneath a bounded, typed property parser.

The evidence in `DEPENDENCY_DECISIONS.md:205-253` and
`tests/_temp/raster_spike/src/main.rs` establishes the library capabilities needed for later work:
both text candidates reached the measured line counts, cosmic-text painted 400/700 glyph runs,
tiny-skia drew the requested visual primitives, and cssparser handled the observed declaration
syntax. Cosmic-text is the smaller complete text-and-glyph stack. This is a provisional library
selection that unblocks the M17 implementation design.

**Rejected as an M16 exit claim:** WP-T3 requires the spike to render the same three hardest corpus
tables, including the monospace table with colspan, gel lane table, and `table_curve_lib` curve.
The spike says it draws feature probes (`raster_spike/src/main.rs:1-5`) and directly constructs
geometry. The gallery confirms the distinction. `curve_python.png` is a three-transition titration
curve with four labeled points and a 1526x1044 canvas; `curve_native.png` is a 620x270 single-hump
curve with different points and labels. The native sequence table has three rows rather than the
oracle's four and no demonstrated colspan; its gel image has one label row and a different lane
matrix. The documents honestly label those probes, but that honesty does not satisfy the plan's
actual-fragment comparison requirement.

**Required correction, without expanding to M17:** extend the ignored spike with three fixed input
fixtures copied from the named corpus fragments and a deliberately minimal fixture adapter that
reads their actual table cells, spans, colors, dimensions, and curve coordinates into the existing
painting calls. It may remain a spike-only adapter; it must not become `qti-raster` or a general
HTML/CSS implementation. Regenerate a six-image gallery pairing each native rendering with its
same-input Python oracle, and record the command and result. The correction proves that the selected
libraries can carry the exact hard inputs while leaving full parsing/layout to WP-R1 through WP-R5.

## M18 ruling: accept macOS binding selection; Linux runtime acceptance remains conditional

**Accepted:** candidate (b), a narrow C ABI shim over `MolDraw2DCairo`, dynamically loaded by the
Rust wrapper. The corpus evidence is sufficient on this macOS arm64 machine: the runner loads the
shim, renders all 58 `CanvasSource` records at declared dimensions, and exercises all four required
typed failures. The shim source explicitly implements dimensions, legends, `explicitMethyl`, atom
and bond highlights, optional RGB colors, peptide-bond SMARTS matching, and its no-match error
(`shim.cpp:30-106`). The decision therefore honors every current corpus option. Candidate (a) did
not exist in the installed Homebrew distribution; candidate (c) is properly excluded from production
because it would add Python.

The native-runtime boundary is sound in design: Rust owns the NUL-free input buffers through the
call, the library owner outlives all loaded symbols, successful foreign PNG bytes are copied before
the matching free function, and error buffers are freed on error (`rdkit_spike/src/main.rs:42-96`).
The main workspace retains no link-time RDKit dependency, so a table-only conversion can run without
RDKit, JavaScript, or Python. The documented 166 MiB/70 MiB runtime footprint is accurately measured
for this installation; 40 KiB is only the shim size, not the full dependency cost.

**Condition before M18 exit:** the Linux x86_64 statement is an install-source claim, not runtime
evidence. Homebrew and conda-forge are documented sources, but this spike compiles only an arm64
macOS dylib with Homebrew paths and rpaths. Before declaring M18 complete, build the versioned shim
against one documented Linux x86_64 RDKit package, run the same 58-record runner and four typed-error
cases there, and record the installed library/shim footprint and loader search path. The release
documentation must name the supported RDKit ABI range, each target's shipped shim, and the exact
native package command; users require no compiler or headers at runtime.

M18 is accepted as the selected architecture and may proceed to WP-M2. Its cross-platform release
gate stays open until that Linux evidence is recorded. No Python or JavaScript fallback is authorized.

## Follow-on verification

After the raster same-input correction, rerun the spike commands listed in
`DEPENDENCY_DECISIONS.md:196-203`, inspect the regenerated gallery, and then record M16 as complete.
For the RDKit release gate, run the equivalent `build_and_run.sh` workflow on Linux with the target
shim and preserve its 58-render/four-error output. These are evidence repairs with defined outcomes,
not new implementation scope.

## M16 re-review: accepted

The same-input correction satisfies the M16 exit criterion. The revised ignored adapter remains
fixture-bound, but its three native outputs now preserve the actual hard-source content and feature
arrangement required to select a stack:

- the tetrad table contains the source labels, three genotype rows, counts, total, four-column
  header span, and nested colored mono genotype cells;
- the gel table contains the seven original labels and band positions; and
- the 1526x1044 curve contains the three source transitions, source labels, dashed guides, radius
  dots, and the source-style corner-border topology rather than an invented hump.

The paired gallery is evidence that cosmic-text and tiny-skia can carry those inputs. It does not
claim a general parser, layout implementation, pixel match, or M17 visual sign-off. The spike is
still outside the workspace, and `cargo run`, `cargo fmt --check`, and `cargo clippy -- -D warnings`
all pass from `tests/_temp/raster_spike/`.

Therefore the M16 stack exit is accepted: use `cosmic-text`, `tiny-skia`, and `cssparser` under the
bounded parser/layout design already recorded above. WP-R1 through WP-R5 remain responsible for the
general implementation and later gallery review.

## M18 final Linux re-review: release gate remains open

The stated target is now correctly limited to Debian amd64 execution under Podman's x86 emulation
on an arm64 host. That is useful target-ABI evidence; it is not native-x86-hardware evidence and
must continue to be described that way.

The present checked proof is insufficient for the M18 exit criterion, so M18 remains **rejected as
complete**. `tests/_temp/linux_rdkit_proof/run_proof.sh` runs only two ignored integration tests:
the all-options success case and atom-index failure. It does not exercise the live shim's
unparseable-SMILES, bond-index, or peptide-no-match status paths. Unit tests of the Rust status
mapping do not prove those native status values against Debian RDKit.

The proof recipe also does not reproduce its documented split route. Its Containerfile installs
native `g++`; its script invokes `rustc` and `cargo` inside the runtime container, while neither
file records the documented arm64-Rust plus `g++-x86-64-linux-gnu` cross-build, extracted amd64
sysroot, copied target artifacts, or `file`/`readelf` receipts. The stale "Fedora proof image"
comment makes the mismatch explicit. The result may be valid evidence from a separate run, but it
is not yet a repeatable artifact-backed gate.

To accept M18, retain the split route and add: (1) a build receipt that names the native arm64
Rust compiler, cross compiler, sysroot packages, and `file`/`readelf` output for the shim and both
test executables; (2) a separate `--arch amd64` runtime recipe that runs only those target
artifacts, records `uname -m` and installed `librdkit1t64` version, and preserves the emulation
label; and (3) native-shim integration assertions for all four required errors: unparseable SMILES,
atom out of range, bond out of range, and peptide SMARTS no match. The existing RGB finite/range
unit test is accepted as the correct pre-FFI public-boundary proof.

The install documents must also distinguish release engineering (`librdkit-dev`, headers, and
compiler) from application runtime (`librdkit1t64` plus the shipped target shim). They currently
instruct a runtime user to install the development package, which conflicts with the optional
prebuilt-shim design.

## M18 final Linux re-review: accepted

The corrected, persisted replay closes the remaining M18 gate. The build receipt at
`/Users/vosslab/.cache/qti-linux-cross-replay-v2/build_receipt.txt` records native `aarch64` Rust
1.98.1, Debian `x86_64-linux-gnu-g++`, extracted amd64 RDKit/Cairo/Boost packages, three x86-64
ELF artifacts, the shim's required RDKit/Cairo shared libraries, and artifact hashes. The separate
runtime receipt records `x86_64`, `librdkit1t64 202503.1-4 amd64`, and the same three hashes.

The revised runtime container has no compiler, Cargo, headers, or Python. It receives the source
and cross-built artifacts read-only, writes its receipt to a distinct mounted directory, and runs
the all-options native case, all remaining live status-error cases (unparseable SMILES, atom,
bond, peptide no-match), and the 58-record public API corpus. The recorded replay exited zero in
2.43 seconds. This is an amd64-runtime result under Podman x86 emulation on an arm64 host; it is
not described as native-x86 hardware proof.

M18 is accepted. The release contract remains the documented split: release engineering installs
`librdkit-dev` to compile a target shim, while users install the target shared-library runtime
(`librdkit1t64` on Debian amd64) plus that shipped shim.
