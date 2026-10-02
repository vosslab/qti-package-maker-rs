# Rust-port roadmap

This is a reader-oriented map of the completed implementation. The plan and its acceptance
criteria are archived in
[archive/rust_port_plan.md](archive/rust_port_plan.md). The progress ledger
is [refactor_progress.md](../refactor_progress.md). This document does not certify milestones.

## Product path

| Area | Delivered direction | Evidence boundary |
| --- | --- | --- |
| Core model | validated seven-kind items, CRC identity, ordered banks, media ownership | focused core tests and cross-language CRC evidence |
| Integrity | independent archive and QTI/Blackboard package checker | pinned-oracle corpus comparison |
| Engines and CLI | ten writers, four readers, static registry, process adapters | format contracts, round trips, and CLI tests |
| Image rendering | Chromium tables and runtime-loaded RDKit canvas adapter | source-faithful gallery, package checks, and browser boundary checks |
| Conversion | select, render, cache, rewrite, and fan out once per run | structural parity and measured benchmark |
| Delivery | Cargo metadata, release scripts, platform builds | local checks and manual source-release preparation |

## Milestone groups

M1 through M14 establish the Rust workspace, validated domain model, integrity checker, engine
registry, format writers/readers, CLI, and differential-parity harness. M15 harvests real tables
and canvases and measures the Python baseline. The 2026-10-01 amendment replaces the M16 custom
raster stack with Chromium after two real rendering failures. M17 renders authored tables.
M18 renders molecular canvases through optional RDKit. M19
converts selected HTML to images once before package-writer fan-out and measures the result.

The plan keeps package integrity ahead of a conflicting observed Python behavior, and it keeps
source item identity separate from writer presentation. See [PARITY.md](PARITY.md),
[MEDIA.md](MEDIA.md), and [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) for those durable contracts.

## Validation and delivery

The progress tracker separates earlier custom-renderer receipts from current Chromium evidence.
Chromium fixes the reported colors, circles, and gel shadows; its gallery renders all 290 tables.
Current macOS and Linux arm64 builds and focused package checks pass. The refreshed full corpus
has 721 integrity-clean packages with zero public-content/grading differences. The benchmark
records a first-screenshot delay in Chromium; it does not claim a speedup. Linux amd64 binaries
build, while native amd64
Chromium runtime validation remains unverified because the Mac emulator cannot launch that browser.

Manual source releases require the intended changes in committed HEAD. Committing and publishing
remain human delivery steps; they are separate from implementation verification. Temporary
implementation checks retain recoverable source and result receipts without adding permanent tests.
See [INSTALL.md](INSTALL.md) for the manual release procedure and
[PARITY.md](PARITY.md) for the raw HTML comparison's documented limitations.

## How to contribute

Use [ENGINE_AUTHORING.md](ENGINE_AUTHORING.md) for a new format, [QUESTION_TYPES.md](QUESTION_TYPES.md)
for the validated data model, and [TESTS_README.md](../tests/TESTS_README.md) for test retention.
Update the changelog for every edit and record settled cross-cutting decisions in
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md).
