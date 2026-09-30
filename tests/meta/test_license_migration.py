"""Behavior tests for content-aware root license migration."""

# Standard Library
import os
import pathlib

# local repo modules
import file_utils
import repolib.console
import repolib.license_migration


TEMPLATE_ROOT = pathlib.Path(file_utils.get_repo_root())


#============================================
def migrate(repo_root: pathlib.Path, dry_run: bool = False) -> None:
	"""Run the production migration against one temporary consumer root."""
	counters = repolib.console.init_counters()
	repolib.license_migration.migrate_legacy_licenses(
		str(repo_root), str(TEMPLATE_ROOT), dry_run, counters,
	)


#============================================
def test_abbreviated_template_body_is_replaced_with_complete_catalog(
	tmp_path: pathlib.Path,
) -> None:
	"""A customized copy of the old CC summary receives the full legal code."""
	legacy = tmp_path / "LICENSE.CC-BY-4.0.md"
	legacy.write_text(
		"Creative Commons Attribution 4.0 International (CC BY 4.0)\n"
		"\n"
		"Copyright (c) 2026 Neil R. Voss\n"
		"\n"
		"You are free to:\n"
		"  Share - copy and redistribute the material in any medium or format\n"
		"  Adapt - remix, transform, and build upon the material for any purpose, even commercially.\n"
		"\n"
		"Under the following terms:\n"
		"  Attribution - You must give appropriate credit, provide a link to the license, and\n"
		"  indicate if changes were made. You may do so in any reasonable manner, but not in any\n"
		"  way that suggests the licensor endorses you or your use.\n"
		"\n"
		"  No additional restrictions - You may not apply legal terms or technological measures\n"
		"  that legally restrict others from doing anything the license permits.\n"
		"\n"
		"To view a copy of this license, visit:\n"
		"https://creativecommons.org/licenses/by/4.0/\n",
		encoding="utf-8",
	)

	migrate(tmp_path)

	canonical = tmp_path / "LICENSE.CC-BY-4.0"
	expected = TEMPLATE_ROOT / "LICENSES" / "LICENSE.CC-BY-4.0"
	assert not legacy.exists()
	assert canonical.read_bytes() == expected.read_bytes()


#============================================
def test_typed_markdown_preserves_body_and_removes_generic_symlink(
	tmp_path: pathlib.Path,
) -> None:
	"""A complete/custom body moves intact before its obsolete alias is removed."""
	body = (TEMPLATE_ROOT / "LICENSES" / "LICENSE.MIT").read_bytes().replace(
		b"[year] [fullname]", b"2022 Example Owner",
	)
	legacy = tmp_path / "LICENSE.MIT.md"
	legacy.write_bytes(body)
	alias = tmp_path / "LICENSE"
	alias.symlink_to(legacy.name)

	migrate(tmp_path)

	assert (tmp_path / "LICENSE.MIT").read_bytes() == body
	assert not os.path.lexists(alias)


#============================================
def test_generic_identified_license_moves_without_rewriting(tmp_path: pathlib.Path) -> None:
	"""An unambiguous generic MIT file keeps its project-specific application text."""
	body = (TEMPLATE_ROOT / "LICENSES" / "LICENSE.MIT").read_bytes().replace(
		b"[year] [fullname]", b"2024 Example Owner",
	)
	generic = tmp_path / "LICENSE.md"
	generic.write_bytes(body)

	migrate(tmp_path)

	assert not generic.exists()
	assert (tmp_path / "LICENSE.MIT").read_bytes() == body


#============================================
def test_known_generic_cc_summary_receives_complete_legal_code(tmp_path: pathlib.Path) -> None:
	"""The exact known five-line CC summary receives the catalog legal code."""
	generic = tmp_path / "LICENSE.md"
	generic.write_text(
		"# License\n\n"
		"This work is licensed under the "
		"[Creative Commons Attribution 4.0 International License]"
		"(https://creativecommons.org/licenses/by/4.0/).\n\n"
		"**SPDX-License-Identifier:** CC-BY-4.0\n",
		encoding="utf-8",
	)

	migrate(tmp_path)

	canonical = tmp_path / "LICENSE.CC-BY-4.0"
	expected = TEMPLATE_ROOT / "LICENSES" / "LICENSE.CC-BY-4.0"
	assert not generic.exists()
	assert canonical.read_bytes() == expected.read_bytes()


#============================================
def test_conflicting_canonical_body_preserves_both_files(tmp_path: pathlib.Path) -> None:
	"""A conflict preserves both legal texts and the alias to the legacy one."""
	legacy = tmp_path / "LICENSE.LGPL_v3"
	legacy_body = (TEMPLATE_ROOT / "LICENSES" / "LICENSE.LGPL-3.0").read_bytes()
	legacy_body += b"\nAdditional legacy notice.\n"
	legacy.write_bytes(legacy_body)
	canonical = tmp_path / "LICENSE.LGPL-3.0"
	canonical_body = b"different canonical LGPL terms\n"
	canonical.write_bytes(canonical_body)
	alias = tmp_path / "LICENSE"
	alias.symlink_to(legacy.name)

	migrate(tmp_path)

	assert legacy.read_bytes() == legacy_body and alias.is_symlink()
	assert canonical.read_bytes() == canonical_body


#============================================
def test_custom_generic_license_remains_untouched(tmp_path: pathlib.Path) -> None:
	"""An unrecognized legal file is not guessed, renamed, or replaced."""
	generic = tmp_path / "LICENSE"
	generic.write_bytes(b"Project-specific license terms\n")

	migrate(tmp_path)

	assert generic.read_bytes() == b"Project-specific license terms\n"


#============================================
def test_marker_only_generic_license_is_not_promoted(tmp_path: pathlib.Path) -> None:
	"""A recognizable title without legal code remains under its generic name."""
	generic = tmp_path / "LICENSE"
	body = b"Attribution 4.0 International\n"
	generic.write_bytes(body)

	migrate(tmp_path)

	assert generic.read_bytes() == body
	assert not (tmp_path / "LICENSE.CC-BY-4.0").exists()


#============================================
def test_short_customized_spdx_notice_is_not_replaced(tmp_path: pathlib.Path) -> None:
	"""An SPDX line does not authorize overwriting additional project terms."""
	generic = tmp_path / "LICENSE.md"
	body = (
		b"GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n\n"
		b"SPDX-License-Identifier: GPL-3.0\n\nAdditional permission for this project.\n"
	)
	generic.write_bytes(body)

	migrate(tmp_path)

	assert generic.read_bytes() == body
	assert not (tmp_path / "LICENSE.GPL-3.0").exists()


#============================================
def test_dry_run_reports_without_changing_legacy_file(tmp_path: pathlib.Path) -> None:
	"""Dry-run migration leaves both filenames and bytes unchanged."""
	legacy = tmp_path / "LICENSE.GPL-3.0.md"
	body = (TEMPLATE_ROOT / "LICENSES" / "LICENSE.GPL-3.0").read_bytes()
	legacy.write_bytes(body)
	alias = tmp_path / "LICENSE"
	alias.symlink_to(legacy.name)

	migrate(tmp_path, dry_run=True)

	assert legacy.read_bytes() == body
	assert alias.is_symlink() and not (tmp_path / "LICENSE.GPL-3.0").exists()


#============================================
def test_actual_root_spelling_prevents_case_alias_double_plan(
	tmp_path: pathlib.Path,
) -> None:
	"""Only the directory entry's real CC alias spelling is planned once."""
	legacy = tmp_path / "LICENSE.CC_BY_4_0"
	legacy.write_bytes((TEMPLATE_ROOT / "LICENSES" / "LICENSE.CC-BY-4.0").read_bytes())
	counters = repolib.console.init_counters()

	actions = repolib.license_migration.migrate_legacy_licenses(
		str(tmp_path), str(TEMPLATE_ROOT), True, counters,
	)

	assert actions == 1


#============================================
def test_nested_generic_symlink_target_is_preserved(tmp_path: pathlib.Path) -> None:
	"""License alias cleanup never follows a target outside the root namespace."""
	nested_target = tmp_path / "nested" / "LICENSE.MIT"
	nested_target.parent.mkdir()
	nested_target.write_bytes(b"nested license text\n")
	alias = tmp_path / "LICENSE"
	alias.symlink_to("nested/LICENSE.MIT")

	migrate(tmp_path)

	assert alias.is_symlink()
