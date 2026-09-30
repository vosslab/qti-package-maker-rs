"""Write the one changelog entry owned by template propagation.

This intentionally narrow module exists because the propagation command is
template-only.  Consumer ``devel/`` tools retain their self-contained
``changelog_lib.py`` helper after reset, while this writer disappears with the
root propagation command and the rest of ``repolib``.
"""

# Standard Library
import os
import re


DATE_RE = re.compile(r"^## (\d{4}-\d{2}-\d{2})\s*$")
CATEGORY_RE = re.compile(r"^###\s+(.+?)\s*$")

CANONICAL_CATEGORIES = [
	"Additions and New Features",
	"Behavior or Interface Changes",
	"Fixes and Maintenance",
	"Removals and Deprecations",
	"Decisions and Failures",
	"Developer Tests and Notes",
]


#============================================
def _is_valid_iso_date(date_str: str) -> bool:
	"""Return whether ``date_str`` is a calendrically valid ISO date."""
	parts = date_str.split("-")
	if len(parts) != 3:
		return False
	year_str, month_str, day_str = parts
	if not (year_str.isdigit() and month_str.isdigit() and day_str.isdigit()):
		return False
	year = int(year_str)
	month = int(month_str)
	day = int(day_str)
	if month < 1 or month > 12 or day < 1 or day > 31:
		return False
	month_lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
	if month == 2:
		is_leap = (year % 4 == 0 and year % 100 != 0) or year % 400 == 0
		max_day = 29 if is_leap else 28
	else:
		max_day = month_lengths[month - 1]
	return day <= max_day


#============================================
def changelog_path_for_repo(repo_dir: str) -> str:
	"""Return the fixed changelog destination beneath a trusted target root."""
	trusted_repo_root = os.path.abspath(repo_dir)
	# ASVS 5.3.2: this fixed, internally generated relative path never accepts a
	# caller-supplied filename, so propagation cannot redirect this file write.
	changelog_path = os.path.join(trusted_repo_root, "docs", "CHANGELOG.md")
	return changelog_path


#============================================
def _validated_blocks(text: str, source: str) -> tuple[str, list[tuple[str, str]]]:
	"""Return the verbatim preamble and valid day blocks, rejecting unsafe input."""
	lines = text.splitlines(keepends=True)
	headings: list[tuple[int, str]] = []
	for index, line in enumerate(lines):
		match = DATE_RE.match(line)
		if match is not None:
			date_str = match.group(1)
			if not _is_valid_iso_date(date_str):
				raise ValueError(
					f"invalid date '{date_str}' at {source}:{index + 1}; "
					"refusing a lossy changelog rewrite"
				)
			headings.append((index, date_str))

	seen_dates: dict[str, int] = {}
	for index, date_str in headings:
		if date_str in seen_dates:
			raise ValueError(
				f"duplicate date '{date_str}' at {source}:{index + 1} "
				f"(first seen at line {seen_dates[date_str] + 1})"
			)
		seen_dates[date_str] = index

	if not headings:
		return text, []
	first_index = headings[0][0]
	preamble = "".join(lines[:first_index])
	blocks: list[tuple[str, str]] = []
	for heading_index, (line_index, date_str) in enumerate(headings):
		if heading_index + 1 < len(headings):
			end_index = headings[heading_index + 1][0]
		else:
			end_index = len(lines)
		raw_text = "".join(lines[line_index:end_index])
		blocks.append((date_str, raw_text))
	return preamble, blocks


#============================================
def _insert_entry_in_day(raw_text: str, category: str, title: str) -> str:
	"""Insert one bullet while preserving every unaffected block byte."""
	lines = raw_text.splitlines(keepends=True)
	bullet = f"- {title}\n"
	category_found = False
	end_index = len(lines)
	for index, line in enumerate(lines):
		match = CATEGORY_RE.match(line)
		if match is None or match.group(1).strip() != category:
			continue
		category_found = True
		for later_index in range(index + 1, len(lines)):
			if CATEGORY_RE.match(lines[later_index]):
				end_index = later_index
				break
		break

	if category_found:
		prefix = "".join(lines[:end_index]).rstrip("\n")
		last_line = prefix.splitlines()[-1]
		separator = "\n\n" if CATEGORY_RE.match(last_line) else "\n"
		suffix = "".join(lines[end_index:]).lstrip("\n")
		result = prefix + separator + bullet
		if suffix:
			result += "\n" + suffix
		return result

	target_order = CANONICAL_CATEGORIES.index(category)
	insert_index = len(lines)
	for index, line in enumerate(lines):
		match = CATEGORY_RE.match(line)
		if match is None:
			continue
		existing_category = match.group(1).strip()
		if existing_category not in CANONICAL_CATEGORIES:
			continue
		if CANONICAL_CATEGORIES.index(existing_category) > target_order:
			insert_index = index
			break
	prefix = "".join(lines[:insert_index]).rstrip("\n")
	suffix = "".join(lines[insert_index:]).lstrip("\n")
	section = f"### {category}\n\n{bullet}"
	result = prefix + "\n\n" + section
	if suffix:
		result += "\n" + suffix
	return result


#============================================
def _write_changelog(path: str, preamble: str, blocks: list[tuple[str, str]]) -> None:
	"""Write verbatim blocks with the established single-final-newline policy."""
	parts = [preamble]
	for _date_str, raw_text in blocks:
		parts.append(raw_text)
	assembled = "".join(parts)
	normalized = assembled.rstrip("\n") + "\n"
	parent = os.path.dirname(path)
	if parent:
		os.makedirs(parent, exist_ok=True)
	with open(path, "w", encoding="utf-8") as handle:
		handle.write(normalized)


#============================================
def record_entry(repo_dir: str, date_str: str, category: str, title: str) -> str:
	"""Record one validated canonical entry and return its fixed destination."""
	if not _is_valid_iso_date(date_str):
		raise ValueError(f"invalid changelog date: {date_str!r}")
	if category not in CANONICAL_CATEGORIES:
		raise ValueError(f"non-canonical changelog category: {category!r}")
	normalized_title = title.strip()
	if not normalized_title or "\n" in normalized_title or "\r" in normalized_title:
		raise ValueError("changelog title must be one non-empty line")

	changelog_path = changelog_path_for_repo(repo_dir)
	text = ""
	if os.path.isfile(changelog_path):
		with open(changelog_path, "r", encoding="utf-8") as handle:
			text = handle.read()
	preamble, blocks = _validated_blocks(text, changelog_path)
	matched = False
	for index, (block_date, raw_text) in enumerate(blocks):
		if block_date == date_str:
			blocks[index] = (block_date, _insert_entry_in_day(
				raw_text, category, normalized_title,
			))
			matched = True
			break
	if not matched:
		new_raw_text = (
			f"## {date_str}\n\n"
			f"### {category}\n\n"
			f"- {normalized_title}\n\n"
		)
		blocks.insert(0, (date_str, new_raw_text))
	if preamble and not preamble.endswith("\n\n"):
		preamble = preamble.rstrip("\n") + "\n\n"
	_write_changelog(changelog_path, preamble, blocks)
	return changelog_path
