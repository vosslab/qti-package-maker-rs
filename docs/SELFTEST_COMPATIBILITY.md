# Selftest compatibility

Rust QPM parses and validates question definitions, then generates selftest HTML fragments with
inline controls and JavaScript for local answer checking. Native and Wasm share that engine.
The browser executes the emitted JavaScript; QPM receives no student submissions and manages no
student grades. References to grading below describe that browser-local selftest feedback.
The website owns variant selection and persisted completion. QPM exposes the complete question
CRC as question identity; the host chooses its progress policy. Current BPW uses BBQ filename-based
achievements in `selftest_progress_v2`: a correct variant earns completion for its problem set.
This website policy changes no QPM progress API; QPM has no persisted-progress API.

BPW is a static educational practice website. Its progress tracking is lightweight motivation,
not an authoritative assessment record. Preserve existing identity and integration behavior without
adding tamper resistance, authoritative storage, progress migrations, or elaborate consistency
mechanisms. Prioritize reliable rendering, grading, feedback, regeneration, and advancement;
engineering complexity should reflect the consequences of failure.

Answer attributes in the fragment serialize the authored correct-answer definitions. Python
emits equivalent data in attributes or generated JavaScript constants. Their encoding and Rust's
shared browser checker are implementation details within the selftest engine.

## Migration and authority

Current Python QPM is a transitional behavior reference. During migration, explicit differential
checks identify missing functionality and regressions against the selected current checkout.
Keeping Rust in parity with current Python remains the present requirement; the eventual retirement
of Python is not permission to defer current presentation or interaction regressions.
Python's existing fragment presentation and behavior are the specification. Treat every visible
or behavioral difference as a Rust defect unless a concrete inability to reproduce it is documented
with side-by-side evidence. Passing tests or supporting the same question types does not establish
parity. Rust may retain its internal architecture and safe answer-data encoding.
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

Presentation parity preserves useful content, colors, themes, layout, and controls, not HTML byte
equality. Short MC/MA choices use compact layouts; long or rich choices remain vertical. Generated
MATCH table rules must not alter nested scientific tables, and wide scientific content stays within
its control. Theme selection follows the Python reference: system preference unless the body's
explicit `data-md-color-scheme="default"` or `"slate"` overrides it. Authored colors remain intact.

`html_selftest` emits an embeddable fragment, just like Python. The caller owns the doctype,
document elements, metadata, viewport, and page theme. Opening the emitted fragment directly
provides an independent preview of that same output contract.
Keep all QPM CSS and JavaScript inline in the emitted fragment.

The shared theme and control CSS originate in Python QPM and are installed once using
its `qti-selftest-theme` identifier. The user-requested 2026-10-10 styling update uses Roosevelt
green primary buttons, shorter action buttons, and tighter MC/MA rows. These are intentional
presentation differences; the frozen Python references remain unchanged.
Preserve the transparent background fallback, inherited host
theme variables, native field appearance, form/control placement, feedback space, and paragraph
folding. Python uses `--md-default-bg-color` when supplied, otherwise transparency; it uses
`--md-default-fg-color` when supplied, otherwise its theme's foreground. Do not introduce separate
Rust document styling, color-scheme defaults, or decorative input feedback.

BPW parses the converted fragment, mounts its content inside `.selftest-reroll-content`, and
executes the embedded scripts in order. QPM supplies content and controls; it does not replace the
host page. Verify both direct opening without website CSS and embedding in a themed standards-mode
host, including explicit light/dark overrides of the opposite system preference.

MATCH presents the prompt table, actions/feedback, instructions, and choice bank in that order.
Short prompts retain compact rows; rich prompts retain their authored content and local overflow.
At narrow container widths, each prompt sits above its feedback and answer slot. Choice letters
remain inline with the full choice text; assigned slots carry the selected palette, clipped plain
text, full title, and accessible name. Instructions are associated with each slot.

MATCH and ORDER shuffle presentation indices using the existing caller seed. Source CRCs and
answer tokens remain unchanged; ORDER reset restores the initial displayed arrangement. The exact
permutation is not a public contract. Regression checks protect behavior without requiring particular
CSS techniques, pixel dimensions, random algorithms, or byte-identical documents.

- Each item exposes `question_html_<CRC>`, `statement_text_<CRC>`, and `result_<CRC>`.
  Preserve Python's question/statement container shape and applicable answer input identifiers.
- Check buttons invoke `window.checkAnswer_<CRC>()` at interaction time. Website wrappers around
  that function must observe grading. Feedback is updated synchronously before it returns.
- Controls initialize once per mounted item. Evaluating scripts again preserves wrapped hooks and
  does not add duplicate listeners. Functions resolve the current DOM element by CRC rather than
  retaining a detached element when a variant is replaced and later redisplayed.
- Check remains available after correct grading. NUM Enter grades once; MULTIFIB Enter does not
  grade. FIB has no additional Enter-grading handler. Buttons retain native keyboard activation.
- NUM follows Python's number conversion: empty and NaN input receive neutral guidance;
  positive/negative Infinity and overflow receive too-high/too-low feedback.
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
not a collision-free hash of every grading field. QPM does not migrate host progress keys.

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
replacement. The independent test host records completion by CRC and verifies its own policy:

1. Complete question A.
2. Display question B from the same bank: B is incomplete.
3. Return to question A: A remains completed and its controls address the current element.

That test host protects question identity and mounted-hook behavior. It does not prescribe current
BPW's filename-based achievement policy; the actual website journey validates that policy separately.

Current-Python execution checks are a separate migration lane. A discrepancy requires a product
fix or an explicitly justified behavior decision, never a weakened comparison to obtain a pass.
Compare the generated HTML source and inline CSS/JavaScript first, ignoring insignificant
whitespace and formatting. Resolve substantive source differences directly. Presentation work
also uses real-bank screenshots at desktop and narrow
widths in both themes, with interaction feedback reviewed. Screenshots are supporting one-time
evidence, not a permanent pixel-equality gate.

The actual website reroll and persistence journey must also be validated after the website
manager's selection fix. QPM's local host-contract proof is not a claim of website deployment or
integration acceptance. QPM's input `limit` continues to trim the bank before selection; the
website must supply the eligible bank when it wants a different variant.
