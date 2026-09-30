# qti-package-maker-rs

An in-progress Rust workspace for producing assessment-package formats from BBQ question banks,
built to reach feature parity with the established Python converter.

## Port status

This repository is at the workspace-foundation stage. The Rust port has no shipped converter yet,
and it does not currently create QTI, Blackboard, or other assessment packages. The active plan
defines the parity work, including native table and canvas rasterization, corpus checks, and
cross-language verification before a converter is presented as ready.

The workspace currently provides these foundations:

- Separate crates for core data and helpers, package integrity, output engines, and the future CLI.
- An `xtask` development entry point for parity and corpus tooling as those checks are implemented.
- A Rust 2024 workspace with a declared minimum Rust version of 1.98.1.

## Quick start

Install Rust 1.98.1 or later, then build the workspace and inspect the available development entry
point:

```bash
cargo build
cargo xtask --help
```

`cargo build` compiles the current workspace. The second command prints the `xtask` usage message
and states that parity and corpus commands are still under implementation. It does not convert a
question bank or write a package.

## Current command

```text
$ cargo xtask --help
Usage: cargo xtask --help
Parity and corpus commands are under implementation.
```

The `xtask` command is a Cargo alias configured in
[.cargo/config.toml](.cargo/config.toml). It is the current executable front door while converter
commands are being built.

## Development roadmap

The implementation target is feature parity with the maintained Python converter, not a smaller
replacement. The current plan covers the workspace crates, question-bank behavior, package
writers, validation, CLI behavior, cross-language parity checks, and the native `--html-to-image`
path.

- [docs/active_plans/active/rust_port_plan.md](docs/active_plans/active/rust_port_plan.md) - active
  Rust-port plan and acceptance criteria.
- [docs/RUST_STYLE.md](docs/RUST_STYLE.md) - Rust and Cargo conventions for contributors.
- [docs/REPO_STYLE.md](docs/REPO_STYLE.md) - repository-wide development conventions.

## License

This project is licensed under the GNU Lesser General Public License v3.0 or later. See
[LICENSE.LGPL-3.0](LICENSE.LGPL-3.0).
