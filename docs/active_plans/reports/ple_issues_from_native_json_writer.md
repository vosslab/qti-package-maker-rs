# PLE issues discovered during QPM Native JSON implementation

This report records PLE behavior and contract gaps found while implementing the QPM Native JSON
writer. QPM supplies canonical question content and grading information; PLE owns grading behavior,
storage, and presentation. These entries describe PLE follow-up work, not QPM compensation
requirements. Paths to PLE source are relative to the qti-package-maker-rs repository root.

## ORDER and MATCH presentation

PLE currently presents ORDER items and MATCH prompts and choices in the order received. In
`../peptidyle-learning-engine/crates/question_model/src/presentation/builder_items.rs:59-75`,
`pending_items` appends matching prompts, matching choices, and ordering items directly from their
input vectors. The Native JSON writer must place ordering items in correct order to communicate the
answer key, and matching pairs use stable IDs for prompt-choice relationships. QPM has those data:
`crates/qti-core/src/item.rs:35-37` represents matching prompts and choices, and `:50-52` stores
ordering answers in answer order. The writer maps matching position `i` to `prompt-i`/`choice-i`
and emits ordering items in canonical correct order
(`docs/active_plans/read-native-json-converter-handoff-spec-scalable-meerkat.md:205-207`). PLE
should randomize these student-facing lists while preserving their IDs and grading bindings, so
authored/correct order does not reveal the answer.

## FIB grading contract preservation

PLE's `normalized` grading rule trims leading and trailing whitespace, collapses internal
whitespace runs, and lowercases text
(`../peptidyle-learning-engine/crates/grading/src/ple_question_json_validate.rs:311-356`). QPM
emits `normalized` for FIB and each MULTI_FIB blank to meet the requested case-insensitive,
incidental-whitespace behavior; the writer decision is recorded in
`docs/active_plans/read-native-json-converter-handoff-spec-scalable-meerkat.md:194-201`.
Preserve and test this defined behavior in PLE's grading contract. In particular,
`caseInsensitive` currently compares ASCII case only and does not trim whitespace
(`ple_question_json_validate.rs:321-323`), so replacing `normalized` with that rule would change
answer acceptance. This is a contract-preservation note required by the plan: current `normalized`
behavior satisfies QPM and is not a current defect.

## FIB response length UX contract

PLE requires a positive `maxLength` for FIB and each MULTI_FIB blank and caps it at 16,384
(`../peptidyle-learning-engine/crates/adapters/ple/src/question_json/source_compile.rs:296-303`).
The student response control displays this value as an authored-looking limit and enforces it
(`../peptidyle-learning-engine/src/components/question_response_controls/short_text.tsx:39-55`).
QPM's current item model stores FIB accepted answers and MULTI_FIB blank answers but carries no
authored response length (`crates/qti-core/src/item.rs:44-49`). The handoff writer mapping has only
`answers` for these item kinds (`NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md:59-60`). QPM can correctly
express grading without a question-specific input length; PLE conflates its platform cap with an
authored UI constraint. PLE should make the platform ceiling distinct from an instructor-authored
limit, or permit the limit to be absent when none is authored.

## External resource roles

PLE validates `externalResources` by inserting each URL alone into a set and rejects a second
entry with the same URL, regardless of `kind`
(`../peptidyle-learning-engine/crates/adapters/ple/src/question_json/source_document.rs:393-405`).
It also tests whether that inventory contains an `image` kind when determining whether the source
has an external image (`source_document.rs:262-265`). QPM item HTML can use the same HTTPS URL in
both `<a href>` and `<img src>`: the scanner reads both attributes and their roles
(`crates/qti-engines/src/ple_native_json/scan.rs:59-111`), while its source document stores
`externalResources` as URL plus kind
(`crates/qti-engines/src/ple_native_json/source.rs:6-13,121-129`).
Keeping only the first role can hide an image from PLE's image predicate; recording both roles
fails PLE's unique-URL check. PLE should represent each distinct URL role, or derive roles from
HTML without losing an image use. Until then QPM rejects cross-role reuse as a lossy export.
Repeated uses of a URL within one role can be deduplicated safely.

## Use of this document

Record an issue here when QPM has the information needed to express the Question correctly and the
remaining problem belongs to PLE. Each entry must cite current PLE file-and-line evidence and QPM
source or contract evidence showing what QPM can express. Do not record QPM capability gaps,
ordinary serialization details that QPM can satisfy, or PLE limits that its decoder correctly
enforces.
