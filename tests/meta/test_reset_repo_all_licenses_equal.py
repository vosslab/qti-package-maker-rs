"""Every license reset_repo.py offers must be installable on equal footing.

reset_repo.py treats licenses uniformly: copy_license is license-agnostic and
preflight_check requires a source file for whatever was chosen. These tests pin
that equality so no license is privileged and none is left without a backing
file. The original bug privileged MIT (only MIT passed the old gate); this guards
against any return of per-license special-casing.
"""

import pathlib

import pytest

import file_utils

import repolib.reset
import repolib.reset_answers

REPO_ROOT = pathlib.Path(file_utils.get_repo_root())

# Every selectable license that ships a body file. "none" is a docs sentinel
# meaning "no docs license", so it ships no file and is excluded here.
INSTALLABLE_LICENSES = repolib.reset_answers.CODE_LICENSES + [
	spdx for spdx in repolib.reset_answers.DOCS_LICENSES if spdx != "none"
]


#============================================
# Every offered license ships a source file
#============================================

@pytest.mark.parametrize("spdx", INSTALLABLE_LICENSES)
def test_offered_license_has_source_file(spdx: str) -> None:
	"""Each selectable license must have a LICENSES/LICENSE.<spdx> to install."""
	source = REPO_ROOT / "LICENSES" / f"LICENSE.{spdx}"
	assert source.is_file()


#============================================
# copy_license installs every license identically
#============================================

@pytest.mark.parametrize("spdx", INSTALLABLE_LICENSES)
def test_copy_license_installs_each_license(spdx: str, tmp_path: pathlib.Path) -> None:
	"""copy_license reproduces every license body byte-for-byte, no exceptions."""
	source = REPO_ROOT / "LICENSES" / f"LICENSE.{spdx}"
	target_filename = f"LICENSE.{spdx}"
	repolib.reset.copy_license(str(tmp_path), str(source), target_filename, dry_run=False)
	installed = (tmp_path / target_filename).read_text(encoding="utf-8")
	assert installed == source.read_text(encoding="utf-8")


#============================================
# Catalog bodies stay portable plain text
#============================================

@pytest.mark.parametrize("spdx", INSTALLABLE_LICENSES)
def test_offered_license_body_is_ascii(spdx: str) -> None:
	"""Every catalog body uses portable ASCII bytes, including CC legal code."""
	source = REPO_ROOT / "LICENSES" / f"LICENSE.{spdx}"
	source.read_bytes().decode("ascii")


#============================================
# Fresh READMEs state the selected license scope
#============================================

def test_write_readme_license_scope_maps_code_and_docs(tmp_path: pathlib.Path) -> None:
	"""A two-license reset maps each license to its covered material."""
	readme_path = tmp_path / "README.md"
	readme_path.write_text("template boilerplate\n", encoding="ascii")
	repolib.reset.write_readme_license_scope(
		str(tmp_path), "GPL-3.0", "CC-BY-SA-4.0", dry_run=False,
	)
	content = readme_path.read_text(encoding="ascii")
	assert content.startswith(f"# {tmp_path.name}\n\n## License\n")
	assert "Source code: [LICENSE.GPL-3.0](LICENSE.GPL-3.0)." in content
	assert (
		"Documentation and other non-code materials: "
		"[LICENSE.CC-BY-SA-4.0](LICENSE.CC-BY-SA-4.0)."
	) in content


def test_write_readme_license_scope_omits_unselected_docs_license(
	tmp_path: pathlib.Path,
) -> None:
	"""A code-only reset does not invent a documentation-license scope."""
	readme_path = tmp_path / "README.md"
	readme_path.write_text("template boilerplate\n", encoding="ascii")
	repolib.reset.write_readme_license_scope(
		str(tmp_path), "MIT", "none", dry_run=False,
	)
	content = readme_path.read_text(encoding="ascii")
	assert "Source code: [LICENSE.MIT](LICENSE.MIT)." in content
	assert "Documentation and other non-code materials" not in content
