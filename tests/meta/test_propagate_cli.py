"""Tests for the propagate_style_guides.py CLI surface and source-vs-target safety.

Coverage areas:
- Source-vs-target: build_context_for_repo target_dir is never the same as source_dir.
"""

import pathlib
import importlib.util

import repolib.process
import propagate_style_guides

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]

# ASVS 5.3.2: this fixed path is derived only from the repository-owned test.
CHANGELOG_LIB_PATH = REPO_ROOT / "devel" / "changelog_lib.py"
CHANGELOG_LIB_SPEC = importlib.util.spec_from_file_location(
	"test_propagate_cli_changelog_lib", CHANGELOG_LIB_PATH,
)
if CHANGELOG_LIB_SPEC is None or CHANGELOG_LIB_SPEC.loader is None:
	raise RuntimeError(f"Cannot load trusted test helper: {CHANGELOG_LIB_PATH}")
CHANGELOG_LIB = importlib.util.module_from_spec(CHANGELOG_LIB_SPEC)
CHANGELOG_LIB_SPEC.loader.exec_module(CHANGELOG_LIB)


#============================================
# Source-vs-target safety
#============================================

def test_build_context_target_is_not_source(tmp_path: pathlib.Path) -> None:
	"""The target repo path passed to process_repo is distinct from source_dir."""
	target_repo = tmp_path / "consumer_repo"
	target_repo.mkdir()

	context = repolib.process.build_context_for_repo(
		repo_path=str(target_repo),
		dry_run=True,
		initial_setup=False,
		auto_discover=False,
		write_marker=False,
	)

	# source and target must never be the same directory
	assert context.source_dir != str(target_repo.resolve())


#============================================
# Consumer changelog recording
#============================================

def test_record_propagation_changelog_creates_canonical_entry(
		tmp_path: pathlib.Path,
		) -> None:
	"""A new consumer changelog parses cleanly through changelog_lib."""
	repo_dir = tmp_path / "consumer_repo"
	(repo_dir / "docs").mkdir(parents=True)
	changelog_path = propagate_style_guides.record_propagation_changelog(
		str(repo_dir), "2099-01-02",
	)

	_blocks, entries, warnings = CHANGELOG_LIB.parse_file(
		changelog_path, strict=True, duplicate_policy="raise",
	)
	assert warnings == []
	assert [(entry.category, entry.title) for entry in entries] == [(
		propagate_style_guides.CHANGELOG_CATEGORY,
		propagate_style_guides.CHANGELOG_TITLE,
	)]


def test_record_propagation_changelog_keeps_canonical_category_order(
		tmp_path: pathlib.Path,
		) -> None:
	"""Today's new maintenance bullet lands before later canonical sections."""
	repo_dir = tmp_path / "consumer_repo"
	docs_dir = repo_dir / "docs"
	docs_dir.mkdir(parents=True)
	changelog_path = docs_dir / "CHANGELOG.md"
	changelog_path.write_text(
		"## 2099-01-02\n\n"
		"### Additions and New Features\n\n"
		"- Existing addition.\n\n"
		"### Decisions and Failures\n\n"
		"- Existing decision.\n",
		encoding="utf-8",
	)
	propagate_style_guides.record_propagation_changelog(
		str(repo_dir), "2099-01-02",
	)

	_blocks, entries, warnings = CHANGELOG_LIB.parse_file(
		str(changelog_path), strict=True, duplicate_policy="raise",
	)
	assert warnings == []
	assert [entry.category for entry in entries] == [
		"Additions and New Features",
		"Fixes and Maintenance",
		"Decisions and Failures",
	]


def test_record_propagation_changelog_appends_to_existing_category(
		tmp_path: pathlib.Path,
		) -> None:
	"""An existing maintenance category receives the new bullet once."""
	repo_dir = tmp_path / "consumer_repo"
	docs_dir = repo_dir / "docs"
	docs_dir.mkdir(parents=True)
	changelog_path = docs_dir / "CHANGELOG.md"
	changelog_path.write_text(
		"## 2099-01-02\n\n"
		"### Fixes and Maintenance\n\n"
		"- Existing fix.\n\n"
		"### Developer Tests and Notes\n\n"
		"- Existing test note.\n",
		encoding="utf-8",
	)
	propagate_style_guides.record_propagation_changelog(
		str(repo_dir), "2099-01-02",
	)

	_blocks, entries, warnings = CHANGELOG_LIB.parse_file(
		str(changelog_path), strict=True, duplicate_policy="raise",
	)
	assert warnings == []
	assert [entry.title for entry in entries] == [
		"Existing fix.",
		propagate_style_guides.CHANGELOG_TITLE,
		"Existing test note.",
	]
