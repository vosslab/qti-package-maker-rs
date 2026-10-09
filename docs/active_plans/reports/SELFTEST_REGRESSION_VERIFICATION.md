# Selftest regression verification

Date: 2026-10-09. Starting base: `3ad5a72c37be06b88112b8cbfb78be90fb453dd9`; verification HEAD:
`faf1161474e1ccd73f208e87a82e0d4f3d7b2d86` (dependency updates supplied during this work).
This report covers the selftest correction and current-Python migration tooling in the working
tree. The behavior contract and drift investigation are in
[SELFTEST_COMPATIBILITY.md](../../SELFTEST_COMPATIBILITY.md).

## Reproduced failure and correction

The retained pre-fix MC output has SHA-256
`c3efaa8759a8c8a8afd7bdad1f70c3a9a8a1f565477968bbbbf2295b4b51ed76`.
In Chromium 156.0.8078.4, clicking its correct answer produced `CORRECT`, disabled the check button,
and invoked a wrapper around `checkAnswer_<CRC>` zero times. The receipt is retained locally at
`tests/_temp/selftest_baseline/mc-hook-bypass-receipt.json`; the temporary probe was removed.

The corrected native and Wasm HTML calls the wrapped grading hook exactly once. The permanent
browser tests use authored answers rather than reading the renderer's answer keys. They exercise
all seven kinds, incorrect/partial/correct feedback, reset announcements, enabled check buttons,
NUM Enter, repeated MULTIFIB blanks, repeated initialization, and multiple mounted questions.
Full scores record completion; partial scores do not. Reset preserves host-owned completion.

The host-contract test completes A, replaces it with incomplete B, gives B an incorrect answer,
then returns to A and verifies its stored completion before grading again. Replayed scripts retain
the original wrapped function, and grading addresses the current DOM node. This establishes QPM's
side of the integration contract without requiring Python or a website-specific implementation.

## Independent checks

Commands were run from the repository root except npm commands, which ran in `packages/qti-wasm`.
Cargo used `CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo`; Python used `source source_me.sh`.

| Check | Result |
| --- | --- |
| `cargo test --workspace --all-targets --locked` | 325 passed; four existing optional tests ignored |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all --check` | Passed |
| `python3 -m pytest tests/ -q` | 1,810 passed |
| `npm run build` and `npm run typecheck` | Passed |
| `npm test` | Nine passed, including all readers/writers and native/Wasm identities |
| `npm run test:browser` | 30 passed across three browser engines |
| Installed local npm tarball, imported public exports, converted NUM to selftest | Passed |

Runtime versions: Node 26.11.0, Chromium 156.0.8078.4, Firefox 157.0, and Playwright WebKit 27.2.
These are the versions exercised in this correction; Node 24 and shipping Safari were not rerun.
Native Chromium screenshot and RDKit code was unchanged and its optional lane was not rerun.
Logs are retained in `tests/_temp/selftest_*.log` and
`packages/qti-wasm/_verification/selftest_*.log`. These local ignored artifacts are not part of a
fresh checkout; the tests and this summary are the durable record.

## Transitional current-Python checks

The selected current sibling checkout was at
`f7a06b55e293490acdb129b74598ea7a00186940`, with working modifications included. The captured
source-tree SHA-256 was
`77b7d3813f97d4ff2879f3e5423e016e44a5780d9c9f4b40dd285479d0115f8e`.
Migration receipts record the source path, revision, tree hash, working-status hash, and verified
import path. This identifies the comparison, not a version to keep pinned.

- `npm run test:python-parity`: three browser tests pass, each comparing all seven kinds' question
  CRCs and correct-grade feedback against authored answers.
- `cargo xtask oracle-crosscheck`: 23 package comparisons pass.
- `cargo xtask crc-corpus`: all 551 question identities agree across 197 files plus seven synthetic
  cases. The command still exits nonzero for seven normalized-field differences; the first is
  redundant rich-text MC display labels retained by Python and removed by Rust. It now checks every
  identity before reporting field differences.
- `cargo xtask parity --fixtures`: the 59-input migration run reports nine findings, with no
  selftest identity or grading differences. Findings remain visible rather than being repaired
  or suppressed inside the comparator. Receipt: `tests/_temp/parity_27514/divergences.json`.

The broader fixture findings include four Python Blackboard exports with missing display-feedback
targets, one Python QTI 2.1 MULTIFIB export without response identifiers, and one Python Canvas
MULTIFIB export with unresolved answer references. Each Rust counterpart passes the relevant
projection. The Canvas export also produces two Python integrity findings that are absent from
Rust. Two further findings record Python's failure to export a selftest with spaces in an image
path (Rust exports it) and Python text2qti merging four questions into one (Rust retains all four).
These findings are not reasons to copy malformed output or remove useful Rust behavior.
Source-side classification is retained at `tests/_temp/selftest_parity_classification.json`.

The retired historical repair modules are removed. A comparator exception now reports that neither
side was compared, rather than incorrectly labeling the exception a Rust failure. Independent Rust
tests remain the long-term correctness baseline; Python execution is a separate migration aid.

The newer locked ZIP 9 version is retained. Filename decoding call sites now handle its fallible
API; no older dependency was pinned to bypass adaptation. No Cargo dependency was added for this fix.

## Handoff artifact and remaining integration check

The rebuilt local package is
`packages/qti-wasm/_verification/vosslab-qti-wasm-0.1.0.tgz` (1,439,304 bytes), SHA-256
`0be5f69224e114268e7d3bdea40ad3efb5a958e702c8ca537f95b57737512988`.
Its Wasm module is 3,647,865 bytes, SHA-256
`d2cee028cb5ace2ba11fe81eff2f7acf49d48db1122cc79b45ee5fd2c3181adc`.
The archive was installed into a temporary consumer and exercised through its public ESM exports.
No package publication or website update is implied.

The website manager owns selecting a different eligible question and persisting progress by the
complete CRC. After adopting this package and correcting the website's pre-selection `limit: 1`,
verify the actual website journey: complete A, reroll to incomplete B, and return to completed A.
QPM's local A-B-A proof does not replace that final website integration check.
