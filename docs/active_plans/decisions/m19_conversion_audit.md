# M19 conversion and measurement audit

## Ruling

Accept the conversion architecture and the run-4704 speed receipt as technical evidence. Do not
mark M19 complete yet. The release CLI converts once before writer fan-out, the cache is scoped to
that conversion, and the reported measurements establish the native speed comparison. The current
stage-attribution implementation needs a correction before its `bookkeeping` value can be used as
a stage, and four explicit M19 exit receipts remain.

This follows **Ground requirements in actual needs** and **Fix the design, not the symptom**:
the correction makes the measurement model state what it measures instead of relabelling an
inclusive elapsed interval as residual orchestration.

## Accepted evidence

`run_bbq_converter` calls `convert_bank` once, before its writer loop, when an image-capable
writer was selected. The loop gives only the already converted bank to the three package writers
and forces their option false. This satisfies the once-before-fan-out design, rather than merely
depending on a cache to hide repeat conversion.

The immutable captured binary receipt is
`output_tables/native_table_bench/run-4704/bbq-converter`, SHA-256
`463a4a506c9b2843b41f3c918ed032bfeb47e3cc05f238cd40974bfa4f6c1ceb`.
Both native modes carried `--html-to-image`, completed all 181 inputs with no errors, and measured
29.703 seconds for Blackboard export and 32.905 seconds for the three-format lane. The repaired,
same-machine Python baseline is 158.022 and 371.674 seconds. The receipt also inspects real ZIP
members: 610 PNGs in 118 Blackboard packages and 1,826 PNGs in 353 three-format packages.

The library counters are internally consistent in both modes: requests equal
hit + wait + miss (`651 = 200 + 60 + 391` and `651 = 195 + 65 + 391`), and each cache miss was one
successful renderer attempt (`391` attempts, successes, and misses). A fresh cache per manifest
input matches the CLI conversion scope. Cache keys include renderer family, prepared bytes, and
the renderer discriminator, so configuration changes cannot reuse old PNGs. Rayon owns local
metrics and reduces them only after item work completes.

## Required metric correction

`conversion_bookkeeping` is currently not a residual orchestration duration. Its first increment
starts before plan preparation and stops after all parallel rendering, so it includes render wait
and elapsed render time; the report then also lists renderer layout, paint, and encode work. Its
70.414/58.101-second values nearly equal the library conversion wall times, which confirms that
the field is inclusive. It must not be described as a separate stage.

Measure and add only non-overlapping serial spans: plan preparation/source snapshot, result-bank
setup, and item reconstruction/naming. Keep materialization separate. Do not time a span around
the Rayon render collection as bookkeeping. Cache timing remains unreported unless it gains its
own clearly named inclusive-wait field; outcome counts are sufficient for this milestone.

`renderer_failures` is also not observable today. A cache miss increments attempts, but a render
error returns `ConversionError::Render` before the metric leaves `render_cached`, so no caller can
receive the failure count. Preserve the accepted metric contract by making the metrics API return
a typed failure containing the original `ConversionError` and the metrics observed through that
failure; the ordinary `convert_bank` wrapper maps that typed failure back to its existing error.
Document that parallel failure metrics cover work observed before cancellation. Add a deliberate
failing-renderer test that asserts one attempt and one failure. This is an internal API only and
does not alter CLI behavior.

After those two changes, rerun the library metric pass. The immutable release-binary wall-time
receipt itself need not be rerun unless the CLI conversion path changes.

## Remaining M19 exits

1. The M17 user visual sign-off remains pending; M19 depends on M17 and cannot close first.
2. Produce an all-corpus conversion receipt that checks every selected table and canvas is replaced
   at its original field position, verifies required alt text, surviving surrounding markup,
   removal of the known loader script, and ASCII serialized HTML. Existing focused selector and
   conversion tests establish pieces of this contract, not the all-corpus exit criterion.
3. Run `-1 -2 -B --html-to-image` outputs through `qti-integrity` and record zero errors including
   the referenced-media trace. ZIP PNG member counts and successful CLI exits do not prove this.
4. Add a process-visible or test-visible one-conversion counter to the three-format path and record
   one conversion per input run. The current control flow is correct, but the benchmark receipt
   does not contain the plan-required run-log proof.

The static parser's real/hostile contract remains governed by the accepted WP-T4 parser ruling;
the implementation tests its static options and negative cases without JavaScript execution.

## Subsequent provenance qualification

The structural corpus audit found a first-item CRC mismatch between run-4704 and the current
source corpus. Run-4704 did not capture input or manifest hashes, so it cannot prove exact corpus
identity for the comparison. Its measured elapsed times remain historical observations; the
same-corpus speed exit requires fresh Python and native receipts with matching input hashes.
Both benchmark paths now record those hashes and reject source mutation during measurement.

## One-conversion process receipt

The current `qti-cli` process suite passes all eight contracts. Its
`three_packaging_formats_share_one_observable_html_conversion_pass` test executes
`-1 -2 -B --html-to-image -v`, requires exactly one conversion-count line, verifies completion
of all three formats, and checks all three ZIP artifacts. `ConverterReport.conversion_passes`
records the shared successful pass; quiet mode preserves its existing output contract.
This supplies the explicit once-before-fan-out process proof without relying on cache counts.

## Correction: metric and package exits accepted

The metric implementation now satisfies the bounded instrumentation contract. Its preliminary
bookkeeping span ends before the Rayon render collection; only serial plan/source preparation,
result setup, and reconstruction are accumulated, with output-root materialization timed
separately. `ConversionFailure` carries the original typed `ConversionError` and boxed metrics
observed before cancellation. The deliberate renderer-failure test now observes one request, one
miss, one attempt, zero successes, and one failure. This keeps ordinary `convert_bank`'s error
surface unchanged while making the metrics API honest about failed work.

The immutable run-80812 receipt records the corrected native implementation and input hashes.
Its counters remain internally consistent: Blackboard is `651 = 197 + 63 + 391`, and three-format
is `651 = 204 + 56 + 391`; both have 391 attempts, 391 successes, and zero failures. Its
bookkeeping work is 14.685 and 13.369 seconds, respectively, distinct from render and
materialization work. The run-80812 output certification checks all 721 ZIPs (181 Blackboard and
540 three-format) with `qti-integrity`: zero errors and zero warnings, including the referenced
media trace. These close the metric and package-integrity exits.

The three-format process test closes the once-per-input exit: it observes exactly one conversion
line and all three artifacts. The control-flow design and the process evidence now agree.

## Remaining M19 exits after correction

1. M17 still needs the user's visual sign-off.
2. Run the existing all-corpus direct converted-bank structural audit and retain its passing
   receipt. The tool checks replacement removal at each field position, image counts and alt
   text, surrounding markup/text, loader removal, ASCII output, bank identity, and input hashes.
3. Re-run the pinned Python benchmark against the input hashes captured in run-80812, then compare
   that matched receipt with run-80812. The prior Python timing cannot satisfy the same-corpus
   speed claim because run-4704 lacked input provenance.

## Historical correction: hash-matched speed exit accepted for run-80812

The fresh pinned-Python receipt at `output_tables/baseline.json` and the native receipt at
`output_tables/native_table_bench/run-80812/receipt.json` carry the identical corpus-manifest
SHA-256 `4446e61f2d7457c1c5a72e13d61bea97b26a3c2b717a157145b6348498abdec1` and identical
181-entry `input_provenance` arrays. Both modes have zero failures. On those exact inputs, Python
plain wall time is 197.233859 seconds for Blackboard and 445.819936 seconds for the three-format
lane; native wall time is 35.844927 and 27.939974 seconds respectively. This closes M19's
same-corpus speed comparison for that captured implementation. The earlier run-4704 remains
historical evidence only.

## Current benchmark acceptance remains open

The run-80812 acceptance above does not certify the current source. Subsequent fixes preserve
numeric source markup, match the pinned MA reader defaults, and correct Canvas score declarations
and MC continuation. Run-40542 also predates the final continuation, score-outcome, and MA-default
repairs; its zero-error certification of 721 packages is evidence for that captured binary only.

After the full parity gate accepts the integrated product, capture a fresh release binary and
rerun both native benchmark lanes using the certified macOS RDKit shim. Compare the manifest and
all 181 ordered input hashes with `output_tables/baseline.json`, and certify every resulting ZIP
with unchanged package and checker hashes. Only that source-bound measurement can close the
current speed and package-integrity exits. The accepted structural v7 audit and parser
differential receipts remain separate evidence; user visual sign-off is still required.

## Parser audit status

The accepted WP-T4 receiver ruling fixes the parser's semantic boundary: it is a non-executing
static extractor, preserves the pinned receiver-agnostic draw-call rule, and retains the stated
literal and bounds restrictions. The current Rust unit tests exercise representative accepted and
rejected scripts, including the pinned `attacker.draw_to_canvas(evil())` spelling. They are not,
by themselves, the M19 real-and-hostile differential receipt required by the active plan.

That remaining receipt must run the frozen Python extractor and Rust extractor over every harvested
real canvas script and a named hostile corpus. It must bind the frozen snapshot and inputs by hash;
compare acceptance plus every extracted `CanvasSource` field for accepted cases; and compare
accept/reject outcome for hostile cases. The hostile set must cover the limits and grammar axes in
WP-T4: dimensions, get-mol and draw-call multiplicity, static SMILES, empty details initialization,
dynamic/duplicate/unsupported keys, literal options, integer-list bounds, RGB range, and the
receiver-agnostic accepted spelling. Error wording need not match. A new `crc_oracle`-based runner
does not close this exit until it produces that actual frozen-Python receipt.

## Remaining M19 exits after current evidence

1. M17 still needs the user's visual sign-off.
2. Reissue the all-corpus structural receipt with frozen Python per-field positional selection
   authority, position-paired alternatives, normalized surviving DOM tree/attributes and text,
   plus converted-bank item-count equality.

## Parser differential accepted

Accept the frozen real-and-hostile static-parser differential as the M19 parser exit. The current
receipt records 141 real source occurrences and 23 hostile cases with no Rust/Python disagreement.
It compares every `CanvasSource` field for accepted cases and acceptance only for hostile cases,
which preserves the plan's free error-wording rule.

The acceptance is provenance-bound: the Python helper proves that `selectors.py` was imported from
the certified `55e5f368777f7809fe2e91b5d070caf6df0cb581` snapshot and records its SHA-256. Before
comparing scripts, the Rust runner verifies the manifest digest, exact ordered 181 input paths, and
each input digest against the fixture, then retains those verified values in the receipt. An
independent rerun reproduced fixture digest
`803e02d7c53e5bf294215aab09f8601808174a374f202a5a4d7033b4cee82ec4` and zero disagreements; a
shortened manifest failed with `manifest SHA-256 does not match the Python fixture` before parsing.

The helper now follows frozen `transform.py`: it visits `question_text`, then recursively visits
each `get_tuple()` supporting field. That includes the three MC `answer_text` occurrences which
repeat canvas-bearing choices, so the 141 real contexts are not erroneously deduplicated. This
implements **Ground requirements in actual needs**: the observed Python conversion traversal, not
a convenient Rust field iterator, defines the proof boundary.

The remaining M19 exits are the user M17 visual sign-off and corrected all-corpus structural
replacement proof. This decision does not accept either of them.
