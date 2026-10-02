# Native HTML-to-image benchmark

This receipt invokes the real release `bbq-converter` binary on each manifest input. It does not model or project Rust performance.

- Binary: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/native_table_bench/run-23373/bbq-converter`
- SHA-256: `841a6612568bb28dcd049308cd55d737ec66550d9b2add454308890dbdb63a0f`
- Corpus: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/corpus`

| Run | Inputs | Wall seconds | Artifacts | Short outputs | Failures |
| --- | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 181 | 915.990 | 181 | 0 | 0 |
| `three_format` | 181 | 909.205 | 540 | 3 | 0 |

## Capability receipt

- `blackboard_export`: 606 PNG members in 117 packages. Inspection runs after the wall timer stops.
- `three_format`: 1822 PNG members in 352 packages. Inspection runs after the wall timer stops.

Both modes request --html-to-image. The table above records actual invocation failures and output counts; detailed failures are retained in receipt.json. A short output can be expected when an input contains only an unsupported item type, such as ORDER for Canvas, but must be assessed alongside the recorded failures.

## Stage attribution

The release-binary lane records only end-to-end wall time and artifacts. The separate library pass below exposes the approved conversion and raster counters without altering CLI behavior.

## Library conversion metrics

A separate library pass uses the same manifest inputs and one fresh run-scoped cache per input, matching the CLI conversion scope. `conversion_wall_seconds` is elapsed sequential wall time for that pass. Chromium does not expose separate layout, paint, or PNG encoding timings here; those fields are null. Materialization and bookkeeping are cumulative work under Rayon, not elapsed wall time.

| Run | Requests | Attempts | Cache hit/wait/miss | Conversion wall | Write work | Bookkeeping work |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 651 | 391 | 178/82/391 | 26.771 | 0.075 | 0.990 |
| `three_format` | 651 | 391 | 184/76/391 | 27.148 | 0.076 | 1.066 |

## Python comparison and validation

The retained Python baseline and this run use identical paths and SHA-256 hashes for all
181 inputs. The comparison receipt is `output_tables/native_table_bench/run-23373/comparison_receipt.json`.

| Mode | Python seconds | Rust Chromium seconds |
| --- | ---: | ---: |
| Blackboard | 197.234 | 915.990 |
| Three formats | 445.820 | 909.205 |

These measurements establish a substantial CLI cost; they do not establish a speedup.
The separate library conversion passes take 26.771 and 27.148 seconds, reusing a renderer
across inputs. A sampled corpus CLI case spends about 10.5 seconds waiting inside table rendering,
while two canvas-only cases finish in about 0.1 seconds. Temporary instrumentation isolates
the wait to screenshot capture: DOM/fonts/geometry finish in 22 milliseconds and activation
adds about 1 millisecond; capture returns at about 10 seconds. The browser is Chrome for
Testing 153.0.8010.12. This is consistent with the reported
[first-capture stall](https://github.com/vercel-labs/agent-browser/issues/1859), though that
report does not establish an identical underlying cause. No speculative browser flags or
fixed sleeps were added. There is no arbitrary speed threshold.

All 721 ZIPs pass package integrity with zero errors or warnings. Their public-content and
grading projections agree with verified run 80228, with identical input provenance and output
paths. Image pixels are excluded from this comparison; gallery and focused rendering checks
cover image correctness. Detailed evidence is in `integrity_receipt.json` and
`semantic_receipt.json` beside the benchmark receipt.
