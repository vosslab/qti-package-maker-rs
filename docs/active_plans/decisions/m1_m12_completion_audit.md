# M1 and M12 current-state completion audit

Audit date: 2026-09-30. This audit applies the carried-forward `rust_port_plan_v1.md`
requirements and the active plan amendments, using the explicit registry rather than inferred
module discovery.

## M1

The workspace structure is present: `qti-core`, `qti-integrity`, `qti-engines`, `qti-cli`, the
amendment-required `qti-raster` and `qti-molecule`, and `xtask` are declared in the root workspace.
All packages inherit edition 2024, Rust 1.98.1, LGPL-3.0-or-later, and the repository URL.
`REPO_TYPE` is `rust`. `docs/DEPENDENCY_DECISIONS.md` records the four WP-F3 decisions, commands,
and observed outcomes. `docs/RUST_STYLE.md` section 16 is the canonical dependency policy: direct
requirements use the manager-selected wildcard or explicit-floor form, while the tracked
`Cargo.lock` is the exact tested resolution. The active plan now states that same rule where its
WP-T3 follow-on previously used ambiguous wording. A fresh `cargo check --workspace --locked` and
`cargo build --workspace --locked` completed successfully in this audit.

M1's "chosen crate versions are pinned" criterion is therefore satisfied by the tracked lockfile,
not by exact `Cargo.toml` requirements. The workspace's wildcard requirements, including
`qti-molecule`'s `libloading = "*"`, conform to the repository-wide policy and must remain so
unless a documented temporary upstream constraint requires an exact version.

## M12

The compile-time registry satisfies its structural contract. `ENGINES` lists exactly ten writers
once, with `EngineEntry` factories and media policies, and exactly four readers:
`bbq_text_upload`, `text2qti`, `okla_chrst_bqgen`, and `blackboard_export_zip`. It uses owned,
cloneable `EngineOptions` and `DocumentMetadata`; factories remain object-safe. Exact and unique
prefix lookup returns typed unknown/ambiguous errors with candidates.

`bbq-converter` implements the carried-forward input/output/limit aliases, quiet and verbose
modes, mixed-bank flag, repeatable format selection, all-format selection, the seven frozen
shortcuts and their `--<engine_name>` aliases, output-singleton rejection, source-derived
`content_name`, all ten default output names, local-date document metadata, startup version check,
writer warnings on stderr, and the amended once-per-run HTML-to-image dispatch. `qti-package-maker`
implements `engines`, `item-types`, and `check`; error-severity integrity findings produce process
failure. The focused CLI library suite passed eight tests during this audit.

The earlier process-evidence gaps are now closed by the eight tests in
`crates/qti-cli/tests/cli_contract.rs`, rerun successfully in the locked all-target
workspace suite after the Canvas and reader-default repairs:

1. `all_formats_write_each_named_artifact_and_report_ten_attempts` verifies the
   ten requested writers and each expected output.
2. Missing input, unknown engine, and a writer I/O failure have explicit process
   status and diagnostic assertions. The approved unsupported-kind replacement
   is documented below; unsupported-kind refusal is not invented.
3. `ambiguous_engine_prefix_lists_candidates_before_opening_input` verifies the
   candidate list and usage status before input access.

The same process suite proves the observable once-per-run conversion receipt
for three packaging formats. These checks close M12's CLI process boundary;
the full engine and media parity gate remains M13 and is still incomplete.

### WP-F2 unsupported-kind failure replacement

The original WP-F2 process-test wording requires an unsupported-item-kind writer failure. The
approved engine contract supersedes that failure at the CLI boundary: `render_bank` skips kinds a
writer does not support before writer hooks, matching Python's base engine loop. In particular, an
ORDER-only Canvas request completes without an artifact, and an ORDER-only Blackboard export
completes with an empty ZIP plus an observable ORDER diagnostic. A CLI refusal would contradict
those pinned outcomes.

The replacement process gates therefore prove both successful skip behavior (the Canvas ORDER-only
no-artifact contract and the Blackboard ORDER receipt in the parity harness) and a real fatal
writer boundary: an output path that is an existing directory exits 1 and names the selected
`bbq_text_upload` engine and inaccessible path. This preserves actionable failure coverage without
inventing an unsupported-kind refusal that the approved writer contract prohibits.

The active-plan HTML-to-image amendment is correctly reflected: with an explicit output it rejects
a non-packaging writer, while a multi-format run converts once for the three packaging writers and
leaves other writers on the original bank. No additional restriction is required.
