# Inline CSS validation

QPM checks inline `style` attributes at the existing strict HTML validation boundary in
question text and other HTML-bearing item fields. XML structure is checked first. CSS errors
then identify the field, element, property, original value, and reason. Input is never repaired
or serialized back into the item. Legacy `color` and `bgcolor` attributes use the color check too.

BBQ, text2qti, BQGen, and Blackboard pool readers preserve typed CSS failures through their existing
malformed-item recovery paths. Confirmed CSS errors abort conversion rather than producing
an empty successful export with an item-skipped warning.

Python uses tinycss2 and cssutils; Rust uses cssparser and Lightning CSS. These libraries parse
declarations and validate common literal values. Every declaration is checked, including an
invalid declaration followed by a valid override. HTML character references are decoded for
inspection, without changing the authored content.

## Coverage and limits

- Reject malformed declarations, missing `#` on hex colors, bad hex digits/lengths, malformed
  RGB/HSL functions, invalid common literal colors, length units, alignment keywords, and
  negative values where the checked property forbids them.
- Preserve standard short/alpha hex, named/system colors, modern color functions, modern
  units, CSS-wide keywords, custom properties, variables, and common modern layouts.
- Reject the confirmed nonexistent properties `vert-align` and `spacing` with suggested
  replacements. Other unclassified property names are accepted, not treated as a whitelist
  violation. This is not exhaustive property-name typo detection.
- Defer unknown functions, relative colors, custom color profiles, and grammar extensions
  outside the reliable shared scope. In particular, expanded display, white-space,
  font-style, text-decoration, and background-size shorthand forms are not fully validated.
  Explicit malformed colors are still rejected before deferring a shorthand.
- Validate inline attributes only. External and embedded stylesheets are outside this feature.

The property sets in both implementation modules select validation scope, not permitted CSS.
A library failing to understand a feature is insufficient reason to reject it. Extend the shared
behavior cases when expanding reliable coverage. BP must correct generator errors; QPM does
not downgrade confirmed failures to warnings.

## BPW survey, 2026-10-12

Surveyed `biology-problems-website/site_docs`: 482 BBQ banks (155.58 MiB), 22,437 question
lines, 1,724,174 inline style attributes, 31,788 distinct styles, and 4,548,211 declarations.
There were 18,172 distinct property/value pairs across 63 property names, plus 374 distinct
legacy color/bgcolor attribute/value pairs. Python and Rust agreed on all 18,546 distinct
cases. The 13 rejected CSS pairs match the independent Chromium CSS.supports survey.

| Confirmed source error | Occurrences | Banks | Question lines |
| --- | ---: | ---: | ---: |
| Ten color values missing `#` | 1,550 | 3 | 150 |
| `height: 25` | 33,740 | 9 | 450 |
| `vert-align: middle` | 2,250 | 3 | 150 |
| `spacing: 20px` | 350 | 1 | 50 |
| Sixteen bgcolor values missing `#` | 2,400 | 1 | 50 |

Across overlapping categories, 800 question lines in 16 banks need BP-side correction.
Representative paths relative to BPW's `site_docs`, all with examples on line 1:

- `biotechnology/topic01/bbq-beadle_tatum-metabolic_pathway-4_metabolites-questions.txt`
- `genetics/topic11/bbq-gene_tree_choice_plus-MC-with_hint-TABLE_mode-LEVEL_1-questions.txt`
- `biochemistry/topic07/bbq-optimal_enzyme-type_1-questions.txt`
- `genetics/topic08/bbq-tetrad_ordered-centromere_distance-MC-questions.txt`

The 84 built HTML pages contained only 24 inline style attributes and no confirmed failures.
Questions load dynamically from banks, so built-page scanning alone misses these defects.
This survey did not regenerate, modify, or publish BPW content.

## Verification

Local checks on 2026-10-12:

- Python QPM: 3,881 tests passed; the final focused validator/reader rerun passed 78 tests.
- Rust QPM: 338 workspace/all-target tests passed, four pre-existing ignored tests, with one
  baseline failure excluded after reproduction. Formatting and Clippy with warnings denied pass.
- Rust repository checks: 1,934 pytest tests passed.
- Release Wasm package built; all 13 Node host/native-Wasm parity tests passed, including
  fatal CSS errors through BBQ, text2qti, and BQGen conversions.

Dependency advisory auditing was not run: cargo-audit is not installed in this environment.

The full Rust run encountered
`input::tests::sizes_are_enforced_for_maps_metadata_and_actual_decompression`:
`package-input` was returned instead of `package-size-limit` for oversized ZIP content.
The same test fails on an isolated archive of unchanged baseline commit `8b36058` with
its original Cargo.lock. This remains an unrelated integrity-test failure, not a passing
full-suite claim. No ZIP integrity code or test expectation was changed.

The permanent behavior cases cover malformed and accepted CSS, answer/table-cell context,
reader error propagation, and native-versus-Wasm conversion diagnostics. The survey compares
every distinct pair through both validators; it is not a full bank-conversion or visual test.

A local Python timing probe (100 calls, minimum of three repeats) measured a one-cell styled
table at 0.034 ms versus 0.004 ms for XML-only validation, and a 100-cell table at 2.414 ms
versus 0.093 ms. These are illustrative validation costs, not website performance measurements.
