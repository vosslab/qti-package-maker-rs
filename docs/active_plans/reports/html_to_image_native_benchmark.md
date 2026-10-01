# Native HTML-to-image benchmark

This receipt invokes the real release `bbq-converter` binary on each manifest input. It does not model or project Rust performance.

- Binary: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/native_table_bench/run-44801/bbq-converter`
- SHA-256: `4d7e94885c1b9646db8e43eec4b67994bf69783e76692e6eb4f7387c705df19c`
- Corpus: `/Users/vosslab/nsh/PROBLEMS/qti-package-maker-rs/output_tables/corpus`

| Run | Inputs | Wall seconds | Artifacts | Short outputs | Failures |
| --- | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 181 | 19.313 | 181 | 0 | 0 |
| `three_format` | 181 | 22.002 | 540 | 3 | 0 |

## Capability receipt

- `blackboard_export`: 606 PNG members in 117 packages. Inspection runs after the wall timer stops.
- `three_format`: 1822 PNG members in 352 packages. Inspection runs after the wall timer stops.

All 181 release CLI invocations completed successfully in both modes with the certified RDKit shim. Both modes request --html-to-image. The three-format run has three successful ORDER-only inputs with two artifacts instead of three because Canvas returns a no-output outcome for unsupported ORDER items; Blackboard export still emits an empty valid package and diagnostic. Short outputs are not errors.

## Stage attribution

The release-binary lane records only end-to-end wall time and artifacts. The separate library pass below exposes the approved conversion and raster counters without altering CLI behavior.

## Library conversion metrics

A separate library pass uses the same manifest inputs and one fresh run-scoped cache per input, matching the CLI conversion scope. `conversion_wall_seconds` is elapsed sequential wall time for that pass. Layout, paint, encode, materialization, and bookkeeping are cumulative work under Rayon and must not be summed or compared directly to wall time.

| Run | Requests | Attempts | Cache hit/wait/miss | Conversion wall | Layout work | Paint work | Encode work | Write work | Bookkeeping work |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `blackboard_export` | 651 | 391 | 183/77/391 | 49.662 | 6.821 | 19.722 | 22.533 | 0.077 | 11.547 |
| `three_format` | 651 | 391 | 194/66/391 | 48.494 | 4.265 | 19.552 | 22.524 | 0.084 | 11.654 |
