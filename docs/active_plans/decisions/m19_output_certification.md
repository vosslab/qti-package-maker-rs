# M19 immutable-output integrity certification

## Decision

The fresh `run-44801` supersedes the historical package evidence below. Its 721 ZIPs pass
the rebuilt release checker with zero errors and zero warnings; all output hashes and the
checker hash remain unchanged. The receipt is
`output_tables/native_table_bench/run-44801-integrity.json`. The captured benchmark binary
has SHA-256 `4d7e94885c1b9646db8e43eec4b67994bf69783e76692e6eb4f7387c705df19c`;
the checker has SHA-256 `0c271eeeb2cade5772273e1cae22c744763d11adb9708a43dd42c1e9d52e15e0`.

The current benchmark matches the pinned Python baseline's manifest and exact ordered
181 input hashes. Native Blackboard export takes 19.313 seconds versus Python's 197.234;
three-format output takes 22.002 seconds versus 445.820. Both native modes have zero
failures. These measurements prove the matched-input performance and local integrity
requirements; full writer parity, final release validation, and user visual signoff remain open.

## Historical certification

The immutable `run-80812` ZIP outputs pass the native `qti-integrity` checker with zero errors
and zero warnings. This closes the M19 package-integrity exit. The later M19 audit accepts the
corrected metric and one-conversion process receipts; visual approval, the all-corpus structural
receipt, and matched-input Python timing remain open.

## Evidence

On 2026-09-30, the release native inspection adapter ran
`qti-package-maker check <zip>` against every ZIP under
`output_tables/native_table_bench/run-80812/`:

| Output lane | ZIPs | Errors | Warnings |
| --- | ---: | ---: | ---: |
| `blackboard_export` | 181 | 0 | 0 |
| `three_format` | 540 | 0 | 0 |
| Total | 721 | 0 | 0 |

The command calls `qti_integrity::check_package` through the native CLI. It found no package or
referenced-media-trace violations. The checker reads completed ZIPs and does not regenerate
conversion output.

The ignored machine-readable receipt is
`output_tables/m19_integrity_certification_run80812.json`. It records every ZIP path and SHA-256,
the certifying binary SHA-256 before and after inspection, findings, process exits, and a
before/after ZIP-hash check. The immutable benchmark executable has SHA-256
`082195a85185c3f2adaf53e354151f2f3e3b4ce5be4bf9f1ce784a7966e6d9fd`; the release checker has
SHA-256 `3bc0f2c6fe38b051057a0f889586542ce360dd3548ffb70aad46743f925fd030` both before and after
the certification.

The earlier `run-4704` result is superseded. It found eight ORDER Blackboard-export errors before
the corrected CRC conversion and ORDER-media writer release build, so it is not release evidence.
