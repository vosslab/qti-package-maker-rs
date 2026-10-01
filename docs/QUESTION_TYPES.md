# Question types

`qti-core::ItemBody` models seven validated question shapes. Every `Item` is constructed with
`Item::new`, normalized, validated, and assigned a stable CRC identity before it enters an
`ItemBank`. The CLI lists these names with `qti-package-maker item-types`.

| CLI name | Rust variant | Required representation |
| --- | --- | --- |
| `MC` | `ItemBody::Mc` | choices and one answer contained in choices |
| `MA` | `ItemBody::Ma` | choices, answer set, minimum count, all-correct policy |
| `MATCH` | `ItemBody::Match` | prompt list and choice list |
| `NUM` | `ItemBody::Num` | finite answer, finite non-negative tolerance |
| `FIB` | `ItemBody::Fib` | one or more accepted answers |
| `MULTI_FIB` | `ItemBody::MultiFib` | named blank keys and accepted answers per key |
| `ORDER` | `ItemBody::Order` | three or more ordered answers |

## Validation boundary

All question text and HTML-bearing fields are validated before an item is accepted. Validation
requires a nonblank question stem, removes duplicate list members, verifies answers appear in
choice lists, bounds `MATCH` prompts by choices, rejects non-finite numeric values, and requires
each `MULTI_FIB` key to appear as `[key]` in the question text. Item HTML is checked using a
safe XML-compatible parse that rejects unsafe declarations and malformed markup.

`ItemBank` preserves insertion order, assigns one-based item numbers, and rejects mixed kinds
unless it was constructed with `allow_mixed = true`. Duplicate CRC identities are retained as a
typed duplicate outcome rather than a second item.

## Format coverage

Format support is a representation contract, not an automatic conversion promise. The full matrix
is in [ENGINES.md](ENGINES.md). The three limited writers currently have these sets:

| Writer | Supported kinds |
| --- | --- |
| `moodle_aiken` | `MC` |
| `okla_chrst_bqgen` | `MC`, `MA`, `MATCH`, `FIB` |
| `text2qti` | `MC`, `MA`, `NUM`, `FIB` |
| `canvas_qti_v1_2` | every kind except `ORDER` |
| `blackboard_export_zip` | every kind except `ORDER` |

All remaining writers declare all seven kinds. A writer skips unsupported kinds through the shared
render loop unless its format contract explicitly rejects a mixed bank before rendering.

## Presentation boundary

Writers receive `ItemRenderView` values, not mutable source items. A view can rewrite image
references for the destination format while retaining the original CRC and item shape. That makes
format-specific display changes unable to alter validation, answer data, or identity.

