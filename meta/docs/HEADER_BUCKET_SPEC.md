# HEADER bucket specification

How the propagator's `header_files` bucket works. Read alongside
[PROPAGATION_RULES.md](PROPAGATION_RULES.md) and [MERGE_BUCKET_SPEC.md](MERGE_BUCKET_SPEC.md).

## Why HEADER exists

NOEXIST seeds a file once, at the moment the consumer holds the least content, and never touches it
again. That fits `docs/AUTHORS.md`, whose text a repo owns outright. It fits poorly when the
template ships instructions the consumer needs to keep reading correctly for years: a survey of
eleven local repos found `docs/HUMAN_GUIDANCE.md` files ranging from 8 to 495 lines, none of which a
NOEXIST stub would ever reach again.

HEADER splits one file between two owners. The template owns a marked region and refreshes it on
every sync; the consumer owns everything else and keeps it byte-for-byte. The result is a
consumer-owned document carrying vendored, correctable instructions.

## Markers

```markdown
<!-- VENDORED HEADER: START -->
<instruction text, ending in an ownership tag>
<!-- VENDORED HEADER: END -->
```

The template file is the single source. Its own marked region is the header, and the rest of the
template is the seed body used only at creation. No separate header-source file exists.

Marker lines are matched on their stripped text, so keep them on their own line. The ownership tag
`[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]` mirrors the `.gitignore` convention of stating block
ownership inline (`repolib.gitignore.managed_gitignore_header`).

## Outcomes

| Consumer state | Action | Outcome |
| --- | --- | --- |
| File missing | Write the template stub verbatim | `created` |
| Both markers present, region already identical | Leave the file alone | `unchanged` |
| Both markers present, region differs | Replace the marked region in place | `merged` |
| Markers absent | Insert the header at the anchor | `merged` |
| Ambiguous marker structure | Report and leave the file alone | `error` |
| Template source missing or unmarked | Report and leave the file alone | `error` |

Counters and log verbs match `merge_at_imports_safe`: `created_count`, `merged_count`, `errors`, and
a quiet `no change` for `unchanged`.

## The anchor

A file without markers receives its header after the first level-one heading, or at the top when the
file has no heading. Every one of the eleven surveyed consumer files carries an H1 on line 1, so the
anchor exists everywhere and no consumer is asked to prepare a file by hand.

The rule keys on the heading line, not on its text: one surveyed file opens with the unrelated
title `# Autonomous completion policy` and still anchors correctly.

This is the difference from the `CLAUDE.md` history recorded in
[MERGE_BUCKET_SPEC.md](MERGE_BUCKET_SPEC.md), where HTML-comment fences were abandoned because
"every existing consumer would have needed a one-time hand edit before the propagator would touch
their file". The anchor removes that cost, and markers buy what set-union cannot give: a bounded
region the template may rewrite inside a file the consumer owns.

## Ambiguous marker structures

The bucket's safety promise is that it rewrites one bounded region, so an ambiguous boundary stops
the sync rather than guessing.

| Structure | Outcome |
| --- | --- |
| Exactly one START and one END, START first | Normal replace |
| START without END | `error` |
| END without START | `error` |
| More than one START, or more than one END | `error` |
| END appearing before its START | `error` |
| Neither marker | Insert at the anchor |

## Ownership within the file

| Part | Owner | Rule |
| --- | --- | --- |
| The region, markers included | HEADER | Rewritten from the template on every sync |
| The separator: the blank-line run immediately following the region, whether consumer content or end of file follows | HEADER | Normalized to exactly one blank line when content follows |
| Everything else: every line before the region, and every line from the first non-blank line after the separator onward | the consumer | Preserved byte-for-byte |

Naming the separator as its own part is what makes the contract self-consistent. Consumer content is
preserved exactly; the blank lines adjacent to the region belong to synchronization, which is what
makes repeated runs converge. The file's trailing-newline state is preserved, as in
`merge_at_imports_safe`.

## Genericity

`repolib.header_sync.sync_vendored_header` takes a source path, a destination path, the dry-run
flag, and the counters dict. It reads the header from the source file's own markers and finds the
anchor by the first `# ` line. Nothing in the helper, the plan routing, or the dispatcher refers to
particular filenames, to `docs/`, or to any section name, so a third file is a manifest entry plus a
marked block with no code change.
[../../tests/meta/test_repolib_header_sync.py](../../tests/meta/test_repolib_header_sync.py)
exercises that with a synthetic unrelated file rather than only asserting it in prose.

## Three tests, three scopes

The bucket is covered on both sides of propagation, and the mechanism is checked separately from
the content it carries, because those fail differently and change on different schedules.

| Test | Scope | Catches |
| --- | --- | --- |
| [../../tests/meta/test_repolib_header_sync.py](../../tests/meta/test_repolib_header_sync.py) | template-meta, never ships | A `header_files` entry whose source lacks a parseable region, which would turn into an `error` in every consumer at once; plus the sync fixed point, so a freshly seeded file reports `unchanged` on its next run |
| [../../tests/test_vendored_headers.py](../../tests/test_vendored_headers.py) | ships to every repo | A damaged, reversed, duplicated, or emptied header region in ANY file carrying the markers. It discovers by marker rather than by filename, so a file added to the bucket later is covered with no edit, and a doc that merely quotes the markers inside a fence stays out |
| [../../tests/test_guidance_doc_format.py](../../tests/test_guidance_doc_format.py) | ships to every repo | Entry shape in the two guidance docs: a HUMAN_GUIDANCE bullet that ran long, a prose paragraph where an entry belongs, or a decision entry missing its fields |

Keeping the header check generic and the format check specific is deliberate. The header rule
belongs to the bucket and applies to whatever the bucket touches; the format rules belong to these
two documents and say nothing about a future third member.

The format test uses formatting to keep the human file honest. Two rules do that work, both
measured against the local corpus before they were set:

- **Bullets stay within three lines.** A bullet that runs longer has usually stopped being a stated
  preference and become an agent's explanation of one.
- **Entries under a section are bullets, not paragraphs.** Prose share separates the two shapes
  cleanly: files that kept the human's statements run 0 to 7 percent prose, while files an agent
  expanded run 19 to 100 percent. Prose above the first section heading is left alone, since that is
  where a repository states its own scope note.

Section names stay unenforced: they legitimately differ per repository, and dictating them would
fight real content rather than catch narration. Repositories bring existing files into compliance;
`REPO_HYGIENE_FILTERS` in a repo's own `tests/conftest.py` is the documented escape hatch while that
work is in progress.

## Bucket dispatch

`compute_propagation_plan` emits `header_files` as a list of repo-relative paths, parallel to the
other buckets. Routing runs after the MERGE step: a `HEADER_FILES` path is removed from
`overwrite_files` and `noexist_files`, since seed-plus-refresh subsumes both. META still wins;
`assert_not_meta()` runs at plan-append time and at dispatcher entry.

## Current members

`docs/HUMAN_GUIDANCE.md` and `docs/DESIGN_DECISIONS.md`. Both are consumer-owned documents whose
vendored header states which kind of entry belongs in which file;
[../../docs/REPO_STYLE.md](../../docs/REPO_STYLE.md) is the authoritative statement that the headers
restate.
