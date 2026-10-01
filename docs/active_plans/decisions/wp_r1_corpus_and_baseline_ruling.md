# WP-R1 corpus and baseline ruling

## Decision

The R1 census extends the published table subset only where a real generator
needs a defined visual or accessibility result. Every other observed feature
is either a named compatibility no-op or a typed `UnsupportedTableFeature`;
the renderer may not silently omit a property, attribute, tag, or table from
the 290-table gallery.

### General table subset additions

| Corpus feature | Ruling |
| --- | --- |
| `display:inline-table`, `inline`, `block`, `inline-block` | Support the observed values. `inline-table` affects only surrounding flow and is raster-equivalent to the table border box; inline/block/inline-block determine inline line breaking and atomic inline layout. Other display values are typed unsupported. |
| `visibility:hidden` | Support it as no-paint while retaining layout dimensions. `visible` is the default; other values are typed unsupported. |
| `min-width`, `width:auto`, and `tr height` | Support them in the CSS 2.1 table layout calculation. `auto` is the normal unspecified width, not an error or zero. |
| `caption-side:top` and `bottom` | Support both; any other value is typed unsupported. |
| legacy `<font size color face>` | Support its legacy inherited text semantics, mapping face to the bundled Next/Mono selection and size to the documented CSS-size scale. Other `<font>` attributes are typed unsupported. |
| `scope` on `th`, `role`, `aria-hidden`, and `class` | Accept these as declared metadata, never as a CSS-selector mechanism. `scope` and role do not change paint. A table-level `aria-label` supplies the replacement image's alt text; otherwise use the existing text-derived alt rule. |
| `box-shadow` | Support the observed single, non-inset shadow grammar (`x y [blur] [spread] color`) as a paint-only effect in the general subset and the named scenes below. Multiple, inset, percentage, or otherwise unparsed shadows remain typed unsupported. |
| `letter-spacing: 2px` | Support non-negative absolute `px` letter spacing in inline measurement and paint. It changes the width of the pathway ellipsis, so it is not a no-op. Other units and negative values remain typed unsupported. |
| `position:relative; top:-0.2em` on an inline table | Support a finite `top` offset in `em` only on `display:inline-table`, after normal layout. It changes paint placement without changing the table's allocated flow box. `left`, `right`, `bottom`, `z-index`, and relative positioning on other general elements remain typed unsupported. |

`vert-align` is an observed misspelling. Preserve browser/Python behavior by recognizing this
exact property as a documented compatibility no-op; do not reinterpret it as `vertical-align`.
The observed invalid `spacing: 20px` declaration is also a compatibility no-op; it has no CSS
layout meaning.
An unprefixed six-hex-digit token used as a color value (the harvested `ff0303`, `ff9000`,
`b9e710`, `1c7d72`, and `6d1685`) is likewise an explicit invalid-declaration no-op, leaving the
inherited color in force, as Chromium does. Both compatibility cases need a census-linked test and
gallery annotation, rather than an unrecorded parser drop. Any other malformed or unknown
declaration remains typed unsupported with its source snippet.

### Boxplot scene exception

`table.boxplot` is not inert metadata: the corpus uses its nested positioned `div` and `span`
elements to draw data boxes, whiskers, ticks, and labels. Add a named, constrained `boxplot`
scene path. It may parse only the harvested pattern: relative container; absolute descendants;
`left`/`top`/`bottom`, width/height, margin offsets, `box-sizing:border-box`, simple backgrounds,
borders, the single shadow grammar above, and literal text labels.

The remaining source-generated positioned figures receive named structural routes. A
`StyledNodeKind::SceneLeaf` is an inline/block leaf inside the ordinary styled tree: R2 still owns
the containing table grid and cell metrics, R3 owns the leaf's intrinsic size, and R5 paints its
bounded display-list hook. It does not replace or bypass the containing HTML table.

| Scene | Recognition and accepted drawing grammar |
| --- | --- |
| `pedigree_glyph` (`SceneLeaf`) | An inline-block relative span with bounded pixel width/height, border, optional circular radius, and a single foreground label. It has either one solid background or exactly two absolute 50%-width left/right background halves plus the foreground label. It has no other positioned descendants. Its intrinsic size is the root box. The ordinary surrounding pedigree table, including its 65px connector sub-tables, remains in the general styled tree. |
| `restriction_digest_map` | A table with `role="img"` and the exact harvested `aria-label` for a Linear or Circular restriction-digest DNA map. Its fixed-size relative canvas accepts only absolute text/tick/line/outline marks using finite pixel coordinates, the harvested translate/rotate forms, simple colors and borders, and literal labels. The image alt remains the table's `aria-label`. |
| `titration_state_tile` (`SceneLeaf`) | A bordered tile remains ordinary block/cell content. Only its fixed-height relative content box becomes a leaf with exactly top-left, top-right, bottom-left, and bottom-right text groups. The containing four-state table, tile heading, arrows, padding, borders, and width calculation remain in the general styled tree. The groups admit the existing inline text, sub/sup, color, and weight subset. |

Each is a small declarative drawing language with bounded geometry and display-list output, not a
general CSS-positioning engine. Positioning, transforms, or selector-based behavior outside these
four named structures return `UnsupportedTableFeature`.

This is a justified scope extension under **ground requirements in actual needs**. A generic
positioning engine would violate the plan's non-goal and **fix the design, not the symptom**;
ignoring the 20 actual boxplots would violate the stated full-corpus objective.

## R5 proof boundary

R5 must render every one of the 290 harvested tables through either the general subset or one of
the four named scene grammars. Its gallery must enumerate each input's source hash, route, result,
and every compatibility no-op. It needs focused parser and PNG proof for all 20 boxplots; the 25
positioned pedigree-containing table fragments, including the 20 ordinary 715px pedigree grids;
both restriction maps; the titration-state table; the six inline offsets; the three general-shadow
tables; the letter-spacing table; `spacing`; and raw-color no-ops. Pedigree and titration proof
must demonstrate both the scene leaf and preserved general table/grid layout. The one table
carrying the known canvas must pass through the approved canvas replacement stage before the table
parser; the gallery records that route rather than reporting it as an R1 parse failure. A typed
unsupported result is allowed only for a newly harvested feature outside this ruling and must
appear as a failed gallery row, never be excluded from the denominator. The existing parser
failures are therefore implementation work, not a reason to redefine the corpus as smaller.

## Benchmark ruling

The horse corpus's unconverted existing-local-image failure is the confirmed Python
`convert_bank` media-base defect, not a harvest failure. It cannot be treated as an expected
failure in the performance comparison: a Rust run that successfully preserves those images and a
Python run that aborts are not the same workload.

Keep an unpatched Python run as a separately reported capability-failure receipt. For the timing
baseline, use a development-only temporary Python snapshot or in-process harness that applies the
same safe media-staging repair ruled in WP-T5: it must preserve all source local assets in a new
owned result root, render the same corpus, and never modify the Python checkout or external input
directory. Label the report **Python baseline with development-only media-base repair** and record
the unpatched failure beside it. Rust and repaired-Python measurements then use identical inputs,
formats, host, and successful output semantics.

## R1 final corpus extensions from run-1790780932

The receipt identifies five concrete semantics that remain implementation work. These are bounded
extensions of the existing general subset, grounded in the harvested source rather than a generic
CSS expansion.

| Observed source feature | Binding behavior |
| --- | --- |
| `border-style:hidden` on collapsed 65px pedigree connector tables | Add `Hidden` to the border style model. In collapsed-border conflict resolution, `hidden` has the CSS 2.1 precedence: it suppresses the competing border and paints no stroke. Its use outside a collapsed table also paints no stroke. |
| `<span style="font-size:xx-large;transform:scale(1.35);display:inline-block;...">U+27EE LEFT PARENTHESIS UPPER HOOK</span>` | `xx-large` already maps to 32px. Add only the observed literal `scale(1.35)` form on an inline-block text span and apply it to that span's glyph paint and measured advance. Other transform functions, values, and elements remain typed unsupported. |
| `box-shadow:0 0 2px #99dbfb` | Extend the accepted single outer-shadow grammar so spread is optional and defaults to zero. Unitless zero is a valid zero length; nonzero lengths still require the existing accepted length grammar. |
| `bgcolor="874e18"` | For the legacy HTML `bgcolor` attribute only, accept a six-digit ASCII hex color without `#` and normalize it to the same RGB value as `#874e18`. A bare CSS `color:874e18` remains invalid/no-op under browser semantics. |
| `table { margin: 0 auto }` | Support horizontal auto margins only on a finite-width table, resolving the two auto sides equally against the available containing width. Keep `auto` in all other margin positions typed unsupported. |

The prepared canvas table's `<a>` content is ordinary noninteractive inline text in the raster
output. Preserve its child text and supported inherited styling; discard hyperlink navigation,
which has no raster representation. Do not drop the glyph or report it as a compatibility no-op.

These changes preserve the general-table route and the existing `SceneLeaf` boundary. Add direct
parser/layout/paint tests for each row and retain them in the complete 290-row gallery receipt.
