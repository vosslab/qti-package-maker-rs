# Engine authoring

Add a focused format module in `crates/qti-engines/src/` and one entry in the compile-time
`ENGINES` registry. Native and Wasm callers use that same module. Rust owns parsing, media policy,
validation, and output assembly; host adapters own input loading and output persistence.

## Byte contracts

The public boundaries live in [traits.rs](../crates/qti-engines/src/traits.rs):

```rust
pub trait Reader: Send + Sync {
    fn name(&self) -> &'static str;
    fn read_items(
        &self,
        input: ReadInput<'_>,
        allow_mixed: bool,
    ) -> Result<ReadOutcome, EngineError>;
}

pub trait Writer: Send + Sync {
    fn name(&self) -> &'static str;
    fn media_policy(&self) -> MediaPolicy;
    fn supported_kinds(&self) -> &'static [ItemKind];
    fn write_package(
        &self,
        bank: &ItemBank,
        assets: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError>;
}
```

`ReadInput::File` borrows a logical name and bytes. `ReadInput::Archive` borrows a logical name
and `EntryMap`. Text readers require file input; Blackboard export accepts ZIP bytes or entries
and validates the package before discovering items. Return an ordered bank, recovered
`MemoryAssets`, and located warnings. Wrong input shape and malformed required data are typed
errors; recoverable skipped records retain warning source order.

`WriteContext::new` validates a relative POSIX output name. The caller supplies title, date, and
shuffle seed explicitly. A writer emits that name literally and returns an optional
`WriteArtifact`: a primary `NamedFile` plus companions, or a named directory of entries.
ZIP writers return ZIP bytes; text2qti returns relative `media/<leaf>` companions. PLE returns
question JSON and media entries; native publication adds its ownership manifest. Preserve each
writer's existing empty/unsupported behavior, including `artifact: None` where applicable.

## Minimal text output

This fragment demonstrates byte assembly after a writer has produced its records and warnings:

```rust
if records.is_empty() {
    return Ok(WriteOutcome { artifact: None, warnings });
}
let primary = NamedFile::new(context.output_name(), records.concat().into_bytes())?;
Ok(WriteOutcome {
    artifact: Some(WriteArtifact::File {
        primary,
        companions: Vec::new(),
    }),
    warnings,
})
```

For complete implementations, read [moodle_aiken/mod.rs](../crates/qti-engines/src/moodle_aiken/mod.rs)
for metadata-only placeholders and [text2qti/writer.rs](../crates/qti-engines/src/text2qti/writer.rs)
for named companion files. The portable engine never creates directories, copies files, reads a
clock, or launches a renderer. Native callers use
[persistence.rs](../crates/qti-native/src/persistence.rs) to persist an artifact.

## Registry and orchestration

[registry.rs](../crates/qti-engines/src/registry.rs) owns supported kinds, media policy,
output naming, reader/writer factories, and native-render eligibility. Trait metadata methods
delegate to the matching registry entry. Factories take no platform or document options.
`engine(name)` returns an optional entry; shared dispatch translates a missing entry into
`UnknownEngine` and a missing factory into `UnsupportedDirection`.

[conversion.rs](../crates/qti-engines/src/conversion.rs) exposes `read_bank`, `write_bank`, and
`convert`. Ordinary conversion reads once, trims, then writes once. `AssetOverlay` serves
reader-recovered bytes first and asks the supplied provider only for absent sources. Native
rendering composes those phases with one host pre-pass and writer fan-out. Add no target-specific
converter or engine list.

## Media and invariants

Select supported items before media inspection. `render_bank` skips unsupported kinds before
hooks. Metadata-only reference and placeholder policies use `describe_asset`,
`inspect_item_assets`, or `ItemBank::inspect_assets`; these functions do not read payloads.
Payload writers use `AssetSource`, `resolve_asset`, and `collect_assets` only for emitted media.
text2qti reads payloads because it emits companions. PLE scans only mapped display fields;
accepted-answer literals and feedback remain outside its media scan.

Apply the shared policy from [MEDIA.md](MEDIA.md). Rewrite `ItemRenderView`, preserving the
validated item's source HTML, CRC, and fingerprint. Asset memory keys and artifact file names
are validated relative POSIX names. Identical memory insertions are harmless; conflicting bytes
fail. Package companions are relative to the primary file's parent. ZIP directory markers follow
a separate validated contract.
For a native presentation transform returning an `ItemBank`, use the fallible
`with_rewritten_items` map to preserve item identity, kind, order, and source numbering.

Warnings are values in rendered item order. Propagate concrete `EngineError` variants with `?`;
printing belongs to the outer caller. `qti-core::media` remains the media function namespace;
the crate root also exports `AssetSource` and `MemoryAssets`.

Resolved `MediaAsset` payloads use `Arc<[u8]>`; `shared_bytes()` and
`NamedFile::from_shared` preserve that ownership when files or dependencies repeat. `NamedFile`
clones share immutable bytes. Use `read_bytes()` or `into_parts()` when an output boundary needs
an independent `Vec<u8>`. `EntryMap` and the provider's Cow read API retain their existing types.

## Authoring checks

- Update [FORMATS.md](FORMATS.md), [MEDIA.md](MEDIA.md), and [USAGE.md](USAGE.md) for the format.
- Test meaningful supported/unsupported, warning, malformed-input, and media behavior.
- Add round-trip or package-integrity coverage for archive formats and readers.
- Compile the same engine code natively and for `wasm32-unknown-unknown`.
- Add CLI or generated TypeScript coverage when a public host contract changes.
- Update [CHANGELOG.md](CHANGELOG.md) and settled decisions in
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md).

The reviewed detailed contract is
[shared_engine_contracts.md](archive/shared_engine_contracts.md).
