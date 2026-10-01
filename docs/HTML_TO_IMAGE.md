# HTML-to-image table contract

`qti-raster::render_table_png` converts one outer table fragment into PNG bytes without starting
a browser, executing JavaScript, fetching a network resource, or reading host fonts. It uses the
bundled Atkinson Hyperlegible Next and Mono fonts. `RasterConfig` defaults to a 1264 CSS-pixel
available width and a device scale factor of 2, matching the Python renderer's 1280-pixel viewport
after its browser body margins.

The renderer accepts one outer `table`, including nested tables in cells. It supports `caption`,
`thead`, `tbody`, `tfoot`, `tr`, `td`, `th`, `colgroup`, `col`, text, `span`, `b`/`strong`,
`i`/`em`, `sub`, `sup`, `br`, `p`, `div`, `ul`, `ol`, `li`, legacy `font`, and embedded `data:`
PNG or SVG images. Table attributes cover borders, padding, spacing, dimensions, alignment,
background color, and spans. Cell backgrounds, the supported border styles, rounded clipping,
text and vertical alignment, white space, fixed or automatic layouts, and the documented CSS
length and color forms are painted natively.

The CSS subset includes `display` values `inline`, `block`, `inline-block`, `inline-table`, and
`table`; `visibility:hidden`; `min-width`; `caption-side`; nonnegative pixel `letter-spacing`;
and the harvested one-shadow grammar. An inline table may use `position:relative; top:-0.2em` as
a post-layout paint offset. `vert-align` and the five harvested unprefixed six-digit colors are
documented compatibility no-ops that preserve Chromium's effective behavior.

Four constrained scene routes handle generated positioned figures: `table.boxplot`,
`pedigree_glyph`, `restriction_digest_map`, and `titration_state_tiles`. Their recognition and
bounded drawing grammars are defined in the [WP-R1 corpus ruling](active_plans/decisions/wp_r1_corpus_and_baseline_ruling.md).
They are declarative display-list inputs, not general CSS positioning support.

Any tag, attribute, property, source image, or geometry outside this contract returns a typed
`RasterError`. The error identifies the unsupported feature or the geometry/paint stage; the
renderer does not substitute an estimated image.

Run `cargo xtask table-gallery` after `cargo xtask table-corpus` to build
`output_tables/gallery/run-<timestamp>/index.html`. Each invocation creates a fresh run directory
so prior visual-review evidence remains available. The page shows the sandboxed live source,
native PNG, and Python reference PNG for every harvested table, retaining typed native failures
for review.
