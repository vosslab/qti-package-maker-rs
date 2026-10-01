# Native HTML-to-image benchmark

This receipt invokes the real release `bbq-converter` binary on each manifest input. It does not model or project Rust performance.

- Binary: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/target/release/bbq-converter`
- SHA-256: `9fa29cc2a3f77c69db37fa2ea32cbb2a1f8ee6ec7961d66a5ecb9e1db0ae47b0`
- Corpus: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/corpus`

| Run | Inputs | Wall seconds | Artifacts | Failures |
| --- | ---: | ---: | ---: | ---: |
| `blackboard_export` | 181 | 3.011 | 178 | 3 |
| `three_format` | 181 | 13.411 | 500 | 15 |

## Stage attribution

Unavailable in this receipt. The native conversion and raster APIs do not yet expose approved per-stage counters; null fields in the JSON make that absence explicit.
