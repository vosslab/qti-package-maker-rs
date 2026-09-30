"""Behavior tests for managed universal development requirements."""

# Standard Library
import pathlib

# PIP3 modules
import pytest

# local repo modules
import repolib.console
import repolib.requirements_sync


#============================================


def canonical_source(pytest_spec: str = "pytest") -> str:
	"""Return a small canonical requirements source with both ownership markers."""
	text = (
		f"{repolib.requirements_sync.UNIVERSAL_HEADER}\n"
		"packaging  # requirement parsing\n"
		f"{pytest_spec}  # test runner\n"
		"\n"
		f"{repolib.requirements_sync.LOCAL_HEADER}\n"
	)
	return text


#============================================


def test_marker_free_migration_preserves_local_content_order() -> None:
	"""Universal duplicates move out while comments, directives, and local packages stay ordered."""
	dest_text = (
		"# Existing repository dependencies\n"
		"pytest>=7  # old local pin\n"
		"numpy==2\n"
		"--extra-index-url https://packages.example.invalid/simple\n"
		"# Platform helper\n"
		"pyobjc; sys_platform == 'darwin'\n"
	)
	result = repolib.requirements_sync.render_synced_text(canonical_source(), dest_text)
	local_content = result.split(repolib.requirements_sync.LOCAL_HEADER, 1)[1]
	assert local_content == (
		"\n\n# Existing repository dependencies\n"
		"numpy==2\n"
		"--extra-index-url https://packages.example.invalid/simple\n"
		"# Platform helper\n"
		"pyobjc; sys_platform == 'darwin'\n"
	)


#============================================


def test_managed_spec_wins_canonical_name_duplicates() -> None:
	"""Extras and normalized spelling still identify a template-owned package."""
	dest_text = "Packaging[docs]>=20\nrepository-only==1\n"
	result = repolib.requirements_sync.render_synced_text(canonical_source(), dest_text)
	local_content = result.split(repolib.requirements_sync.LOCAL_HEADER, 1)[1]
	assert "Packaging[docs]" not in local_content
	assert "repository-only==1" in local_content


#============================================


def test_later_update_replaces_only_the_managed_block() -> None:
	"""Once marked, a sync refreshes universal lines and retains the local tail verbatim."""
	old_text = canonical_source("pytest<9") + "\n# Keep this comment\nnumpy==2\n"
	new_text = repolib.requirements_sync.render_synced_text(
		canonical_source("pytest>=9"), old_text,
	)
	assert "pytest>=9" in new_text and "pytest<9" not in new_text
	assert new_text.endswith("\n# Keep this comment\nnumpy==2\n")


#============================================


def test_marked_local_duplicate_yields_to_managed_specification() -> None:
	"""Marked consumers cannot retain a conflicting universal package below LOCAL."""
	dest_text = canonical_source() + "\n# Local note\npytest<9\nnumpy==2\n"
	result = repolib.requirements_sync.render_synced_text(canonical_source("pytest>=9"), dest_text)
	local_content = result.split(repolib.requirements_sync.LOCAL_HEADER, 1)[1]
	assert "pytest<9" not in local_content
	assert local_content.endswith("\n# Local note\nnumpy==2\n")


#============================================


def test_synced_output_is_idempotent() -> None:
	"""A second requirements synchronization has no further text change."""
	first = repolib.requirements_sync.render_synced_text(
		canonical_source(), "numpy==2\n",
	)
	second = repolib.requirements_sync.render_synced_text(canonical_source(), first)
	assert second == first


#============================================


def test_missing_consumer_is_seeded_from_canonical_file(tmp_path: pathlib.Path) -> None:
	"""A missing consumer file receives the complete canonical seed."""
	source = tmp_path / "source.txt"
	dest = tmp_path / "consumer" / "pip_requirements-dev.txt"
	source.write_text(canonical_source(), encoding="utf-8")
	counters = repolib.console.init_counters()
	outcome = repolib.requirements_sync.sync_development_requirements(
		str(source), str(dest), False, counters,
	)
	assert outcome == "created"
	assert dest.read_text(encoding="utf-8") == canonical_source()


#============================================


@pytest.mark.parametrize(
	"broken_text",
	[
		f"{repolib.requirements_sync.UNIVERSAL_HEADER}\npytest\n",
		(
			f"{repolib.requirements_sync.UNIVERSAL_HEADER}\n"
			f"{repolib.requirements_sync.UNIVERSAL_HEADER}\n"
			f"{repolib.requirements_sync.LOCAL_HEADER}\n"
		),
		(
			"# === UNIVERSAL DEVELOPMENT DEPENDENCIES === [LOCAL]\n"
			f"{repolib.requirements_sync.LOCAL_HEADER}\n"
		),
	],
)
def test_malformed_markers_are_refused_without_writing(
	tmp_path: pathlib.Path,
	broken_text: str,
) -> None:
	"""Unpaired, duplicate, or malformed ownership markers leave the consumer untouched."""
	source = tmp_path / "source.txt"
	dest = tmp_path / "pip_requirements-dev.txt"
	source.write_text(canonical_source(), encoding="utf-8")
	dest.write_text(broken_text, encoding="utf-8")
	counters = repolib.console.init_counters()
	outcome = repolib.requirements_sync.sync_development_requirements(
		str(source), str(dest), False, counters,
	)
	assert outcome == "error"
	assert dest.read_text(encoding="utf-8") == broken_text
