# Cohesive source splits

## Purpose and scope

Split seven source files that triggered the repository's near-limit advisory into focused modules
by responsibility. Preserve behavior and meaningful comments; the line advisory is a signal to
organize the code, not a reason to compress or remove useful explanation. This refactor is separate
from the completed Rust-port and PLE Native JSON plans.

## Work ledger

| Source area | Owner | Status | Handoff and evidence |
| --- | --- | --- | --- |
| `crates/qti-core/src/bank.rs` | `split_bank` | Reviewed | Media handling, bank merging, and tests moved to focused private modules. All 31 public declarations and original documentation comments are preserved. `cargo test -p qti-core` (67 tests), focused Clippy, and formatting passed; fresh spec and quality reviews passed. |
| `crates/qti-engines/src/blackboard_export_zip/write.rs` | `split_bb_export` | Reviewed | ZIP assembly, item/grading XML, and HTML normalization now have focused modules; `write.rs` keeps the entry point. Comments and tests remain with owning code. Fresh spec and quality reviews passed; final workspace tests passed. |
| `crates/qti-engines/src/blackboard_qti_v2_1/mod.rs` | `split_bb_qti` | Reviewed | `mod.rs` is a facade for writer, item XML, fragment/XML escaping helpers, and tests. The XML helper import cycle was corrected with helpers kept in `fragment.rs`; eight focused tests passed. Fresh spec and quality reviews passed. |
| `crates/qti-engines/src/text2qti/mod.rs` | `split_text2qti` | Reviewed | `mod.rs` retains `NAME` and reader/writer constructors; reader, parser, writer, media, and tests are separate. Original comments are preserved. Fresh spec and quality reviews passed; final workspace tests passed. |
| `crates/qti-integrity/src/checker.rs` | `split_integrity` | Reviewed | `checker.rs` is reduced to a 69-line dispatcher; XML, grading, media, Blackboard, identifiers, and manifest checks are separate. All 29 original function bodies are preserved; 26 tests passed. Fresh spec and quality reviews passed. |
| `xtask/src/parity.rs` | `split_parity_rust` | Reviewed | Parity orchestration, input selection, process execution, and receipt handling are separated. `cargo check -p xtask` and 15 focused tests passed; fresh spec and quality reviews passed. |
| `xtask/support/parity_oracle.py` | `split_parity_python` | Reviewed | Writer, QTI 1.2, and projection behavior are separate support modules; all 31 original function bodies are unchanged. Self-test, help, and 198 Python hygiene tests passed; fresh spec and quality reviews passed. |

The owners work in disjoint source areas. Integration records the resulting module map and checks
cross-file references after all handoffs arrive.

## Questions and decisions

| Question | Status | Resolution or next action |
| --- | --- | --- |
| What cohesive module boundaries best fit each source file? | Closed | Fresh specification and quality reviews accepted all seven splits and the Blackboard QTI helper correction. |
| Are comments and observable behavior preserved through each split? | Closed | Fresh reviews verified original APIs, comments, function bodies, and test counts against the source at `HEAD`; no behavior differences were found. |
| What final checks establish the split is complete? | Closed | Final Rust tests (282 passed, 5 intentionally ignored), strict Clippy, formatting, and 37 source hashes pass. Parity passes on 59 banks with zero divergences against pinned Python `55e5f368777f7809fe2e91b5d070caf6df0cb581`; final Python suite passes 1,576 tests with no failures or advisories. The source/architecture integration review passed conditionally on the final Python result; that condition was satisfied by the final 1,576-test run. |

## Review and correction log

| Review area | Result | Evidence and next action |
| --- | --- | --- |
| Fresh specification review | Pass | All seven splits, the Blackboard QTI escaping-helper correction, and the concurrent PLE cleanup passed. |
| Fresh quality review | Pass | All seven splits and the PLE cleanup passed. The Blackboard QTI correction leaves fragment escaping in one owner and a one-way dependency from writer/item rendering. |
| Combined verification | Pass | Final Rust tests (282 passed, 5 intentionally ignored), strict Clippy, formatting, 37 source hashes, 59-bank parity with zero divergences, and Python (1,576 passed, no failures or advisories) all pass. Documentation checks pass (71 Markdown links and ASCII checks). The source/architecture integration review passed conditionally on the final Python result; that condition was satisfied by the final 1,576-test run. Evidence receipt: `tests/_temp/cohesive_splits_gate_receipt.md` (SHA-256 `fde3d622ab229ddfa73abe2acfbe2e7b877169f1b4f2bccc99c1103b2640112b`). |

## Completion gates

- All seven owners hand off their module map, preserved responsibilities/comments, and focused
  verification evidence.
- Fresh specification, quality, and integration reviews report results; any findings are resolved
  and re-reviewed by the affected owner.
- Architecture and file-structure references describe the resulting modules accurately.
- The manager records final check results here. Do not mark this ledger complete before those
  results are provided.
