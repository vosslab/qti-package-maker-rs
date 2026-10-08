# Execution ledger

This file is the active execution copy of the full handoff plan. The authority is
[read-native-json-converter-handoff-spec-scalable-meerkat.md](../read-native-json-converter-handoff-spec-scalable-meerkat.md).
Milestone wording, dependencies, and baseline criteria below follow that specification. Approved
execution amendments are recorded in this copy and supersede conflicting baseline wording; the
authority file remains unchanged. The settled decisions must also be recorded in
`docs/DESIGN_DECISIONS.md` by the docs owner.

## Delegated workflow

The QPM manager owns coordination, status updates, integration, and milestone decisions. Work is
split into the milestone owners below. Fresh independent reviewers assess the review gates defined
in the plan; the manager records outcomes and evidence here. Agents should raise consequential
uncertainty to the manager, state the evidence and affected milestone, and keep the question open
until the manager decides and affected owners apply and verify the decision.

| Work item | Milestone | Owner | Status | Review and evidence |
| --- | --- | --- | --- | --- |
| Source model | M1 | `source_model` | Complete | Included in the 16 focused normal-crate tests; strict all-target Clippy and `cargo fmt --check` passed. Fresh spec and quality reviews accepted. |
| Kind mapping | M2-M4 | `mapping_spec`, `mapping_quality` | Complete | Included in the 16 focused normal-crate tests. Fresh spec and quality reviews accepted. |
| Lossy conversion scan | M5 | `lossy_scan` | Complete | URL ruling D3 applied; scanner and source focused tests passed. Fresh spec and quality reviews accepted. |
| In-memory export | M6 | `mapping_spec`, `mapping_quality` | Complete | Included in the 16 focused normal-crate tests. Fresh spec and quality reviews accepted. |
| Associated files and external inventory | M7-M8 | `media_export`, scanner owner | Complete | Per-question files and inventory implemented; seven media tests passed, including the entity-decoded filename regression. D5-D6 corrections and independent spec/quality reviews accepted. |
| Directory writer | M9 | QPM manager | Complete | D7 ownership proof and D8 staged exact-path/hash verification implemented. The filesystem-aware `staged_media_names_must_represent_distinct_files_before_publish` regression failed before D8 and passes now; 32 native tests, strict Clippy, and formatting passed. `stage_respec` and `stage_requality` accepted D8. |
| Registration and CLI | M10 | QPM manager | Complete | Registered as writer 11 with no reader and included in `--html-to-image`. Workspace tests, strict Clippy, and formatting passed before D6/D7. Spec and quality reviews accepted; wording now says supported formats derive from the registered list, and one focused CLI test passed. |
| Repository docs | M11 | `repository_docs` | Complete | Product docs, operational PLE import and API boundaries, roadmap, and checklist were corrected against source. `FORMATS.md` now says `--all` selects eleven writers. `docs_spec`, quality, and final integration reviews accepted. |
| PLE issues report | M12 | `ple_issues` | Complete | D1/D2 accepted and D3 inclusion verified. `docs_spec` and quality review accepted the source-bound report. |
| Contract smoke harness | M13 | `smoke_harness` | Complete | `harness_correction`, `harness_respec`, and `harness_requality` accepted. Valid input exits 0; missing files exit 1; HTML-like answers yield zero references; `..` and symlink paths exit 2 with input unchanged, verified independently. |
| Smoke run | M14 | QPM manager | Complete | Final CLI SHA-256 `0475e805b75186ffe9dea7e91678105b27c5cb394d1b306cee3c585a8713eeee`: 373 attempts, 359 successful outputs and decoder receipts, 708/708 JSON parse-and-compile passes, all seven kinds. Independent classification of 14 QPM capability-gap rejections accepted; see `output_ple_native_json/m14_classification.md`. |
| Final review | M15 | QPM manager | Complete | `final_spec`, `final_quality`, and fresh `final_integration_acceptance` report no unresolved implementation or plan blockers. Final Rust gates, 1,514-test Python suite, archive hash verification, and temporary-source cleanup pass. |

## Decision and question ledger

| Question or decision | Owner | Status | Evidence and next action |
| --- | --- | --- | --- |
| URL inventory and lossy URL handling (D3) | QPM manager | Closed | Reject protocol-relative `//` URLs as malformed PLE input. Reject one URL assigned conflicting resource kinds (first-kind classification would otherwise lose the image predicate); deduplicate repeats of the same URL and kind. Scanner and source focused tests passed. Docs owner records the ruling. |
| Media collection boundary (D5) | QPM manager | Closed | Whole-bank collection/rewrite treats FIB/MULTI_FIB grading literals as media, causing false missing-asset errors or answer mutation. M7 uses shared `qti-core` media primitives on mapped display fields only, with no `qti-core` changes or surrogate bank. Implementation and seven media tests complete. |
| Entity-decoded media paths (D6) | QPM manager | Closed | Every final mapped display `img` source must identify exactly one supplied file or return `InvalidFormat` with item, source, and cause. The literal-entity regression passes; spec and quality reviews accepted. |
| Output ownership proof (D7) | QPM manager | Closed | `.qpm-ple-native-json` records SHA-256 and exact generated file set. Nonempty replacement requires valid, unmodified proof; foreign or modified contents are refused, and empty destinations are allowed. Ownership tests and spec/quality reviews accepted. |
| Staged output integrity (D8) | QPM manager | Closed | Case-insensitive staging can collapse `media/A.png` and `media/a.png` to one physical file. Before publish, the writer verifies exact staged manifest paths and hashes. Its filesystem-aware regression failed before the fix and passes now; `stage_respec` and `stage_requality` accepted. |
| PLE FIB grading and maxLength issues (D1-D2) | `ple_issues` | Closed | D1 records preservation of current-correct FIB grading behavior under normalization. D2 records that PLE's required `maxLength` is student-visible. Both accepted; D3 inclusion verified by `docs_spec`. |
| CLI directory naming (D4) | QPM manager | Closed | Use `ple-<content_name>/`; when no output directory is supplied, use `ple/`. Implement with M9-M10. |
| Harness HTML false references and path aliases | `harness_correction` | Closed | Accepted correction/respec/requality. Valid input exits 0, missing files exit 1, HTML-like answer text produces zero file references, and `..` plus symlink paths exit 2 without changing input. |
| Final workspace gate | QPM manager | Closed | Final-tree `cargo test --workspace`, strict all-target Clippy, `cargo fmt --check`, and `source source_me.sh && pytest tests/` pass. Logs in `output_ple_native_json/gates/`; full Python suite: 1,514 passed, seven existing near-limit advisories. |
| Baseline Python failures and documentation checks | QPM manager | Closed | Full final Python suite passes 1,514 tests with seven existing near-limit advisories. Changelog rotation, Graphify link corrections, and documentation updates accepted by final integration review. |

## Execution status

Update milestone status only with owner and evidence. Record review findings, corrections, smoke
classifications, and receipts here as work proceeds. Completion requires M1-M15 criteria as amended
here and final gates from the authority.

Implementation ownership ruling: `mod.rs` handles routing only. Behavior belongs in `export.rs`
and `writer.rs`, consistent with Rust style; preserve the existing dependency interfaces. No Git
disposition changed.

## Approved execution amendments

These decisions are approved for this execution copy and supersede conflicting details below and
in the authority plan. The authority file is unchanged.

- D3 (closed): reject protocol-relative `//` external URLs as malformed PLE input; reject a URL
  repeated with conflicting resource kinds; deduplicate a URL repeated with the same kind. This
  preserves each resource predicate instead of silently retaining whichever kind appears first.
- D4 (closed): the output directory is `ple-<content_name>/`; the default output directory is
  `ple/`.
- D5 (closed): whole-bank media collection and rewriting can mistake
  FIB/MULTI_FIB grading literals for display media, causing false missing-asset errors or answer
  mutation. M7 applies the shared `qti-core` media primitives only to mapped display fields. Keep
  `qti-core` unchanged and do not build a surrogate bank. Seven media tests passed.
- D6 (closed; spec and quality accepted): `qti-core` may output an
  entity-like filename whose HTML-decoded display path differs from the actual file bytes. Each
  final mapped display `img` source must semantically identify exactly one supplied file. Otherwise
  return `EngineError::InvalidFormat` naming the item, source, and cause. Do not change `qti-core`;
  record the capability gap. The literal-entity regression passes.
- D7 (closed; spec and quality accepted): valid foreign PLE JSON can be
  overwritten, and JSON schema alone cannot prove ownership. A directory marker named
  `.qpm-ple-native-json` records the SHA256 and exact file set under
  `qti-package-maker-rs/ple_native_json/v1`. A nonempty destination is replaceable only with a
  valid, unmodified marker; foreign or modified contents are refused, while an empty destination
  is allowed. Keep staged-backup revalidation and rollback. The in-memory API is unchanged. The
  `ownership_fix` restated this rule and `fidelity_ownership_spec` accepted it.
- D8 (closed; `stage_respec` and `stage_requality` accepted):
  case-insensitive filesystems can collapse distinct media paths such as `media/A.png` and
  `media/a.png` during
  staging. Before publishing, validate the staged tree against exact manifest paths and hashes,
  preserving in-memory output names. The pre-fix regression failed and the post-fix suite of 32
  native tests plus strict Clippy and formatting passed.
- File ownership: `mod.rs` routes only; `export.rs` and `writer.rs` own export and write behavior.

---

# Plan: PLE Native JSON writer for qti-package-maker-rs

## Context

PLE (peptidyle-learning-engine) will import Questions directly from qti-package-maker-rs (QPM) as
its existing, private, unversioned `pleQuestionJson` source plus the files that source references.
PLE's Human Guidance says PLE uses QPM as an external library for all conversion and prefers
assets alongside text-only JSON over a ZIP package. The handoff inputs are the temporary root files
`NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md` and `NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md`; they are
reference material only.

The handoff assigns QPM three converter milestones: a non-media writer for seven response kinds, a
contract smoke through PLE's decoder, and associated-file emission. PLE owns import binding, asset
IDs and checksums, HTML rendering, presentation, and HOTSPOT surface binding.

Human direction (2026-10-07):

- QPM supplies canonical content and correct-answer/grading information. PLE owns student
  presentation, including any shuffling. QPM adds no presentation permutations.
- Ordinary FIB answers ignore capitalization (DNA = dNa = DnA) and incidental whitespace. Target
  that intended behavior; PLE can simplify its matching model separately.
- The handoff files are task inputs. Stay on the writer; Git disposition of those files is outside
  this task except for moves the implementation requires.
- Record PLE problems in a QPM-discovered PLE issues document: discover, record, keep going. It is
  not part of the QPM implementation spec.
- Media scope follows QPM's existing capabilities: reuse the table-to-image and media pipeline for
  assets QPM already produces. Inline images QPM cannot currently represent are a separate QPM
  capability gap, recorded rather than added here.
- The plan completes with manager and subagents only. Independent agent assessments and automated
  tests replace human gates. Prefer more, smaller milestones.

An external LLM plan review (forwarded source material, not a human requirement) raised six
corrections; this revision adopts them: explicit validation boundary, FIB `normalized` evidence,
`externalResources` coverage, QPM-side lossy-conversion detection, kind coverage in the smoke,
and evidence-driven PLE issue entries.

## Objectives

- Emit valid `pleQuestionJson` for MC, MA, FIB, MULTI_FIB, NUM, MATCH, and ORDER items, with stable
  response IDs and unchanged answer meaning.
- Preserve authored HTML and inline CSS and supply each referenced file's bytes at the relative
  path the JSON names.
- Fail loudly on conversions QPM cannot represent faithfully, and leave PLE's contract limits to
  PLE's decoder.
- Offer the output as an in-memory library export for PLE and as a CLI directory (JSON plus
  `media/`), with no ZIP.
- Prove decoder compatibility: every emitted JSON across all seven kinds and the associated-file
  path passes PLE `parse` and `compile`, or each failure carries an independent classification.

## Design philosophy

- Validation boundary. QPM owns its input model (existing `validate_item`), mapping integrity
  (every emitted answer reference resolves; nothing is silently dropped), media references (each
  `src` names a supplied file), and lossy-conversion detection. PLE's decoder is the authority on
  whether emitted JSON meets PLE's contract (counts, lengths, size). QPM neither emits content it
  knows is lossy nor re-implements PLE limits. Rejected alternative: a QPM copy of PLE's limit
  checks, which would drift against an unversioned private format.
- Fix the design, not the symptom: PLE presentation and matching-model issues go to the PLE issues
  document instead of QPM workarounds.
- KISS: one fixed mapping per kind, no new CLI flags or engine options; reuse `render_bank`,
  existing media collection, policy, rewrite, and `--html-to-image` conversion.
- Atomic task decomposition: small milestones, each with one outcome and one automated check.
- Evidence strategy: the cross-repo smoke on regenerated corpus inputs settles residual mapping
  questions; an independent reviewer agent rules on any evidence that contradicts a resolved
  decision.

## Scope

- Add a write-only `ple_native_json` engine in `qti-engines`: serde source structs, seven-kind
  mapping, lossy-conversion checks, public in-memory export, associated files,
  `externalResources` inventory, and a staged directory writer.
- Register the engine, add CLI output naming, and include it in `HTML_TO_IMAGE_ENGINES`.
- Update engine-authoring checklist docs, design decisions, human guidance, and the changelog.
- Create and maintain the PLE issues document.
- Run a temporary cross-repo contract smoke against PLE's `adapter_ple` decoder.
- Close with an independent review agent and the repository's automated gates.

## Non-goals

- Implement PLE import binding, asset hashing or storage, metadata persistence, HTML rendering, or
  rendered import smoke (handoff milestones 4 and 5 belong to PLE).
- Emit HOTSPOT: QPM has no hotspot item kind or input format.
- Permute, shuffle, or rename IDs to hide ORDER or MATCH answers.
- Re-implement PLE count, length, or size limits in QPM.
- Add general inline-image ingestion (for example data-URI materialization) or map RDKit canvases
  to PLE `authorScript`.
- Emit `questionHint`, outcome or per-choice feedback, `authorScript`, `randomizeChoices`, or
  `unit`; the QPM item model carries none of them.
- Add Python parity lanes; Python qti-package-maker has no counterpart engine.
- Modify the PLE repository or the Git state of the handoff files.

## Current state summary

- Engine contract: `Writer` (`crates/qti-engines/src/traits.rs:11-33`) with
  `save_package(&ItemBank, Option<&Path>) -> Result<WriteOutcome, EngineError>`; `render_bank` and
  pre/post hooks (traits.rs:79-126). Trait boundary is frozen
  (`docs/active_plans/decisions/engine_trait_ruling.md`); every writer tests its zero-render case.
  `EngineError::InvalidFormat` carries serializer and format errors (`exam_yaml/mod.rs:123-128`).
- Registry: `ENGINES` (`crates/qti-engines/src/registry.rs:66-127`); registry tests hard-code ten
  writers (160-210). Graphify `affected` on `registry.rs`, verified in source: `--all`
  (`crates/qti-cli/src/app.rs:289-292`) and the `engines` listing (app.rs:248-260) pick up new
  entries automatically; `output_name` (app.rs:418-433) ends in `unreachable!`;
  `HTML_TO_IMAGE_ENGINES` (app.rs:15-19) and its error text (app.rs:360-369) name ZIP engines;
  `cargo xtask parity` uses its own list (`xtask/src/parity.rs:25-41`). CLI contract test
  (`crates/qti-cli/tests/cli_contract.rs:80-111`) expects `saved 10 of 10` and file artifacts.
- Item model (`crates/qti-core/src/item.rs:24-66`): seven bodies, no choice IDs, no hotspot.
  MULTI_FIB keys are a sorted `BTreeMap`, each present as `[key]` in the stem. NUM has absolute
  tolerance and a `tolerance_message` flag that display writers render as student-visible text
  (`html_selftest/mod.rs:229-233`, `human_readable/mod.rs:199-203`). QPM minimums already match
  PLE minimums (`crates/qti-core/src/validate.rs:62-143`).
- Media: `ItemBank::collect_assets` and `dependencies_for` (`crates/qti-core/src/bank.rs:343-408`),
  `rewrite_item_media` (`media/rewrite.rs:32-39`), `apply_media_policy` (`media/policy.rs:59-129`;
  `Package` keeps and warns on external URLs, rejects data URIs, packages SVG with a warning),
  `assign_output_names` (`media/naming.rs`). Loose-file write order precedent: `text2qti/mod.rs:221-351`.
- HTML-to-image: RDKit canvases appear as `<canvas>` plus `<script>` in item HTML
  (`html_to_image/selectors.rs:84-126`). `convert_bank` returns a bank with a temporary
  `MediaBaseDir` holding generated PNGs (`html_to_image/convert.rs:224-289`), so existing media
  collection handles them.
- PLE contract (`peptidyle-learning-engine/crates/adapters/ple`): crate `adapter_ple`;
  `PleQuestionJsonDocument::parse(&[u8])` then `compile()` (regex checked only in `compile`).
  `deny_unknown_fields` throughout; IDs `^[a-z][a-z0-9_-]*$`, 1-64 bytes; present strings
  non-blank; raw size <= 256 KiB. MULTI_FIB blanks render as labelled inputs under the prompt in
  array order. FIB `maxLength` is required (1-16,384) and shown as "Up to N characters"
  (`src/components/question_response_controls/short_text.tsx:41-55`). `externalResources` holds
  absolute `https` URLs with kind `link`, `image`, `script`, `stylesheet`, or `other`; PLE HG says
  recorded URLs include links, images, scripts, and stylesheets. `normalized` matching trims,
  collapses internal whitespace runs, and lowercases (`crates/grading/src/ple_question_json_validate.rs:350-356`);
  `caseInsensitive` compares ASCII case without trimming (:321-323). PLE renders HTML as text today
  and presents ORDER items and MATCH choices in authored order
  (`crates/question_model/src/presentation/builder_items.rs:59-75`).
- Corpus inputs are not on disk. `cargo xtask parity --fixtures` regenerates deterministic inputs,
  including media-local fixtures (`xtask/src/parity.rs:500-548`); `cargo xtask table-corpus`
  harvests table and canvas inputs from `../biology-problems`.

## Architecture boundaries and ownership

- `ple_native_json` engine module: `mod.rs` routes and re-exports; `export.rs` owns mapping,
  lossy checks, per-question file sets, inventory, and in-memory export; `writer.rs` owns directory
  writing and the `Writer` implementation. No `qti-core` changes.
- `qti-core`: unchanged; media classification, naming, and rewrite stay authoritative there.
- `qti-cli`: output naming, conversion-engine list, artifact reporting.
- PLE decoder: contract authority, exercised only by the temporary smoke.

### Mapping (milestones -> components)

| Milestone | Component | Review boundary |
| --- | --- | --- |
| M1 | `source.rs` | Serde shape vs PLE members |
| M2-M4 | `mapping.rs` | Per-kind answer meaning and IDs |
| M5 | `mapping.rs` checks | Lossy-conversion boundary |
| M6-M8 | `export.rs` | Library API, references, bytes, inventory |
| M9 | `writer.rs` | Staged directory write |
| M10 | Registry and CLI | Registration and CLI artifact |
| M11-M12 | Docs | Checklist docs; PLE issues document |
| M13-M14 | `tests/_temp/ple_contract_smoke/` | PLE `parse` + `compile`, coverage |
| M15 | Whole change | Independent final review |

### File scope

| File or directory | Intended change | Canonical source, for generated output |
| --- | --- | --- |
| `crates/qti-engines/src/ple_native_json/mod.rs` (new) | Module routing and public re-exports | |
| `crates/qti-engines/src/ple_native_json/source.rs` (new) | Serde structs: camelCase, `kind`-tagged response and tolerance, absent members skipped | |
| `crates/qti-engines/src/ple_native_json/mapping.rs` (new) | Seven-kind mapping, IDs, lossy checks | |
| `crates/qti-engines/src/ple_native_json/export.rs` (new) | Public export, mapped display-field media rewrite, per-question files, inventory | |
| `crates/qti-engines/src/ple_native_json/writer.rs` (new) | `Writer` impl, factory, staged directory write and replace rule | |
| `crates/qti-engines/src/lib.rs` | Declare module | |
| `crates/qti-engines/src/registry.rs` | `ENGINES` entry (`MediaPolicy::Package`, no reader); test counts | |
| `crates/qti-cli/src/app.rs` | `output_name` arm, `HTML_TO_IMAGE_ENGINES` entry, error text, name-list test | |
| `crates/qti-cli/tests/cli_contract.rs` | Eleven writers; directory artifact | |
| `docs/FORMATS.md`, `docs/ENGINES.md`, `docs/MEDIA.md`, `docs/USAGE.md` | Format entry, counts, media row, CLI example | |
| `README.md`, `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md` | Writer counts or lists, where enumerated | |
| `docs/DESIGN_DECISIONS.md` | Authority, boundary, and mapping decisions | |
| `docs/HUMAN_GUIDANCE.md` | Human directions in Context | |
| `docs/CHANGELOG.md` | Day entry; QPM capability gaps found by the smoke | |
| `docs/active_plans/reports/ple_issues_from_native_json_writer.md` (new) | PLE issues document | |
| `docs/active_plans/active/ple_native_json_writer_plan.md` (new) | Execution copy with milestone status | This plan |
| `tests/_temp/ple_contract_smoke/` (untracked) | Harness, fixtures, receipts | |

Split engine sources by responsibility within the repository's per-file line limit.

## Resolved decisions

- Output: compact JSON (PLE's size limit counts whitespace) with `format`, `prompt`, `response`,
  and `externalResources` only when non-empty. Absent optional members are omitted, never `null`.
- IDs: positional per collection in canonical source order: `choice-N`, `prompt-N`, `item-N`,
  `blank-N`.
- MC `singleChoice`: authored choice order; `correctChoice` by position lookup as in
  `canvas_qti_v1_2/mod.rs:263-270`.
- MA `multipleAnswer`: same choices; `correctChoices` by position. `min_answers_required` and
  `allow_all_correct` constrain QPM authoring validation only, so omitting them loses no grading
  or display content.
- FIB `fillIn`: accepted answers in order, exact duplicates removed (PLE requires uniqueness; no
  meaning lost). `matchMode: "normalized"`: it is the only PLE mode that ignores capitalization
  and incidental whitespace, the human direction. Its extra effect, collapsing internal whitespace
  runs, never joins or splits words ("A B" stays distinct from "AB") and falls within "incidental
  whitespace". One constant holds the mode. `maxLength: 16384`, fixed: BBQ sets no limit, and an
  answer-derived value would print an answer-length hint to students.
- MULTI_FIB `multiFillIn`: one blank per key, ordered by first `[key]` appearance in the stem.
  Prompt keeps its `[key]` markers; `label` is the escaped `[key]` text. Answers, mode, and
  `maxLength` as FIB.
- NUM `numeric`: `answer` as JSON number; `tolerance: {"kind":"absolute","epsilon":<tolerance>}`.
  When `tolerance_message` is true, append `<p>Answer must be within &plusmn;{tolerance}.</p>` to
  the prompt, matching `html_selftest`, so the student-visible note survives.
- MATCH `matching`: canonical prompt and choice order; `matches` pair `prompt-i` with `choice-i`;
  extra choices remain distractors.
- ORDER `ordering`: items in canonical correct order; `correctOrder` lists `item-1..item-N`.
- Lossy-conversion errors (QPM-owned, `EngineError::InvalidFormat` naming item and cause):
  display HTML containing `<script>`, which PLE display content never executes (the existing
  `--html-to-image` path or library `convert_bank` turns RDKit canvases into PNGs); absolute
  non-`https` and protocol-relative `//` URLs in scanned attributes; and one URL assigned
  conflicting resource kinds. Same-URL/same-kind entries are deduplicated. Data URIs keep the
  existing `Package` rejection. Missing local media keep the existing collection error.
- Media: apply shared `qti-core` collection, rewrite, and naming primitives only to mapped display
  fields. Local and generated images rewrite to `media/<output_name>`; each question carries the
  files its own display dependencies name. Do not collect or rewrite the source bank because
  FIB/MULTI_FIB grading literals can resemble media paths.
- `externalResources`: absolute `https` URLs from `img[src]` (`image`), `a[href]` (`link`), and
  `link[href]` (`stylesheet`) in represented display HTML. Deduplicate same-URL/same-kind entries;
  reject same-URL/conflicting-kind entries.
- Contract limits (counts, lengths, size): PLE's decoder reports them; the smoke classifies any hit.
- CLI artifact: directory `ple-<content_name>/` by default, or `<prefix>-<content_name>/` with an
  explicit prefix, containing `item_NNNNN.json` and `media/`. Stage in
  a sibling temporary directory, then replace the destination; refuse a destination holding files
  this engine does not write.
- Library export: public function returning per-question `{item_number, crc, source_json, files:
  [{path, bytes}]}` plus warnings; `save_package` calls it.
- Conversion: the engine joins `HTML_TO_IMAGE_ENGINES` so `--html-to-image` reuses the existing
  table and canvas conversion for PLE output.

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Source structs | Serde model of the emitted `pleQuestionJson` subset | Serialized shape matches PLE members |
| M2 | Choice kinds | MC and MA mapping | Correct IDs and answer references |
| M3 | Text-entry kinds | FIB and MULTI_FIB mapping | Mode, length, blank order, labels |
| M4 | Numeric, match, order | NUM, MATCH, ORDER mapping | Tolerance, note, pairs, order |
| M5 | Lossy-conversion checks | Script, non-`https`, and protocol-relative URL errors; conflicting inventory roles | QPM-side faithful-conversion boundary |
| M6 | In-memory export | `render_bank` export without media | Library API and zero-render case |
| M7 | Associated files | Rewrite mapped display fields to `media/`, per-question bytes | Each display reference names one supplied file; grading literals stay unchanged |
| M8 | External inventory | `externalResources` from scanned attributes | `https` URLs recorded per question |
| M9 | Directory writer | `writer.rs` `Writer` impl with staged replace | Safe CLI artifact; default directory is `ple/` |
| M10 | Registration and CLI | Registry, output name, conversion list | Engine usable from CLI |
| M11 | Repository docs | Checklist docs, decisions, guidance, changelog | Docs match behavior |
| M12 | PLE issues document | Seed and evidence-checked entries | PLE defects recorded with evidence |
| M13 | Smoke harness | Temporary PLE decoder harness | Directory in, receipt out |
| M14 | Smoke run | Regenerated corpus through harness | Full-kind decoder evidence |
| M15 | Final review | Independent review, gates, cleanup | Plan closed |

### Milestone: M1 source structs

- Depends on: none.
- Deliverables: `source.rs` with top-level, seven response kinds, tolerance, external resource.
- Entry criteria: none.
- Exit criteria: unit test serializes the handoff's single-choice example structs and compares to
  the expected `serde_json::Value`; strict Clippy passes.
- Parallel-plan ready: yes, with M12 and M13.

### Milestone: M2 choice kinds

- Depends on: M1 (struct names).
- Deliverables: MC and MA mapping functions.
- Entry criteria: M1 exit met.
- Exit criteria: unit test covers MC and MA, including an MA with several correct choices; every
  correct reference resolves to an emitted ID matching PLE's ID syntax.
- Parallel-plan ready: yes, with M3 and M4.

### Milestone: M3 text-entry kinds

- Depends on: M1.
- Deliverables: FIB and MULTI_FIB mapping with `normalized`, fixed `maxLength`, exact-duplicate
  removal, first-appearance blank order, `[key]` labels.
- Entry criteria: M1 exit met.
- Exit criteria: unit test covers a MULTI_FIB whose sorted key order differs from stem order and a
  FIB with an exact duplicate answer.
- Parallel-plan ready: yes, with M2 and M4.

### Milestone: M4 numeric, match, order

- Depends on: M1.
- Deliverables: NUM (absolute tolerance, tolerance note), MATCH (pairs, distractors), ORDER.
- Entry criteria: M1 exit met.
- Exit criteria: unit test covers NUM with and without `tolerance_message`, MATCH with a
  distractor, and ORDER; `correctOrder` is a permutation of emitted IDs.
- Parallel-plan ready: yes, with M2 and M3.

### Milestone: M5 lossy-conversion checks

- Depends on: M2-M4 (checks run over mapped display fields).
- Deliverables: single HTML scan per display field that reports `<script>`, non-`https`, and
  protocol-relative URLs in scanned attributes as `InvalidFormat` errors; the M8 inventory reuses
  this scan and rejects conflicting resource kinds for one URL.
- Entry criteria: M2-M4 exit met.
- Exit criteria: focused scanner/source tests cover script-bearing content, non-`https` and
  protocol-relative URLs, conflicting URL kinds, and same-kind deduplication. Independent review
  confirms the checks match the validation boundary and add no PLE limit checks. Complete: scanner
  and source tests passed; fresh spec and quality reviews accepted.
- Parallel-plan ready: no, it integrates M2-M4.

### Milestone: M6 in-memory export

- Depends on: M5.
- Deliverables: public export over `render_bank` returning per-question results and warnings.
- Entry criteria: M5 exit met.
- Exit criteria: unit test exports a seven-kind bank, decodes each JSON as `serde_json::Value`, and
  checks `format` and `kind`; zero-render case returns no questions.
- Status: complete. M1-M6 pass 16 focused normal-crate tests; strict all-target Clippy and
  `cargo fmt --check` pass. Fresh spec and quality reviews accepted.
- Parallel-plan ready: no.

### Milestone: M7 associated files

- Depends on: M6.
- Deliverables: shared `qti-core` media primitives on mapped display fields only, `Package` policy,
  rewrite to `media/<output_name>`, and per-question file sets with bytes read once. Do not pass the
  source bank through collection/rewrite: D5 records the demonstrated FIB/MULTI_FIB grading-literal
  corruption. No `qti-core` changes or surrogate bank.
- Entry criteria: M6 exit met.
- Exit criteria: unit test with a local image shared by two items confirms each relative `src`
  equals one file path in that question's set with identical bytes; FIB/MULTI_FIB grading literals
  remain unchanged. D6 also requires each final mapped display `img` source to semantically
  identify exactly one supplied file, otherwise `InvalidFormat` names item, source, and cause.
  Complete: seven media tests pass; `media_fidelity_fix` restated the rule, and independent spec
  and quality reviews accepted it.
- Parallel-plan ready: no, it extends the M6 pre-render hook.

### Milestone: M8 external inventory

- Depends on: M5 (shared scan), M6 (per-question result).
- Deliverables: `externalResources` population for `img`, `a`, and `link` `https` URLs.
- Entry criteria: M5 and M6 exit met.
- Exit criteria: tests show `https` image and link kinds, same-kind deduplication, and rejection of
  conflicting kinds. Complete with M7; per-question file and inventory integration is implemented
  and covered by the seven media tests.
- Parallel-plan ready: yes, with M7.

### Milestone: M9 directory writer

- Depends on: M7, M8.
- Deliverables: `writer.rs` staged write and replace rule; `Writer` impl and factory. The default
  directory is `ple/`; explicit output uses `ple-<content_name>/`.
- Entry criteria: M7 and M8 exit met.
- Exit criteria: unit tests show a rerun replaces a prior output directory, a directory with a
  foreign file is refused unchanged, and written JSON plus `media/` match the export. D7 requires
  `.qpm-ple-native-json` proof before replacing a nonempty output; foreign or modified outputs are
  refused, empty destinations are allowed, and staged-backup revalidation and rollback remain.
  D8 staged-tree revalidation is implemented and its alias regression passed as part of 32 native
  tests, strict Clippy, and formatting. `stage_respec` and `stage_requality` accepted D8.
- Parallel-plan ready: no.

### Milestone: M10 registration and CLI

- Depends on: M9.
- Deliverables: registry entry and tests; `output_name` arm; `HTML_TO_IMAGE_ENGINES` entry and
  error text; CLI contract test update.
- Entry criteria: M9 exit met.
- Exit criteria: `cargo test --workspace`, strict all-target Clippy, and `cargo fmt --check` pass;
  CLI `--all` selects eleven writers including the directory. Supported formats derive from the
  registered engine list. Workspace gates passed before D6/D7; after the wording correction, one
  focused CLI test passed.
- Parallel-plan ready: yes, with M11.

### Milestone: M11 repository docs

- Depends on: M5 (decisions fixed); final wording after M10.
- Deliverables: all docs in File scope, including D3-D5 and current milestone evidence.
- Entry criteria: M5 exit met.
- Exit criteria: `source source_me.sh && pytest tests/` passes (links, ASCII, line limits);
  independent docs reviewer agent confirms docs match implemented behavior.
- Status: complete. `docs_spec`, quality, and final integration review accepted the source-bound
  product docs and PLE report. `FORMATS.md` accurately describes `--all` as writer selection.
  Full final Python suite passes; unavailable Graphify artifacts are recorded as unavailable.
- Parallel-plan ready: yes, with M10.

### Milestone: M12 PLE issues document

- Depends on: none.
- Deliverables: document per "PLE issues document" below.
- Entry criteria: none.
- Exit criteria: every entry cites PLE file:line evidence and the QPM information showing QPM can
  express the Question correctly; independent reviewer agent confirms each entry meets the
  document's inclusion rule; Markdown checks pass.
- Status: complete. D1 and D2 accepted; D3 inclusion verified by `docs_spec`. Quality review
  accepted the source-bound report.
- Parallel-plan ready: yes, with M1-M11.

### Milestone: M13 smoke harness

- Depends on: none.
- Deliverables: `tests/_temp/ple_contract_smoke/` cargo binary with its own `[workspace]` table and
  a path dependency on `adapter_ple` only. Input: an emitted directory. For each JSON it runs
  `parse` and `compile`, checks each relative `src` resolves to exactly one supplied file, and
  writes a receipt with per-kind counts, associated-file and inventory counts, and each failure.
- Entry criteria: none.
- Exit criteria: harness builds and passes on a hand-written directory holding the handoff example
  and its image, and reports a failure for a deliberately broken copy.
- Status: complete. `harness_correction`, `harness_respec`, and `harness_requality` accepted. Valid
  input exits 0, missing files exit 1, HTML-like answers create zero references, and `..` plus
  symlink paths exit 2 with input unchanged, independently verified.
- Parallel-plan ready: yes, with M1-M12.

### Milestone: M14 smoke run

- Depends on: M10, M13.
- Deliverables: regenerated inputs (`cargo xtask parity --fixtures`; `cargo xtask table-corpus`),
  CLI runs with `-f ple_native_json` in plain mode and with `--html-to-image` for table and canvas
  inputs, harness receipts.
- Entry criteria: M10 and M13 exit met.
- Exit criteria: receipts show at least one passing question for each of the seven kinds and for
  the associated-file path; when a kind or path is missing, a minimal temporary fixture under the
  harness supplies it. Every PLE rejection and QPM lossy error is classified by an independent
  reviewer agent: QPM mapping defect (fix, rerun), content outside PLE limits (recorded in the
  receipt), PLE defect (M12 entry), or QPM capability gap (changelog entry).
- Status: complete. Final binary SHA-256
  `0475e805b75186ffe9dea7e91678105b27c5cb394d1b306cee3c585a8713eeee` ties 373 CLI
  attempts to 359 successful outputs and decoder receipts. All 708 JSON questions pass PLE
  `parse` and `compile`; 447 associated-file uses, 448 display `src` references, and 32 external
  inventory entries are accounted for. All seven kinds pass. The 14 rejected attempts are QPM
  capability gaps: 13 plain-mode script banks pass with the same source bytes in image mode; one
  `data:` image cannot be packaged. No mapping defect, PLE decoder rejection, or PLE limit hit was
  observed. Independent classification: `output_ple_native_json/m14_classification.md`.
  Final 59-bank parity comparison has zero divergences; all 76 regenerated parity inputs match
  the final fixture bytes. Corpus harvest tried 205 generators, yielding 180 BBQ banks; 17
  produced no BBQ and eight failed upstream. The smoke covers the 180 emitted banks, not all 205
  generators. Primary logs and receipts: `output_ple_native_json/verification/`.
- Parallel-plan ready: no.

### Milestone: M15 final review

- Depends on: M1-M14.
- Deliverables: fresh independent reviewer agent audit of the diff against this plan and the
  handoff; fixes for its blocking findings; temporary harness source removed with receipts
  referenced from the changelog.
- Entry criteria: M14 exit met.
- Exit criteria: reviewer reports no unresolved blocking findings; `cargo test --workspace`,
  strict Clippy, `cargo fmt --check`, and `pytest tests/` pass on the final tree.
- Status: complete. `final_spec`, `final_quality`, and fresh `final_integration_acceptance` found
  no unresolved implementation or plan blockers. Final-tree
  workspace tests, strict all-target Clippy, and formatting pass (logs in
  `output_ple_native_json/gates/`). The `output_ple_native_json/verification/` archive holds 1,145
  files (7.3 MB); all `files.sha256` entries verified, including 19 QPM product-source and 11 PLE
  decoder/grading source hashes. Its README records reconstruction and path mapping. Eight
  temporary harness source files and two bytecode files were removed; no Rust/Python source or
  Cargo manifests remain in native scratch or archive. `source source_me.sh && pytest tests/`
  passed 1,514 tests with seven existing near-limit advisories; `git diff --check` passed.
- Parallel-plan ready: no.

## Test and verification strategy

- Permanent tests protect the emitted contract PLE consumes: per-kind mapping tests (M2-M4),
  lossy-conversion tests (M5), export and zero-render (M6), file references (M7), inventory (M8),
  directory replace rule (M9), registry and CLI contract updates (M10). No golden files.
- Temporary checks: M13-M14 harness and fixtures were archived under
  `output_ple_native_json/verification/`, then their runnable source was removed from
  `tests/_temp/`. PLE is not a published dependency, so this cross-repo evidence is retained as
  receipts rather than permanent tests here.
- Blocking gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo fmt --check`, `source source_me.sh && pytest tests/`. Failure response: fix
  the code or doc that failed; never suppress the check.
- Smoke failure response: an independent reviewer classifies each failure as in M14; QPM defects
  loop back to the owning milestone; other classes are recorded and the plan continues.
- Independent agent assessments at M5, M11, M12, M14, and M15 use fresh reviewer subagents.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Unversioned PLE format changes | Emitted JSON rejected | PLE decoder edit | QPM manager | Rerun M13-M14 recipe recorded in changelog |
| PLE simplifies FIB matching | `normalized` invalid | PLE `matchMode` change | QPM manager | Single constant; smoke catches it |
| Corpus regeneration fails | No real-content evidence | Python harvest or Chromium unavailable | QPM manager | M14 minimal fixtures still cover all seven kinds and files; record the gap |
| Script check rejects common content | Banks fail without `--html-to-image` | Canvas items in plain mode | QPM manager | Error names the existing conversion path; smoke counts occurrences |
| Directory replace deletes files | Data loss | Output points at unrelated directory | QPM implementer | Refuse destinations holding non-engine files (M9 test) |
| Library weight for PLE | Heavy compile from Chromium deps | PLE adds QPM dependency | QPM manager | Record as follow-up in changelog if raised |
| Scope drift | Work outside handoff | Additions beyond Scope | QPM manager | Non-goals; M15 audit against plan |

## Documentation close-out requirements

- Active plan / progress tracker: `docs/active_plans/active/ple_native_json_writer_plan.md`,
  milestone status updated as each exit criterion passes.
- docs/CHANGELOG.md entry: engine, CLI artifact, decisions, smoke receipts, QPM capability gaps.
- docs/DESIGN_DECISIONS.md: authority, validation boundary, mapping decisions.
- docs/HUMAN_GUIDANCE.md: human directions in Context.
- PLE issues document current, with every M14 PLE-defect classification recorded.
- Temporary verification closeout: receipts referenced from the changelog; harness source removed.

## Open questions and decisions needed

- Manager/subagent decision procedure:
  - Decision owner: QPM manager; a fresh independent reviewer agent rules when evidence contradicts
    a resolved decision, and the ruling goes to `docs/DESIGN_DECISIONS.md`.
  - Evidence and decision rule: change a resolved decision only when the smoke shows a PLE
    rejection or a lossy mapping on real or fixture content. When QPM already has the information
    to express the Question correctly and the remaining problem belongs to PLE, record it in the
    PLE issues document and continue.
- Non-blocking follow-up: PLE chooses library call or directory transport; both exist.
- Non-blocking follow-up: HOTSPOT pre-binding shape stays idle until a QPM input format carries
  hotspots; this is coordination, not a PLE issue.

## PLE issues document

`docs/active_plans/reports/ple_issues_from_native_json_writer.md`, titled "PLE issues discovered
during QPM Native JSON implementation". A handoff report for PLE, not part of the QPM
implementation spec. Seeded from the supplied draft (ASCII, sentence-case headings):

- Opening: what it records; QPM produces canonical content and grading information while PLE owns
  storage, grading behavior, and presentation; entries are PLE follow-up work, not QPM
  compensation requirements.
- `## ORDER and MATCH presentation`: supplied text, citing authored-order presentation
  (`crates/question_model/src/presentation/builder_items.rs:59-75`).
- `## FIB and MULTI_FIB text matching`: supplied text, citing `text_matches` and `normalize_text`
  (`crates/grading/src/ple_question_json_validate.rs:311-356`), noting QPM emits `normalized`.
- `## Use of this document`: supplied text, including the inclusion rule.

Candidate entry for M12 evidence check: required FIB `maxLength` shown to students as "Up to N
characters" (`short_text.tsx:41-55`) while the source format has no length, so QPM must invent a
student-visible value. Explicit-`null` rejection for `feedback` and `externalResources` is an
ordinary serialization contract that QPM satisfies by omission and is not an entry.
