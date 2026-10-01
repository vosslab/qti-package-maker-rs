# M19 structural corpus receipt

## Static canvas parser differential

`output_tables/m19_canvas_differential.json` is an ignored, reproducible receipt for the pinned
Python revision `55e5f368777f7809fe2e91b5d070caf6df0cb581`. It passes all 141 CanvasSource
occurrences reachable from the current 181-input generated corpus and 23 hostile cases. Each
accepted case compares every CanvasSource field; each hostile case compares acceptance or
rejection without executing JavaScript. Twenty-one cases must reject; the 4096-dimension boundary
and approved `attacker.draw_to_canvas(evil())` receiver spelling are named accepted controls. The
hostile matrix covers dimensions, get_mol and draw counts, SMILES limits, mdetails initialization
and keys, option literals, list bounds, and RGB validation.

Before parsing, the Rust runner checks the actual manifest bytes, ordered `bbq_files` paths, and
each of the 181 input SHA-256 values against the Python fixture. The receipt retains the verified
per-file hash map, input-fixture SHA-256, pinned snapshot path, and `selectors.py` SHA-256. Run it
with the oracle root exported before the bootstrap so the Python helper can prove its imported
selector is inside the certified snapshot:

```bash
export QTI_ORACLE_ROOT="$PWD/output_tables/oracle_snapshot/55e5f368777f7809fe2e91b5d070caf6df0cb581"
source source_me.sh
python3 xtask/support/m19_canvas_oracle.py output_tables/corpus/manifest.json > /private/tmp/m19_canvas_input.json
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo CARGO_TARGET_DIR=/private/tmp/qti-m19-target \
  cargo run --locked -p xtask --bin m19_canvas_differential -- \
  /private/tmp/m19_canvas_input.json output_tables/corpus/manifest.json \
  output_tables/m19_canvas_differential.json
```

The receipt follows the production transform exactly: `question_text`, then every string leaf of
each `get_tuple()` supporting field. Its 141 contexts include three intentionally repeated
CanvasSource strings: a correct MC answer is a separate supporting field reconstructed by the
transform, even when it has the same text as a choice. These occur for alanine (`0474_4195`),
alpha-amino-acid identification (`61a8_970d`), and histidine (`f43e_c549`). The receipt retains
all three field contexts rather than deduplicating them; it has 138 unique source records and one
canvas nested in an outer table. The 58 unique JSON records in `output_tables/corpus/canvases`
remain the deduplicated render corpus, not a source-occurrence count.

## Status

The captured native release packages from `output_tables/native_table_bench/run-4704` cannot serve
as the M19 structural-conversion receipt. They remain the accepted immutable speed evidence, but
their source-input hashes were not captured and their item identities do not match the current
frozen-source corpus.

For example, the current corpus manifest is SHA-256
`4446e61f2d7457c1c5a72e13d61bea97b26a3c2b717a157145b6348498abdec1` and input 0000 is
SHA-256 `38c8252b75fc0f88c80a41fcbb07340196fe670bd471351e5573eb37dfcb1930`. The pinned Python
reader reports its item identity as `0474_4195`. The `run-4704` native Blackboard package reports
`6f7c_12b5` in its QTI item title. The Blackboard writer uses `item.crc()` for that title, so it is
not a writer alias. The capture preserves no input copy or manifest hash, which prevents assigning
the discrepancy to corpus regeneration or to the then-current native reader/CRC behavior.

## Reproducible rejected probe

```bash
source source_me.sh && \
PYTHONPATH="$PWD/output_tables/oracle_snapshot/55e5f368777f7809fe2e91b5d070caf6df0cb581" \
python3 tests/_temp/m19_conversion_receipt/audit_native_packages.py \
  output_tables/corpus/manifest.json \
  output_tables/native_table_bench/run-4704/blackboard_export \
  tests/_temp/m19_conversion_receipt/native_receipt.json
```

The ignored JSON records every input and the failed identity/image checks. It reports 118 packages
with missing expected current-corpus images and 125 missing current source CRC titles. These are
provenance mismatches, not product failures, because the tested package set cannot be tied to the
current manifest.

## Remaining receipt

A fresh all-181 native run must record the manifest and every input hash before the structural
auditor can establish selected-element replacement at field positions, alternative text, surviving
markup, loader removal, ASCII serialization, and unchanged source CRCs. The frozen-Python package
comparison is recorded by the dedicated parity workstream and should be linked to that fresh run.

## Fresh hash-bound native run

The temporary runner then built the current release CLI and converted all 181 current manifest
inputs with `-B --html-to-image`. Every process exited zero, produced one ZIP, and retained the
input byte hash before and after conversion. Its detailed receipt is
`output_tables/m19_structural_native/run_receipt.json`.

This run found a real M19 blocker: the source item for input 0000 is `0474_4195`, while its
converted Blackboard package writes item title `6f7c_12b5`. The writer takes this title directly
from `item.crc()`, so conversion is rebuilding an item identity from rewritten HTML. This violates
the required original-CRC immutability. The fresh structural JSON is
`tests/_temp/m19_conversion_receipt/fresh_native_structural.json`.

The image-name comparison in that JSON is not an independent failure count. The converter reuses
identical field results through its per-item cache and the Blackboard writer rewrites owned media
to `__xid` names; a field-level converted-bank receipt must account for both before it can certify
positions and image references. The loader-removal, ASCII, and no-selected-markup checks are still
recorded as observations, not an M19 completion claim.

## Current direct-bank structural receipt

The core identity API subsequently retained the original item CRC through the HTML-derived display
view. The ignored `xtask/src/bin/m19_conversion_receipt.rs` receipt then read and converted every
current corpus input through the native library API with the RDKit shim. Its JSON is
`tests/_temp/m19_conversion_receipt/direct_bank_structural.json`.

```bash
QTI_RDKIT_SHIM=/Users/vosslab/.cache/qti-rdkit-shim/macos-arm64/libqti_rdkit_shim.dylib \
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo \
cargo run --locked -q -p xtask --bin m19_conversion_receipt -- \
  output_tables/corpus/manifest.json \
  tests/_temp/m19_conversion_receipt/direct_bank_structural.json
```

It passed all 181 inputs, 515 items, and 3,426 HTML fields. The native static plan selected 510
outer tables and 141 canvases; one canvas is nested in a selected table, so the final image count
is 650. All 650 original-field markers became generated images at the same tree positions. The
receipt recorded zero failures for CRC identity, field count, selected markup removal, generated
image alternative text, surrounding tag structure, surrounding text order, loader removal, ASCII
serialization, input byte hashes, and conversion errors.

The source-selection authority is the frozen Python snapshot, not native absence alone. This
command uses its selector and exact reader traversal over the same manifest:

```bash
source source_me.sh && \
PYTHONPATH="$PWD/output_tables/oracle_snapshot/55e5f368777f7809fe2e91b5d070caf6df0cb581:$PWD/tests/_temp/m19_conversion_receipt" \
python3 - <<'PY'
import audit_native_packages
import json
import pathlib
from qti_package_maker.package_interface import QTIPackageInterface
inputs = json.loads(pathlib.Path("output_tables/corpus/manifest.json").read_text())["bbq_files"]
selected = [audit_native_packages.source_expectations(pathlib.Path(path)) for path in inputs]
field_count = 0
for path in inputs:
\tpackage = QTIPackageInterface("receipt", allow_mixed=True)
\tpackage.read_package(path, "bbq_text_upload")
\tfield_count += sum(1 for item in package.item_bank for _field in audit_native_packages.fields(item))
print({
\t"inputs": len(inputs), "items": sum(len(records) for records in selected),
\t"html_fields": field_count,
\t"tables": sum(1 for records in selected for _crc, entries in records for _name, _alt, family in entries if family == "table"),
\t"standalone_canvases": sum(1 for records in selected for _crc, entries in records for _name, _alt, family in entries if family == "canvas"),
})
PY
```

It reports 181 inputs, 515 items, 3,426 fields, 510 tables, and 140 standalone canvases: the
same final 650 outputs as the native receipt. The 141st native canvas is the one nested in an
outer table and is intentionally absorbed by that table's final image. `source_expectations` was
corrected to retain one expected list per item; its earlier shared-list bug multiplied each bank's
selection total and was an audit error, not a product discrepancy.

The current hash-bound package benchmark is separately captured at
`output_tables/native_table_bench/run-80812/receipt.json` with the manifest and every input SHA-256.
The dedicated frozen-Python output parity workstream remains the authority for byte/package-level
comparison; this receipt establishes the required native field-level structural facts.

## Frozen positional audit: currently rejected

The preceding direct-bank receipt predates the required frozen per-position fixture and is not M19
completion evidence. The regenerated fixture records the pinned Python manifest SHA-256, all 181
input SHA-256 values, and every `(item CRC, item occurrence, field index, selected ordinal)` with
tag-plus-sibling-index paths, fragment family, and normalized alternative text. It uses the
production traversal: `question_text`, followed by every recursive string leaf of `get_tuple()`.

The following command reached all 181 inputs on the current manifest. It is intentionally
nonzero while reporting the detailed JSON because the frozen authority still disagrees with the
native reader/parser:

```bash
source source_me.sh
PYTHONPATH="$PWD/output_tables/oracle_snapshot/55e5f368777f7809fe2e91b5d070caf6df0cb581" \
  python3 tests/_temp/m19_conversion_receipt/frozen_position_fixture.py \
  output_tables/corpus/manifest.json tests/_temp/m19_conversion_receipt/frozen_positions.json
QTI_RDKIT_SHIM=/Users/vosslab/.cache/qti-rdkit-shim/macos-arm64/libqti_rdkit_shim.dylib \
  CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo CARGO_INCREMENTAL=0 \
  cargo run --locked -q -p xtask --bin m19_conversion_receipt -- \
  output_tables/corpus/manifest.json \
  tests/_temp/m19_conversion_receipt/frozen_positions.json \
  tests/_temp/m19_conversion_receipt/direct_bank_structural_v4.json
```

The fixture has 181 inputs, 515 items, 3,426 fields, 510 tables, 141 canvas occurrences, and 650
final replacement images. The terminal v4 receipt preserves all aggregate conversion facts: zero
failures for converted-bank identity and field count, selected markup removal, paired alternative
text, projected surrounding DOM tree and text, loader removal, ASCII serialization, source input
hashes, and conversion errors. It remains rejected with 93 frozen-position failures.

Fourteen failures are reader CRC disagreements for the same input/item occurrence; for example,
input 29 is `9b10_8251` in the pinned Python reader and `d6cb_8251` in the native reader. The
other 79 failures are 65 fields where the selected element and alternative text match but
tag-plus-sibling paths differ after malformed-markup tree repair. For example, the frozen Python
path is `table[5]` while scraper/html5ever reports `table[6]` for the same tetrad table in input
104, CRC `c7e0_10fb`. Generated images match the native projected marker path in every selected
field, so this is not a replacement-position failure. An architect ruling is required to resolve
the cross-parser source-path authority and reader CRC contract; no M19 structural completion is
claimed.

## Frozen positional audit: v7 resolution pending review

The v4 rejection above was retained as the failure record. The NUM reader CRC repair and the
architect-approved parser-neutral source ordinal replaced its invalid cross-parser sibling path
comparison. The test-only scanner adds a deterministic `data-qti-source-ordinal` only to literal
`table` and `canvas` start tags. It skips comments, quoted attributes, and script, style, textarea,
and title raw-text bodies; it accepts a raw-text close only at a tag-name boundary and rejects an
authored ordinal attribute. The native auditor proves that annotation preserves the unannotated
Rust selection family/order/alternative-text sequence, compares the annotated Rust sequence with
the frozen Python sequence, and compares generated-image locations with projected marker
locations in the unannotated DOM.

The v7 receipt at `tests/_temp/m19_conversion_receipt/direct_bank_structural_v7.json` passed all
181 inputs with zero failures. It records manifest SHA-256
`4446e61f2d7457c1c5a72e13d61bea97b26a3c2b717a157145b6348498abdec1`, fixture SHA-256
`0bd5fd878907523c9e30dc724d55e2884288a82d775a4df4ed2bcdb875a5af53`, fixture-generator
SHA-256 `710a44bfd1d257893faf1f92df48234a6f81e9b5d0f7df67cb2159ef397e68dd`, pinned revision
`55e5f368777f7809fe2e91b5d070caf6df0cb581`, and frozen `selectors.py` SHA-256
`5f905658b5b12d0a95203207cc993e9f47927022b52cd754bf0a19549e21b2b9`. Its 181 individual input
hashes are checked before conversion and retained in the receipt. The Rust scanner tests cover
non-ASCII text, comments, quoted attributes, script/style/textarea/title fake markup, a
`</scriptx>` non-closer, marker collision rejection, and actual table/canvas starts.

A copy of the fixture with a mutated manifest hash was rejected before any conversion with
`frozen fixture manifest hash differs`. Independent reviewer approval remains required before
calling this M19 structural exit complete.
