# Standard Library
import pathlib
import importlib.util

# PIP3 modules
import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]

# ASVS 5.3.2: this fixed path is derived only from the repository-owned test.
SCRIPT_PATH = REPO_ROOT / "devel" / "flatten_broken_md_links.py"
SPEC = importlib.util.spec_from_file_location(
	"test_flatten_broken_md_links_module", SCRIPT_PATH,
)
if SPEC is None or SPEC.loader is None:
	raise RuntimeError(f"Cannot load trusted test script: {SCRIPT_PATH}")
FLATTEN_BROKEN_MD_LINKS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FLATTEN_BROKEN_MD_LINKS)


#============================================
def test_relative_pattern_anchors_at_repo_root(tmp_path: pathlib.Path) -> None:
	"""A repo-relative pattern hangs off the repo root, not the CWD."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	doc = docs_dir / "usage.md"
	doc.write_text("# usage\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/*.md"])
	assert source_files == [doc.resolve()]


#============================================
def test_pattern_escaping_the_repo_root_is_rejected(tmp_path: pathlib.Path) -> None:
	"""A '..' traversal pattern raises instead of reaching outside the repo."""
	with pytest.raises(ValueError, match="outside the repo root"):
		FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
			tmp_path, ["../../*.md"])


#============================================
def test_double_star_pattern_reaches_nested_markdown(tmp_path: pathlib.Path) -> None:
	"""'**' spans directories, which needs glob's recursive mode."""
	nested_dir = tmp_path / "docs" / "specs"
	nested_dir.mkdir(parents=True)
	nested_doc = nested_dir / "format.md"
	nested_doc.write_text("# format\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/**/*.md"])
	assert source_files == [nested_doc.resolve()]


#============================================
def test_single_star_pattern_stays_at_one_level(tmp_path: pathlib.Path) -> None:
	"""'docs/*.md' is top-level only, so a nested doc stays out of scope."""
	nested_dir = tmp_path / "docs" / "specs"
	nested_dir.mkdir(parents=True)
	(nested_dir / "format.md").write_text("# format\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/*.md"])
	assert source_files == []


#============================================
def test_bare_directory_walks_recursively(tmp_path: pathlib.Path) -> None:
	"""Naming a folder means every markdown under it, at any depth."""
	nested_dir = tmp_path / "docs" / "specs"
	nested_dir.mkdir(parents=True)
	nested_doc = nested_dir / "format.md"
	nested_doc.write_text("# format\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs"])
	assert source_files == [nested_doc.resolve()]


#============================================
def test_trailing_slash_does_not_change_the_scope(tmp_path: pathlib.Path) -> None:
	"""'docs/specs' and 'docs/specs/' name the same directory."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	doc = docs_dir / "usage.md"
	doc.write_text("# usage\n", encoding="utf-8")
	without_slash = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs"])
	with_slash = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/"])
	assert with_slash == without_slash == [doc.resolve()]


#============================================
def test_non_markdown_matches_are_filtered_out(tmp_path: pathlib.Path) -> None:
	"""A wide pattern hands only markdown to the link rewriter."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	(docs_dir / "helper.py").write_text("x = 1\n", encoding="utf-8")
	doc = docs_dir / "usage.md"
	doc.write_text("# usage\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/*"])
	assert source_files == [doc.resolve()]


#============================================
def test_overlapping_patterns_yield_each_file_once(tmp_path: pathlib.Path) -> None:
	"""A file matched by two patterns is rewritten once, not twice."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	changelog = docs_dir / "CHANGELOG.md"
	changelog.write_text("# changelog\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, ["docs/*.md", "docs/CHANGELOG*.md"])
	assert source_files == [changelog.resolve()]


#============================================
def test_missing_archive_yields_no_files(tmp_path: pathlib.Path) -> None:
	"""A repo that has never rotated its changelog has no archive to walk."""
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, [FLATTEN_BROKEN_MD_LINKS.DEFAULT_GLOB])
	assert source_files == []


#============================================
def test_default_pattern_finds_archive_markdown(tmp_path: pathlib.Path) -> None:
	"""When docs/archive/ exists, the default scope picks its markdown up."""
	archive_dir = tmp_path / "docs" / "archive"
	archive_dir.mkdir(parents=True)
	archived = archive_dir / "CHANGELOG-2026-06a.md"
	archived.write_text("# archived\n", encoding="utf-8")
	source_files = FLATTEN_BROKEN_MD_LINKS.collect_markdown_files(
		tmp_path, [FLATTEN_BROKEN_MD_LINKS.DEFAULT_GLOB])
	assert source_files == [archived.resolve()]
