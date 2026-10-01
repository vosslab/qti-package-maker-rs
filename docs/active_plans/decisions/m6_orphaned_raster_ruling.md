# M6 unmanifested raster ruling

## Decision: reject the Rust-only finding

`orphaned-packaged-image` must be removed from the standard `qti-integrity`
result and from M6's negative corpus. An otherwise unreferenced raster ZIP
member is not an item-media trace failure: no item points to it, so no learner
or LMS needs to resolve it. The cross-check must continue to compare complete
lists; it must not filter this finding or treat it as an accepted divergence.

## Evidence and parity authority

At the required Python source revision `55e5f368777f7809fe2e91b5d070caf6df0cb581`,
`package_integrity.check_entries` runs IMS manifest, answer linkage, and
referenced-media checks, then the all-raster dimension check. Its
`_check_image_dimensions` iterates every raster only to verify readable and
visible dimensions. It has no test that every ZIP raster has a manifest
`<file href>`.

Rust currently adds that condition in `qti-integrity/src/checker.rs` after the
referenced-source trace and labels it `Provenance::FormatRequirement`. The
negative corpus's `orphaned_image` case consequently yields `[]` in Python and
`orphaned-packaged-image` in Rust.

The active plan's tier 2 requires a recorded LMS failure or an invariant the
package format requires. Neither was supplied for an unused member, and the
M6 media-trace purpose only requires that an image *referenced by item HTML*
can be followed through manifest declarations to readable bytes. The fact that
the older M6 prose listed an image absent from the manifest does not establish
that provenance; the four-tier authority expressly prevents the checker from
becoming a specification by accident. Python behavior therefore controls at
tier 3.

## Required correction and gate

Remove the all-raster manifest-membership loop, its code mapping and Rust-only
extension classification, and the `orphaned_image` negative case/assertion.
Keep these distinct checks: a referenced image missing from the ZIP, an item
resource absent from the manifest, and a referenced image unavailable through
the declaring resource/dependency chain. Those have the media-trace purpose
and may retain their documented provenance.

After that change, rerun `cargo xtask oracle-crosscheck` at the pinned Python
revision and record its full-list agreement. The current live sibling checkout
is at `78632e507129278d036e4fb188f07a94c835ced4`, so the command correctly
refuses to certify M6 until the required `55e5f36` revision is restored or a
new plan survey explicitly repins it.

This is a required correction, not M6 acceptance.
