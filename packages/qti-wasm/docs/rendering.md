# Stateless browser rendering

Build the browser/worker package with `npm run build` in `packages/qti-wasm`.
The canonical bindings target `wasm32-unknown-unknown`; generated release files
are in `dist/generated`, with the public TypeScript facade in `dist/src`.

Call `planRenderJobs(originalRequest)` after initialization. Successful plans
contain table and canvas jobs, dependencies, content hashes, a static wrapper,
the selected source item count, and reader warnings. The wrapper contains
`#qti-render-root`, embedded fonts, and a policy that disables authored scripts.
Canvas jobs contain only options recognized by the portable source parser.

Each job has `id`, `kind` (`"table"` or `"canvas"`), `contentHash`, and
`dependencies`. Table jobs include `html`; canvas jobs include `canvasSpec`.
The canvas specification supplies `smiles`, requested pixel `width` and `height`,
and `drawingDetails`: `{ explicitMethyl, atoms, bonds, legend?, highlightColour? }`.
Highlight indices are arrays of integers; an optional color is an RGB triple
in the range zero through one. Optional `peptideQuery` supplies
`{ smarts, bondAtoms: [number, number] }`. Execute that query through RDKit and
use each match's indicated atom pair to find the highlighted bond. The query
and drawing options belong to Rust; the host executes them without parsing
authored scripts or supplying its own chemical rule. Generated declarations
remain the authority for the complete transport type.

Render canvases before dependent tables. Table HTML uses
`qti-render:<canvas-job-id>` image placeholders. Substitute the dependent PNG
and its logical CSS width and height, then capture the table in the wrapper.
The host returns `{ id, png: Uint8Array, width, height }` for every job.
PNG pixel resolution can exceed the logical CSS dimensions.

Call `finishConvert(originalRequest, renders)` with the original bytes and
options. This stateless call reparses the source, rebuilds deterministic job
bindings, validates PNGs and dimensions, and writes the rewritten bank through
the existing writer. Original identities and grading survive equal rendered
presentations. Recovered media and supplied companion assets remain available.
Missing, duplicate, unknown, or invalid completions return render diagnostics.
No JavaScript object handle or retained mutable Rust state crosses calls.

Completion dimensions must be positive finite logical CSS values, independently
of PNG pixel resolution. Validation checks the actual eight-byte PNG signature,
decodes image rows with the portable PNG decoder's default allocation limit,
verifies checksums, and finishes decoding to validate the end chunk. Signature-only,
truncated, or corrupt data fails. This decoder limit is not a bound on total
host process memory. Transport also permits at most 10,000 completions, 32 MiB
per PNG, and 256 MiB of PNG bytes in total.

Expected failures return `status: "error"` with the ordinary typed diagnostic;
render planning/completion failures use category `"render"`, while transport size
violations use category `"inputLimit"`. Success from `finishConvert` returns the ordinary
conversion artifact and reader-then-writer warnings. Initialization and unexpected
transport serialization failures may throw. No artifact is written on a render
completion failure.

Direct conversions continue to use `convert`. Both rendering calls use the
same request validation, reader, item limit, metadata defaults, and shuffle seed.
Run `cargo test -p qti-wasm`, `npm run typecheck`, and `npm test` for the native
transport and exported Node runtime checks. Browser capture acceptance belongs
to the consuming website's browser tests.
