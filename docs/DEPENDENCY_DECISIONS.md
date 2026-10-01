# Dependency decisions

## WP-F3 dependency spikes (2026-09-30)

### QTI HTML entity decoding: `markup5ever`

The QTI 2.1 writer uses the existing `markup5ever` HTML named-entity table directly.
The corpus contains `&alpha;`; preserving it verbatim in XML makes the assessment item
unparseable because XML does not define that entity. The writer must decode HTML entities
and preserve XML escaping while keeping the authored item and its CRC unchanged.

The workspace declares `markup5ever = "*"`; `Cargo.lock` already pins version 0.39.0 through
the HTML parser. Making it a direct engine dependency reuses that version and avoids a
partial handwritten entity map. Offline engine dependency checking passes. Writer regression
and corpus verification are recorded separately from dependency resolution.

### Exact numeric JSON readback: `serde_json`

Enable `float_roundtrip` on the existing workspace `serde_json` dependency. Independent
Blackboard receipt replay showed the default parser changed the pinned JSON decimal
`0.009999999999999787` into an adjacent floating-point value. The feature preserves the
actual readback rather than rounding comparison values or weakening numeric equality.
This changes feature selection for the locked version, not the dependency version.

The ignored spike crate at `tests/_temp/dependency_spike/` is the reproducible
evidence for these selections. It uses its own `Cargo.lock`, generated with
Rust 1.98.1 and `CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo`; production
manifests retain wildcard requirements and their workspace lock pins the
versions used in a build.

From the repository root, recreate the ignored fixture inputs with:

```sh
mkdir -p tests/_temp/dependency_spike/fixtures
unzip -p ../qti-package-maker/examples/minimal_qti_2.1_sample.zip imsmanifest.xml > tests/_temp/dependency_spike/fixtures/imsmanifest.xml
unzip -p ../qti-package-maker/tests/fixtures/bb_export_slice.zip res00002.dat > tests/_temp/dependency_spike/fixtures/res00002.dat
```

### HTML source scanning and rewrite: `lol_html`

**Decision:** use `lol_html = "*"` for `scan_html_for_assets` and
`rewrite_html_srcs`. The spike resolved it to 3.0.1. Use `scraper = "*"` for
tree-based HTML-to-text/table extraction in `qti-core::strings`, where source
preservation is not a requirement.

**Command:**

```sh
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run --offline
```

Run from `tests/_temp/dependency_spike/`.

**Input:** five WP-B1 permanent cases (mixed quotes, uppercase `IMG`,
`data-src`, an image-shaped string in `script`, and an unclosed tag), plus
entities beside an image, a table cell, self-closing forms, query strings, and
multiple images.

**Observed result:** `lol_html` changed only the selected `src` values and
preserved non-image bytes for every case, including the malformed unclosed
fragment. It preserves ordinary input spelling while rewriting, but the
changed `src` attribute itself is serialized with double quotes; the media
contract only requires content outside the rewritten image source to remain
byte-identical. A `scraper`/html5ever fragment parse then serialization differed
from its input on the first mixed-quote fragment, so tree serialization fails
the source-preservation decision rule.

The official `lol_html` documentation identifies `rewrite_str` and element
handlers as its one-shot streaming rewrite API. Its current 3.0.1 release
requires Rust 1.85, below the workspace Rust 1.98.1 MSRV.

### XML parsing and indented writing: `quick-xml`

**Decision:** use `quick-xml = "*"`, resolved to 0.42.0. It is suitable for
manifest and Blackboard pool parsing plus explicitly constructed indented XML
output. `qti-integrity` also uses this crate for package validation.

**Command:**

```sh
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run --offline
```

**Input:** real `imsmanifest.xml` from `minimal_qti_2.1_sample.zip`, and the
real `res00002.dat` from `tests/fixtures/bb_export_slice.zip` in the Python
repository at source commit `55e5f368777f7809fe2e91b5d070caf6df0cb581`.

**Observed result:** each fixture was parsed into an ordered semantic event
signature, written back through `Writer::new_with_indent`, then parsed a second
time. The signatures matched for both files. A signature contains each start,
empty, and end tag, every attribute name and raw value in order, every
non-whitespace text node, CDATA, declaration, comment, processing instruction,
DOCTYPE, and general entity reference. The manifest retained `xmlns:imsmd` and
`xmlns:imsqti`; its namespace declarations are therefore checked in the real
round trip. The output from both inputs contains indentation line breaks.

The preserved minimal round-trip recipe is:

```rust
loop {
    let event = reader.read_event()?;
    if matches!(event, Event::Eof) {
        break;
    }
    if let Some(signature) = event_signature(&event) {
        before.push(signature);
    }
    writer.write_event(event.into_owned())?;
}
assert_eq!(before, xml_semantics(&String::from_utf8(writer.into_inner())?));
```

Whitespace-only text nodes are deliberately excluded from the semantic
signature because indentation adds them. This gate prevents a writer from
silently dropping pool content, names, attributes, or namespace declarations
while allowing structural XML formatting to differ from lxml output.

The current official crate documentation describes namespace resolution,
reader support, and writer support; version 0.42.0 was released in August
2026.

### Exam YAML: `serde_yaml_ng`

**Decision:** use `serde_yaml_ng = "*"`, resolved to 0.10.0.

**Command:**

```sh
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run --offline
```

**Input:** a typed `exam_yaml` document covering top-level `title` and `date`,
a section heading, MC choices, an image-bearing HTML statement, and a MATCH
table (`columns` and two `rows`). It matches the shape emitted by Python
`engines/exam_yaml/engine_class.py` and `write_item.py` at the pinned source
commit.

**Observed result:** both `serde_yaml_ng` 0.10.0 and `serde_norway` 0.9.42
read and wrote the complete emitted shape to the same typed value. The seven
questions exercise every Python `write_item.py` shape: MC, MA, and ORDER have
`statement` plus `choices`; MATCH has `statement` plus a two-column `table`;
NUM, FIB, and MULTI_FIB have only `statement`. The statement corpus includes
verbatim image-bearing HTML. Each candidate's serialized output was parsed
again and compared to its input value.

`serde_yaml_ng` is selected because its official repository says the maintainer
is still maintaining the fork, and its official current API documents the
required Serde derive, `from_str`, and `to_string` operations. `serde_norway`
also passed the shape test and exposes the same API, so it remains the fallback
if the selected crate becomes unsuitable. The active plan deliberately removed
the obsolete release-date rule; this decision rests on current maintenance and
the measured full-shape round trip.

### English cardinal and ordinal spelling: direct implementation

**Decision:** implement English cardinal and ordinal spelling directly for the
small integer range used by the package. Keep Roman numeral conversion direct
as already required by the plan. Do not add a spelling crate.

**Command:**

```sh
source source_me.sh && python3 -c 'import num2words; values = (0, 1, 2, 3, 4, 11, 12, 20, 21, 42, 100, 101); print(";".join("{}:{}|{}".format(n, num2words.num2words(n, to="cardinal", lang="en_US"), num2words.num2words(n, to="ordinal", lang="en_US")) for n in values))'
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run --offline
```

**Observed result:** Rust `num2words` 1.2.0 and `num2en` 1.0.0 agree with the
Python output for 3 and 42. Rust `num2words` agrees with Python over 0, 1, 2,
3, 4, 11, 12, 20, 21, 42, and 100, but diverges at 101: Python `en_US` writes
`one hundred and one` and `one hundred and first`; Rust writes forms without
`and`. A direct implementation is small, transparent, and is the only option
that satisfies exact Python behavior across the declared range.

## Re-run rule

Re-run the ignored spike after changing any selected dependency:

```sh
cd tests/_temp/dependency_spike
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run --offline
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo fmt --check
```

The XML test requires its two extracted fixture files. Recreate them from the
pinned Python source with the commands at the start of this document if the
ignored directory has been removed.

## WP-T3 raster stack spike (M16, 2026-09-30)

The ignored native spike at `tests/_temp/raster_spike/` is the reproducible
evidence for this proposal. It uses a separate lockfile, generated with Rust
1.98.1 and `CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo`. Its gallery is
generated at `output_tables/raster_spike/gallery.html`; that ignored directory
contains the three Python browser-oracle PNGs beside native same-input adapters.

**Proposal for M16 architect review:** use `cosmic-text = "*"` for text
shaping/layout and glyph rasterization, `tiny-skia = "*"` for painting and its
integrated PNG encoding, and `cssparser = "*"` as the CSS tokenizer beneath a
small, explicit supported-property parser. The spike lock resolved
`cosmic-text` 0.19.0, `tiny-skia` 0.12.0, `cssparser` 0.38.0, and `png` 0.18.1.
The `png` crate is transitive through tiny-skia; production does not need a
second encoder abstraction.

### Text: cosmic-text selected over parley

**Inputs:** corpus fragments
`2928c82b21be64dd3ba16636b15df3e8d1691daefbd6d282f0b58cdbfc7313d8.html`
(a 400 px tetrad table with a four-column header colspan and nested colored
Atkinson Mono genotype cells),
`616a38bc81ec110f5fca50e6cd8971b5c83c0f8c5ab68078a660fa9c55a5220e.html`
(gel cells and rounded colored bands), and
`fafcacbdb6bc3d0d2b2e6fec0b71e20d20847c0cae3ade4e97378b67ec272e21.html`
(`table_curve_lib`, fixed cells, dashed guides, radii, and sub/sup labels).
The existing Python `TableRenderer` generated the corresponding browser
oracle PNGs with the same font resources and corpus fragments.

**Command:**

```sh
cd tests/_temp/raster_spike
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo run
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo fmt --check
CARGO_HOME=/Users/vosslab/.cache/qti-rust-cargo cargo clippy -- -D warnings
```

**Observed result:** both candidates loaded both bundled Atkinson variable
TTFs and agreed on fixed-width 150 px line counts: default 400 = 5, default
700 = 5, mono 400 = 8. The cosmic-text probe paints glyph pixels through its
Swash cache into tiny-skia, including 400 and 700 weight requests. It also
draws the curve labels with a separately shaped 11 px raised superscript and
lowered subscript run; baseline offset remains an explicit inline-layout
responsibility in WP-R3. Parley 0.11.1 matched these line counts and supports
variable weights, but only supplies layout here: adding a second glyph
rasterization path would add more code. Cosmic-text provides the required
font database, HarfRust shaping, line breaking, and Swash rasterization in the
same stack, so it is selected under the least-code decision rule. This is a
stack-selection spike: its fixed adapters preserve these three source fixtures'
content and cell arrangements, while WP-R1/R2 will provide the general parser
and table layout engine.

Official API evidence checked 2026-09-30: [cosmic-text 0.19.0 documentation]
(https://docs.rs/cosmic-text/0.19.0/cosmic_text/) describes `FontSystem`,
`Buffer`, shaping, layout, and optional Swash rasterization; [parley 0.11.1
documentation](https://docs.rs/parley/0.11.1/parley/) describes its rich text
layout API and Rust 1.88 MSRV. Both are below this workspace's Rust 1.98.1
MSRV.

### Painter and PNG: tiny-skia selected

**Observed result:** the native gallery shows anti-aliased glyph compositing,
rounded gel bands in each of the source's seven rows, the tetrad table's
four-column header colspan and colored nested mono cells, and the curve source's
three-step titration arrangement. The curve adapter paints its 100 px cells'
top-left and bottom-right blue border-radius borders, three four-pixel radius
dots, and dashed guide cells; it does not substitute a generic cubic plot.
`tiny-skia::StrokeDash` painted the dashed lines and its anti-aliased path
painter rendered both the source-style curved borders and rounded backgrounds.
`Pixmap::save_png` produced valid RGBA PNGs. WP-R1/R2 must still generalize this
fixed proof into parsing and layout for the corpus before M17.

Official API evidence checked 2026-09-30: [tiny-skia 0.12.0
`Paint`](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Paint.html) shows
anti-aliasing and dashed stroke use; [tiny-skia 0.12.0
`Stroke`](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Stroke.html)
exposes `StrokeDash`; [png 0.18.1](https://docs.rs/png/0.18.1/png/) is the
pure-Rust encoder used by tiny-skia's PNG feature.

### CSS: cssparser tokenizer selected

**Observed result:** `cssparser` tokenized the combined hardest-fragment
declarations `border:1px dashed #999`, `border-radius:50%`, `width:100px`,
`line-height:0`, and `background:#fff` into 28 tokens. The corpus census also
shows the real value families that WP-R1 must interpret: hex/named/rgb colors,
px/percent/em/pt lengths, unitless zero, border shorthands, and inline style
declaration lists. `cssparser` handles syntax/tokenization while `qti-raster`
owns the allowlist, property parsing, and typed unsupported-feature errors;
it is not a general CSS acceptance layer. That keeps the declared subset
contract intact with less handwritten tokenization code.

Official API evidence checked 2026-09-30: [cssparser
0.38.0](https://docs.rs/cssparser/0.38.0/cssparser/) documents its CSS Syntax
Level 3 tokenizer and `Parser` API.

### Same-input adapter revision for architect re-review

The earlier M16 feature probes were rejected because their curve image drew a
single invented hump instead of the `table_curve_lib` fixture. The regenerated
gallery pairs browser-oracle and native PNGs for the same three source fragments:
the tetrad fixture above, `616a38bc81ec110f5fca50e6cd8971b5c83c0f8c5ab68078a660fa9c55a5220e.html`,
and `fafcacbdb6bc3d0d2b2e6fec0b71e20d20847c0cae3ade4e97378b67ec272e21.html`.
The ignored adapter is intentionally static and fixture-bound, but it now draws
the original text, spans, fixed cells, colored bands, dashed guides, radius dots,
and curve-cell border topology. M16 remains **pending architect acceptance**;
this revision supplies the requested evidence and does not claim a production
renderer or visual-parity exit.

## Chromium table renderer (2026-10-01)

### `chromiumoxide` 0.9.1 selected for table screenshots

**Decision:** render valid HTML tables with a locally installed Chromium browser controlled by
`chromiumoxide` 0.9.1. The Rust process launches Chromium lazily, reuses it for one conversion,
and accepts `QTI_CHROMIUM` as an optional executable-path override.

**Why:** HTML/CSS rendering is a moving browser target. The native subset renderer rejected the
gel fragment `3057867908e9cff30bd1536d9056db77e89c3ae85309540d928eaaab8e516c56` because of its
`box-shadow`, and lost colors and circles from
`08b73c60946140c7370d9a3ab8d990f833052e384b09b68ae0867fe9715bd360`. Those are content failures,
not tolerable layout variation. Chromium renders the source HTML/CSS directly without imposing a
Python or Node.js runtime dependency on users.

**Consequence:** the native raster stack documented above is retained as historical spike
evidence, not the production table-rendering direction. Table acceptance compares readable output
and browser rendering of valid source HTML/CSS; Python's older PNG layout and pixel equality are
not requirements. Benchmarking reports browser lifecycle and conversion cost to identify useful
improvements, without requiring Rust to outperform Python. Static RDKit canvases retain their
bounded parser and local-shim renderer, and sugar-library PNG/SVG exports bypass Chromium.

The user classifies four malformed HTML inputs and two obsolete HTML sugars outside this table
acceptance scope. [../refactor_progress.md](../refactor_progress.md) records their exact hashes.
