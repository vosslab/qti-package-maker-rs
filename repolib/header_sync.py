"""Vendored header synchronization for consumer-owned files (HEADER bucket).

The HEADER bucket seeds a whole file when the consumer lacks it, then refreshes a
marked header region inside that file on every later sync while the repository's
own entries stay untouched. NOEXIST alone reaches a repo once, at the moment it
holds the least content; HEADER keeps the vendored wording correctable for as
long as the file lives.

The template file is the single source: the region between its own markers is the
header, and the rest of the template is the seed body used only at creation.

File ownership, from meta/docs/HEADER_BUCKET_SPEC.md:

	region     -- the markers and everything between them; rewritten every sync.
	separator  -- the blank-line run immediately following the region; normalized
	              to exactly one blank line when consumer content follows.
	content    -- every other line; preserved byte-for-byte.
"""

# Standard Library
import os

# local repo modules
import repolib.console
import repolib.files


# Marker pair delimiting the vendored region inside a consumer-owned file.
HEADER_START_MARKER = '<!-- VENDORED HEADER: START -->'
HEADER_END_MARKER = '<!-- VENDORED HEADER: END -->'


#============================================
def split_lines(text: str) -> tuple[list[str], bool]:
	"""
	Split text into lines while remembering its trailing-newline state.

	str.split('\\n') leaves a final empty element for text that ends with a
	newline. Dropping that element keeps line math simple, and the returned flag
	lets join_lines restore the original trailing-newline state exactly.

	Args:
		text (str): File content to split.

	Returns:
		tuple[list[str], bool]: The lines, and True when the text ended with a newline.
	"""
	trailing_newline = text.endswith('\n')
	lines = text.split('\n')
	# Drop the artifact empty element produced by a trailing newline.
	if trailing_newline and lines and lines[-1] == '':
		lines.pop()
	return lines, trailing_newline


#============================================
def join_lines(lines: list[str], trailing_newline: bool) -> str:
	"""
	Join lines back into text, restoring the recorded trailing-newline state.

	Args:
		lines (list[str]): Lines to join.
		trailing_newline (bool): True to end the text with a newline.

	Returns:
		str: The joined text.
	"""
	text = '\n'.join(lines)
	if trailing_newline:
		text += '\n'
	return text


#============================================
def find_marker_lines(lines: list[str]) -> tuple[int, int]:
	"""
	Locate the vendored region's marker pair.

	The bucket's safety promise is that it rewrites one bounded region, so an
	ambiguous region boundary stops the sync rather than guessing which text to
	replace. Duplicate markers, an unpaired marker, and an end marker preceding
	its start marker all raise.

	Args:
		lines (list[str]): Lines of the file being examined.

	Returns:
		tuple[int, int]: Indexes of the start and end marker lines, or (-1, -1)
			when the file carries neither marker.

	Raises:
		ValueError: The marker structure is unpaired, duplicated, or out of order.
	"""
	# Collect every marker occurrence so duplicates are detectable.
	start_indexes = [idx for idx, line in enumerate(lines) if line.strip() == HEADER_START_MARKER]
	end_indexes = [idx for idx, line in enumerate(lines) if line.strip() == HEADER_END_MARKER]
	# Neither marker: the caller inserts a fresh header at the anchor.
	if not start_indexes and not end_indexes:
		return -1, -1
	if len(start_indexes) > 1:
		raise ValueError(f"duplicate vendored header start markers on lines {start_indexes}")
	if len(end_indexes) > 1:
		raise ValueError(f"duplicate vendored header end markers on lines {end_indexes}")
	if not end_indexes:
		raise ValueError("vendored header start marker present without its end marker")
	if not start_indexes:
		raise ValueError("vendored header end marker present without its start marker")
	start_index = start_indexes[0]
	end_index = end_indexes[0]
	if end_index < start_index:
		raise ValueError("vendored header end marker appears before its start marker")
	return start_index, end_index


#============================================
def extract_header(lines: list[str]) -> list[str]:
	"""
	Return the vendored region from a template file, markers included.

	Args:
		lines (list[str]): Lines of the template source file.

	Returns:
		list[str]: The marker lines and everything between them.

	Raises:
		ValueError: The template lacks a well-formed marker pair.
	"""
	start_index, end_index = find_marker_lines(lines)
	if start_index < 0:
		raise ValueError("template source carries no vendored header markers")
	header_lines = lines[start_index:end_index + 1]
	return header_lines


#============================================
def anchor_index(lines: list[str]) -> int:
	"""
	Return the index where a fresh header begins in a marker-free file.

	The anchor is the line after the first level-one heading, which every
	observed consumer file carries on line 1. The rule keys on the heading line
	itself rather than its text, so a file whose title differs still anchors
	correctly. A file with no heading anchors at the top.

	Args:
		lines (list[str]): Lines of the consumer file.

	Returns:
		int: Index at which the header should start.
	"""
	for index, line in enumerate(lines):
		if line.startswith('# '):
			return index + 1
	return 0


#============================================
def first_content_index(lines: list[str], start: int) -> int:
	"""
	Return the index of the first non-blank line at or after start.

	The blank-line run this skips is the separator, which synchronization owns
	and rewrites; everything from the returned index onward is consumer content.

	Args:
		lines (list[str]): Lines of the consumer file.
		start (int): Index to begin scanning from.

	Returns:
		int: Index of the first non-blank line, or len(lines) when none remains.
	"""
	index = start
	while index < len(lines) and not lines[index].strip():
		index += 1
	return index


#============================================
def compose_replacement(lines: list[str], header_lines: list[str], start_index: int, end_index: int) -> list[str]:
	"""
	Rebuild a file whose vendored region is replaced in place.

	Content before the region and from the first non-blank line after the
	separator onward is carried through unchanged.

	Args:
		lines (list[str]): Lines of the consumer file.
		header_lines (list[str]): Replacement region, markers included.
		start_index (int): Index of the existing start marker.
		end_index (int): Index of the existing end marker.

	Returns:
		list[str]: The rebuilt lines.
	"""
	# Consumer content resumes after the separator that follows the old region.
	tail_index = first_content_index(lines, end_index + 1)
	tail_lines = lines[tail_index:]
	new_lines = lines[:start_index] + header_lines
	# Emit exactly one separator line when consumer content follows.
	if tail_lines:
		new_lines = new_lines + [''] + tail_lines
	return new_lines


#============================================
def compose_insertion(lines: list[str], header_lines: list[str]) -> list[str]:
	"""
	Rebuild a marker-free file with the vendored header inserted at its anchor.

	Args:
		lines (list[str]): Lines of the consumer file.
		header_lines (list[str]): Region to insert, markers included.

	Returns:
		list[str]: The rebuilt lines.
	"""
	anchor = anchor_index(lines)
	head_lines = lines[:anchor]
	# Consumer content resumes after the blank run at the anchor.
	tail_index = first_content_index(lines, anchor)
	tail_lines = lines[tail_index:]
	new_lines = list(head_lines)
	# Separate the header from a preceding heading with exactly one blank line.
	if head_lines:
		new_lines = new_lines + ['']
	new_lines = new_lines + header_lines
	if tail_lines:
		new_lines = new_lines + [''] + tail_lines
	return new_lines


#============================================
def render_synced_text(dest_text: str, header_lines: list[str]) -> str:
	"""
	Return the consumer file text with its vendored header current.

	Replaces an existing region when the markers are present, and inserts one at
	the anchor when they are absent. The file's trailing-newline state is
	preserved either way.

	Args:
		dest_text (str): Current consumer file content.
		header_lines (list[str]): Vendored region from the template.

	Returns:
		str: The rebuilt file content.

	Raises:
		ValueError: The consumer marker structure is ambiguous.
	"""
	lines, trailing_newline = split_lines(dest_text)
	start_index, end_index = find_marker_lines(lines)
	if start_index < 0:
		new_lines = compose_insertion(lines, header_lines)
	else:
		new_lines = compose_replacement(lines, header_lines, start_index, end_index)
	synced_text = join_lines(new_lines, trailing_newline)
	return synced_text


#============================================
def sync_vendored_header(source: str, dest: str, dry_run: bool, counters: dict) -> str:
	"""
	Seed a consumer-owned file, then keep its vendored header region current.

	Outcomes:
		'created'   -- dest missing; wrote the template stub verbatim.
		'merged'    -- header region inserted or replaced.
		'unchanged' -- consumer header already matches the template.
		'error'     -- source missing, or an ambiguous marker structure.

	Args:
		source (str): Template file carrying the vendored region.
		dest (str): Consumer file to seed or refresh.
		dry_run (bool): True to report the action without writing.
		counters (dict): Shared counter dict from repolib.console.init_counters.

	Returns:
		str: One of 'created', 'merged', 'unchanged', 'error'.
	"""
	if not os.path.isfile(source):
		counters['errors'] += 1
		repolib.console.log_action("error", f"header source missing: {source}")
		return 'error'

	src_text = repolib.files.read_text(source)

	# Missing consumer file: the whole template stub is the seed.
	if not os.path.isfile(dest):
		dest_parent = os.path.dirname(dest)
		if dest_parent and not os.path.isdir(dest_parent):
			repolib.files.make_dir_safe(dest_parent, dry_run)
		repolib.files.write_text(dest, src_text, dry_run, action='create')
		if not dry_run:
			repolib.console.log_action("create", dest)
			counters['created_count'] += 1
		return 'created'

	# The template's own markers define the region this bucket owns.
	src_lines, _src_trailing = split_lines(src_text)
	try:
		header_lines = extract_header(src_lines)
	except ValueError as source_error:
		counters['errors'] += 1
		repolib.console.log_action("error", f"header source malformed: {source}: {source_error}")
		return 'error'

	dest_text = repolib.files.read_text(dest)
	# An ambiguous consumer region stops the sync; the file stays as written.
	try:
		synced_text = render_synced_text(dest_text, header_lines)
	except ValueError as dest_error:
		counters['errors'] += 1
		repolib.console.log_action("error", f"header region ambiguous: {dest}: {dest_error}")
		return 'error'

	if synced_text == dest_text:
		repolib.console.log_action("no change", dest, counters)
		return 'unchanged'

	repolib.files.write_text(dest, synced_text, dry_run, action='merge')
	if not dry_run:
		repolib.console.log_action("merge", dest)
		counters['merged_count'] += 1
	return 'merged'
