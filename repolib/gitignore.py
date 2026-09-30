"""Gitignore rendering helpers for propagated and consumer-owned sections."""

# local repo modules
import repolib.model


GITIGNORE_LOCAL_HEADER_PREFIX = '# ADD YOUR CUSTOM IGNORES BELOW'
GITIGNORE_LOCAL_HEADER = GITIGNORE_LOCAL_HEADER_PREFIX
GITIGNORE_LOCAL_NOTICE = '# Propagation preserves this section.'
GITIGNORE_LOCAL_RULE = '# -------------------- LOCAL REPOSITORY RULES --------------------'
# Accepted on input for legacy consumers, but never rendered.
GITIGNORE_LOCAL_RULE_END = '# ------------------ END LOCAL REPOSITORY RULES ------------------'
GITIGNORE_PREVIOUS_LOCAL_RULES = frozenset({
	'# -----------------------------------------------------------------------------',
	'# =============================================================================',
})
GITIGNORE_PREVIOUS_LOCAL_HEADER = (
	'# === LOCAL REPOSITORY RULES === [ADD CUSTOM IGNORES HERE]'
)
GITIGNORE_PREVIOUS_LOCAL_HEADER_PREFIX = '# === LOCAL REPOSITORY RULES ==='
GITIGNORE_LEGACY_LOCAL_HEADER = '# === LOCAL ==='


#============================================
def spaced_block(block_lines: list[str]) -> list[str]:
	"""Return managed content with exactly one trailing blank line."""
	trimmed = list(block_lines)
	while trimmed and trimmed[-1].strip() == '':
		trimmed.pop()
	result = trimmed + ['']
	return result


#============================================
def managed_gitignore_header(block_name: str) -> str:
	"""Return a managed heading that states its propagation ownership."""
	header = f'# === {block_name} === [PROPAGATED - LOCAL EDITS OVERWRITTEN]'
	return header


#============================================
def _is_propagated_gitignore_header(line: str) -> bool:
	"""Return whether line is a recognized propagated gitignore header."""
	block_names = (
		'UNIVERSAL',
		*(
			repo_type.upper()
			for repo_type in repolib.model.KNOWN_REPO_TYPES
			if repo_type != repolib.model.LANG_ALL
		),
	)
	for block_name in block_names:
		header = f'# === {block_name} ==='
		if line == header or line == managed_gitignore_header(block_name):
			return True
	return False


#============================================
def _is_gitignore_local_heading(lines: list[str], index: int) -> bool:
	"""Return whether one line identifies a current or accepted legacy LOCAL section."""
	line = lines[index]
	if line == GITIGNORE_LEGACY_LOCAL_HEADER:
		return True
	if line == GITIGNORE_PREVIOUS_LOCAL_HEADER:
		return True
	if line == GITIGNORE_LOCAL_HEADER:
		return index + 1 < len(lines) and lines[index + 1] == GITIGNORE_LOCAL_NOTICE
	if line != GITIGNORE_PREVIOUS_LOCAL_HEADER_PREFIX:
		return False
	result = (
		index > 0
		and lines[index - 1] in GITIGNORE_PREVIOUS_LOCAL_RULES
		and index + 1 < len(lines)
		and lines[index + 1] in GITIGNORE_PREVIOUS_LOCAL_RULES
	)
	return result


#============================================
def ensure_gitignore_local_section(lines: list[str]) -> list[str]:
	"""Return lines with one clearly labeled repository-owned section.

	The LOCAL section follows propagated sections. Legacy renderer markers are
	replaced while their consumer-owned body retains its exact line order.
	"""
	local_index: int | None = None
	for index, line in enumerate(lines):
		if _is_gitignore_local_heading(lines, index):
			local_index = index
			break

	if local_index is not None:
		local_start = local_index
		local_body_start = local_index + 1
		if local_index + 1 < len(lines) and lines[local_index + 1] == GITIGNORE_LOCAL_NOTICE:
			local_body_start += 1
			if local_body_start < len(lines) and lines[local_body_start] == GITIGNORE_LOCAL_RULE_END:
				local_body_start += 1
			if local_index > 0 and lines[local_index - 1] == GITIGNORE_LOCAL_RULE:
				local_start -= 1
		elif (
			local_index > 0
			and lines[local_index - 1] in GITIGNORE_PREVIOUS_LOCAL_RULES
			and local_index + 1 < len(lines)
			and lines[local_index + 1] in GITIGNORE_PREVIOUS_LOCAL_RULES
		):
			local_start -= 1
			local_body_start += 1

		local_end = local_index + 1
		while local_end < len(lines) and not _is_propagated_gitignore_header(lines[local_end]):
			local_end += 1
		local_body = list(lines[local_body_start:local_end])
		remaining_lines = list(lines[:local_start]) + list(lines[local_end:])
		result: list[str] = []
		prefix_body: list[str] = []
		in_managed_block = False
		for line in remaining_lines:
			if _is_propagated_gitignore_header(line):
				in_managed_block = True
			if in_managed_block:
				result.append(line)
			else:
				prefix_body.append(line)
		local_body = prefix_body + local_body
	else:
		result = []
		local_body = []
		in_managed_block = False
		for line in lines:
			if _is_propagated_gitignore_header(line):
				in_managed_block = True
			if in_managed_block:
				result.append(line)
			else:
				local_body.append(line)

	local_banner = [
		GITIGNORE_LOCAL_RULE,
		GITIGNORE_LOCAL_HEADER,
		GITIGNORE_LOCAL_NOTICE,
	]
	output = result + local_banner + local_body
	return output


#============================================
def extract_gitignore_consumer_lines(lines: list[str]) -> list[str]:
	"""Remove every recognized propagated section while retaining consumer text."""
	consumer_lines: list[str] = []
	index = 0
	while index < len(lines):
		if not _is_propagated_gitignore_header(lines[index]):
			consumer_lines.append(lines[index])
			index += 1
			continue
		index += 1
		while (
			index < len(lines)
			and not _is_propagated_gitignore_header(lines[index])
			and not _is_gitignore_local_heading(lines, index)
			and not (
				lines[index] == GITIGNORE_LOCAL_RULE
				and index + 1 < len(lines)
				and _is_gitignore_local_heading(lines, index + 1)
			)
		):
			index += 1
	return consumer_lines
