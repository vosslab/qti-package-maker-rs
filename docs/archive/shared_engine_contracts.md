# Shared engine contracts

## Decision and authority

D1-D9 approved by the manager on 2026-10-08. Native and Wasm consumers use the same portable
`qti-core` model and `qti-engines` readers/writers. `qti-native` owns filesystem access,
rendering, time resolution, and persistence. This contract follows **Fix the design, not the
symptom**, **Long-term over short-term**, and **Ground requirements in actual needs** in
[REPO_STYLE.md](../REPO_STYLE.md). API ownership and typed fallible boundaries follow
[RUST_STYLE.md](../RUST_STYLE.md).

The exact public signatures below are the implementation handoff. Documentation snippets omit
derives and doc comments; implementations document their public promises. String-bearing structs
use fallible constructors and accessors where validation is an invariant. Raw entry maps remain
available for interoperability and are revalidated at ZIP and host persistence boundaries.

## Core byte and media API

`qti-core` owns these types. `EntryMap`, `NamedFile`, ZIP functions, `AssetSource`, and
`MemoryAssets` are reexported from its thin crate root. Media functions and types retain their
established `qti_core::media` namespace:

```rust
pub type EntryMap = BTreeMap<String, Vec<u8>>;

pub struct NamedFile {
    name: String,
    bytes: Arc<[u8]>,
}

impl NamedFile {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Result<Self, ZipError>;
    pub fn from_shared(name: impl Into<String>, bytes: Arc<[u8]>) -> Result<Self, ZipError>;
    pub fn name(&self) -> &str;
    pub fn bytes(&self) -> &[u8];
    pub fn into_parts(self) -> (String, Vec<u8>);
}

pub fn validate_entry_name(name: &str) -> Result<(), ZipError>;
pub fn encode_zip<I, S>(entries: &EntryMap, empty_dirs: I) -> Result<Vec<u8>, ZipError>
where I: IntoIterator<Item = S>, S: AsRef<str>;

pub trait AssetSource: Send + Sync {
    fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError>;
}

pub struct MemoryAssets { /* private exact-src-key map */ }

impl MemoryAssets {
    pub fn new() -> Self;
    pub fn from_entries(entries: EntryMap) -> Result<Self, MediaError>;
    pub fn insert(&mut self, src: impl Into<String>, bytes: Vec<u8>) -> Result<(), MediaError>;
    pub fn get(&self, src: &str) -> Option<&[u8]>;
    pub fn entries(&self) -> &EntryMap;
    pub fn into_entries(self) -> EntryMap;
}

impl AssetSource for MemoryAssets { /* borrowed bytes */ }

pub fn describe_asset(src: &str) -> Result<MediaAsset, MediaError>;
pub fn resolve_asset(src: &str, source: &dyn AssetSource) -> Result<MediaAsset, MediaError>;
pub fn resolve_item_media_refs(
    item: &Item,
    source: &dyn AssetSource,
) -> Result<Vec<MediaRef>, MediaError>;
pub fn inspect_item_assets(item: &Item) -> Result<Vec<MediaAsset>, MediaError>;

impl ItemBank {
    pub fn collect_assets(&self, source: &dyn AssetSource) -> Result<CollectedAssets, BankError>;
    pub fn inspect_assets(&self) -> Result<CollectedAssets, BankError>;
    pub fn with_rewritten_items<E>(&self, rewrite: impl FnMut(&Item) -> Result<Item, E>)
        -> Result<Self, E>
    where E: From<BankError>;
}
```

`validate_entry_name` accepts nonempty relative POSIX file names with no trailing slash,
backslash, control character, colon, empty component, `.` component, or `..` component.
Spaces and UTF-8 remain supported. ZIP directory markers are separately validated relative
POSIX names ending in `/`; retain existing sorted markers and sorted file ordering. Supplied
memory assets use validated relative POSIX names; keys retain their exact accepted spelling.
Duplicate identical asset insertion is harmless; conflicting bytes at one key are an error.

`AssetSource::read` is read-only. A missing source is `MediaError::MissingAsset { src }`;
provider failures are `MediaError::AssetRead { src, message }`. Never fall back after an
arbitrary provider failure. Native directory access can accept an authored absolute local
source inside its authorized root, preserving current native behavior. Browser memory input
accepts relative names only. Neither provider fetches external URLs.

Core `describe_asset` inspects classification, MIME, and basename without reading a local
payload. `inspect_item_assets` and `ItemBank::inspect_assets` expose metadata-only traversal.
Reference/placeholder-only writers use this metadata inspection. `resolve_asset`
preserves classification, MIME rules, data URI parsing, source spelling,
hashing, naming, and field ordering. It resolves local bytes once into the returned asset.
`MediaAsset` drops `file_path`; `data_bytes` holds both local and data URI payloads. Existing
`read_bytes()` becomes an in-memory operation. Filesystem normalization and symlink confinement
leave core. Basename/MIME extraction uses POSIX string operations in portable code.

D23: `MediaAsset.data_bytes` is `Option<Arc<[u8]>>`; `shared_bytes()` returns a cheaply cloned
`Arc<[u8]>`. `NamedFile` also retains shared immutable payloads and offers `from_shared`.
`read_bytes()` and `NamedFile::into_parts()` make explicit `Vec<u8>` copies at output boundaries.
`EntryMap`, `MemoryAssets`, and the `AssetSource::read` Cow contract remain unchanged. Asset
dependencies and per-question PLE file lists share one resolved payload for an exact source.
JavaScript results retain independent owned `Uint8Array` copies.

`ItemBank` retains ordering, validated items, deduplication, mixed-kind policy, trimming, and
merge semantics. Remove `MediaBaseDir`, directory keepalives, `with_media_base_dir`,
`media_base_dir`, `set_media_base_dir`, and `add_image`. Media bytes belong to providers and
reader results. Merge no longer compares or retains media directory owners.

Writers select supported/rendered items before inspecting media. They do not eagerly collect
the full bank and then skip unsupported items. Reference/placeholder-only writers inspect
source metadata without calling `AssetSource::read`; text2qti does read payloads because it
emits companion files despite its reference warning policy. Payload writers resolve only media
actually needed for their emitted output. PLE additionally scans only its mapped display fields.

Keep `BankError::CollectAsset` provenance (`item_crc`, authored `src`, action, source);
drop `resolved_path`. Drop directory/create/write/symlink variants from `BankError`.
`MediaError` retains HTML, data URI, unsupported MIME, and payload errors; move every
path-bearing filesystem variant to the native error boundary. `ZipError` retains name,
directory-marker, and encoding failures; filesystem source/persistence errors become native
errors. Remove `ArchiveEntry::SourcePath`; engines build `EntryMap` byte values directly.
`build_zip` persistence and `collect_directory` move to native responsibilities, without
compatibility aliases in core.

## Reader and writer API

`qti-engines` owns these signatures:

```rust
pub enum ReadInput<'a> {
    File { name: &'a str, bytes: &'a [u8] },
    Archive { name: &'a str, entries: &'a EntryMap },
}

impl ReadInput<'_> {
    pub fn name(&self) -> &str;
    pub fn file_bytes(&self, engine: &'static str) -> Result<&[u8], EngineError>;
    pub fn archive_entries(&self, engine: &'static str) -> Result<&EntryMap, EngineError>;
}

pub trait Reader: Send + Sync {
    fn name(&self) -> &'static str;
    fn read_items(&self, input: ReadInput<'_>, allow_mixed: bool)
        -> Result<ReadOutcome, EngineError>;
}

pub struct ReadOutcome {
    pub bank: ItemBank,
    pub assets: MemoryAssets,
    pub warnings: Vec<ReadWarning>,
}

pub struct DocumentMetadata {
    pub title: String,
    pub date: String,
}

pub struct WriteContext {
    output_name: String,
    pub document: DocumentMetadata,
    pub shuffle_seed: u64,
}

impl WriteContext {
    pub fn new(output_name: impl Into<String>, document: DocumentMetadata, shuffle_seed: u64)
        -> Result<Self, EngineError>;
    pub fn output_name(&self) -> &str;
}

pub enum WriteArtifact {
    File { primary: NamedFile, companions: Vec<NamedFile> },
    Directory { name: String, entries: EntryMap },
}

pub struct WriteOutcome {
    pub artifact: Option<WriteArtifact>,
    pub warnings: Vec<MediaWarning>,
}

pub trait Writer: Send + Sync {
    fn name(&self) -> &'static str;
    fn media_policy(&self) -> MediaPolicy;
    fn supported_kinds(&self) -> &'static [ItemKind];
    fn write_package(&self, bank: &ItemBank, assets: &dyn AssetSource,
        context: &WriteContext) -> Result<WriteOutcome, EngineError>;
}
```

Input `name` is a logical relative name used for diagnostics, never a host path. Readers
reject the wrong input variant with `InvalidFormat`. Three text readers consume `File`;
Blackboard export accepts both `File` ZIP bytes and `Archive` entries. The reader decodes
`File` bytes through `qti_integrity::read_zip_entries(&[u8])`, which validates raw names,
duplicates, member types, compression, and bounds. Supplied entry maps are checked with
the same integrity input validation before Blackboard discovery. qti-integrity stays
independent of qti-core/models and engines; qti-engines may depend on it.

D14/D19: ZIP preflight checks every plausible EOCD and associated ZIP64 candidate
the decoder may select through fallback before constructing zip-rs. Original member-count and
ZIP64 extensible-record bounds apply to every candidate; selected-directory raw-name, duplicate,
and entry guards remain. An unusual input embedding a plausible overlimit archive directory may
be conservatively rejected because the decoder can fall back to it. D19 indexes candidates once
before decoder construction, avoiding repeated prefix scans. The index grows with input candidates;
the decoder's untrusted member metadata and extensible sectors remain bounded before allocation.

Text readers return empty recovered memory assets. Blackboard rewrites recovered media
tokens to accepted relative names and returns corresponding bytes in `assets`, replacing
temp-directory extraction. Preserve pool-relative lookup, hotspot handling, warning source
order, mixed-kind rejection, and CRC/fingerprint semantics.

`WriteContext` has no ambient `Default`: callers supply all fields. The constructor validates
the output logical name. Native callers resolve local date and practice selection seed once;
Wasm callers supply explicit values or use adapter defaults: title `Exam`, seed zero, and the UTC
date resolved once per conversion when omitted. Selftest chooses
one candidate using the supplied seed (bounded modulo is sufficient for current behavior),
replacing `SystemTime::now`. Metadata continues to affect exam YAML title/heading/date.

File companions are relative to the primary file's parent. text2qti emits `media/<leaf>`
companions. ZIP writers emit one primary ZIP byte buffer and no companions. PLE emits a
directory of `item_00001.json`-style question names plus `media/<leaf>` entries, preserving
the existing exact question filename convention. PLE's export resolves display fields only;
answer keys and feedback excluded today stay outside its asset scan. Its filesystem ownership
manifest and staged replacement remain native persistence concerns. The portable directory
contains question/media payloads; native adds its ownership manifest before publication.

`artifact: None` preserves the current no-rendered-output behavior of each writer. Keep the
existing selftest empty-bank error and unsupported-kind behavior; do not impose one new
global rule. Policy warnings retain order and are emitted only for rendered items.
`EngineError::Io { path, source }` moves to native. Shared errors remain concrete and typed;
retain `InvalidFormat`, unsupported-kind, validation, bank, manifest, ZIP, and portable media
errors. `ReadLocation`, `ReadWarning`, `RenderHooks`, and `render_bank` remain portable.

Registry lookup failures use `UnknownEngine { name: String }`; requesting an unavailable
direction uses `UnsupportedDirection { engine: &'static str, direction: &'static str }`.
Where an outer conversion failure needs source provenance, use `Context { engine, source_name,
source: Box<EngineError> }` once rather than repeating identical nested diagnostic context.

## Registry authority

Move every `KINDS` list into the single registry. Trait metadata methods delegate to the
matching entry; format modules contain no second supported-kind inventory. The registry owns
eleven writer entries and four reader entries:

```rust
pub struct EngineEntry {
    pub name: &'static str,
    pub media_policy: MediaPolicy,
    pub supported_kinds: &'static [ItemKind],
    pub default_output_name: &'static str,
    pub output_name_for_content: fn(&str) -> String,
    pub native_rendering: bool,
    pub make_writer: Option<fn() -> Box<dyn Writer>>,
    pub make_reader: Option<fn() -> Box<dyn Reader>>,
}

pub fn engine(name: &str) -> Option<&'static EngineEntry>;
```

Native-render eligibility is true for Canvas QTI 1.2, Blackboard QTI 2.1, Blackboard export ZIP,
and PLE Native JSON, matching the actual current CLI selection. Registry selection is
platform-independent; the native
adapter alone acts on rendering eligibility. Remove `EngineOptions`, writer render flags,
current UTC date helpers, and metadata held inside writer constructors.

| Engine | Default output | Content-derived output | Kinds |
| --- | --- | --- | --- |
| html_selftest | selftest.html | selftest-N.html | All seven |
| exam_yaml | exam.yaml | exam-N.yaml | All seven |
| okla_chrst_bqgen | okla.txt | okla-N.txt | MC, MA, MATCH, FIB |
| text2qti | text2qti-package.txt | text2qti-N.txt | MC, MA, NUM, FIB |
| blackboard_export_zip | blackboard-export.zip | bez-N.zip | All except ORDER |
| moodle_aiken | moodle-aiken.txt | aiken-N.txt | MC |
| human_readable | human-readable.html | human-N.html | All seven |
| canvas_qti_v1_2 | qti12-package.zip | qti12-N.zip | All except ORDER |
| blackboard_qti_v2_1 | qti21-package.zip | qti21-N.zip | All seven |
| bbq_text_upload | bbq-upload.txt | bbq-N.txt | All seven |
| ple_native_json | ple | ple-N | All seven |

`N` is the caller-resolved content name. All media policies remain as currently registered.

## Shared orchestration

Keep orchestration small, with reusable dispatch for native rendering and a complete ordinary
conversion for Wasm. No platform mode, path argument, renderer callback, or compatibility facade:

```rust
pub struct ConversionRequest<'a> {
    pub input_format: &'a str,
    pub output_format: &'a str,
    pub input: ReadInput<'a>,
    pub assets: &'a dyn AssetSource,
    pub allow_mixed: bool,
    pub max_items: Option<usize>,
    pub context: &'a WriteContext,
}

pub struct ConversionOutcome {
    pub artifact: Option<WriteArtifact>,
    pub read_warnings: Vec<ReadWarning>,
    pub write_warnings: Vec<MediaWarning>,
    pub item_count: usize,
}

pub fn read_bank(input_format: &str, input: ReadInput<'_>, allow_mixed: bool,
    max_items: Option<usize>)
    -> Result<ReadOutcome, EngineError>;
pub fn write_bank(output_format: &str, bank: &ItemBank, assets: &dyn AssetSource,
    context: &WriteContext) -> Result<WriteOutcome, EngineError>;
pub fn convert(request: ConversionRequest<'_>) -> Result<ConversionOutcome, EngineError>;
```

`convert` validates selected registry capabilities, calls `read_bank` once (read then trim),
then calls `write_bank` once. Readers preserve their warnings; writers preserve their
individual media warnings. `item_count` is the trimmed bank count, not a promise that every
format supports every item. Internal asset composition first serves recovered reader assets,
then caller assets only on `MissingAsset`. Empty source maps work for text-only conversions.

Native rendering uses `read_bank`, records loaded count before trimming when progress reporting
requires it, performs one native pre-pass, then calls `write_bank` for each selected
original/converted bank. The rendering result owns
its generated memory assets or native files separately from the bank. This is real workflow
composition, not a duplicated converter or target-selected implementation.

D17-D18: native fragment conversion takes the eligible selected writers' supported item kinds.
It reads/inlines authored images only inside selected rendered display-table fragments and
returns generated assets only. It excludes FIB/MULTIFIB grading literals. The core's fallible
`with_rewritten_items` map preserves identity, kind, order, and source item numbers when creating
the converted bank; it rejects a transform that changes identity or kind.

## Native migration ownership

| Current owner | New owner | Responsibility |
| --- | --- | --- |
| core bank media assets | qti-core | Read-only asset collection against AssetSource |
| core media resolve | qti-core | HTML/URI classification, MIME, byte snapshots, hashing |
| core media resolve filesystem | qti-native | DirectoryAssets canonical root and symlink confinement |
| core ZIP filesystem and tempfile | qti-native | Directory loading, atomic file persistence |
| engine html_to_image | qti-native | Chromium, renderer cache, conversion pre-pass |
| engine molecule calls | qti-native | Native-only molecule rendering/provider integration |
| Blackboard reader ZIP/directory access | qti-integrity plus qti-native | Safe ZIP decode; native bounded directory loading |
| Blackboard reader media extraction | qti-engines | Returned MemoryAssets, no extraction filesystem |
| text2qti copy_media | qti-engines plus qti-native | Named companion bytes; host persistence |
| PLE output/ownership staging | qti-native | Ownership manifest, staged replacement, rollback |
| PLE export/media mapping | qti-engines | Source JSON and display-only named bytes |
| CLI local date/input paths/progress | qti-native plus qti-cli | Host resolution and thin CLI reporting |

Native host API exposes `DirectoryAssets::new(root: impl Into<PathBuf>) -> Result<Self, NativeError>`
and implements the shared read-only source trait. The constructor canonicalizes and validates
the authorized directory once without scanning or reading media. Host errors retain path/source detail;
trait reads convert host access failure to portable `AssetRead` with safe diagnostic text.
Host persistence revalidates every logical name before joining an authorized output root,
preflights payloads before replacing output, and preserves existing PLE refusal of foreign
or modified output directories. Keep the fixed headless renderer behavior.

## Acceptance evidence

- Same `qti-core` and `qti-engines` sources compile natively and for `wasm32-unknown-unknown`.
- All eleven writers and four readers dispatch from one registry with existing supported kinds.
- Native CLI filenames, warnings, package content, and PLE ownership behavior stay observable.
- Native/Wasm conversion uses the same explicit context and compares normalized logical output.
- Lazy sources resolve referenced payloads only; PLE does not read grading-only media.
- Malformed archive inputs fail before model discovery or host persistence.
- Portable production modules contain no filesystem, temporary directory, process, ambient time,
  Chromium, molecule-native, or host-path runtime dependency.
