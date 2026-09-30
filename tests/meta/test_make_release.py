"""Pure and stdlib-only unit tests for templates/shared/devel/make_release.py.

This is the FAST pytest tier. make_release.py's release path drives real git
subprocess (git archive, git tag --list, git cat-file); PYTEST_STYLE.md forbids
real subprocess CLI round-trips and slow tests in pytest. That happy path lives
in the E2E tier instead (tests/meta/e2e/e2e_make_release.py). This file covers
only logic that is pure, stdlib-only, or driven through a monkeypatched
changelog_lib.run_git so no real git process ever runs.

Covered here:
 - check_calver_freshness: well-formed passes, malformed raises (pure string logic)
 - ensure_tag_free / ensure_committed_license: missing, unsupported-name, mode, and pass cases
 - compute_change_range: initial-release fallback (run_git faked)
 - build_llm_prompt: version token and notes guidance present (compute faked)
 - build_tag_command / build_gh_release_command / build_archive_arg_lists: builders
 - verify_archive_license: stdlib zip and tgz, byte-match passes, mismatch raises
 - prepend_release_history / prepend_news: prepend preserves older bytes; repeat raises
"""

# Standard Library
import io
import os
import re
import zipfile
import tarfile
import pathlib
import subprocess
import importlib.util
import collections.abc

# PIP3 modules
import pytest

# local repo modules
import changelog_lib
import file_utils
import repolib.reset_answers

REPO_ROOT = file_utils.get_repo_root()

# Load the shared canonical copy of make_release by file path so tests always
# exercise the shared template, not a locally installed version.
_MAKE_RELEASE_PATH = os.path.join(
	REPO_ROOT, "templates", "shared", "devel", "make_release.py"
)
_spec = importlib.util.spec_from_file_location("make_release", _MAKE_RELEASE_PATH)
make_release = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(make_release)


#============================================
# Fakes for monkeypatching changelog_lib.run_git (no real subprocess runs)

def _fake_run_git_empty(args: list[str]) -> subprocess.CompletedProcess:
	"""Simulate a git command that returns empty stdout and exit 0."""
	return subprocess.CompletedProcess(args, 0, stdout="", stderr="")


def _fake_run_git_tag_exists(args: list[str]) -> subprocess.CompletedProcess:
	"""Simulate git tag --list returning the requested tag name (tag exists)."""
	if args[:2] == ["tag", "--list"] and len(args) >= 3:
		tag_name = args[2]
		return subprocess.CompletedProcess(args, 0, stdout=tag_name + "\n", stderr="")
	return subprocess.CompletedProcess(args, 0, stdout="", stderr="")


def _fake_run_git_license_missing(args: list[str]) -> subprocess.CompletedProcess:
	"""Simulate a committed root tree with no policy-named license."""
	return subprocess.CompletedProcess(args, 0, stdout="", stderr="")


def _fake_run_git_license_present(args: list[str]) -> subprocess.CompletedProcess:
	"""Simulate two regular policy-named licenses in the committed root tree."""
	stdout = (
		"100644 blob 1111111\tLICENSE.CC-BY-4.0\n"
		"100644 blob 2222222\tLICENSE.MIT\n"
	)
	return subprocess.CompletedProcess(args, 0, stdout=stdout, stderr="")


def _fake_run_git_license_entry(
	path: str, mode: str = "100644",
) -> collections.abc.Callable[[list[str]], subprocess.CompletedProcess]:
	"""Build a run_git fake with one committed root license entry."""
	def fake_run_git(args: list[str]) -> subprocess.CompletedProcess:
		stdout = f"{mode} blob 1111111\t{path}\n"
		return subprocess.CompletedProcess(args, 0, stdout=stdout, stderr="")
	return fake_run_git


def _fake_compute_no_prior_tag() -> dict:
	"""Simulate compute_change_range when no prior v* tag exists."""
	return {"prev_tag": None, "prev_date": None, "log_command": "git log"}


#============================================
# CalVer freshness (pure string logic)

def test_check_calver_freshness_raises_on_no_dot_separator() -> None:
	"""A version string with no dot separator raises RuntimeError."""
	with pytest.raises(RuntimeError, match="Malformed"):
		make_release.check_calver_freshness("not-a-version")

def test_check_calver_freshness_raises_on_non_numeric_month() -> None:
	"""Non-numeric month part raises RuntimeError."""
	with pytest.raises(RuntimeError, match="Malformed"):
		make_release.check_calver_freshness("26.XX")

def test_check_calver_freshness_accepts_well_formed_version() -> None:
	"""A well-formed YY.MM version does not raise (may warn about month mismatch)."""
	# 24.01 is a legitimate past CalVer month; it may print a warning but must not raise.
	make_release.check_calver_freshness("24.01")


#============================================
# Tag-free check (run_git faked, no real subprocess)

def test_ensure_tag_free_raises_when_tag_exists(monkeypatch: pytest.MonkeyPatch) -> None:
	"""ensure_tag_free raises RuntimeError when the version tag already exists."""
	monkeypatch.setattr(changelog_lib, "run_git", _fake_run_git_tag_exists)
	with pytest.raises(RuntimeError, match="already exists"):
		make_release.ensure_tag_free("26.07")

def test_ensure_tag_free_passes_when_tag_absent(monkeypatch: pytest.MonkeyPatch) -> None:
	"""ensure_tag_free does not raise when the tag is absent from the repo."""
	monkeypatch.setattr(changelog_lib, "run_git", _fake_run_git_empty)
	# No exception means the tag-free check passed.
	make_release.ensure_tag_free("26.07")


#============================================
# Committed-LICENSE-present check (run_git faked)

def test_release_license_allow_list_matches_reset_catalog() -> None:
	"""The propagated release contract stays synchronized with reset choices."""
	identifiers = repolib.reset_answers.CODE_LICENSES + [
		spdx for spdx in repolib.reset_answers.DOCS_LICENSES if spdx != "none"
	]
	expected_filenames = frozenset(f"LICENSE.{spdx}" for spdx in identifiers)
	assert make_release.SUPPORTED_LICENSE_FILENAMES == expected_filenames

def test_ensure_committed_license_raises_when_missing(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""ensure_committed_license raises when HEAD has no LICENSE.<SPDX> file."""
	monkeypatch.setattr(changelog_lib, "run_git", _fake_run_git_license_missing)
	with pytest.raises(RuntimeError, match=r"No committed LICENSE\.<SPDX>"):
		make_release.ensure_committed_license()

def test_ensure_committed_license_passes_when_present(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""ensure_committed_license returns every regular LICENSE.<SPDX> file."""
	monkeypatch.setattr(changelog_lib, "run_git", _fake_run_git_license_present)
	paths = make_release.ensure_committed_license()
	assert paths == ["LICENSE.CC-BY-4.0", "LICENSE.MIT"]


def test_ensure_committed_license_rejects_markdown_alias(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A compound LICENSE.MIT.md alias fails the release filename contract."""
	monkeypatch.setattr(
		changelog_lib, "run_git", _fake_run_git_license_entry("LICENSE.MIT.md"),
	)
	with pytest.raises(RuntimeError, match="Unsupported root license filename"):
		make_release.ensure_committed_license()


@pytest.mark.parametrize("filename", ["LICENSE", "LICENSE.MIT.adoc"])
def test_ensure_committed_license_rejects_other_unsupported_names(
	monkeypatch: pytest.MonkeyPatch, filename: str,
) -> None:
	"""Generic and unlisted-rendering names fail the release contract."""
	monkeypatch.setattr(
		changelog_lib, "run_git", _fake_run_git_license_entry(filename),
	)
	with pytest.raises(RuntimeError, match="Unsupported root license filename"):
		make_release.ensure_committed_license()


def test_ensure_committed_license_rejects_symlink(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A committed symlink cannot stand in for the real license body."""
	monkeypatch.setattr(
		changelog_lib, "run_git", _fake_run_git_license_entry("LICENSE.MIT", "120000"),
	)
	with pytest.raises(RuntimeError, match="symlinks are not releasable"):
		make_release.ensure_committed_license()


def test_ensure_committed_license_rejects_executable_file(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An executable regular file cannot stand in for a license body."""
	monkeypatch.setattr(
		changelog_lib, "run_git", _fake_run_git_license_entry("LICENSE.MIT", "100755"),
	)
	with pytest.raises(RuntimeError, match="regular, non-executable"):
		make_release.ensure_committed_license()


#============================================
# compute_change_range fallback (run_git faked)

def test_compute_change_range_initial_release_fallback(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""compute_change_range returns None prev_tag when no v* tags exist in the repo."""
	monkeypatch.setattr(changelog_lib, "run_git", _fake_run_git_empty)
	result = make_release.compute_change_range()
	# No prior tag: prev_tag must be None and log_command must cover full history.
	assert result["prev_tag"] is None
	assert "git log" in result["log_command"]


#============================================
# LLM prompt builder (compute_change_range faked so no git runs)

def test_build_llm_prompt_contains_version_and_notes_guidance(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""build_llm_prompt includes the v{version} token and Markdown notes guidance."""
	monkeypatch.setattr(make_release, "compute_change_range", _fake_compute_no_prior_tag)
	prompt = make_release.build_llm_prompt("26.07", "my-repo")
	# The version token must appear so the LLM stamps the right release.
	assert "v26.07" in prompt
	# Notes-writing guidance must steer the LLM toward Markdown release notes.
	assert "Markdown" in prompt

def test_build_llm_prompt_includes_repo_name(monkeypatch: pytest.MonkeyPatch) -> None:
	"""build_llm_prompt includes the repository name in the prompt."""
	monkeypatch.setattr(make_release, "compute_change_range", _fake_compute_no_prior_tag)
	prompt = make_release.build_llm_prompt("26.07", "my-test-repo")
	assert "my-test-repo" in prompt

def test_build_llm_prompt_initial_release_fallback(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""build_llm_prompt uses the initial-release text when no prior v* tag exists."""
	monkeypatch.setattr(make_release, "compute_change_range", _fake_compute_no_prior_tag)
	prompt = make_release.build_llm_prompt("26.07", "my-repo")
	# Fallback text must surface when there is no previous tag.
	assert "initial release" in prompt.lower() or "full repo history" in prompt.lower()

def test_build_llm_prompt_prior_tag_path(monkeypatch: pytest.MonkeyPatch) -> None:
	"""build_llm_prompt includes the prior tag when a previous v* tag exists."""
	def fake_compute_prior_tag() -> dict:
		return {
			"prev_tag": "v26.05",
			"prev_date": "2026-05-01",
			"log_command": "git log v26.05..HEAD",
		}
	monkeypatch.setattr(make_release, "compute_change_range", fake_compute_prior_tag)
	prompt = make_release.build_llm_prompt("26.07", "my-repo")
	# Prior tag must appear in the prompt so the LLM knows the change window.
	assert "v26.05" in prompt


#============================================
# Command builders (pure: assert the exact produced command string)

def test_build_tag_command_produces_annotated_tag_command() -> None:
	"""build_tag_command produces the exact annotated git tag command for the version."""
	cmd = make_release.build_tag_command("26.07")
	assert cmd == 'git tag -a v26.07 -m "v26.07"'

def test_build_gh_release_command_produces_create_command() -> None:
	"""build_gh_release_command produces the exact gh release create command."""
	cmd = make_release.build_gh_release_command(
		"26.07", "notes.md", "dist/repo-v26.07.zip", "dist/repo-v26.07.tgz"
	)
	expected = (
		'gh release create v26.07 --title "v26.07" '
		'--notes-file notes.md -- '
		'dist/repo-v26.07.zip dist/repo-v26.07.tgz'
	)
	assert cmd == expected

def test_build_archive_arg_lists_select_zip_and_targz_formats() -> None:
	"""build_archive_arg_lists yields git archive arg lists targeting each output path."""
	arg_lists = make_release.build_archive_arg_lists(
		"repo-v26.07/", "dist/repo-v26.07.zip", "dist/repo-v26.07.tgz"
	)
	# The zip list must request the zip format and write to the zip path.
	assert "--format=zip" in arg_lists["zip"]
	assert "dist/repo-v26.07.zip" in arg_lists["zip"]
	# The tgz list must request tar.gz and write to the tgz path.
	assert "--format=tar.gz" in arg_lists["tgz"]
	assert "dist/repo-v26.07.tgz" in arg_lists["tgz"]


#============================================
# verify_archive_license (stdlib zip/tgz built in tmp_path; no git)

def test_verify_archive_license_zip_passes_on_match(tmp_path: pathlib.Path) -> None:
	"""verify_archive_license accepts a matching zip LICENSE.MIT member."""
	license_bytes = b"MIT License\n"
	zip_path = str(tmp_path / "test.zip")
	with zipfile.ZipFile(zip_path, "w") as zf:
		zf.writestr("prefix/LICENSE.MIT", license_bytes)
	# No exception means the byte-match passed.
	make_release.verify_archive_license(zip_path, "prefix/LICENSE.MIT", license_bytes)

def test_verify_archive_license_zip_raises_on_mismatch(tmp_path: pathlib.Path) -> None:
	"""verify_archive_license raises RuntimeError when zip LICENSE differs from expected."""
	zip_path = str(tmp_path / "test.zip")
	with zipfile.ZipFile(zip_path, "w") as zf:
		zf.writestr("prefix/LICENSE.MIT", b"Different content\n")
	with pytest.raises(RuntimeError, match="does not match"):
		make_release.verify_archive_license(zip_path, "prefix/LICENSE.MIT", b"MIT License\n")

def test_verify_archive_license_tgz_passes_on_match(tmp_path: pathlib.Path) -> None:
	"""verify_archive_license accepts a matching tgz LICENSE.MIT member."""
	license_bytes = b"MIT License\n"
	tgz_path = str(tmp_path / "test.tgz")
	with tarfile.open(tgz_path, "w:gz") as tf:
		info = tarfile.TarInfo(name="prefix/LICENSE.MIT")
		info.size = len(license_bytes)
		tf.addfile(info, io.BytesIO(license_bytes))
	# No exception means the byte-match passed.
	make_release.verify_archive_license(tgz_path, "prefix/LICENSE.MIT", license_bytes)

def test_verify_archive_license_tgz_raises_on_mismatch(tmp_path: pathlib.Path) -> None:
	"""verify_archive_license raises RuntimeError when tgz LICENSE differs from expected."""
	wrong_bytes = b"GPL content\n"
	tgz_path = str(tmp_path / "test.tgz")
	with tarfile.open(tgz_path, "w:gz") as tf:
		info = tarfile.TarInfo(name="prefix/LICENSE.MIT")
		info.size = len(wrong_bytes)
		tf.addfile(info, io.BytesIO(wrong_bytes))
	with pytest.raises(RuntimeError, match="does not match"):
		make_release.verify_archive_license(tgz_path, "prefix/LICENSE.MIT", b"MIT License\n")


#============================================
# Doc writers: prepend preserves older bytes; repeat-version heading raises.
# Use version 26.07 (not the live VERSION) so the writer never collides with a
# real seeded entry, and seed an OLDER 26.05 entry to check prepend ordering.

def test_release_history_writer_prepends_above_and_preserves_older(
	tmp_path: pathlib.Path,
) -> None:
	"""prepend_release_history inserts the new heading above the old, preserving its bytes."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	history_file = docs_dir / "RELEASE_HISTORY.md"
	older_entry = "## v26.05 - 2026-05-01\n\nOlder entry body.\n"
	history_file.write_text("# Release History\n\n" + older_entry, encoding="utf-8")
	notes_file = tmp_path / "notes.md"
	notes_file.write_text("Added new features.\n", encoding="utf-8")
	make_release.prepend_release_history(str(tmp_path), "26.07", str(notes_file))
	result = history_file.read_text(encoding="utf-8")
	# The new entry must sit ABOVE the older entry (prepend ordering).
	assert result.index("v26.07") < result.index("v26.05")
	# The older entry's bytes must be preserved verbatim.
	assert older_entry in result

def test_release_history_writer_raises_on_duplicate_version(
	tmp_path: pathlib.Path,
) -> None:
	"""prepend_release_history raises and names the file:line when the version repeats."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	history_file = docs_dir / "RELEASE_HISTORY.md"
	history_file.write_text(
		"# Release History\n\n## v26.07 - 2026-07-01\n\nExisting entry.\n",
		encoding="utf-8",
	)
	notes_file = tmp_path / "notes.md"
	notes_file.write_text("Duplicate notes.\n", encoding="utf-8")
	with pytest.raises(RuntimeError, match="already has an entry") as excinfo:
		make_release.prepend_release_history(str(tmp_path), "26.07", str(notes_file))
	# The error must name the target file and a 1-based line number for the human.
	message = str(excinfo.value)
	assert re.search(re.escape(str(history_file)) + r":\d+", message)

def test_news_writer_prepends_above_and_preserves_older(tmp_path: pathlib.Path) -> None:
	"""prepend_news inserts the new heading above the old, preserving its bytes."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	news_file = docs_dir / "NEWS.md"
	older_entry = "## v26.05 - 2026-05-01\n\nOlder announcement body.\n"
	news_file.write_text("# News\n\n" + older_entry, encoding="utf-8")
	notes_file = tmp_path / "notes.md"
	notes_file.write_text("Big launch.\n", encoding="utf-8")
	make_release.prepend_news(str(tmp_path), "26.07", str(notes_file))
	result = news_file.read_text(encoding="utf-8")
	# The new entry must sit ABOVE the older entry (prepend ordering).
	assert result.index("v26.07") < result.index("v26.05")
	# The older entry's bytes must be preserved verbatim.
	assert older_entry in result

def test_news_writer_raises_on_duplicate_version(tmp_path: pathlib.Path) -> None:
	"""prepend_news raises and names the file:line when the version repeats in NEWS.md."""
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	news_file = docs_dir / "NEWS.md"
	news_file.write_text(
		"# News\n\n## v26.07 - 2026-07-01\n\nExisting announcement.\n",
		encoding="utf-8",
	)
	notes_file = tmp_path / "notes.md"
	notes_file.write_text("Duplicate notes.\n", encoding="utf-8")
	with pytest.raises(RuntimeError, match="already has an entry") as excinfo:
		make_release.prepend_news(str(tmp_path), "26.07", str(notes_file))
	# The error must name the target file and a 1-based line number for the human.
	message = str(excinfo.value)
	assert re.search(re.escape(str(news_file)) + r":\d+", message)


if __name__ == "__main__":
	pytest.main([__file__, "-v"])
