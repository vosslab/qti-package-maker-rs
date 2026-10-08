# QTI Package Maker for Rust

Turn one BBQ question bank into Canvas and Blackboard import packages, a self-grading practice
quiz, or a readable review copy. It helps instructors reuse assessments across learning systems
while keeping one editable source.

## Native status

The native converter builds from this source checkout and has completed cross-format and table-image
verification. No release installer is published yet. It preserves useful Python workflows while
allowing harmless presentation differences. Table rendering always runs headlessly and automatically
prefers Chromium headless shell; see [docs/INSTALL.md](docs/INSTALL.md) for setup and known limitations. Review an
imported package in its target LMS before relying on it for a course or exam.

## One bank, many uses

Start with one tab-delimited Blackboard Question Upload (BBQ) text file. Select the output that
serves the next teaching task.

| Teaching task | Native output | What it gives you |
| --- | --- | --- |
| Import into Canvas | Canvas QTI 1.2 ZIP | A package ready for Canvas import review |
| Import into Blackboard | QTI 2.1 or Original pool ZIP | A Blackboard package or pool export |
| Let students practice | Self-grading HTML | A standalone browser quiz with immediate feedback |
| Review before release | Human-readable HTML | A readable copy of the questions and answers |
| Reuse elsewhere | Aiken, YAML, text2qti, BBQ, or BQGen | Text formats for Moodle, exams, and other tools |
| Hand questions to PLE | PLE Native JSON directory | One JSON document per question with associated media |

The same bank can produce several of these outputs in one command, so edits stay in the source
questions instead of becoming separate copies in each learning system.

## Quick start

Install Rust 1.98.1 or later. From this repository, build the native commands:

```bash
cargo build --locked --release -p qti-cli --bins
```

Create `bbq-demo-questions.txt` with this tab-delimited row:

```text
MC	Which base pairs with A?	T	correct	C	incorrect
```

Create a Canvas package, a Blackboard package, an instructor review copy, and a student practice
quiz:

```bash
./target/release/bbq-converter \
  --input bbq-demo-questions.txt \
  --qti12 --qti21 --human --selftest
```

The command writes `qti12-demo.zip`, `qti21-demo.zip`, `human-demo.html`, and
`selftest-demo.html`. Open the self-test in a browser to try the learner-facing question, read the
human copy before publishing, then import the ZIP appropriate to the LMS.

<!-- screenshots:begin (managed by screenshot-docs) -->
<!-- screenshots:end -->

## Choose an output

Use the short flags for common destinations. `--all` selects all eleven registered writers; use
`--format <engine-name>` when the output does not have a shortcut.

```bash
# Canvas QTI 1.2
./target/release/bbq-converter --input bbq-genetics-questions.txt --qti12

# Blackboard QTI 2.1, with supported tables and static RDKit canvases rendered as PNGs
./target/release/bbq-converter --input bbq-genetics-questions.txt --qti21 --html-to-image

# Moodle Aiken text
./target/release/bbq-converter --input bbq-genetics-questions.txt --aiken

# PLE Native JSON and associated files
./target/release/bbq-converter --input bbq-genetics-questions.txt --format ple_native_json
```

`--html-to-image` is available for Canvas QTI 1.2, Blackboard QTI 2.1, Blackboard Original
pool exports, and PLE Native JSON. Table rendering requires Chromium or Chrome. It needs the
optional RDKit shim
only when the bank contains an RDKit canvas. See
[docs/HTML_TO_IMAGE.md](docs/HTML_TO_IMAGE.md) for the supported content and setup.

## Check before upload

Use the companion command to see the formats compiled into your binary or to inspect a finished
ZIP for package-integrity problems:

```bash
./target/release/qti-package-maker engines
./target/release/qti-package-maker check qti12-demo.zip
```

`check` prints `OK` when it finds no integrity violations. It checks package structure and answer
bindings; it does not replace review of the imported questions in the target LMS.

## Learn more

- [docs/INSTALL.md](docs/INSTALL.md) - source build, platform requirements, and the optional RDKit
  shim.
- [docs/USAGE.md](docs/USAGE.md) - CLI flags, output names, and practical conversion commands.
- [docs/FORMATS.md](docs/FORMATS.md) - all eleven outputs, media behavior, and format limits.
- [docs/QUESTION_TYPES.md](docs/QUESTION_TYPES.md) - fields for the seven supported assessment
  types.
- [docs/ENGINES.md](docs/ENGINES.md) - the current reader, writer, and media-capability inventory.

The maintained [Python QTI Package Maker](https://github.com/vosslab/qti-package-maker) remains the
reference implementation. The Rust implementation has completed its documented verification;
real LMS imports and release publication remain separate steps. Developers can read the
[completed port plan](docs/archive/rust_port_plan.md) and
[refactor_progress.md](refactor_progress.md).

## License

This project is licensed under the GNU Lesser General Public License v3.0 or later. See
[LICENSE.LGPL-3.0](LICENSE.LGPL-3.0).
