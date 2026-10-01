# Usage

The native CLI provides `bbq-converter` for BBQ question banks and `qti-package-maker` for
inspection. Build both binaries as described in [INSTALL.md](INSTALL.md).

## Inspect capabilities

```bash
./target/release/qti-package-maker engines
./target/release/qti-package-maker item-types
./target/release/qti-package-maker check finished-package.zip
```

`engines` lists each registered format and its read/write capability. `item-types` prints the seven
validated item kinds. `check` accepts a ZIP or extracted package tree, prints `OK` when it finds no
violations, and exits nonzero when any error-severity violation is present.

## Convert one BBQ bank

Input filenames must follow `bbq-<content-name>-questions.txt`, which supplies the document title
and default output name. For example:

```bash
./target/release/bbq-converter \
  --input bbq-genetics-questions.txt \
  --human
```

This selects the human-readable writer and creates `human-genetics.html` in the working directory.
Writer warnings, such as media-policy warnings, print on standard error with the engine and item
identity. Use `--quiet` to suppress progress messages; the final completion receipt still prints.

## Select formats

Choose a writer with a shortcut, an exact `--format` name, or a unique registry prefix. `--all`
selects every registered writer. An explicit `--output` works only when one writer is selected.

| Writer | Shortcut | Default output for `bbq-genetics-questions.txt` |
| --- | --- | --- |
| Canvas QTI 1.2 | `--qti12` | `qti12-genetics.zip` |
| Blackboard QTI 2.1 | `--qti21` | `qti21-genetics.zip` |
| Human-readable HTML | `--human` | `human-genetics.html` |
| Blackboard Questions text | `--bbq` | `bbq-genetics.txt` |
| HTML self-test | `--selftest` | `selftest-genetics.html` |
| Moodle Aiken | `--aiken` | `aiken-genetics.txt` |
| Blackboard Original export | `--bbexport` | `bez-genetics.zip` |
| Exam YAML | `--format exam_yaml` | `exam-genetics.yaml` |
| OKLA Christian BQGen | `--format okla_chrst_bqgen` | `okla-genetics.txt` |
| text2qti | `--format text2qti` | `text2qti-genetics.txt` |

The native CLI records the content name as document metadata and supplies the local date to writers.
The writer registry, not this table, is the authority for available formats. Use `engines` to
inspect the compiled binary.

## Convert tables and canvases

`--html-to-image` replaces supported tables and static RDKit canvases with native PNGs for the three
ZIP package writers. It may be combined with `--qti12`, `--qti21`, or `--bbexport`:

```bash
./target/release/bbq-converter \
  --input bbq-genetics-questions.txt \
  --qti21 \
  --html-to-image
```

If the selected input contains an RDKit canvas, set `QTI_RDKIT_SHIM` before running the command.
The optional runtime dependency and its error behavior are documented in
[INSTALL.md](INSTALL.md). The table renderer's visual acceptance and end-to-end speed comparison
remain open gates.

## Development tools

`cargo xtask --help` lists development corpus, integrity, parity, and benchmark tools. Some of
those commands require the pinned Python oracle through `source_me.sh`; the native production CLI
does not. See [PARITY.md](PARITY.md) for the cross-language evidence boundary.
