# Requirements bucket specification

How the propagator's `requirements_files` bucket refreshes universal development dependencies
without replacing repository-specific requirements. Read alongside
[PROPAGATION_RULES.md](PROPAGATION_RULES.md).

## Why it exists

`pip_requirements-dev.txt` has two owners. The template maintains tools every repository uses;
each repository maintains packages needed only by its own code and tests. NOEXIST cannot refresh
the shared set after creating a repository, while OVERWRITE would erase the local set.

The requirements bucket parses package identity and gives each owner an explicit region. It is a
separate policy rather than a special case inside the generic HEADER or MERGE buckets.

## Ownership markers

Both markers are valid pip comments and must appear exactly once, in this order:

```text
# === UNIVERSAL DEVELOPMENT DEPENDENCIES === [PROPAGATED - LOCAL EDITS OVERWRITTEN]
<template-managed requirements>

# === LOCAL DEVELOPMENT DEPENDENCIES === [ADD REPOSITORY-SPECIFIC DEPENDENCIES HERE]
<repository-managed requirements, comments, and directives>
```

The universal marker and every line before the local marker form the managed block. The local
marker and all later content belong to the repository. Duplicate, unpaired, reversed, or
heading-like malformed markers make the synchronization report an error without writing.

## Synchronization behavior

| Consumer state | Action | Outcome |
| --- | --- | --- |
| File missing | Seed the complete canonical file | `created` |
| Marker-free file | Install both markers, remove universal package duplicates, preserve other lines in order | `merged` |
| Marked file, managed block differs | Replace only the managed block | `merged` |
| Marked file already current | Leave the file unchanged | `unchanged` |
| Ownership markers ambiguous | Report the error and leave the file unchanged | `error` |

Requirement lines are parsed with `packaging.requirements.Requirement` and compared with
`packaging.utils.canonicalize_name`. Extras, version constraints, environment markers, and name
punctuation therefore do not create a second owner for the same package. The canonical managed
specification wins when a marker-free consumer duplicates a universal package.

Unparseable lines remain local during migration. This preserves repository comments and pip
directives such as `--extra-index-url`, `-r`, and editable or VCS forms in their original order.
Managed source lines must be parseable requirements or comments; malformed canonical input is a
template error and stops synchronization.

## Current universal set

The canonical managed block contains:

- `bandit`
- `graphifyy[ollama,sql,terraform]`
- `packaging`
- `pyflakes`
- `pytest`
- `rich`
- `tree-sitter`
- `tree-sitter-rust`

The canonical source is [../../pip_requirements-dev.txt](../../pip_requirements-dev.txt). Change
that file to update the managed specifications and comments; do not copy the package list into
propagation code.

## Routing and verification

`requirements_files` in [../propagation/manifests.yaml](../propagation/manifests.yaml) claims the
root file after normal discovery, removing it from overwrite and noexist. The dispatcher calls
`repolib.requirements_sync.sync_development_requirements` with the same dry-run and counter
contract as the other file policies.

[../../tests/meta/test_repolib_requirements_sync.py](../../tests/meta/test_repolib_requirements_sync.py)
covers marker-free migration, canonical-name duplicate removal, local-content preservation,
idempotent updates, missing-file seeding, and refusal of malformed markers.
