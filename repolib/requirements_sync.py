"""Synchronize universal development requirements without replacing local ones."""

# Standard Library
import os
import re

# PIP3 modules
import packaging.utils
import packaging.requirements

# local repo modules
import repolib.console
import repolib.files


UNIVERSAL_HEADER = (
	"# === UNIVERSAL DEVELOPMENT DEPENDENCIES === "
	"[PROPAGATED - LOCAL EDITS OVERWRITTEN]"
)
LOCAL_HEADER = (
	"# === LOCAL DEVELOPMENT DEPENDENCIES === "
	"[ADD REPOSITORY-SPECIFIC DEPENDENCIES HERE]"
)
UNIVERSAL_PREFIX = "# === UNIVERSAL DEVELOPMENT DEPENDENCIES ==="
LOCAL_PREFIX = "# === LOCAL DEVELOPMENT DEPENDENCIES ==="
INLINE_COMMENT_PATTERN = re.compile(r"\s+#")


#============================================


def split_lines(text: str) -> tuple[list[str], bool]:
	"""Split requirement text while retaining its trailing-newline state."""
	trailing_newline = text.endswith("\n")
	lines = text.split("\n")
	if trailing_newline and lines and lines[-1] == "":
		lines.pop()
	return lines, trailing_newline


#============================================


def join_lines(lines: list[str], trailing_newline: bool) -> str:
	"""Join requirement lines and restore the selected trailing newline."""
	text = "\n".join(lines)
	if trailing_newline:
		text += "\n"
	return text


#============================================


def find_marker_lines(lines: list[str]) -> tuple[int, int]:
	"""Return exact ownership-marker indexes or reject ambiguous marker text."""
	universal_indexes = []
	local_indexes = []
	for index, line in enumerate(lines):
		stripped = line.strip()
		if stripped.startswith(UNIVERSAL_PREFIX):
			if stripped != UNIVERSAL_HEADER:
				raise ValueError(f"malformed universal ownership marker on line {index}")
			universal_indexes.append(index)
		if stripped.startswith(LOCAL_PREFIX):
			if stripped != LOCAL_HEADER:
				raise ValueError(f"malformed local ownership marker on line {index}")
			local_indexes.append(index)
	if not universal_indexes and not local_indexes:
		return -1, -1
	if len(universal_indexes) > 1:
		raise ValueError(f"duplicate universal ownership markers on lines {universal_indexes}")
	if len(local_indexes) > 1:
		raise ValueError(f"duplicate local ownership markers on lines {local_indexes}")
	if not universal_indexes or not local_indexes:
		raise ValueError("development requirement ownership markers must appear as one pair")
	universal_index = universal_indexes[0]
	local_index = local_indexes[0]
	if local_index <= universal_index:
		raise ValueError("local ownership marker must follow the universal marker")
	return universal_index, local_index


#============================================


def requirement_name(line: str) -> str | None:
	"""Return one parseable requirement's canonical package name."""
	stripped = line.strip()
	if not stripped or stripped.startswith("#") or stripped.startswith("-"):
		return None
	comment_match = INLINE_COMMENT_PATTERN.search(stripped)
	if comment_match is not None:
		stripped = stripped[:comment_match.start()].rstrip()
	# ASVS 1.5.2 and 2.2.1: packaging parses requirement grammar without
	# executing directives or evaluating consumer-controlled code.
	try:
		requirement = packaging.requirements.Requirement(stripped)
	except packaging.requirements.InvalidRequirement:
		return None
	name = packaging.utils.canonicalize_name(requirement.name)
	return name


#============================================


def managed_source(lines: list[str]) -> tuple[list[str], set[str]]:
	"""Return the canonical managed block and the package names it owns."""
	universal_index, local_index = find_marker_lines(lines)
	if universal_index < 0:
		raise ValueError("canonical requirements source has no ownership markers")
	managed_lines = lines[universal_index:local_index]
	package_names = set()
	for line in managed_lines[1:]:
		if not line.strip() or line.lstrip().startswith("#"):
			continue
		name = requirement_name(line)
		if name is None:
			raise ValueError(f"unparseable managed requirement: {line!r}")
		if name in package_names:
			raise ValueError(f"duplicate managed requirement ownership: {name}")
		package_names.add(name)
	return managed_lines, package_names


#============================================


def marker_free_local_lines(lines: list[str], managed_names: set[str]) -> list[str]:
	"""Preserve local lines in order while removing template-owned requirements."""
	local_lines = []
	for line in lines:
		name = requirement_name(line)
		if name is not None and name in managed_names:
			continue
		local_lines.append(line)
	return local_lines


#============================================


def compose_marker_free(
	dest_lines: list[str],
	managed_lines: list[str],
	managed_names: set[str],
) -> list[str]:
	"""Migrate a marker-free consumer into managed and local ownership blocks."""
	local_lines = marker_free_local_lines(dest_lines, managed_names)
	new_lines = list(managed_lines) + [LOCAL_HEADER]
	if local_lines:
		if local_lines[0].strip():
			new_lines.append("")
		new_lines.extend(local_lines)
	return new_lines


#============================================


def render_synced_text(source_text: str, dest_text: str) -> str:
	"""Return consumer text with only canonical requirements template-owned."""
	source_lines, _source_trailing = split_lines(source_text)
	managed_lines, managed_names = managed_source(source_lines)
	dest_lines, dest_trailing = split_lines(dest_text)
	universal_index, local_index = find_marker_lines(dest_lines)
	if universal_index < 0:
		new_lines = compose_marker_free(dest_lines, managed_lines, managed_names)
	else:
		# The local tail stays consumer-owned, except a package the template owns.
		# This keeps one authoritative specification when a consumer duplicated a
		# universal package below the LOCAL marker.
		local_lines = marker_free_local_lines(dest_lines[local_index + 1:], managed_names)
		new_lines = dest_lines[:universal_index] + managed_lines + [LOCAL_HEADER] + local_lines
	synced_text = join_lines(new_lines, dest_trailing)
	return synced_text


#============================================


def sync_development_requirements(
	source: str,
	dest: str,
	dry_run: bool,
	counters: dict,
) -> str:
	"""Seed or synchronize one consumer development-requirements file."""
	if not os.path.isfile(source):
		counters["errors"] += 1
		repolib.console.log_action("error", f"requirements source missing: {source}")
		return "error"

	source_text = repolib.files.read_text(source)
	source_lines, _source_trailing = split_lines(source_text)
	try:
		managed_source(source_lines)
	except ValueError as source_error:
		counters["errors"] += 1
		repolib.console.log_action(
			"error", f"requirements source malformed: {source}: {source_error}",
		)
		return "error"

	if not os.path.isfile(dest):
		dest_parent = os.path.dirname(dest)
		if dest_parent and not os.path.isdir(dest_parent):
			repolib.files.make_dir_safe(dest_parent, dry_run)
		repolib.files.write_text(dest, source_text, dry_run, action="create")
		if not dry_run:
			repolib.console.log_action("create", dest)
			counters["created_count"] += 1
		return "created"

	dest_text = repolib.files.read_text(dest)
	try:
		synced_text = render_synced_text(source_text, dest_text)
	except ValueError as dest_error:
		counters["errors"] += 1
		repolib.console.log_action(
			"error", f"requirements ownership ambiguous: {dest}: {dest_error}",
		)
		return "error"
	if synced_text == dest_text:
		repolib.console.log_action("no change", dest, counters)
		return "unchanged"

	repolib.files.write_text(dest, synced_text, dry_run, action="merge")
	if not dry_run:
		repolib.console.log_action("merge", dest)
		counters["merged_count"] += 1
	return "merged"
