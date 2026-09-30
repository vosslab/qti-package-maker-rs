"""Migrate recognized root license files to the canonical Vosslab layout.

Propagation uses this module to replace legacy filenames with ``LICENSE.<SPDX>``.
Known abbreviated template bodies are upgraded from the local complete-text
catalog. Complete and customized bodies are moved byte-for-byte, while ambiguous
or custom license files remain untouched.
"""

# Standard Library
import hashlib
import os
import stat
import tempfile

# local repo modules
import repolib.console
import repolib.reset_answers


CANONICAL_LICENSE_IDS: tuple[str, ...] = tuple(
	repolib.reset_answers.CODE_LICENSES
	+ [license_id for license_id in repolib.reset_answers.DOCS_LICENSES if license_id != "none"]
)

CANONICAL_FILENAMES: dict[str, str] = {
	f"LICENSE.{license_id}": license_id for license_id in CANONICAL_LICENSE_IDS
}

# These names come from earlier starter-repo-template resets and the observed
# pre-SPDX Vosslab convention. Exact root basenames keep path construction closed.
LEGACY_TYPED_FILENAMES: dict[str, str] = {
	**{f"LICENSE.{license_id}.md": license_id for license_id in CANONICAL_LICENSE_IDS},
	"LICENSE.AGPL_v3": "AGPL-3.0",
	"LICENSE.GPL_v3": "GPL-3.0",
	"LICENSE.LGPL_v3": "LGPL-3.0",
	"LICENSE.CC_BY_4_0": "CC-BY-4.0",
	"LICENSE.CC_by_4_0": "CC-BY-4.0",
	"LICENSE.CC_BY_SA_4_0": "CC-BY-SA-4.0",
	"LICENSE.CC_by_SA_4_0": "CC-BY-SA-4.0",
}

GENERIC_LICENSE_FILENAMES: tuple[str, ...] = ("LICENSE", "LICENSE.md")

# A generic filename migrates only when its body identifies exactly one license.
# Multiple matches remain ambiguous and are deliberately preserved.
BODY_MARKERS: dict[str, tuple[bytes, ...]] = {
	"AGPL-3.0": (
		b"GNU AFFERO GENERAL PUBLIC LICENSE",
		b"Version 3, 19 November 2007",
	),
	"Apache-2.0": (
		b"Apache License",
		b"Version 2.0, January 2004",
	),
	"CC-BY-4.0": (b"Attribution 4.0 International",),
	"CC-BY-SA-4.0": (b"Attribution-ShareAlike 4.0 International",),
	"GPL-3.0": (
		b"GNU GENERAL PUBLIC LICENSE",
		b"Version 3, 29 June 2007",
	),
	"LGPL-3.0": (
		b"GNU LESSER GENERAL PUBLIC LICENSE",
		b"Version 3, 29 June 2007",
	),
	"MIT": (
		b"MIT License",
		b"Permission is hereby granted, free of charge",
	),
	"MPL-2.0": (b"Mozilla Public License Version 2.0",),
}

# These markers occur near the end of each complete publisher body. They keep a
# one-line title or short project notice from being promoted to a canonical file.
COMPLETE_BODY_MARKERS: dict[str, tuple[bytes, ...]] = {
	"AGPL-3.0": (
		b"END OF TERMS AND CONDITIONS",
		b"How to Apply These Terms to Your New Programs",
	),
	"Apache-2.0": (
		b"END OF TERMS AND CONDITIONS",
		b"APPENDIX: How to apply the Apache License to your work.",
	),
	"CC-BY-4.0": (
		b"Section 8 -- Interpretation.",
		b"Creative Commons may be contacted at creativecommons.org.",
	),
	"CC-BY-SA-4.0": (
		b"Section 8 -- Interpretation.",
		b"Creative Commons may be contacted at creativecommons.org.",
	),
	"GPL-3.0": (
		b"END OF TERMS AND CONDITIONS",
		b"How to Apply These Terms to Your New Programs",
	),
	"LGPL-3.0": (
		b"6. Revised Versions of the GNU Lesser General Public License.",
		b"permanent authorization for you to choose that version",
	),
	"MIT": (
		b'THE SOFTWARE IS PROVIDED "AS IS"',
		b"OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
	),
	"MPL-2.0": (
		b"10.4. Distributing Source Code Form that is Incompatible With Secondary",
		b"Exhibit B - \"Incompatible With Secondary Licenses\" Notice",
	),
}

# Fingerprints of the pre-2026-08-26 abbreviated template bodies after replacing
# their application-specific copyright line. MIT is intentionally absent because
# its former 21-line body was already the complete license application template.
LEGACY_SUMMARY_FINGERPRINTS: dict[str, str] = {
	"AGPL-3.0": "96dc7733910f872ae5ec29dc96ba095ab2b51ae5bb72d7ecd5084494492b393f",
	"Apache-2.0": "dba1b65c3b11e854492d81ceac64821ec00ae874ceaaa9ead0a38f9bfa54e7b8",
	"CC-BY-4.0": "c842af90cf3d531fcc3338d9f794c6e9b5a1377db15088f868e756b7afdc6dd8",
	"CC-BY-SA-4.0": "4690e5839731a320c27ee03aa394744c5a91e2128e90df1a94c363aaf631da98",
	"GPL-3.0": "2eb9531002ea219f12e7ea24333a80656cc32a862671c752a082603cd375acfa",
	"LGPL-3.0": "f7a573b3376f70c52f58d0131f05913f4f777783c9e85a5762d2d12892fca333",
	"MPL-2.0": "7e45b77a80a089ee44bb38e6b591f821ad7dd022bd839c03feb8447d81e4dcc0",
}

# One additional short summary was found under a generic LICENSE.md name. Its
# exact fingerprint permits a full-body upgrade without generalizing from size.
KNOWN_SHORT_SUMMARY_FINGERPRINTS: dict[str, frozenset[str]] = {
	"CC-BY-4.0": frozenset({
		"a6ef5f90d5f4d6df60f02545fd1b2c05f7108a690af0f914098ffa0f7025dca1",
	}),
}


#============================================
def _root_path(repo_dir: str, filename: str) -> str:
	"""Return one allow-listed root license path without following symlinks."""
	# ASVS 5.3.2: filenames come only from the closed constants above.
	if (
		"/" in filename or "\\" in filename
		or os.path.basename(filename) != filename
		or filename in ("", ".", "..")
	):
		raise ValueError(f"Invalid root license filename: {filename!r}")
	repo_root = os.path.abspath(repo_dir)
	path = os.path.abspath(os.path.join(repo_root, filename))
	if os.path.dirname(path) != repo_root:
		raise ValueError(f"License path escapes repository root: {filename!r}")
	return path


#============================================
def _normalized_body(body: bytes) -> bytes:
	"""Normalize line endings and trailing newlines for stable fingerprints."""
	normalized = body.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
	normalized = normalized.rstrip(b"\n") + b"\n"
	return normalized


#============================================
def _legacy_summary_fingerprint(body: bytes) -> str:
	"""Hash an old summary while ignoring its project copyright notice."""
	lines = _normalized_body(body).splitlines()
	for line_index, line in enumerate(lines[:10]):
		if line.lstrip().lower().startswith(b"copyright"):
			lines[line_index] = b"Copyright [application notice]"
			break
	normalized = b"\n".join(lines) + b"\n"
	digest = hashlib.sha256(normalized).hexdigest()
	return digest


#============================================
def _spdx_line(license_id: str) -> bytes:
	"""Return the exact SPDX declaration line for one catalog identifier."""
	line = f"SPDX-License-Identifier: {license_id}".encode("ascii")
	return line


#============================================
def _identify_generic_license(body: bytes) -> str | None:
	"""Identify exactly one supported license from a generic file body."""
	lines = set(_normalized_body(body).splitlines())
	matches: set[str] = set()
	for license_id, markers in BODY_MARKERS.items():
		if all(marker in body for marker in markers):
			matches.add(license_id)
		if _spdx_line(license_id) in lines:
			matches.add(license_id)
	if len(matches) != 1:
		return None
	license_id = next(iter(matches))
	return license_id


#============================================
def _is_complete_body(license_id: str, body: bytes) -> bool:
	"""Return True when body contains the supported license's identity and ending."""
	markers = BODY_MARKERS[license_id] + COMPLETE_BODY_MARKERS[license_id]
	is_complete = all(marker in body for marker in markers)
	return is_complete


#============================================
def _needs_catalog_body(license_id: str, body: bytes) -> bool:
	"""Return True only for an exact known abbreviated summary fingerprint."""
	fingerprint = _legacy_summary_fingerprint(body)
	if LEGACY_SUMMARY_FINGERPRINTS.get(license_id) == fingerprint:
		return True
	known_fingerprints = KNOWN_SHORT_SUMMARY_FINGERPRINTS.get(license_id, frozenset())
	if fingerprint in known_fingerprints:
		return True
	return False


#============================================
def _read_catalog_body(template_root: str, license_id: str) -> bytes:
	"""Read one complete body from the trusted local license catalog."""
	filename = f"LICENSE.{license_id}"
	catalog_dir = os.path.join(os.path.abspath(template_root), "LICENSES")
	catalog_path = _root_path(catalog_dir, filename)
	with open(catalog_path, "rb") as handle:
		body = handle.read()
	return body


#============================================
def _write_body_atomically(path: str, body: bytes, source_mode: int) -> None:
	"""Atomically write a migrated body and remove executable permission bits."""
	parent = os.path.dirname(path)
	descriptor, temporary_path = tempfile.mkstemp(prefix=".license-migration-", dir=parent)
	with os.fdopen(descriptor, "wb") as handle:
		handle.write(body)
	file_mode = stat.S_IMODE(source_mode) & 0o666
	if file_mode == 0:
		file_mode = 0o644
	os.chmod(temporary_path, file_mode)
	os.replace(temporary_path, path)


#============================================
def _record_update(counters: dict, dry_run: bool) -> None:
	"""Increment the shared update counter after an actual migration."""
	if not dry_run:
		counters["updated_count"] += 1


#============================================
def _migrate_regular_file(
	source_path: str,
	target_path: str,
	license_id: str,
	template_root: str,
	dry_run: bool,
	counters: dict,
) -> int:
	"""Migrate one regular file, preserving custom text unless it is a known summary."""
	with open(source_path, "rb") as handle:
		source_body = handle.read()
	use_catalog = _needs_catalog_body(license_id, source_body)
	if not use_catalog and not _is_complete_body(license_id, source_body):
		repolib.console.log_action(
			"warn", f"incomplete or unverified license body; preserved {source_path}"
		)
		return 0
	target_body = _read_catalog_body(template_root, license_id) if use_catalog else source_body
	same_path = os.path.abspath(source_path) == os.path.abspath(target_path)

	# A different canonical file is a legal-content conflict, never an overwrite.
	if not same_path and os.path.lexists(target_path):
		if os.path.islink(target_path) or not os.path.isfile(target_path):
			repolib.console.log_action(
				"warn", f"license conflict: {target_path} is not a regular file; preserved {source_path}"
			)
			return 0
		with open(target_path, "rb") as handle:
			existing_body = handle.read()
		if existing_body != target_body:
			repolib.console.log_action(
				"warn", f"license conflict: {source_path} differs from {target_path}; preserved both"
			)
			return 0

		message = f"duplicate license: {source_path} (canonical copy is {target_path})"
		if dry_run:
			repolib.console.log_action("removed", message, dry_run=True)
		else:
			os.remove(source_path)
			repolib.console.log_action("removed", message)
		_record_update(counters, dry_run)
		return 1

	detail = " (installed complete catalog body)" if use_catalog else ""
	message = f"license: {source_path} -> {target_path}{detail}"
	if dry_run:
		repolib.console.log_action("update", message, dry_run=True)
		return 1

	source_mode = os.stat(source_path, follow_symlinks=False).st_mode
	if same_path or use_catalog:
		_write_body_atomically(target_path, target_body, source_mode)
		if not same_path:
			os.remove(source_path)
	else:
		os.replace(source_path, target_path)
		file_mode = stat.S_IMODE(source_mode) & 0o666
		if file_mode == 0:
			file_mode = 0o644
		os.chmod(target_path, file_mode)
	repolib.console.log_action("update", message)
	_record_update(counters, dry_run)
	return 1


#============================================
def _migrate_canonical_bodies(
	repo_dir: str, template_root: str, dry_run: bool, counters: dict,
) -> int:
	"""Upgrade known abbreviated bodies already stored under canonical names."""
	action_count = 0
	root_entries = set(os.listdir(repo_dir))
	for filename, license_id in CANONICAL_FILENAMES.items():
		if filename not in root_entries:
			continue
		path = _root_path(repo_dir, filename)
		if not os.path.lexists(path):
			continue
		if os.path.islink(path) or not os.path.isfile(path):
			repolib.console.log_action(
				"warn", f"canonical license is not a regular file; preserved {path}"
			)
			continue
		with open(path, "rb") as handle:
			body = handle.read()
		if not _needs_catalog_body(license_id, body):
			continue
		action_count += _migrate_regular_file(
			path, path, license_id, template_root, dry_run, counters,
		)
	return action_count


#============================================
def _migrate_typed_names(
	repo_dir: str, template_root: str, dry_run: bool, counters: dict,
) -> tuple[int, set[str]]:
	"""Migrate recognized legacy typed filenames in deterministic order."""
	action_count = 0
	migrated_filenames: set[str] = set()
	root_entries = set(os.listdir(repo_dir))
	for filename, license_id in LEGACY_TYPED_FILENAMES.items():
		if filename not in root_entries:
			continue
		source_path = _root_path(repo_dir, filename)
		if not os.path.lexists(source_path):
			continue
		if os.path.islink(source_path) or not os.path.isfile(source_path):
			repolib.console.log_action(
				"warn", f"legacy typed license is not a regular file; preserved {source_path}"
			)
			continue
		target_path = _root_path(repo_dir, f"LICENSE.{license_id}")
		file_actions = _migrate_regular_file(
			source_path, target_path, license_id, template_root, dry_run, counters,
		)
		action_count += file_actions
		if file_actions > 0:
			migrated_filenames.add(filename)
	result = (action_count, migrated_filenames)
	return result


#============================================
def _symlink_target_license_id(target: str) -> str | None:
	"""Map a safe root-local symlink target basename to a canonical identifier."""
	if os.path.isabs(target):
		return None
	normalized_target = target[2:] if target.startswith("./") else target
	if (
		"/" in normalized_target or "\\" in normalized_target
		or os.path.basename(normalized_target) != normalized_target
	):
		return None
	license_id = CANONICAL_FILENAMES.get(normalized_target)
	if license_id is None:
		license_id = LEGACY_TYPED_FILENAMES.get(normalized_target)
	return license_id


#============================================
def _remove_generic_symlink(
	path: str,
	repo_dir: str,
	dry_run: bool,
	counters: dict,
	migrated_typed_filenames: set[str],
) -> int:
	"""Remove a recognized generic alias after its canonical real file exists."""
	target = os.readlink(path)
	license_id = _symlink_target_license_id(target)
	if license_id is None:
		repolib.console.log_action(
			"warn", f"unrecognized license symlink target {target!r}; preserved {path}"
		)
		return 0
	normalized_target = target[2:] if target.startswith("./") else target
	if normalized_target in LEGACY_TYPED_FILENAMES:
		legacy_target_path = _root_path(repo_dir, normalized_target)
		legacy_target_remains = (
			not os.path.islink(legacy_target_path) and os.path.isfile(legacy_target_path)
		)
		if legacy_target_remains and normalized_target not in migrated_typed_filenames:
			repolib.console.log_action(
				"warn", f"license alias target was preserved; retained {path} -> {target}"
			)
			return 0
	canonical_path = _root_path(repo_dir, f"LICENSE.{license_id}")
	canonical_exists = not os.path.islink(canonical_path) and os.path.isfile(canonical_path)
	legacy_target_exists = False
	if dry_run and not canonical_exists:
		legacy_target_path = _root_path(repo_dir, normalized_target)
		legacy_target_exists = (
			not os.path.islink(legacy_target_path) and os.path.isfile(legacy_target_path)
		)
	if not canonical_exists and not legacy_target_exists:
		repolib.console.log_action(
			"warn", f"license symlink has no canonical regular target; preserved {path}"
		)
		return 0
	message = f"generic license alias: {path} -> {target}"
	if dry_run:
		repolib.console.log_action("removed", message, dry_run=True)
	else:
		os.remove(path)
		repolib.console.log_action("removed", message)
	_record_update(counters, dry_run)
	return 1


#============================================
def _migrate_generic_names(
	repo_dir: str,
	template_root: str,
	dry_run: bool,
	counters: dict,
	migrated_typed_filenames: set[str],
) -> int:
	"""Migrate generic names only when content or a safe alias is unambiguous."""
	action_count = 0
	root_entries = set(os.listdir(repo_dir))
	for filename in GENERIC_LICENSE_FILENAMES:
		if filename not in root_entries:
			continue
		source_path = _root_path(repo_dir, filename)
		if not os.path.lexists(source_path):
			continue
		if os.path.islink(source_path):
			action_count += _remove_generic_symlink(
				source_path, repo_dir, dry_run, counters, migrated_typed_filenames,
			)
			continue
		if not os.path.isfile(source_path):
			repolib.console.log_action(
				"warn", f"generic license is not a regular file; preserved {source_path}"
			)
			continue
		with open(source_path, "rb") as handle:
			body = handle.read()
		license_id = _identify_generic_license(body)
		if license_id is None:
			repolib.console.log_action(
				"warn", f"custom or ambiguous generic license; preserved {source_path}"
			)
			continue
		target_path = _root_path(repo_dir, f"LICENSE.{license_id}")
		action_count += _migrate_regular_file(
			source_path, target_path, license_id, template_root, dry_run, counters,
		)
	return action_count


#============================================
def migrate_legacy_licenses(
	repo_dir: str, template_root: str, dry_run: bool, counters: dict,
) -> int:
	"""Bring recognized root licenses into the current filename/body policy.

	Args:
		repo_dir: Consumer repository root.
		template_root: Starter-template root containing the complete ``LICENSES`` catalog.
		dry_run: Report changes without modifying the consumer.
		counters: Shared propagation counters updated after actual migrations.

	Returns:
		Number of migrations performed or planned.
	"""
	# Canonical bodies are repaired first. Typed legacy names then take precedence
	# over generic files, and generic symlinks are considered only after targets move.
	action_count = _migrate_canonical_bodies(repo_dir, template_root, dry_run, counters)
	typed_actions, migrated_typed_filenames = _migrate_typed_names(
		repo_dir, template_root, dry_run, counters,
	)
	action_count += typed_actions
	action_count += _migrate_generic_names(
		repo_dir, template_root, dry_run, counters, migrated_typed_filenames,
	)
	return action_count
