# Selftest compatibility

Rust QPM owns generated selftest HTML, controls, and grading in both native and Wasm exports.
The website owns variant selection and persisted completion. Completion belongs to the complete
question CRC, never the question bank or just the stem component of the CRC.

## Migration and authority

Current Python QPM is a transitional behavior reference. During migration, explicit differential
checks identify missing functionality and regressions against the selected current checkout.
Evaluate differences on their merits: preserve useful behavior, fix defects, and prefer a simple
Rust design. Do not add aliases or compatibility layers solely because an older interface existed.
Each run records the source revision and content hashes; historical receipts keep their original
source identity. A reproducible receipt does not make an old revision the ongoing behavior target.

Important behaviors become independent Rust and browser regression tests with explicit expected
outcomes. Ordinary Rust and browser tests run without Python QPM. The migration comparison is an
explicit development lane, not a production runtime dependency or a permanent correctness oracle.

Rust becomes the sole maintained implementation when the required workflows have independent
regression coverage and remaining migration differences have been resolved or intentionally
accepted. At that point, remove Python comparison commands and their development dependencies
from active workflows and archive their evidence. Do not keep maintaining Python just to test Rust.

## Website-facing contract

- Each item exposes `question_html_<CRC>`, `statement_text_<CRC>`, and `result_<CRC>`.
  Preserve Python's question/statement container shape and applicable answer input identifiers.
- Check buttons invoke `window.checkAnswer_<CRC>()` at interaction time. Website wrappers around
  that function must observe grading. Feedback is updated synchronously before it returns.
- Controls initialize once per mounted item. Evaluating scripts again preserves wrapped hooks and
  does not add duplicate listeners. Functions resolve the current DOM element by CRC rather than
  retaining a detached element when a variant is replaced and later redisplayed.
- Check remains available after correct grading. NUM Enter grades once; MULTIFIB Enter does not
  grade. FIB has no additional Enter-grading handler. Buttons retain native keyboard activation.
- NUM requires a finite number. Infinity and overflow are invalid input, rather than merely a
  too-high/too-low answer. This preserves Rust's useful validation instead of copying Python's
  NaN-only check.
- MA clear and MATCH/ORDER reset change the displayed answers and feedback, not host completion.
  There is no Reveal action that silently grades a supplied answer.
- Preserve grading rules and completion-relevant feedback: `CORRECT`, `Correct: X of Y`,
  `Total Score: X out of Y`, and `Correct positions: X of Y`. Partial scores are not completion.
- Repeated named MULTIFIB blanks have distinct element IDs while sharing their accepted answers.
- Reset and new attempts do not erase the website's record of a previous completion. The website
  owns that persistence; generated HTML must keep feedback and actions scoped to its own question.

The existing Python-compatible CRC16-XMODEM scheme remains unchanged. It hashes the source stem
and kind-specific secondary content. Bank position, selection seed, document metadata, and media
presentation rewrites do not redefine an authored question. It is an established identity scheme,
not a collision-free hash of every grading field; this fix does not migrate stored progress keys.

Answer payloads remain data, not executable JavaScript. Keep attribute escaping and safe text
assignment at their output contexts (ASVS 1.2.1 and 1.2.3). Authored question HTML retains the
existing export policy; this compatibility fix does not introduce a new sanitizer.

## Why drift happened

The initial Rust selftest implementation landed in `5407b3912333fe4ff2608f877327aee422131c64`.
It registered global grading functions but called a private grading function from click and
Enter handlers. This bypassed website wrappers even against the original Python reference.
The missing statement ID and reinitialization of existing questions were part of that same
implementation. The grading asset remained unchanged through the native/Wasm migration.

Python commit `344f9809f61b92a406704ad86a10533e5904f161` subsequently changed button availability
and MATCH interactions. Rust checks still targeted `55e5f368777f7809fe2e91b5d070caf6df0cb581`,
so those improvements could not be detected by the frozen comparison. Historical pinning provided
reproducibility, but its use as a continuing compatibility target prevented detection of drift.

The archived port plan called for embedding Python control assets. Rust copied some interaction
assets but implemented a separate general grading dispatcher. Content projections extracted
embedded answers without executing that dispatcher. Statement comparison accepted a Rust-only
class as an alternative to the Python identifier. Initial enabled-control checks did not exercise
post-grading button state. Native/Wasm comparisons exercised the same Rust implementation on both
sides, and package browser tests checked conversion/downloads without grading the resulting HTML.

Earlier seven-kind browser proof remains historical evidence. It did not leave a durable test of
the website's wrapped-hook contract. Runtime host-contract tests now protect that boundary;
static answer projections alone cannot establish selftest compatibility.

The [verification report](active_plans/reports/SELFTEST_REGRESSION_VERIFICATION.md) records the
reproduced old-output failure, independent runtime checks, and remaining migration findings.

## Acceptance boundary

Independent tests must exercise native and delivered-Wasm HTML in Chromium, Firefox, and WebKit.
They check all seven grading kinds, public-hook interception, repeated initialization, and variant
replacement. The host test records completion by CRC and verifies:

1. Complete question A.
2. Display question B from the same bank: B is incomplete.
3. Return to question A: A remains completed and its controls address the current element.

Current-Python execution checks are a separate migration lane. A discrepancy requires a product
fix or an explicitly justified behavior decision, never a weakened comparison to obtain a pass.

The actual website reroll and persistence journey must also be validated after the website
manager's selection fix. QPM's local host-contract proof is not a claim of website deployment or
integration acceptance. QPM's input `limit` continues to trim the bank before selection; the
website must supply the eligible bank when it wants a different variant.
