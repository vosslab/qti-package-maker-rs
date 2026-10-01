# qti-package-maker-rs

An in-progress Rust workspace for producing assessment packages from BBQ question banks, built to
reach feature parity with the established Python converter.

## Current status

The native converter and inspection binaries are implemented. This is not a release certification:
format parity, native-table visual acceptance, and the required speed comparison remain open gates.

The workspace currently provides these foundations:

- Ten registered writers and four readers, with native CLI reporting for writer media warnings.
- A validated core library for the seven Python assessment-item types, item banks, media, ZIPs, and
  package integrity checks.
- A pinned CRC corpus check that agrees for 1,108 Python items from 191 files and seven synthetic
  shape cases.

The workspace uses the Rust 2024 edition and is compiled with Rust 1.98.1 or later.

## Quick start

Install Rust 1.98.1 or later, then build the native binaries and inspect registered engines:

```bash
cargo build --release -p qti-cli --bins
./target/release/qti-package-maker engines
```

The first command creates `bbq-converter` and `qti-package-maker`. The second lists the current
read/write and media-policy capabilities. See [docs/INSTALL.md](docs/INSTALL.md) for source builds
and the optional native RDKit canvas shim.

## Native commands

```text
$ ./target/release/bbq-converter --help
Convert BBQ questions to assessment formats

Usage: bbq-converter [OPTIONS] --input <INPUT>
```

`bbq-converter` converts a BBQ input to selected formats. `qti-package-maker` lists engines and
item kinds or checks a finished package. [docs/USAGE.md](docs/USAGE.md) documents their options,
output names, and warning behavior.

## Development progress

The implementation target is feature parity with the maintained Python converter. Current package
writers and CLI behavior have evidence, while full cross-format parity, native raster visual
acceptance, speed comparison, and release gates remain in progress.

- [docs/active_plans/active/rust_port_plan.md](docs/active_plans/active/rust_port_plan.md) - active
  Rust-port plan and acceptance criteria.
- [refactor_progress.md](refactor_progress.md) - current milestone states and remaining gates.
- [docs/INSTALL.md](docs/INSTALL.md) - source build and optional native dependencies.
- [docs/USAGE.md](docs/USAGE.md) - native CLI commands and output contracts.
- [docs/PARITY.md](docs/PARITY.md) - parity authority and currently completed evidence.
- [docs/RDKIT_DEPENDENCY_DECISION.md](docs/RDKIT_DEPENDENCY_DECISION.md) - optional canvas-renderer
  dependency and recorded platform evidence.
- [docs/RUST_STYLE.md](docs/RUST_STYLE.md) - Rust and Cargo conventions for contributors.
- [docs/REPO_STYLE.md](docs/REPO_STYLE.md) - repository-wide development conventions.

## License

This project is licensed under the GNU Lesser General Public License v3.0 or later. See
[LICENSE.LGPL-3.0](LICENSE.LGPL-3.0).
