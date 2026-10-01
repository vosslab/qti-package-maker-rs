# Native HTML-to-image benchmark

This receipt invokes the real release `bbq-converter` binary on each manifest input. It does not model or project Rust performance.

- Binary: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/native_table_bench/run-80228/bbq-converter`
- SHA-256: `0d7094f78f1e1bb0f543a40db4c73eb4184e1dac905c3f8cfb9555b3fcb09d39`
- Corpus: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/corpus`

| Run | Inputs | Wall seconds | Artifacts | Short outputs | Failures |
| --- | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 181 | 18.124 | 181 | 0 | 0 |
| `three_format` | 181 | 19.202 | 540 | 3 | 0 |

## Capability receipt

- `blackboard_export`: 606 PNG members in 117 packages. Inspection runs after the wall timer stops.
- `three_format`: 1822 PNG members in 352 packages. Inspection runs after the wall timer stops.

Both modes request --html-to-image. The table above records actual invocation failures and output counts; detailed failures are retained in receipt.json. A short output can be expected when an input contains only an unsupported item type, such as ORDER for Canvas, but must be assessed alongside the recorded failures.

## Stage attribution

The release-binary lane records only end-to-end wall time and artifacts. The separate library pass below exposes the approved conversion and raster counters without altering CLI behavior.

## Library conversion metrics

A separate library pass uses the same manifest inputs and one fresh run-scoped cache per input, matching the CLI conversion scope. `conversion_wall_seconds` is elapsed sequential wall time for that pass. Layout, paint, encode, materialization, and bookkeeping are cumulative work under Rayon and must not be summed or compared directly to wall time.

| Run | Requests | Attempts | Cache hit/wait/miss | Conversion wall | Layout work | Paint work | Encode work | Write work | Bookkeeping work |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 651 | 391 | 196/64/391 | 44.520 | 6.003 | 17.747 | 20.879 | 0.073 | 10.364 |
| `three_format` | 651 | 391 | 202/58/391 | 43.007 | 3.758 | 17.689 | 20.859 | 0.076 | 10.320 |
