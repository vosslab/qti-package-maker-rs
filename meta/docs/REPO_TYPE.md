# REPO_TYPE.md

`REPO_TYPE` is the root marker that declares which shared template families a
repository consumes. It classifies a repository; it does not define repository
style. Repository conventions live in [docs/REPO_STYLE.md](../../docs/REPO_STYLE.md).

## Marker format

- Store `REPO_TYPE` at the repository root.
- Write one or more lowercase type names followed by a newline.
- Separate several names with commas and no spaces, for example `python,rust`.
- Preserve declaration order.
- Maintain the marker when the repository changes; it remains live after bootstrap.

## Available types

The available names, in canonical display order, are `python`, `pypi`,
`typescript`, `githubpages`, `rust`, `swift`, `other`, `scripted`, `website`,
`compiled`, and `all`.

Inheritance adds the complete parent rule set:

- `pypi` -> `python` -> `scripted`
- `githubpages` -> `typescript` -> `website`
- `typescript` -> `website`
- `rust` -> `compiled`
- `swift` -> `compiled`

`scripted`, `website`, `compiled`, and `other` are root types. Every listed type
is valid as a direct marker. `all` expands to every concrete type supported by
the template.

Use `githubpages` alone for a TypeScript repository deployed with the template's
GitHub Pages build. Inheritance already supplies the TypeScript and website
families; writing `githubpages,typescript` is redundant.

During `reset_repo.py`, selecting `typescript` prompts whether the repository
uses the template's GitHub Pages build. An affirmative answer writes the
canonical `githubpages` child marker, matching the Python-to-PyPI promotion flow.

Existing Pages consumers previously marked `typescript` must change their marker
to `githubpages` to keep receiving updates to the four Pages front doors. Existing
generic TypeScript consumers may remove old copies of those files when they do not
fit the repository; propagation does not infer whether a previously copied file is
still locally owned.

## Multiple types

Declare several types only when a repository genuinely ships several families,
such as a Python CLI with a Rust extension. The repository receives the union of
the declared types and their inherited rule sets. Declaration order determines
which typed overlay wins if several overlays provide the same path.
