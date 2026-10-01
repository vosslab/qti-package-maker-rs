# M3 untyped numeric boolean ruling

## Decision

Remove `ValidationError::BooleanAnswer` and `ValidationError::BooleanTolerance` from the validated
`Item` model. The public model accepts already typed `f64` values only. Its derived Serde
deserializer rejects JSON `true` and `false` for an `f64` field before `Item::from_parts` and
`validate_item` run, producing the deserializer's input error.

There is no public untyped-item conversion API in `qti-core`: the only `serde_json::Value` use is
the development `xtask` CRC corpus tool. The two variants have no construction path and therefore
cannot receive a meaningful behavior test.

If a future reader needs Python-style untyped field conversion, it must introduce one explicit
boundary function that accepts the untyped representation and returns these two errors for boolean
numeric fields. That function owns its own tests. It must not be invented merely to make dormant
enum variants reachable.

## Why

This keeps the validated Rust model honest about its input boundary and makes every public error
variant testable. It follows **fix the design, not the symptom** and **ground requirements in
actual needs**: Serde already rejects the supplied invalid type, while no production caller needs a
second JSON-to-domain adapter.

## Required correction

Delete both variants and any unreachable match arms or documentation. Add a focused deserialization
test showing booleans are rejected for `NUM.answer` and `NUM.tolerance`; assert only that the
deserialization fails, rather than coupling the domain type to Serde's exact wording. Keep the
existing finite and negative `f64` validation tests.
