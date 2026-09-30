# Gitignore system

Canonical reference for how this template defines, renders, canonicalizes, and removes
`.gitignore` rules across consumer repositories.

The output-directory naming contract lives in
[docs/REPO_STYLE.md](../../docs/REPO_STYLE.md#data-and-outputs). This document owns the
template-side `.gitignore` machinery and maintainer workflow.

## Design goals

- Keep one canonical source for each ownership layer.
- Keep generated artifacts out of Git without hiding legitimate project content.
- Prefer root-scoped rules when artifacts belong at the repository root.
- Keep caches visible when the maintainer wants explicit cleanup instead of silent growth.
- Treat ignored state and cleanup as separate concerns: `.gitignore` never deletes files.
- Verify behavior with tracked-file evidence and `git check-ignore`, not pattern appearance alone.

## Source ownership

| Concern | Canonical source |
| --- | --- |
| Universal rules for every repo | [gitignore.universal](../../templates/gitignore.universal) |
| Python rules | [gitignore.python](../../templates/python/gitignore.python) |
| Rust rules | [gitignore.rust](../../templates/rust/gitignore.rust) |
| TypeScript rules | [gitignore.typescript](../../templates/typescript/gitignore.typescript) |
| Repository-specific rules | Consumer `.gitignore` `LOCAL REPOSITORY RULES` section |
| Exact spelling conversions | [gitignore_replacements.txt](../propagation/gitignore_replacements.txt) |
| Exact forbidden-rule cleanup | [deprecated_gitignore.txt](../propagation/deprecated_gitignore.txt) |
| Naming and placement conventions | [REPO_STYLE.md](../../docs/REPO_STYLE.md#data-and-outputs) |
| Managed-block parsing and LOCAL placement | [gitignore.py](../../repolib/gitignore.py) |
| Render orchestration and cleanup | [files.py](../../repolib/files.py) and [process.py](../../repolib/process.py) |

The root [.gitignore](../../.gitignore) is the template repository's rendered working copy,
not the universal source. The propagator always skips the template checkout. When a universal
rule also applies to this repository, update both `templates/gitignore.universal` and the matching
managed block in the root `.gitignore`.

## Pattern semantics

The managed `.gitignore` lives at each consumer repository root. Interpret patterns relative to
that file.

| Pattern shape | Meaning |
| --- | --- |
| `/output*/` | Root directories whose names begin with `output` |
| `output*/` | Matching directories at any depth |
| `/graphify-out/` | The root Graphify artifact directory only |
| `/tests/_temp/` | The root temporary-test subtree only |
| `*.out` | Matching basenames at any depth; not an output-directory alias |
| `/out/` | A root directory named exactly `out` |
| `**/target/` | A directory named `target` at any depth |

Patterns with no slash other than an optional trailing slash are recursive basename matches.
A leading slash anchors a rule to the directory containing the `.gitignore`. A trailing slash
limits the match to directories. `*` matches within one path component; `**` expresses recursive
directory matching.

Adding an ignore rule does not untrack a file already in Git. Use `git ls-files` to establish the
tracked boundary and `git check-ignore --no-index` when testing ignore behavior independent of
tracking state.

## Temporary verification

The universal `/tests/_temp/` rule keeps implementation-time tests and one-time checks untracked.
Pytest collection is a separate concern: `tests/conftest.py` leaves this subtree visible, so
pytest-suitable `test_*.py` files run with `pytest tests/` while work is active. Heavier temporary
checks run explicitly with their appropriate tool. Plan closeout promotes or removes each remaining
temporary check.

## Rendered ownership

Consumer `.gitignore` files render all propagated blocks first and the preserved local section
last. The local banner is deliberately unlike a propagated heading:

```gitignore
# === UNIVERSAL === [PROPAGATED - LOCAL EDITS OVERWRITTEN]
# === PYTHON === [PROPAGATED - LOCAL EDITS OVERWRITTEN]

# -------------------- LOCAL REPOSITORY RULES --------------------
# ADD YOUR CUSTOM IGNORES BELOW
# Propagation preserves this section.
# Template-only ignores.
```

- Add repository-specific rules below `ADD YOUR CUSTOM IGNORES BELOW`.
- Edit the owning template for a universal or type rule.
- Propagation relocates the local section after all propagated blocks, preserving its body in order.
- Propagation recognizes and replaces the legacy `# === LOCAL ===`, the previous
  `# === LOCAL REPOSITORY RULES === [ADD CUSTOM IGNORES HERE]` heading, and the older divided
  `LOCAL REPOSITORY RULES` banner, including its former `END LOCAL REPOSITORY RULES` line.
- Propagation rebuilds active managed blocks from their canonical templates rather than updating
  blocks in place.
- Canonical rebuild removes every recognized stale managed type block. Removing a token from
  `REPO_TYPE` therefore removes that old propagated block while preserving consumer-owned LOCAL
  content.
- Source-template comments and blank lines are maintainer annotations. `load_gitignore_block()`
  emits only active non-comment rules into managed consumer blocks.

Do not duplicate a managed rule in the local section. Global exact-line deduplication keeps the
first occurrence, so a local duplicate placed earlier can obscure the visible ownership boundary.

## Processing pipeline

[repolib/process.py](../../repolib/process.py) applies one ordered pipeline to each consumer:

1. Load exact replacements and forbidden-rule entries from trusted template policy files.
2. Extract consumer-owned lines from the existing `.gitignore`.
3. Build the canonical `UNIVERSAL` block and one nonempty block for each declared repository type.
4. Restore consumer-owned lines in the LOCAL section after every propagated block.
5. Apply exact source-to-destination spelling replacements.
6. Remove duplicate exact lines and trailing whitespace.
7. Remove every exact line listed in `deprecated_gitignore.txt`.

Replacement and removal entries are treated as literal `.gitignore` lines. The cleanup code does
not expand their wildcard syntax against the filesystem. For example, a negative-policy entry of
`output*/` removes only a line exactly equal to `output*/`.

### Replacement or removal

Use `gitignore_replacements.txt` when an existing rule remains valid but must converge to one
preferred spelling. The destination is the retained positive rule.

Use `deprecated_gitignore.txt` when the source rule must not survive. If another rule should
replace its behavior, supply that rule independently through a universal, typed, or local source.
Never put a retained canonical rule in the negative list.

Every negative entry affects every propagated consumer, regardless of where the line originated.
Add only exact patterns that no consumer should retain, and justify each addition in
[docs/CHANGELOG.md](../../docs/CHANGELOG.md).

## Output family

The canonical generated-output family is root-scoped:

- Use root `output/` for general generated output.
- Use stable root `output_<purpose>/` names such as `output_smoke/` when separate lifecycles help.
- Ignore the family with the universal `/output*/` rule.
- Keep nested paths such as `tests/output/` visible; they may be tracked source or evidence.
- Keep `/graphify-out/` separate because its name is not part of the `output*` family.
- Treat `*.out`, `*.sout`, `build_output.log`, `fastlane/test_output`, and similar patterns as
  file-format or tool policies, not aliases for the output-directory family.
- When a tool mandates `out/`, keep an explicit root-scoped `/out/` local rule instead of the
  recursive `out/` spelling.

The negative list removes the obsolete `output*/`, `output/`, `output_smoke/`, `/output/`, and
`/output_smoke/` lines. The universal template independently supplies the retained `/output*/`
rule.

## Change routing

| Desired change | Edit |
| --- | --- |
| Add a rule to every repository | `templates/gitignore.universal` |
| Add a language-family rule | The owning `templates/<type>/gitignore.<type>` file |
| Add one repository exception | That consumer's preserved local section |
| Convert one exact spelling to another | `meta/propagation/gitignore_replacements.txt` |
| Forbid an exact rule everywhere | `meta/propagation/deprecated_gitignore.txt` |
| Change artifact naming or placement | `docs/REPO_STYLE.md` first, then the owning rule source |
| Change block parsing or LOCAL placement | `repolib/gitignore.py` |
| Change render orchestration or cleanup | `repolib/files.py` or `repolib/process.py` |

Do not edit a consumer managed block as the only change. The next propagation overwrites it.

## Maintainer workflow

1. Survey current rules and identify whether each match is a directory, filename, or incidental
   substring.
2. Inspect tracked paths before narrowing or broadening a rule.
3. Decide whether the change is universal, type-specific, local, a replacement, or a removal.
4. Update the canonical source and [docs/CHANGELOG.md](../../docs/CHANGELOG.md).
5. Dry-run propagation against a representative consumer.
6. Apply propagation only to the repositories in the authorized scope.
7. Verify positive and negative path examples with `git check-ignore`.
8. Run the focused policy tests and the complete suite.
9. Repeat the dry run after application; it should propose no second change.

Survey `.gitignore` rules without treating every `out` substring as the same artifact:

```bash
rg --hidden --no-ignore -n '^[^#]*out[^#]*$' /Users/vosslab/nsh \
  -g .gitignore -g '!**/OTHER_REPOS/**' -g '!**/OLD/**'
```

Inspect tracked paths that could be hidden by an unanchored output rule:

```bash
git ls-files | rg '(^|/)output[^/]*/'
```

Preview one consumer without writing:

```bash
source source_me.sh && python3 propagate_style_guides.py -n -R ../consumer-repo
```

Apply the same bounded propagation:

```bash
source source_me.sh && python3 propagate_style_guides.py -R ../consumer-repo
```

## Validation

Confirm positive matches and identify the owning rule:

```bash
git check-ignore -v --no-index output_smoke/probe.txt graphify-out/probe.json
```

Confirm a nested tracked-content candidate stays visible:

```bash
if git check-ignore -q --no-index tests/output/probe.txt; then
  echo "unexpectedly ignored"
else
  echo "visible to Git"
fi
```

Run focused policy and documentation gates:

```bash
source source_me.sh && pytest \
  tests/meta/test_load_deprecation_lists.py \
  tests/meta/test_repolib_multi_type.py \
  tests/test_markdown_links.py \
  tests/test_ascii_compliance.py \
  tests/test_whitespace.py -q
```

Run the complete repository suite:

```bash
source source_me.sh && pytest tests/ -q
git diff --check
```

## Common failures

- Root-only edit: a rule added only to a rendered `.gitignore` disappears on propagation.
- Broad recursive rule: `output*/` hides legitimate nested `tests/output/` content.
- False unification: `*.out`, `/out/`, and `/graphify-out/` are treated as one output family.
- Cleanup misunderstanding: a wildcard in `deprecated_gitignore.txt` is expected to scan files.
- Hidden disk growth: caches are ignored when the intended policy is explicit deletion.
- Tracking misunderstanding: a new ignore rule is expected to untrack committed content.
- Ownership drift: a managed rule is copied into the local section and later survives source
  changes.
- Unbounded migration: propagation is applied across repositories without a representative dry run.
