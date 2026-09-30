#!/usr/bin/env python3
"""Run legacy-license migration through the real propagation CLI.

The harness creates a disposable Git consumer with an old typed Markdown
license, an abbreviated SPDX-tagged body, and a generic symlink alias. It then
runs ``propagate_style_guides.py`` and verifies the complete catalog body and
canonical filename on disk.

Run directly outside pytest:

    source source_me.sh && python3 tests/meta/e2e/e2e_license_migration.py

Template-meta: lives under tests/meta/e2e/; never propagates; removed by reset.
"""

# Standard Library
import os
import pathlib
import subprocess
import sys
import tempfile


TEMPLATE_ROOT = pathlib.Path(subprocess.run(
	["git", "rev-parse", "--show-toplevel"],
	cwd=os.path.dirname(os.path.abspath(__file__)),
	capture_output=True,
	text=True,
	check=True,
).stdout.strip())
LEGACY_BODY = (
	"# License\n\n"
	"This work is licensed under the "
	"[Creative Commons Attribution 4.0 International License]"
	"(https://creativecommons.org/licenses/by/4.0/).\n\n"
	"**SPDX-License-Identifier:** CC-BY-4.0\n"
)


#============================================
def initialize_consumer(repo_root: pathlib.Path) -> None:
	"""Create the smallest real Git consumer needed by propagation."""
	repo_root.mkdir()
	subprocess.run(
		["git", "init", "--quiet", str(repo_root)],
		check=True,
		capture_output=True,
		text=True,
	)
	(repo_root / "REPO_TYPE").write_text("other\n", encoding="utf-8")
	legacy = repo_root / "LICENSE.CC-BY-4.0.md"
	legacy.write_text(LEGACY_BODY, encoding="utf-8")
	(repo_root / "LICENSE").symlink_to(legacy.name)


#============================================
def run_propagation(repo_root: pathlib.Path) -> subprocess.CompletedProcess[str]:
	"""Run the production CLI against the disposable consumer."""
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
	return result


#============================================
def main() -> None:
	"""Build a disposable legacy repo and verify its propagated final state."""
	with tempfile.TemporaryDirectory(prefix="license-migration-e2e-") as temporary_dir:
		repo_root = pathlib.Path(temporary_dir) / "consumer"
		initialize_consumer(repo_root)
		result = run_propagation(repo_root)
		if result.returncode != 0:
			print(result.stdout, file=sys.stderr)
			print(result.stderr, file=sys.stderr)
			raise RuntimeError("propagate_style_guides.py failed")

		legacy = repo_root / "LICENSE.CC-BY-4.0.md"
		alias = repo_root / "LICENSE"
		canonical = repo_root / "LICENSE.CC-BY-4.0"
		expected = TEMPLATE_ROOT / "LICENSES" / "LICENSE.CC-BY-4.0"

		assert not legacy.exists(), "legacy typed Markdown license remained"
		assert not os.path.lexists(alias), "generic symlink alias remained"
		assert canonical.read_bytes() == expected.read_bytes(), "catalog body mismatch"
		assert "installed complete catalog body" in result.stdout, "migration was not reported"

	print("PASS: propagation migrated the legacy license system.")


if __name__ == "__main__":
	main()
