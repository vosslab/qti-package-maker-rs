# Rust-port roadmap

This is a reader-oriented map of the active implementation plan. The authoritative work list,
acceptance criteria, and open gates are in
[active_plans/active/rust_port_plan.md](active_plans/active/rust_port_plan.md). The progress ledger
is [refactor_progress.md](../refactor_progress.md). This document does not certify milestones.

## Product path

| Area | Delivered direction | Evidence boundary |
| --- | --- | --- |
| Core model | validated seven-kind items, CRC identity, ordered banks, media ownership | focused core tests and cross-language CRC evidence |
| Integrity | independent archive and QTI/Blackboard package checker | pinned-oracle corpus comparison |
| Engines and CLI | ten writers, four readers, static registry, process adapters | format contracts, round trips, and CLI tests |
| Native rendering | table rasterizer and runtime-loaded RDKit canvas adapter | corpus render, visual gallery, and renderer tests |
| Conversion | select, render, cache, rewrite, and fan out once per run | structural parity and measured benchmark |
| Delivery | Cargo metadata, release scripts, platform builds | local checks and manual source-release preparation |

## Milestone groups

M1 through M14 establish the Rust workspace, validated domain model, integrity checker, engine
registry, format writers/readers, CLI, and differential-parity harness. M15 harvests real tables
and canvases and measures the Python baseline. M16 selects the native raster stack from that
corpus. M17 renders supported tables. M18 renders molecular canvases through optional RDKit. M19
converts selected HTML to images once before package-writer fan-out and measures the result.

The plan keeps package integrity ahead of a conflicting observed Python behavior, and it keeps
source item identity separate from writer presentation. See [PARITY.md](PARITY.md),
[MEDIA.md](MEDIA.md), and [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) for those durable contracts.

## Remaining acceptance work

The progress tracker records completed corpus comparisons, platform builds, and the renderer
benchmark. Remaining acceptance work is the user's table-gallery review and release verification
from the intended committed source, followed by archival of the accepted plan. Temporary
implementation checks are retired with recoverable source and result receipts retained.
See [INSTALL.md](INSTALL.md) for the manual release procedure and
[PARITY.md](PARITY.md) for the raw HTML comparison's documented limitations.

## How to contribute

Use [ENGINE_AUTHORING.md](ENGINE_AUTHORING.md) for a new format, [QUESTION_TYPES.md](QUESTION_TYPES.md)
for the validated data model, and [TESTS_README.md](../tests/TESTS_README.md) for test retention.
Update the changelog for every edit and record settled cross-cutting decisions in
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md).
