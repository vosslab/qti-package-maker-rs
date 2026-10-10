# Python self-test reference gallery

These are captured outputs from the current Python QPM checkout, requested on 2026-10-09.
They provide a source-bound reference for inspecting Python's self-test behavior and presentation.
The captures do not establish Rust parity or create a human approval gate.

## View the gallery

From the Rust repository root, serve the captures locally:

```bash
source source_me.sh && python3 -m http.server 8124 --bind 127.0.0.1 --directory tests/fixtures/python_selftest
```

Open [the local gallery](http://127.0.0.1:8124/) in a browser. [index.html](index.html) contains
ten responsive iframe previews with direct links to each original HTML fragment and input record.
Each iframe loads its capture unchanged and runs the capture's original scripts. Scroll inside a
preview for longer questions, or open its HTML link for a full browser view. Stop the server with
Ctrl-C when finished.

The two RDKit previews require network access to unpkg for RDKit JavaScript and its Wasm resource.
The other eight examples have no external script dependencies. The gallery does not replace,
vendor, or pin the original RDKit URLs.

## Captured cases

Each `.txt` file contains one unchanged record from the existing BPW question bank.
Each `.html` file is the unmodified output of Python's `html_selftest` engine for
that record. All seven supported question types are represented. The additional
MATCH table and RDKit cases cover authored content that simple text examples miss.

| Case | Purpose | Input | Python output |
| --- | --- | --- | --- |
| MC | Buffer pH, short choices, MathML | [mc.txt](mc.txt) | [mc.html](mc.html) |
| MA | Multiple selections, escaped Greek letter | [ma.txt](ma.txt) | [ma.html](ma.html) |
| MATCH | The reported dominance example, colored terms | [match.txt](match.txt) | [match.html](match.html) |
| NUM | Numeric answer and tolerance | [num.txt](num.txt) | [num.html](num.html) |
| FIB | RNA transcription, accepted text answers | [fib.txt](fib.txt) | [fib.html](fib.html) |
| MULTI_FIB | Genetic map, several blanks, authored table | [multi_fib.txt](multi_fib.txt) | [multi_fib.html](multi_fib.html) |
| ORDER | Ordering, move controls, reset | [order.txt](order.txt) | [order.html](order.html) |
| MATCH tables | Tables inside matching prompts | [match_tables.txt](match_tables.txt) | [match_tables.html](match_tables.html) |
| MC RDKit | Molecule drawing in the question | [mc_rdkit.txt](mc_rdkit.txt) | [mc_rdkit.html](mc_rdkit.html) |
| MATCH RDKit | Molecule drawings in matching prompts | [match_rdkit.txt](match_rdkit.txt) | [match_rdkit.html](match_rdkit.html) |

[manifest.json](manifest.json) records the original bank and line, Python commit,
working-tree status, self-test source hashes, fixed random seed, captured file
hashes, correct-answer definitions, and external script URLs. Hashes identify the
capture; they are not permanent byte-equality or version gates.

## Exact output boundaries

- These HTML files are fragments. Python adds no `html`, `head`, or `body` wrapper.
- Authored inline CSS means `style="..."` attributes. Those remain in the content.
- Python's self-test controls use **script-injected CSS**: the leading script
  creates the shared `style#qti-selftest-theme` element. This is not inline CSS.
- The emitted stylesheet uses transparent background fallback and host theme
  variables. The browser or review host supplies the surrounding page background.
- The two RDKit inputs already reference
  `https://unpkg.com/@rdkit/rdkit/dist/RDKit_minimal.js`. Python preserves that
  authored dependency. Those drawings require network access to RDKit and its
  Wasm resource; the capture does not vendor or replace them.
- These are browser self-test examples. Blackboard export is a separate engine;
  its generated question content does not acquire the self-test stylesheet.

Open the output links directly to inspect Python's fragment. The gallery supplies a light page
background around each isolated preview; its CSS does not enter the captured document. Any themed
review page or screenshot is supporting evidence with its own host settings, separate from these
reference files. Use the answer keys alongside the rendered questions when comparing behavior.
No screenshot or snapshot test is added. The manifest's capture-time status is provenance,
not a release or approval requirement.

## Reproduce the capture

From the Rust repository, use the existing sibling Python checkout:

```bash
source source_me.sh
cd ../qti-package-maker
source source_me.sh
python3 ../qti-package-maker-rs/devel/capture_python_selftest_fixtures.py
```

This explicit command reads the saved inputs, sets Python's random seed to zero
for each question, and calls `QTIPackageInterface.save_package("html_selftest", ...)`.
It refreshes the captured output and provenance using the current Python code.
Review the resulting diff; the command makes no changes to either renderer.
