"""Behavior tests for the HEADER bucket helper sync_vendored_header.

Weighted toward the bucket's safety invariant: synchronization rewrites the
vendored region and preserves arbitrary consumer content byte-for-byte. Test
names double as -k selectors for running one behavior in isolation: parse,
replace, insert, malformed, generic, manifest.
"""

# Standard Library
import importlib
import os
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils
import repolib.console
import repolib.header_sync
import repolib.model


REPO_ROOT = file_utils.get_repo_root()


START = repolib.header_sync.HEADER_START_MARKER
END = repolib.header_sync.HEADER_END_MARKER

TEMPLATE_SOURCE = (
	"# Human guidance\n"
	"\n"
	f"{START}\n"
	"> Vendored header, refreshed by propagation.\n"
	"\n"
	"Record short entries here.\n"
	f"{END}\n"
	"\n"
	"## Decision priority\n"
)

# Header region as it appears in a synced consumer file, derived from the template
# so the expectation tracks TEMPLATE_SOURCE instead of restating it.
TEMPLATE_HEADER_TEXT = '\n'.join(
	repolib.header_sync.extract_header(repolib.header_sync.split_lines(TEMPLATE_SOURCE)[0])
) + '\n'

# Distinctive multiline consumer content, above and below the vendored region.
CONSUMER_ABOVE = "# Autonomous completion policy\n"
CONSUMER_BODY = (
	"## Direct preferences\n"
	"\n"
	"- Keep it simple: avoid speculative machinery.\n"
	"  A wrapped continuation line that must survive intact.\n"
	"\n"
	"- Target a 16:10 desktop aspect for screenshots.\n"
)


#============================================
def write_file(path: pathlib.Path, content: str) -> None:
	"""Write content to path, creating parent directories as needed."""
	path.parent.mkdir(parents=True, exist_ok=True)
	with open(path, 'w', encoding='utf-8') as file_handle:
		file_handle.write(content)


#============================================
def run_sync(tmp_path: pathlib.Path, dest_text: str | None, source_text: str = TEMPLATE_SOURCE) -> tuple[str, pathlib.Path]:
	"""Write a source and optional dest, run one sync, and return the outcome and dest path."""
	source = tmp_path / "template.md"
	dest = tmp_path / "consumer" / "guidance.md"
	write_file(source, source_text)
	if dest_text is not None:
		write_file(dest, dest_text)
	counters = repolib.console.init_counters()
	outcome = repolib.header_sync.sync_vendored_header(str(source), str(dest), False, counters)
	return outcome, dest


#============================================
def test_parse_locates_marker_pair() -> None:
	"""find_marker_lines returns the marker indexes for a well-formed region."""
	lines, _trailing = repolib.header_sync.split_lines(TEMPLATE_SOURCE)
	start_index, end_index = repolib.header_sync.find_marker_lines(lines)
	assert (lines[start_index].strip(), lines[end_index].strip()) == (START, END)


#============================================
def test_parse_reports_no_markers_for_plain_file() -> None:
	"""A marker-free file reports the sentinel rather than raising."""
	lines, _trailing = repolib.header_sync.split_lines(CONSUMER_ABOVE + "\n" + CONSUMER_BODY)
	assert repolib.header_sync.find_marker_lines(lines) == (-1, -1)


#============================================
def test_parse_anchor_follows_first_heading() -> None:
	"""The anchor is the line after the first level-one heading, whatever its text."""
	lines, _trailing = repolib.header_sync.split_lines(CONSUMER_ABOVE + "\n" + CONSUMER_BODY)
	assert repolib.header_sync.anchor_index(lines) == 1


#============================================
def test_replace_preserves_consumer_content(tmp_path: pathlib.Path) -> None:
	"""Replacing a stale region leaves every consumer byte outside it untouched."""
	stale = (
		CONSUMER_ABOVE
		+ "\n"
		+ f"{START}\n"
		+ "> Old vendored wording.\n"
		+ f"{END}\n"
		+ "\n"
		+ CONSUMER_BODY
	)
	outcome, dest = run_sync(tmp_path, stale)
	result = dest.read_text(encoding='utf-8')
	assert outcome == 'merged'
	assert result == CONSUMER_ABOVE + "\n" + TEMPLATE_HEADER_TEXT + "\n" + CONSUMER_BODY


#============================================
def test_replace_is_idempotent(tmp_path: pathlib.Path) -> None:
	"""A second sync over synced content reports unchanged and rewrites nothing."""
	stale = CONSUMER_ABOVE + "\n" + f"{START}\n> Old.\n{END}\n" + "\n" + CONSUMER_BODY
	_outcome, dest = run_sync(tmp_path, stale)
	first_pass = dest.read_text(encoding='utf-8')
	source = tmp_path / "template.md"
	counters = repolib.console.init_counters()
	outcome = repolib.header_sync.sync_vendored_header(str(source), str(dest), False, counters)
	assert (outcome, dest.read_text(encoding='utf-8')) == ('unchanged', first_pass)


#============================================
def test_insert_preserves_consumer_content(tmp_path: pathlib.Path) -> None:
	"""A marker-free file gains the header at its anchor with the body intact."""
	outcome, dest = run_sync(tmp_path, CONSUMER_ABOVE + "\n" + CONSUMER_BODY)
	result = dest.read_text(encoding='utf-8')
	assert outcome == 'merged'
	assert result == CONSUMER_ABOVE + "\n" + TEMPLATE_HEADER_TEXT + "\n" + CONSUMER_BODY


#============================================
def test_insert_falls_back_to_top_without_heading(tmp_path: pathlib.Path) -> None:
	"""A file with no level-one heading receives the header at the top."""
	_outcome, dest = run_sync(tmp_path, CONSUMER_BODY)
	assert dest.read_text(encoding='utf-8') == TEMPLATE_HEADER_TEXT + "\n" + CONSUMER_BODY


#============================================
def test_insert_creates_missing_file_from_template(tmp_path: pathlib.Path) -> None:
	"""A consumer lacking the file receives the whole template stub."""
	outcome, dest = run_sync(tmp_path, None)
	assert (outcome, dest.read_text(encoding='utf-8')) == ('created', TEMPLATE_SOURCE)


#============================================
@pytest.mark.parametrize(
	"broken_region",
	[
		f"{START}\n> Orphan start.\n",
		f"> Orphan end.\n{END}\n",
		f"{START}\n> One.\n{END}\n{START}\n> Two.\n{END}\n",
		f"{END}\n> Reversed.\n{START}\n",
	],
	ids=["start_alone", "end_alone", "duplicate_pair", "end_before_start"],
)
def test_malformed_region_leaves_file_untouched(tmp_path: pathlib.Path, broken_region: str) -> None:
	"""Every ambiguous marker structure reports error and rewrites nothing."""
	original = CONSUMER_ABOVE + "\n" + broken_region + "\n" + CONSUMER_BODY
	outcome, dest = run_sync(tmp_path, original)
	assert (outcome, dest.read_text(encoding='utf-8')) == ('error', original)


#============================================
@pytest.mark.parametrize(
	"shipped_module",
	["test_vendored_headers", "test_guidance_doc_format"],
)
def test_manifest_shipped_markers_match_the_helper(shipped_module: str) -> None:
	"""A shipped test's marker literals stay equal to the helper's constants.

	The shipped tests cannot import repolib, which never propagates, so they carry
	their own copies of the marker strings. Both discover the files they check by
	matching those strings, so a drifted copy would not fail loudly: discovery
	would return nothing and the test would pass having checked no files. This
	guard fails here, in the template, before propagation carries the drift out.
	"""
	module = importlib.import_module(shipped_module)
	shipped = (module.HEADER_START_MARKER, module.HEADER_END_MARKER)
	canonical = (
		repolib.header_sync.HEADER_START_MARKER,
		repolib.header_sync.HEADER_END_MARKER,
	)
	assert shipped == canonical


#============================================
@pytest.mark.parametrize("file_rel", sorted(repolib.model.HEADER_FILES))
def test_manifest_source_carries_a_parseable_header(file_rel: str) -> None:
	"""Every header_files entry resolves to a template file with a usable region.

	A manifest entry whose source lacks markers turns into an 'error' outcome in
	every consumer at once, so the mismatch is worth catching here instead.
	"""
	source_path = os.path.join(REPO_ROOT, file_rel)
	source_text = pathlib.Path(source_path).read_text(encoding='utf-8')
	source_lines, _trailing = repolib.header_sync.split_lines(source_text)
	header_lines = repolib.header_sync.extract_header(source_lines)
	assert header_lines[0].strip() == START and header_lines[-1].strip() == END


#============================================
@pytest.mark.parametrize("file_rel", sorted(repolib.model.HEADER_FILES))
def test_manifest_source_survives_its_own_sync(tmp_path: pathlib.Path, file_rel: str) -> None:
	"""Syncing a template stub onto a copy of itself changes nothing.

	This is the fixed point the bucket depends on: a freshly seeded consumer must
	report 'unchanged' on its next sync rather than churning every run.
	"""
	source_path = os.path.join(REPO_ROOT, file_rel)
	dest = tmp_path / os.path.basename(file_rel)
	dest.write_text(pathlib.Path(source_path).read_text(encoding='utf-8'), encoding='utf-8')
	counters = repolib.console.init_counters()
	outcome = repolib.header_sync.sync_vendored_header(source_path, str(dest), False, counters)
	assert outcome == 'unchanged'


#============================================
def test_generic_file_carries_no_documentation_assumptions(tmp_path: pathlib.Path) -> None:
	"""An unrelated file with unrelated sections syncs through the same helper."""
	source_text = f"# Release checklist\n\n{START}\n> Vendored checklist header.\n{END}\n\n## Steps\n"
	consumer_text = "# Release checklist\n\n## Steps\n\n1. Tag the release.\n"
	source = tmp_path / "checklist_template.md"
	dest = tmp_path / "repo" / "notes" / "checklist.md"
	write_file(source, source_text)
	write_file(dest, consumer_text)
	counters = repolib.console.init_counters()
	outcome = repolib.header_sync.sync_vendored_header(str(source), str(dest), False, counters)
	expected = (
		"# Release checklist\n"
		"\n"
		f"{START}\n> Vendored checklist header.\n{END}\n"
		"\n"
		"## Steps\n"
		"\n"
		"1. Tag the release.\n"
	)
	assert (outcome, dest.read_text(encoding='utf-8')) == ('merged', expected)
