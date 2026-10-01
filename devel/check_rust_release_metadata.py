#!/usr/bin/env python3
"""Validate the Cargo metadata that must agree before a Rust release."""

import json
import pathlib
import re
import subprocess
import sys
import version_lib


EXPECTED_LICENSE = "LGPL-3.0-or-later"
EXPECTED_REPOSITORY = "https://github.com/vosslab/qti-package-maker-rs"
CALVER_SEMVER = re.compile(r"^[0-9]{2}\.(?:[1-9]|1[0-2])\.[0-9]+$")


def main() -> None:
	"""Reject version or release metadata drift across workspace packages."""
	repository = pathlib.Path(
		subprocess.run(
			["git", "rev-parse", "--show-toplevel"],
			check=True,
			capture_output=True,
			text=True,
		).stdout.strip()
	)
	version = version_lib.normalize_cargo_version(
		(repository / "VERSION").read_text(encoding="ascii").strip()
	)
	if not CALVER_SEMVER.fullmatch(version):
		raise RuntimeError(
			"VERSION must use normalized CalVer Cargo SemVer YY.M.P, "
			f"such as 26.9.0; got {version!r}"
		)
	metadata = json.loads(
		subprocess.run(
			["cargo", "metadata", "--no-deps", "--locked", "--format-version", "1"],
			cwd=repository,
			check=True,
			capture_output=True,
			text=True,
		).stdout
	)
	workspace_members = set(metadata["workspace_members"])
	errors: list[str] = []
	for package in metadata["packages"]:
		if package["id"] not in workspace_members:
			continue
		name = package["name"]
		if package["version"] != version:
			errors.append(f"{name}: version {package['version']} != VERSION {version}")
		if package["license"] != EXPECTED_LICENSE:
			errors.append(f"{name}: license must be {EXPECTED_LICENSE!r}")
		if package["repository"] != EXPECTED_REPOSITORY:
			errors.append(f"{name}: repository must be {EXPECTED_REPOSITORY!r}")
	if errors:
		raise RuntimeError("Rust release metadata mismatch:\n" + "\n".join(errors))
	print(f"release metadata: {len(workspace_members)} workspace packages use {version}")


if __name__ == "__main__":
	try:
		main()
	except (OSError, RuntimeError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
		print(f"release metadata check failed: {error}", file=sys.stderr)
		raise SystemExit(2) from error
