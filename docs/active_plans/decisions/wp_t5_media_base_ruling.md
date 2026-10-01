# WP-T5 media-base preservation ruling

## Decision

Accept the confirmed Python defect and require a safe Rust departure. A successful
`html_to_image::convert_bank` that emits one or more PNGs returns a **new,
bank-owned temporary media root**. It never writes the source bank's external
directory. The source bank is unchanged; a failed conversion returns no derived
bank and leaves no conversion directory.

This replaces the plan's provisional phrase "carries forward `media_base_dir`".
The result carries forward the *resolvable source media*, not the source directory
handle. `MediaBaseDir::External` remains an input-only reference in this workflow.

## Evidence and authority

The isolated probe against certified Python revision
`55e5f368777f7809fe2e91b5d070caf6df0cb581` creates a bank rooted at an external
directory containing `existing.png`, converts a table with a stub PNG renderer,
then calls `collect_assets()`. Python creates a distinct `qti_media_*` root in
`_convert_bank_with_pairs` and loses the old base, so resolution tries
`<new root>/existing.png` and raises `FileNotFoundError`. The same result at
`78632e507129278d036e4fb188f07a94c835ced4` corroborates it.

This is a package-integrity failure: a converted package must still be able to
resolve every pre-existing local image. It outranks Python behavior under parity
tier 2. It also follows the repository's source-of-truth and "fix the design,
not the symptom" principles: a conversion result needs one owned, complete media
workspace, rather than a new root that silently drops part of its input.

## Required conversion contract

1. Before creating a directory, scan and resolve every source asset, prepare all
   replacements, and render every selected canvas/table into memory. A source read,
   selection, render, or HTML-rebuild error ends here with no derived bank or
   media directory.
2. If no fragment is selected, return `bank.clone()`; its existing typed ownership
   remains intact. Otherwise create `MediaBaseDir::temporary()` only after phase 1.
3. Materialize every local source asset into that root from its resolved bytes.
   Portable relative `src` paths retain their normalized relative paths and their
   existing HTML spelling, so untouched items retain their identity. Data URIs and
   remote URLs remain as authored; remote URLs are never fetched.
4. An absolute local `src` cannot be valid beneath the new root. Map it
   deterministically to a confined relative alias below `__qti_input/`, rewrite
   only the affected HTML fields, and reconstruct those validated `Item` values;
   their new CRCs are honest identities of the new content. The alias map is keyed
   by the resolved asset (content hash plus a safe extension), so equal inputs do
   not conflict.
5. Put generated PNGs below a deterministic reserved generated directory, choosing
   a suffixed root if an authored relative source already occupies that component.
   Keep the required `<original-crc>_<family>_<n>.png` filename as the leaf and
   rewrite only the converted fragment to that relative path. This prevents a user
   image from being overwritten while keeping generated filenames recognizable.
6. Build the result as `ItemBank::with_media_base_dir(allow_mixed,
   temporary_root)` and add reconstructed converted items in source order. Copy and
   write failures drop the unreturned temporary root. Do not call `add_image` on
   the source bank or on an `External` base.

The existing `ItemRenderView` remains the writer-boundary tool for package URL
rewrites. It is not a substitute for the new validated `Item` values that table
or absolute-path conversion genuinely changes.

## Required proof

Add focused coverage for: an external read-only source directory with an existing
relative image plus a rendered table; source immutability; a generated-name
collision; an authorized absolute local source; no-selected-fragment cloning; and
each phase-1 failure producing no conversion root. The post-conversion bank must
resolve all local images, and its package must pass the M6 referenced-media trace.
