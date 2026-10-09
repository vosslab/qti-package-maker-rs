# Corrected QPM self-test renderer: BPW handoff

The renderer is ready for BPW integration. Final review found no blocking correctness or
architecture issues. Repairs remain in the shared Rust HTML writer used by native and WASM.

## Completed repairs

- Restored choice colors, light/dark themes, compact short MC/MA choices, paragraph spacing,
  and existing button feedback.
- Fixed overlapping MATCH headers, answer-control sizing, and scientific table/canvas overflow.
  MATCH control styles no longer alter nested authored tables.
- Restored seeded MATCH/ORDER shuffling with the already-locked random library. Presentation
  indices change; CRC generation, question identity, source answer tokens, grading, and Reset
  behavior are preserved. No progress architecture or website lifecycle changes were introduced.

## Validation

- 333 Rust tests passed, four optional tests ignored; formatting and strict Clippy passed.
- 12 Node package/parity tests and TypeScript checks passed.
- 21 self-test checks passed across Chromium, Firefox, and WebKit, covering native/WASM output,
  all seven question kinds, grading hooks, Clear/Reset, themes, layout, and scientific content.
- 1,855 repository hygiene checks passed. Temporary eight-seed checks verified variation,
  stable CRCs, correct grading, reproducibility, and Reset.
- The packed JS/WASM files match the validated build. The extracted package imports successfully
  and renders all seven kinds. Standalone screenshots were reviewed.

## Integration artifact

Package: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_selftest_parity/vosslab-qti-wasm-0.1.0.tgz`

SHA-256: `78256bab6fd6ade9bb1c4eb8925299d4fc1fa003056fe54af6e9b70bce478d65`

Use this tarball through BPW's existing vendoring workflow. It contains the matched JS glue,
WASM binary, declarations, and license. BPW owns the subsequent integrated rendering,
regeneration, grading, and advancement check; QPM verification was standalone.

The native executable is `target/release/bbq-converter`. Examples, screenshots, and supporting
results are in `output_selftest_parity/`. Authored fixed colors remain unchanged, including any
source-level dark-background contrast limitations.

This local package was built from the current working checkout, including pre-existing portable
rendering changes. It is not a published release or committed revision. Source changes are ready
for human review/commit under repository policy; unrelated work and staging were preserved.
