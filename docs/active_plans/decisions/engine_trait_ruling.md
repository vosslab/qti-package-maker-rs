# Engine trait ruling and reader survey

## Status

Binding M7 architecture decision. This is a design-only result: implementation waits for the M5
manifest/ZIP and M6 integrity gates. The Python survey used source commit `55e5f36` (the required
`55e` head). Repeat the survey against the pinned Python source immediately before M8 dispatch;
new reader behavior changes this decision before it fans out.

## Evidence from the four readers

| Reader | Recoverable records | Location available | Fatal boundary | Media ownership |
| --- | --- | --- | --- | --- |
| `bbq_text_upload` | Blank lines and malformed lines are skipped with a warning. | One-based input line. | Opening the input is fatal. Unsupported type is a recoverable line error. | The returned bank resolves local HTML images relative to the input directory; it does not own that directory. |
| `text2qti` | Invalid or unrecognized question blocks are skipped with a warning. | One-based question-block number. | Opening the input is fatal. Recognized but malformed blocks are skipped. | The returned bank resolves local HTML images relative to the input directory; it does not own that directory. |
| `okla_chrst_bqgen` | Unrecognized blocks are silently omitted by Python. | One-based block number is available. | Opening the input is fatal. | No reader-created media directory. |
| `blackboard_export_zip` | Unknown `bbmd_questiontype` and malformed individual items are skipped with a warning. | Pool `.dat` resource name plus one-based item position; media failures also name a manifest resource or token. | Invalid input shape, ZIP-slip, missing manifest, empty/missing pool resource, malformed required XML, and unresolved declared media are fatal. | It merges every declared pool resource. Recovered media is copied into a persistent extraction directory, assigned as bank-owned media, and its HTML is rewritten to those filenames. |

This is based on `base_engine.py:84-135`, the three text readers' `read_items_from_file` loops,
and the Blackboard reader's `read_package.py:33-60,90-114,199-227` plus
`read_items.py:35-83`. Python's `print` warnings become data in Rust; no source location is lost.
The Okla omission becomes an explicit warning in Rust because silent partial success cannot be
reported or audited otherwise.

## Reader contract

Use the following concrete, object-safe boundary:

```rust
pub trait Reader {
    fn name(&self) -> &'static str;
    fn read_items(&self, input: &Path, allow_mixed: bool)
        -> Result<ReadOutcome, EngineError>;
}

pub struct ReadOutcome {
    pub bank: ItemBank,
    pub warnings: Vec<ReadWarning>,
}

pub struct ReadWarning {
    pub location: ReadLocation,
    pub message: String,
}

pub enum ReadLocation {
    Input,
    Line { line: usize },
    Block { number: usize },
    ArchiveEntry { name: String },
    PoolItem { resource: String, number: usize },
    MediaToken { resource: String, token: String },
}
```

`EngineError` is the fatal channel. It must keep the engine name, input path, and typed cause
(I/O, archive, XML, validation, media, unsupported item kind, or invalid package shape). A fatal
failure returns no `ReadOutcome`; successful partial reads return their bank and all warnings.
Warnings retain input order. A reader may use `ReadLocation::Input` where the format has no
record-level position.

The reader owns no separate lifetime handle. A Blackboard reader creates `MediaBaseDir::Temporary`
for recovered images and places it in `ReadOutcome.bank`; `Arc<TempDir>` then keeps it alive across
bank merges until the last bank drops. File-authored readers use `MediaBaseDir::External` for the
caller-owned input directory. This uses the established bank ownership rule rather than adding a
second ownership channel to `ReadOutcome`.

## Writer and render contract

```rust
pub trait Writer {
    fn name(&self) -> &'static str;
    fn media_policy(&self) -> MediaPolicy;
    fn supported_kinds(&self) -> &'static [ItemKind];
    fn save_package(&self, bank: &ItemBank, output: Option<&Path>)
        -> Result<PathBuf, EngineError>;
}

pub fn render_bank<R>(
    bank: &ItemBank,
    render_item: impl Fn(&ItemRenderView) -> Result<Option<R>, EngineError>,
    hooks: RenderHooks<'_, R>,
) -> Result<Vec<R>, EngineError>;
```

`RenderHooks` carries the two generic, per-call hooks from Python's
`BaseEngine.process_item_bank`: an optional pre-render `Fn(&Item) -> Result<ItemRenderView,
EngineError>` and an optional post-render `Fn(&Item, R) -> Result<R, EngineError>`. The loop skips
unsupported writer functions before the pre-render hook and drops `None` render results, preserving
Python's control flow. `R` remains the writer's private representation, so the render helper is
generic and is not a trait method.

`ItemRenderView` is the only pre-render value. Its base constructor must be a public read-only
`Item::render_view()` accessor; only `qti-core` media code may mutate its HTML fields while building
a rewritten view. Writers consume its accessors. The validated `Item`, raw body, and CRC remain
unchanged, while the view carries the original CRC and item number. This preserves Python's
observable rewrite identity without permitting an invalid rewritten `Item` or serializing
writer-specific HTML.

The traits have no associated types, generic methods, or `Self`-returning methods, so
`Box<dyn Writer>` and `Box<dyn Reader>` remain valid. The local Rust reference's trait-object
guidance supports this split: trait objects provide the heterogeneous registry boundary, while
generic rendering stays statically dispatched at each writer call site.

## Registry and options

One compile-time `const ENGINES: &[EngineEntry]` in `qti-engines/src/registry.rs` is the sole
engine authority. Each entry holds a fixed name, media policy, and optional constructor function
for reader and writer trait objects. `can_read` and `can_write` derive only from whether those
constructors are present. There is no filesystem scan, import inspection, feature probing, or
runtime capability discovery.

`EngineOptions { html_to_image: bool }` is the common factory input so the registry type stays
uniform. Only `canvas_qti_v1_2`, `blackboard_qti_v2_1`, and `blackboard_export_zip` may honor a
true value; every other writer receives the default. The CLI validation and output-name restrictions
remain M12 work, including the startup version-check survey. They are intentionally outside this
M7 design decision.

## M7 implementation order and decision

First compile a populated three-probe registry containing trait objects for `human_readable`,
`canvas_qti_v1_2`, and `bbq_text_upload`. Then implement the probes and show that text and XML
representations both use `render_bank`, while BBQ returns warnings through `ReadOutcome` and
round-trips by `ItemFingerprint`. No async or concurrent reader/writer API is authorized: the
survey supplies no requirement for it and deterministic source-order warnings are required.

This decision is accepted for M7 once M5 and M6 pass. A changed Python-head survey or an
object-safety compile failure reopens the design before M8-M11 work begins.

## M7 implementation review: corrections required before fan-out

The object-safe trait and populated three-entry registry are accepted in shape. The fresh
`cargo test -p qti-engines` result is 29 passing tests, and each entry constructs its declared
trait object. The four-reader survey remains sufficient: the stated Python source re-survey found
no reader change between certified `55e5f36` and current `e88a` except a writer-side
`engine_class` change.

M7 is nevertheless **not accepted for M8/M9/M11 fan-out** yet. The following concrete corrections
are required by the published WP-G0 gate:

1. `canvas_qti_v1_2::save_package` must not pre-reject `ORDER`. Pinned Python exposes an `ORDER`
   writer that returns `None`, and `BaseEngine.process_item_bank` skips it before its transform.
   Rust must let `render_bank` skip the unsupported kind, so a mixed bank produces the supported
   six-type package without calling its pre-render media hook for `ORDER`.
2. Add a focused generic `render_bank` test proving unsupported-before-pre, pre-render,
   `None`-drop, and post-render behavior. Exercise it with the text and XML private result types,
   or equally direct writer tests. At present all three writers use no post-render hook, so the
   required two-hook contract is unproved.
3. Make probe completeness reviewable: Human must render all seven kinds and its placeholder
   media rewrite; BBQ must round-trip all seven kinds by `ItemFingerprint`, retain an external
   input media base, and return ordered located warnings for blank, malformed, and mixed-kind
   records; Canvas must structurally prove all six supported kinds, `ORDER` skipping, local media
   packaging/rewrite, and external-media behavior, then pass `qti-integrity` with zero error
   findings. The current tests cover only MC in each format (plus Canvas's incorrect ORDER error),
   so they cannot establish these format claims.

These are focused acceptance tests and one control-flow repair. They preserve the frozen traits,
registry, and reader contract; no redesign or shared HTML-conversion work is authorized.

## M7 implementation re-review: accepted

The corrections meet the gate. `render_bank` now skips an unsupported kind before its pre-hook,
runs the pre-hook for each supported item, drops a `None` private render value, and runs the
post-hook only for retained values. The direct event-sequence test proves all four steps.

`ItemRenderView` remains the appropriate presentation boundary. `replace_item_images` builds a
view, inserts replacement text through the HTML rewriter's text-content API, and leaves the source
item, its raw serialization body, and its CRC unchanged. It therefore extends Human's image
placeholder without creating an invalid reserializable item.

The writer probes now establish the declared format scope: Human renders all seven kinds and its
media placeholder; BBQ round-trips all seven by fingerprint and preserves external input-base plus
ordered line warnings; Canvas renders its six declared kinds, skips `ORDER`, packages a local
image, retains an external URL, and has no integrity errors. A fresh `cargo test -p qti-engines`
passes 34 tests. M8, M9, and M11 may fan out against these frozen trait, registry, and render-view
contracts.

## M8 text2qti delimiter repair

The Rust text2qti writer shall separate every rendered question block with one blank line. This is
an intentional Python-parity exception, recorded here rather than hidden in a writer test.

Pinned Python `text2qti/write_item.py` returns each supported item with a single trailing newline,
and `engine_class.py::save_package` writes the returned strings consecutively. Its
`read_package.py::split_questions` starts another numbered block only when the preceding line is
blank. Consequently, a two-item file emitted by that same Python writer is one reader block and
only its first item survives. The pinned unit tests round-trip one item at a time, so they do not
exercise this contradiction.

Rust's `join("\n")` supplies the missing delimiter. The text2qti grammar and the M8 reader/writer
contract require a multi-item output to be readable as the same ordered supported-item sequence;
that observable data-preservation requirement outweighs the defective Python byte stream. Keep the
direct multi-item fingerprint round-trip and exact rendered separator test. Do not reproduce the
defect or loosen the reader's blank-line grammar.

## M12 document metadata factory contract

Replace the `Copy` boolean-only `EngineOptions` with an owned, cloneable configuration:

```rust
pub struct EngineOptions {
    pub html_to_image: bool,
    pub document: DocumentMetadata,
}
pub struct DocumentMetadata {
    pub title: String,
    pub date: String, // ISO-8601 civil date: YYYY-MM-DD
}
```

`EngineOptions::default()` provides title `exam` and the current **UTC** date, preserving direct
library factory behavior. M12's CLI constructs it once per run with the input file's UTF-8 stem as
the title and the CLI host's local civil date, then clones it for registry factories. The CLI is the
only place that may obtain local time; engines consume the already-resolved values and do not read a
clock. `exam_yaml` uses both fields for its top-level title/date and section heading. All other
writers receive the same options and ignore `document` until a demonstrated output contract needs
it.

This has one owned value path, makes output reproducible when a caller supplies metadata, and
keeps the object-safe `fn(EngineOptions) -> Box<dyn Writer>` registry constructor unchanged. It
does not add output-path naming, format-specific option fields, or a new metadata trait.

## M11 html_selftest control and warning contract

M11 remains **not accepted** until the embedded controls implement the pinned Python control
contract. The amended plan explicitly says that the four added `control_styles`, `drag_controls`,
`match_controls`, and `order_controls` assets are embedded verbatim. The Rust output may use
`include_str!` templates with item identifiers substituted at render time, but it must preserve the
emitted DOM behavior and strings of those source assets; a new global control program is not an
equivalent substitute by assertion alone.

The required repair is bounded to the demonstrated deviations:

1. Preserve the Python answer semantics for MC, MA, FIB, NUM, MULTI_FIB, MATCH, and ORDER. In
   particular, FIB uses only lowercase-and-trim normalization; it does not remove punctuation,
   whitespace, or units. Correct answers disable the pinned Check button. Result strings and the
   `question_html_<crc>`, `result_<crc>`, and `checkAnswer_<crc>` contract remain exact.
2. Port the four control assets' state transitions: drag source validation, insertion placement,
   MATCH's one-use choice disabling/re-enabling and truncated slot label/title, explicit reset,
   keyboard and status behavior, ORDER move-button endpoint state, and scoped CSS including the
   180x44 MATCH slot. The current Rust bundle's duplicate MATCH choice assignment, missing
   disabled-bank update, and different slot content are observable departures.
3. Keep the separately approved Reveal and generalized Reset buttons as an additive layer. They
   may populate answers or invoke the pinned reset transitions, but may not alter ordinary Check
   grading, answer normalization, result strings, IDs, drag/drop constraints, or source CSS.
4. Add one browser-facing test per item kind that exercises a correct and an incorrect path. The
   MATCH/ORDER cases must also exercise click/tap or drag plus keyboard movement and reset; tests
   assert the exact completion strings consumed by the website. This is the needed evidence for a
   browser control contract, not an arbitrary seventh test category.

Writer media diagnostics need a structured output path. `MediaPolicyDecision` already produces
provenance-complete `MediaWarning`s, but `Writer::save_package -> Result<PathBuf, EngineError>`
currently discards them, including html_selftest's kept-external-URL warning. Do not print from a
library writer. Replace its success value with:

```rust
pub struct WriteOutcome {
    pub path: PathBuf,
    pub warnings: Vec<MediaWarning>,
}
```

Each writer accumulates the policy warnings for the items it actually renders and returns them with
the output path. M12's CLI prints those warnings in returned order; library callers can inspect
them. `html_selftest` must return a warning for an external URL kept verbatim, while local images
become data URIs. Existing callers change only from consuming `PathBuf` to `outcome.path`.

This is a narrow reopening of the M7 writer result boundary because the published media contract
requires every warning to carry usable provenance. It is a design correction, not a console-only
workaround; readers retain their independent `ReadOutcome`.

## WPF4 pinned-oracle and no-output contract

**Accepted:** retain frozen Python `55e5f368777f7809fe2e91b5d070caf6df0cb581`. Its
`tools/bbq_converter.py` exposes exactly seven hard-coded format choices, whereas its registered
package interface can construct all ten writers. M13 therefore has two named oracle lanes: actual
CLI-versus-CLI comparison for the seven exposed formats, and a fixed package-interface adapter for
only `text2qti`, `okla_chrst_bqgen`, and `exam_yaml`. The adapter reads BBQ through
`QTIPackageInterface` and calls its registered writer; it neither reimplements CLI behavior nor
discovers a changing engine list. Reports must label the lanes separately and must not describe the
three adapter comparisons as CLI parity. A repin needs independent evidence of a changed authority,
not merely this frozen CLI limitation.

Writer semantic fixtures contain only each writer's declared `supported_kinds`, in source order.
Unsupported handling remains a separate explicit gate. In particular, an ORDER-only input verifies
that Canvas completes with no artifact, while Blackboard export completes with an ORDER omission
and its observable unsupported-kind diagnostic. This prevents a seven-kind input from disguising a
partial writer as a full semantic comparison.

`WriteOutcome` is revised to make the completed artifact optional:

```rust
pub struct WriteOutcome {
    pub path: Option<PathBuf>,
    pub warnings: Vec<MediaWarning>,
}
```

`None` means this particular writer completed without an artifact; `Some(path)` means it created
one. This is deliberately writer-specific: pinned `text2qti` always opens, writes, and returns an empty file, while pinned Okla, Aiken, and exam YAML return `None` with no file when no item
renders. The CLI counts and prints every `Some(path)`, including an empty text2qti artifact; its
existing quiet skip behavior is preserved for `None`. Every writer must test its zero-render case.

For CLI-produced document metadata, `DocumentMetadata.title` is the already-validated
`content_name` extracted from `bbq-<name>-questions.txt`, not the raw file stem. The CLI resolves
the local civil date once. This restores pinned Python's package-name flow into exam YAML's title
and heading; parity retains those YAML fields without normalization.

## HTML selftest literal-space media repair

The pinned HTML selftest path percent-encodes a literal-space image source before resolving it as a
filesystem path, then fails on the encoded literal. Keep that failed end-to-end Python receipt.
Rust must nevertheless inline a literal-space local filename: the direct Python asset resolver and
the Rust resolver both read the literal-space file, and the Rust selftest output must contain a data
URI whose decoded bytes hash to that file. This is a package-usability repair over a tier-3 Python
defect, not an excluded parity case. Do not globally decode percent escapes: an authored `%20` URI
spelling needs its own source-backed contract.

## M11 asset re-review and M19 measurement boundary

**M11 remains rejected.** The 2026-09-30 independent comparison invoked the four pinned
Python generators at `55e5f368777f7809fe2e91b5d070caf6df0cb581` with the same `{{CRC}}`
placeholder used by Rust. Only `order_controls.js` was byte-identical. The expected and embedded
SHA-256 pairs were: `drag_controls.js` `5e8c85bc...` / `fb50326c...`,
`match_controls.js` `ec9c4884...` / `11b371a3...`, and `control_styles.css`
`03bd6372...` / `e423db9d...`. These are not formatting differences: the current assets change
MATCH and drag behavior and responsive control layout. The repository's only
`browser_proof.rs` is an ignored HTML-page generator, not a browser assertion or receipt.

The M11 repair must replace each nonmatching asset with its exact pinned-generator output under
`{{CRC}}` substitution and retain the output's item-scoped script placement. Additive Reveal/Reset
code must remain outside those assets. After that repair, record a seven-kind browser run proving
correct and incorrect grading for each kind and mouse, keyboard, touch, and reset transitions for
MATCH/ORDER. This follows the repository rule that a behavioral claim needs direct evidence and
fixes the source-boundary error rather than papering over its symptoms.

**M19 instrumentation is accepted with this bounded contract.** Add an internal
`convert_bank_with_metrics` that returns the converted bank and `ConversionMetrics`; the existing
`convert_bank` delegates and discards metrics, so CLI behavior and ordinary callers do not grow a
measurement concern. A `FragmentRenderer` returns a `RenderedPng` containing bytes and local
`RenderMetrics`. Native table rendering reports non-overlapping `layout`, `paint`, and `encode`
durations around their actual stages. Each conversion work item owns its metrics and Rayon reduces
them after completion; it must not use shared timing atomics.

Metrics distinguish requested fragments, cache hit/wait/render outcomes, actual renderer attempts,
successes, and failures. The cache outcome is recorded before invoking the renderer; failures count
as attempts and retain only completed spans. Filesystem materialization/write has its own duration.
The benchmark records end-to-end wall time separately. Per-stage values are cumulative under
parallel execution and must be labelled as such, never summed or presented as elapsed wall time.
`conversion_bookkeeping` may only be a documented residual orchestration duration, not a claimed
precise stage. CLI benchmark code owns load/write outer timings.

### Correction: M11 provenance recheck

The preceding M11 asset-hash rejection is superseded. Its independent command imported the sibling
working Python checkout through `source_me.sh`, not the certified `55e5f368777f7809fe2e91b5d070caf6df0cb581`
source. The detached-worktree receipt at `tests/_temp/html_selftest_asset_receipt.json` identifies
the certified commit and literal `{{CRC}}` input. Current Rust asset hashes match that receipt:
`control_styles.css` `67987f47...`, `drag_controls.js` `fb50326c...`,
`match_controls.js` `11b371a3...`, and `order_controls.js` `cd29cfef...`.

The four-asset provenance requirement is therefore accepted. M11's remaining acceptance condition
is the recorded browser proof after the workspace compiles: each of seven kinds has correct and
incorrect grading, with MATCH/ORDER mouse, keyboard, touch, and reset paths. The existing ignored
Rust page generator alone does not satisfy that evidence condition; the claimed Playwright hash and
interaction gate must run from its stated certified input and leave a receipt. This correction is
required by the repository evidence rule: the frozen authority, rather than an accidentally imported
sibling checkout, controls the parity finding.

## M13 Canvas QTI 1.2 MATCH validity repair

**Accepted as a plan-priority repair, with the frozen defect retained visibly.** In the certified
Python Canvas writer, `create_matching_response_lid` declares each MATCH response label as
`response_<prompt>_choice_<choice>`, while `create_MATCH_resprocessing` emits the unattested
`choice_<prompt>` token. The expanded supported-kind fixture consequently yields two dangling
`varequal` references, one per prompt. This violates the established QTI integrity contract that
scored answer references name a response label in the same item. That contract governs LMS-import
semantics and outranks tier-3 frozen Python output.

Rust must emit valid MATCH response identifiers and corresponding `varequal` values, preserving
prompt/choice positional semantics. The M13 semantic projection must dereference every answer
identifier and fail on an unknown identifier; it must not silently omit unknown values. Preserve the
Python invalid-package result as an explicit, labelled defect receipt with its two integrity
findings. The normal strong parity lane compares Rust's repaired semantic projection to the
fixture's intended item semantics, then requires `qti-integrity` zero findings for Rust. Reports
must call this a Canvas MATCH validity repair, not an exact XML-parity pass.

### Correction: M13 Canvas defect is MULTI_FIB, not MATCH

The preceding Canvas MATCH attribution is superseded by the isolated certified packages and
integrity receipt. Frozen Canvas MATCH is valid. Frozen Canvas MULTI_FIB declares labels as
`response_<blank>_choice_<answer>` but scores raw answer text; its two blank fixture therefore
emits exactly two `qti12-dangling-varequal` findings. Current Rust has the same raw-answer
serialization defect.

Rust repairs only MULTI_FIB by mapping every acceptable answer, in its sorted blank and source
answer order, to that response's declared label identifier. Its semantic projection dereferences
those identifiers back to authored answer text and `qti-integrity` must report zero findings. The
frozen Python MULTI_FIB result remains a labelled two-finding source-defect receipt. MATCH returns
to the normal exact semantic XML parity lane. The projection continues to fail unknown identifiers;
it must not suppress this evidence.

## M11 final browser evidence

**M11 accepted.** `tests/_temp/html_selftest_asset_receipt.json` records the four certified 55e
asset hashes and `tests/_temp/html_selftest_browser_receipt.json` records a passed Playwright run
from the generated standalone documents. It proves wrong and correct grading for MC, MA, FIB, NUM,
MULTI_FIB, MATCH, and ORDER, including FIB punctuation rejection; it also exercises MATCH mouse,
keyboard Escape, touch, and reset, plus ORDER keyboard ArrowUp, touch, reset, and reveal. The
receipt names the preserved item-root, result, and control selectors. This meets the source asset,
DOM contract, and behavioral evidence requirements without broadening the control design.

### Correction: Canvas MULTI_FIB interaction-preserving repair

The preceding statement that current Rust shares the dangling-reference defect is incorrect.
Current Rust uses `response_str` with `render_fib`, so raw answer text is valid and
`qti-integrity` correctly reports no dangling label. It is nevertheless not the required final
repair: frozen Canvas MULTI_FIB presents a `response_lid` and `render_choice` labels, so changing
it to free-text entry changes the learner's input interaction.

Rust must preserve the frozen presentation shape: for each sorted blank, emit `response_lid`,
`render_choice`, and one `response_<blank>_choice_<answer>` label for each accepted answer in
source value order. Its `varequal` entries use those declared label IDs. This is the smallest
validity repair because presentation, available choices, and scoring structure stay faithful while
only the malformed Python score token changes. The semantic projection dereferences the repaired
IDs to authored answer text; integrity remains zero. No assertion that current Rust has dangling
references is permitted.
