#!/usr/bin/env python3
"""Run the HEADER bucket through the real propagation CLI.

The harness builds disposable Git consumers from captured file shapes rather
than copying live repositories, so it runs on any machine. It then runs
``propagate_style_guides.py`` twice and verifies the bucket's contract: the
vendored region is refreshed, every consumer byte outside that region and its
separator survives unchanged, missing files are seeded whole, an ambiguous
marker structure is refused, and a second run changes nothing.

Run directly outside pytest:

    source source_me.sh && python3 tests/meta/e2e/e2e_header_bucket.py

Template-meta: lives under tests/meta/e2e/; never propagates; removed by reset.
"""

# Standard Library
import os
import pathlib
import subprocess
import sys
import tempfile
import importlib.util


# Anchor sys.path on the local checkout (this file's repo) so repolib imports
# regardless of cwd, then import the header helper the bucket is built on.
TEMPLATE_ROOT = pathlib.Path(subprocess.run(
	["git", "rev-parse", "--show-toplevel"],
	cwd=os.path.dirname(os.path.abspath(__file__)),
	capture_output=True,
	text=True,
	check=True,
).stdout.strip())
if str(TEMPLATE_ROOT) not in sys.path:
	sys.path.insert(0, str(TEMPLATE_ROOT))

# local repo modules (sys.path insert above ensures this resolves to the local checkout)
import repolib.header_sync
import propagate_style_guides

# ASVS 5.3.2: this path is fixed beneath the repository root resolved above.
CHANGELOG_LIB_PATH = TEMPLATE_ROOT / "devel" / "changelog_lib.py"
CHANGELOG_LIB_SPEC = importlib.util.spec_from_file_location(
	"e2e_header_bucket_changelog_lib", CHANGELOG_LIB_PATH,
)
if CHANGELOG_LIB_SPEC is None or CHANGELOG_LIB_SPEC.loader is None:
	raise RuntimeError(f"Cannot load trusted E2E helper: {CHANGELOG_LIB_PATH}")
CHANGELOG_LIB = importlib.util.module_from_spec(CHANGELOG_LIB_SPEC)
CHANGELOG_LIB_SPEC.loader.exec_module(CHANGELOG_LIB)

GUIDANCE_REL = "docs/HUMAN_GUIDANCE.md"
DECISIONS_REL = "docs/DESIGN_DECISIONS.md"
START = repolib.header_sync.HEADER_START_MARKER
END = repolib.header_sync.HEADER_END_MARKER

# A populated guidance file with no markers: the shape every surveyed repo has.
POPULATED_GUIDANCE = (
	"# Human guidance\n"
	"\n"
	"## Direct preferences\n"
	"\n"
	"- Keep it simple: avoid speculative machinery when a focused design will do.\n"
	"  A wrapped continuation line that must survive the sync intact.\n"
	"\n"
	"- Target a 16:10 desktop aspect for screenshots.\n"
)

# A file whose level-one heading is unrelated to its role; the anchor keys on the
# heading line rather than its text.
UNRELATED_TITLE_GUIDANCE = (
	"# Autonomous completion policy\n"
	"\n"
	"- Plans continue to completion while I am unavailable.\n"
)

# A file carrying an outdated vendored region that the sync replaces.
STALE_HEADER_DECISIONS = (
	"# Design decisions\n"
	"\n"
	f"{START}\n"
	"Outdated vendored wording from an earlier sync.\n"
	f"{END}\n"
	"\n"
	"## Software design\n"
	"\n"
	"### Storage stays server-side\n"
	"\n"
	"**Decision.** Keep grading on the server.\n"
)

# An unpaired marker: an ambiguous region the bucket refuses to rewrite.
BROKEN_GUIDANCE = (
	"# Human guidance\n"
	"\n"
	f"{START}\n"
	"An orphan start marker with no end marker.\n"
	"\n"
	"## Decision priority\n"
)


#============================================
def initialize_consumer(repo_root: pathlib.Path, seed_files: dict[str, str]) -> None:
	"""Create the smallest real Git consumer holding the given seed files."""
	repo_root.mkdir(parents=True)
	subprocess.run(
		["git", "init", "--quiet", str(repo_root)],
		check=True,
		capture_output=True,
		text=True,
	)
	(repo_root / "REPO_TYPE").write_text("other\n", encoding="utf-8")
	for file_rel, content in seed_files.items():
		target = repo_root / file_rel
		target.parent.mkdir(parents=True, exist_ok=True)
		target.write_text(content, encoding="utf-8")


#============================================
def run_propagation(repo_root: pathlib.Path, expect_clean: bool = True) -> subprocess.CompletedProcess[str]:
	"""
	Run the production CLI against a disposable consumer.

	Args:
		repo_root (pathlib.Path): Consumer repository to propagate into.
		expect_clean (bool): True when the run should report no errors. The
			ambiguous-marker fixture expects a reported error, so it passes False.

	Returns:
		subprocess.CompletedProcess[str]: The completed CLI run.
	"""
	command = [
		sys.executable,
		str(TEMPLATE_ROOT / "propagate_style_guides.py"),
		"-R",
		str(repo_root),
	]
	result = subprocess.run(
		command,
		cwd=TEMPLATE_ROOT,
		capture_output=True,
		text=True,
		check=False,
	)
	if expect_clean and result.returncode != 0:
		print(result.stdout, file=sys.stderr)
		print(result.stderr, file=sys.stderr)
		raise RuntimeError(f"propagate_style_guides.py failed for {repo_root}")
	return result


#============================================
def template_header(file_rel: str) -> list[str]:
	"""Return the vendored region the template currently ships for a path."""
	source_text = (TEMPLATE_ROOT / file_rel).read_text(encoding="utf-8")
	source_lines, _trailing = repolib.header_sync.split_lines(source_text)
	return repolib.header_sync.extract_header(source_lines)


#============================================
def consumer_content(text: str) -> list[str]:
	"""Return the consumer-owned lines: everything outside the region and separator.

	Splitting the file this way is what lets the harness assert byte preservation
	on the part the consumer owns while the header region is expected to change.
	"""
	lines, _trailing = repolib.header_sync.split_lines(text)
	start_index, end_index = repolib.header_sync.find_marker_lines(lines)
	if start_index < 0:
		return lines
	tail_index = repolib.header_sync.first_content_index(lines, end_index + 1)
	return lines[:start_index] + lines[tail_index:]


#============================================
def check_header_refreshed(repo_root: pathlib.Path, file_rel: str) -> None:
	"""Verify the consumer file now carries the template's current region."""
	text = (repo_root / file_rel).read_text(encoding="utf-8")
	lines, _trailing = repolib.header_sync.split_lines(text)
	region = repolib.header_sync.extract_header(lines)
	expected = template_header(file_rel)
	assert region == expected, f"{file_rel}: vendored region does not match the template"


#============================================
def check_content_preserved(before_text: str, after_text: str, file_rel: str) -> None:
	"""Verify every consumer-owned line survived the sync unchanged."""
	before_lines = consumer_content(before_text)
	after_lines = consumer_content(after_text)
	# Blank lines around the old anchor belong to the separator, which the bucket
	# owns; compare the non-blank consumer lines that carry the actual content.
	before_content = [line for line in before_lines if line.strip()]
	after_content = [line for line in after_lines if line.strip()]
	message = (
		f"{file_rel}: consumer content changed\n"
		f"  before: {len(before_content)} content lines\n"
		f"  after:  {len(after_content)} content lines"
	)
	assert before_content == after_content, message


#============================================
def check_changelog_compatible(repo_root: pathlib.Path) -> None:
	"""Verify the generated entry parses cleanly through changelog_lib."""
	changelog_path = repo_root / "docs" / "CHANGELOG.md"
	_blocks, entries, warnings = CHANGELOG_LIB.parse_file(
		str(changelog_path), strict=True, duplicate_policy="raise",
	)
	matching_entries = [
		entry for entry in entries
		if entry.title == propagate_style_guides.CHANGELOG_TITLE
	]
	assert warnings == [], f"generated changelog warnings: {warnings}"
	assert len(matching_entries) == 1, "expected exactly one propagation changelog entry"


#============================================
def check_populated_repo(parent: pathlib.Path) -> None:
	"""A repo with existing guidance keeps its entries and gains a header."""
	repo_root = parent / "populated"
	seed = {GUIDANCE_REL: POPULATED_GUIDANCE, DECISIONS_REL: STALE_HEADER_DECISIONS}
	initialize_consumer(repo_root, seed)
	run_propagation(repo_root)
	for file_rel, before_text in seed.items():
		after_text = (repo_root / file_rel).read_text(encoding="utf-8")
		check_header_refreshed(repo_root, file_rel)
		check_content_preserved(before_text, after_text, file_rel)
	check_changelog_compatible(repo_root)
	second_pass_before = {
		file_rel: (repo_root / file_rel).read_text(encoding="utf-8")
		for file_rel in (*seed, "docs/CHANGELOG.md", ".gitignore")
	}
	run_propagation(repo_root)
	for file_rel, expected_text in second_pass_before.items():
		actual_text = (repo_root / file_rel).read_text(encoding="utf-8")
		assert actual_text == expected_text, f"{file_rel}: second run changed the file"


#============================================
def check_unrelated_title_repo(parent: pathlib.Path) -> None:
	"""A file whose heading text is unrelated still anchors correctly."""
	repo_root = parent / "unrelated-title"
	initialize_consumer(repo_root, {GUIDANCE_REL: UNRELATED_TITLE_GUIDANCE})
	run_propagation(repo_root)
	text = (repo_root / GUIDANCE_REL).read_text(encoding="utf-8")
	lines, _trailing = repolib.header_sync.split_lines(text)
	assert lines[0] == "# Autonomous completion policy", "consumer heading was rewritten"
	check_header_refreshed(repo_root, GUIDANCE_REL)
	check_content_preserved(UNRELATED_TITLE_GUIDANCE, text, GUIDANCE_REL)


#============================================
def check_empty_repo(parent: pathlib.Path) -> None:
	"""A repo lacking both files receives the template stubs verbatim."""
	repo_root = parent / "empty"
	initialize_consumer(repo_root, {})
	run_propagation(repo_root)
	for file_rel in (GUIDANCE_REL, DECISIONS_REL):
		seeded = (repo_root / file_rel).read_text(encoding="utf-8")
		expected = (TEMPLATE_ROOT / file_rel).read_text(encoding="utf-8")
		assert seeded == expected, f"{file_rel}: seeded copy differs from the template stub"


#============================================
def check_broken_marker_repo(parent: pathlib.Path) -> None:
	"""An ambiguous marker structure is refused and the file is left alone."""
	repo_root = parent / "broken-marker"
	initialize_consumer(repo_root, {GUIDANCE_REL: BROKEN_GUIDANCE})
	# An ambiguous region is a reported error, so the CLI exits non-zero by design.
	result = run_propagation(repo_root, expect_clean=False)
	after_text = (repo_root / GUIDANCE_REL).read_text(encoding="utf-8")
	assert after_text == BROKEN_GUIDANCE, "file with an unpaired marker was rewritten"
	assert "header region ambiguous" in result.stdout, "ambiguous region was not reported"


#============================================
def main() -> None:
	"""Build disposable consumers and verify the HEADER bucket contract."""
	with tempfile.TemporaryDirectory(prefix="header-bucket-e2e-") as temporary_dir:
		parent = pathlib.Path(temporary_dir)
		check_populated_repo(parent)
		check_unrelated_title_repo(parent)
		check_empty_repo(parent)
		check_broken_marker_repo(parent)

	print("PASS: the HEADER bucket refreshed vendored regions and preserved consumer content.")


if __name__ == "__main__":
	main()
