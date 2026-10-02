# Website export speed comparison

Measured 2026-10-01 on macOS arm64. Rust used 42% less elapsed time in this sample.

12 copied website question sets, 570 questions, three rounds, alternating implementation order.
Each export starts a fresh process. Inputs are unchanged and outputs use fresh directories.
Only Blackboard enables HTML-to-image, matching the website. Rust uses an optimized release build.
Python uses its current local source checkout. OS caches are warm; application caches are process-local.

This measures the website subprocess export stage, not a complete build_site.py run.
Question generation, title generation, page writing, and the in-process human-readable API are excluded.
The balanced plain/table/canvas sample is diagnostic, not weighted to the full website workload.
Both use Chrome 153 headless shell. Rust discovers it automatically without an executable override.

## Results

Times are medians of the three per-round totals for matched successful exports.

| Workload | Rust seconds | Python seconds | Rust / Python |
| --- | ---: | ---: | ---: |
| All matched exports | 18.38 | 31.64 | 0.58 |
| HTML-to-image exports | 15.12 | 23.27 | 0.65 |
| Exports without HTML-to-image | 3.26 | 8.47 | 0.39 |
| plain | 1.90 | 5.12 | 0.37 |
| table | 12.25 | 18.02 | 0.68 |
| canvas | 4.24 | 8.51 | 0.50 |
| blackboard_export_zip | 15.76 | 26.48 | 0.60 |
| canvas_qti_v1_2 | 1.31 | 2.53 | 0.52 |
| html_selftest | 1.31 | 2.64 | 0.50 |

A ratio above 1 means Rust took longer.

## Validation and exclusions

204 successful timed exports across both implementations; 6 paired attempts excluded.
ZIP CRCs and XML parsing passed for produced ZIPs. Accepted pairs have matching question and PNG counts.
This is a completeness check, not full grading or visual equivalence validation.

The ordering set `bbq-order_glycolysis_molecules-ORD-4_choices-questions.txt` was excluded
from Blackboard and Canvas totals in every round. Neither writer exported its ordering questions:
Blackboard produced empty pools, while Canvas produced no usable package. Python additionally
rendered images for the empty Blackboard pool. These attempts do not represent equivalent useful
work and are excluded from both implementations' totals. Its self-test export remains included.

## Run provenance

The benchmark follows the subprocess export paths in the website's
`bioproblems_site/topic_page.py`. It copies existing question sets and requests Blackboard,
Canvas QTI 1.2, and HTML self-test outputs, with HTML-to-image enabled only for Blackboard inputs
containing tables or canvases. Each format is a separate process, matching that caller.

Python: `3.12.14 (main, Aug 12 2026, 13:57:54) [Clang 21.0.0 (clang-2100.1.1.101)]`. Browser: `Google Chrome for Testing 153.0.8010.12`.

Source commits (Rust included uncommitted headless-discovery changes):

- `qti-package-maker-rs`: `68ac795f8773afa552ea1ec55a899ccbaa595456`.
- `biology-problems-website`: `613ad3e9bca1a8620270a3551243c441f8683d91`.
- `qti-package-maker`: `f7a06b55e293490acdb129b74598ea7a00186940`.

Measured Rust executable SHA-256:

`1fcba0600e03dc50525b43660fe8ab7180692de7e289d8be2f8043496a214605`

Per-round totals for matched successful exports:

| Implementation | Round 1 seconds | Round 2 seconds | Round 3 seconds |
| --- | ---: | ---: | ---: |
| rust | 18.92 | 18.38 | 18.19 |
| python | 32.82 | 31.64 | 31.63 |

After measurement, an equivalent Clippy-requested sorting-expression cleanup changed the binary
hash. The rebuilt binary passed a separate pathway rendering smoke check with 50 PNGs and clean
ZIP integrity; the three-round timings above belong to the measured binary.

## Local evidence

Raw evidence is ignored local output, not included in a fresh Git checkout:

- `output_website_speed/automatic_headless/metadata.json`: inputs, hashes, source commits,
  browser version, and measured binary hash.
- `output_website_speed/automatic_headless/results.jsonl`: every command, duration, exit code,
  output size, question count, and PNG count.
- `output_website_speed/automatic_headless/round_0/`, `round_1/`, and `round_2/`: logs and outputs.
- `output_website_speed/automatic_headless/final_build_smoke.json`: final-build smoke receipt.
- `tests/_temp/website_speed/automatic_headless.py`: one-time benchmark runner.

This report is the durable measurement summary. The temporary runner is not a permanent test or
performance gate. Results describe this host and sample, not all platforms or complete website
build times. Package checks establish completeness, not visual or grading equivalence.
