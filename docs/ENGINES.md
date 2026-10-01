# Engine inventory

`qti-engines` is the format boundary for this workspace. Its compile-time `ENGINES` slice is the
sole inventory of supported engines. Each entry has a stable name, media policy, and optional
factory for a dynamic `Writer` and/or `Reader` trait object. Inspect the compiled inventory with:

```bash
cargo run -p qti-cli --bin qti-package-maker -- engines
```

The registry contains ten writers and four readers. Writers report their completed artifact and
ordered recoverable media warnings through `WriteOutcome`; a missing path means the selected bank
contained no item that the writer emitted. Readers return valid records and recoverable warnings in
source order through `ReadOutcome`. Fatal input, validation, archive, XML, media, and I/O failures
return `EngineError`.

## Writer inventory

This table describes the current registry, including each writer's declared media policy and item
kinds. `MC`, `MA`, `MATCH`, `NUM`, `FIB`, `MULTI_FIB`, and `ORDER` are defined in
[QUESTION_TYPES.md](QUESTION_TYPES.md).

| Engine name | Artifact | Kinds | Media policy |
| --- | --- | --- | --- |
| `bbq_text_upload` | Blackboard Questions text | all seven | `ReferenceWarn` |
| `blackboard_export_zip` | Blackboard Original pool ZIP | all except `ORDER` | `Package` |
| `blackboard_qti_v2_1` | Blackboard QTI 2.1 ZIP | all seven | `Package` |
| `canvas_qti_v1_2` | Canvas QTI 1.2 ZIP | all except `ORDER` | `Package` |
| `exam_yaml` | print-oriented exam YAML | all seven | `ReferenceWarn` |
| `html_selftest` | standalone HTML self-test | all seven | `Package` |
| `human_readable` | human-readable HTML | all seven | `ReferenceWarn` |
| `moodle_aiken` | Moodle Aiken text | `MC` | `PlaceholderWarn` |
| `okla_chrst_bqgen` | OKLA Christian BQGen text | `MC`, `MA`, `MATCH`, `FIB` | `PlaceholderWarn` |
| `text2qti` | text2qti source text | `MC`, `MA`, `NUM`, `FIB` | `ReferenceWarn` |

The `canvas_qti_v1_2` writer returns no path for an `ORDER`-only bank. The
`blackboard_export_zip` writer instead creates its valid empty package and returns an ordered
diagnostic. Do not infer a successful artifact from a selected engine alone; inspect `WriteOutcome`.

## Reader inventory

| Engine name | Input | Recoverable location |
| --- | --- | --- |
| `bbq_text_upload` | Blackboard Questions text | one-based line |
| `text2qti` | text2qti source | one-based line |
| `okla_chrst_bqgen` | BQGen text | one-based block |
| `blackboard_export_zip` | Blackboard Original pool ZIP | archive entry, pool item, or media token |

Readers create a validated `ItemBank`. File-based text readers keep their input directory as the
bank's external media base. The Blackboard ZIP reader restores referenced media beneath a
bank-owned temporary base that stays available while a derived bank retains it.

## Engine selection

`bbq-converter` accepts `--format NAME` and unique registry prefixes, plus shortcuts for seven
commonly used writers. `--all` selects every registered writer. The CLI resolves a content name
from `bbq-<name>-questions.txt` and uses it for default output names. See [USAGE.md](USAGE.md) for
commands and [FORMATS.md](FORMATS.md) for the output contracts.

`EngineOptions` carries owned document title/date metadata. The CLI resolves them once per run and
clones them into factories. The three ZIP package writers participate in the shared
`--html-to-image` conversion path; conversion happens once before multi-format fan-out.

## Extension boundary

The engine registry is deliberately compiled into `qti-engines`; it does not discover plugins or
load format code at runtime. Trait objects let the static registry store heterogeneous engines,
while keeping supported formats, media policies, and factories auditable in one source file. See
[ENGINE_AUTHORING.md](ENGINE_AUTHORING.md) before adding an engine.

