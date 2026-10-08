# Supported formats

The Rust CLI writes eleven format engines from a validated BBQ item bank. It reads BBQ text for its
conversion entry point; four engines additionally expose readers for library callers. The current
authoritative inventory is [ENGINES.md](ENGINES.md).

| Format | Registry name | Default artifact | Direction |
| --- | --- | --- | --- |
| Blackboard Questions text | `bbq_text_upload` | `bbq-<name>.txt` | read/write |
| Canvas QTI 1.2 | `canvas_qti_v1_2` | `qti12-<name>.zip` | write |
| Blackboard QTI 2.1 | `blackboard_qti_v2_1` | `qti21-<name>.zip` | write |
| Blackboard Original export | `blackboard_export_zip` | `bez-<name>.zip` | read/write |
| HTML self-test | `html_selftest` | `selftest-<name>.html` | write |
| Human-readable HTML | `human_readable` | `human-<name>.html` | write |
| Moodle Aiken | `moodle_aiken` | `aiken-<name>.txt` | write |
| Exam YAML | `exam_yaml` | `exam-<name>.yaml` | write |
| OKLA Christian BQGen | `okla_chrst_bqgen` | `okla-<name>.txt` | read/write |
| text2qti source | `text2qti` | `text2qti-<name>.txt` | read/write |
| PLE Native JSON | `ple_native_json` | `ple-<name>/` | write |

`<name>` is the core of an input named `bbq-<name>-questions.txt`. An explicit `--output` is valid
only with one selected writer. `--all` selects all eleven writers; unsupported item kinds can
leave an individual writer with no rendered output.

## Packaging formats

Canvas QTI 1.2, Blackboard QTI 2.1, and Blackboard Original export create ZIP packages with their
format-specific XML and packaged local media. External image URLs remain authored URLs and produce
warnings; a data URI is rejected for package-file transport. The independent `qti-integrity` crate
checks a completed ZIP or extracted package without depending on writer implementation.

Canvas and Blackboard Original do not represent `ORDER`. Canvas creates no artifact for an
`ORDER`-only bank; Blackboard Original creates its format-valid empty package and returns a
structured diagnostic. Blackboard QTI 2.1 represents all seven item kinds.

`--html-to-image` is relevant to these three ZIP writers and PLE Native JSON. The CLI renders
selected table and canvas fragments once into a derived in-memory bank before writing eligible
formats. See
[HTML_TO_IMAGE.md](HTML_TO_IMAGE.md) for its supported subset and runtime requirements.

## PLE Native JSON

PLE Native JSON writes `item_NNNNN.json` documents and their associated files under `media/` in
one directory. It represents all seven item kinds and preserves each question's answer meaning.
The library export returns JSON and associated-file bytes per question as a converter handoff for
PLE to decode. QPM does not import the handoff into PLE, assign PLE asset IDs, or control student
presentation; server import, asset binding, and rendering remain PLE-owned follow-up work with no
operational connection from QPM. The CLI directory also includes a hidden ownership manifest; see
[USAGE.md](USAGE.md).

## Text and document formats

BBQ transports all seven item kinds. text2qti transports `MC`, `MA`, `NUM`, and `FIB`; its writer
copies referenced local media beside its text output. BQGen transports `MC`, `MA`, `MATCH`, and
`FIB`. Moodle Aiken transports only `MC`. These text formats do not package image files; their
reference or placeholder behavior is defined in [MEDIA.md](MEDIA.md).

Exam YAML and human-readable HTML render all seven kinds for print or review workflows. Exam YAML
intentionally projects student-facing material and omits answer keys, scores, numeric tolerances,
and ordering semantics. The HTML self-test picks one supported item and embeds its controls and
local media in a self-contained page.

## Inspection and validation

Use these commands to inspect a compiled binary and a package:

```bash
cargo run -p qti-cli --bin qti-package-maker -- engines
cargo run -p qti-cli --bin qti-package-maker -- item-types
cargo run -p qti-cli --bin qti-package-maker -- check path/to/package.zip
```

`check` reports `OK` only when no integrity violations are found. It reports diagnostics for ZIP
and XML safety, manifests, QTI answer binding, Blackboard side resources, and referenced media.
