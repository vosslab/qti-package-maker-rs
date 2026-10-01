# WP-T4 draw-receiver ruling

## Decision

Preserve the pinned Python rule: accept exactly one textual
`.draw_to_canvas(` or `.draw_to_canvas_with_highlights(` occurrence regardless
of its receiver. Do **not** add an exact `mol.draw_to_canvas...(canvas,
JSON.stringify(mdetails))` allowlist and do not classify this as a security
divergence.

The parser is a non-executing **static CanvasSource extractor**, despite the
plan's shorter "static script grammar" wording. It extracts independently
bounded literals (SMILES, `mdetails`, canvas dimensions, and the declared
options) and gives those values to the native renderer. It never evaluates the
receiver, the draw-call arguments, or any other item JavaScript. Thus a script
containing `attacker.draw_to_canvas(evil())` with otherwise valid static setup
has the same extracted result in Python and Rust; `evil()` is never called.

## Evidence and parity authority

At the pinned Python revision `55e5f368777f7809fe2e91b5d070caf6df0cb581`,
`selectors.py` defines `_DRAW_CALL_RE` as
`\.draw_to_canvas(?:_with_highlights)?\s*\(` and only requires one match.
The current Rust `draw_call_re()` has the same rule. The 154 real scripts,
including 58 CanvasSource records, agree under that extraction contract.

No observed LMS behavior, package-integrity rule, or execution path gives a
receiver spelling higher parity authority. The active plan requires every
Python rejection, and the repository's parity authority makes Python runtime
behavior tier 3 when no tier-1 or tier-2 signal exists. A receiver allowlist
would be an ungrounded stricter rejection and would reject a pinned-Python
accepted input without improving the no-execution property.

## Required clarification and proof

Keep the limits and literal validation already imposed on values passed to
RDKit: one `get_mol(smiles)`, one draw-method occurrence, static non-empty
SMILES, an empty initialized `mdetails`, allowlisted non-duplicate literal
options, RGB range, and dimensions/list bounds. Describe this component as an
extractor in its public documentation and errors; do not imply that acceptance
proves the JavaScript call itself is valid.

Add a focused parity test in both the Python oracle fixture and Rust suite:
the `attacker.draw_to_canvas(evil())` spelling with valid static source/details
extracts the same `CanvasSource`; a separate assertion confirms no JavaScript
engine is involved. Preserve the existing negative set for every actual Python
rejection.
