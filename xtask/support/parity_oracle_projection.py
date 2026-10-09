"""Text, YAML, and HTML semantic projections for parity comparison."""

import base64
import html
import json
import pathlib
import re
from xtask.support.parity_oracle_writer import collapse, normalized_value



def yaml_projection(path: pathlib.Path) -> object:
	try:
		import yaml
	except ModuleNotFoundError as error:
		raise RuntimeError("PyYAML is required to compare exam_yaml") from error
	return normalized_value(yaml.safe_load(path.read_text(encoding="utf-8")))


def aiken_projection(path: pathlib.Path) -> object:
	questions = []
	for block in re.split(r"\n\s*\n", path.read_text(encoding="utf-8").strip()):
		lines = [line.strip() for line in block.splitlines() if line.strip()]
		if not lines:
			continue
		choices = []
		answer = ""
		for line in lines[1:]:
			match = re.match(r"([A-Z])\.\s*(.*)", line)
			if match:
				choices.append((match.group(1), collapse(match.group(2))))
			elif line.startswith("ANSWER:"):
				answer = line.partition(":")[2].strip()
		questions.append({"question": collapse(lines[0]), "choices": choices, "answer": answer})
	return questions


def selftest_projection(path: pathlib.Path) -> object:
	"""Extract each HTML self-test's embedded answer data without executing JavaScript."""
	text = html.unescape(path.read_text(encoding="utf-8"))

	def attribute(source: str, name: str) -> str | None:
		match = re.search(rf"\b{re.escape(name)}=(['\"])(.*?)\1", source, flags=re.I | re.S)
		return match.group(2) if match else None

	def answer_values(value: str) -> list[str]:
		if value.startswith("["):
			parsed = json.loads(value)
			if not isinstance(parsed, list) or not all(isinstance(item, str) for item in parsed):
				raise ValueError("self-test answer JSON must be a string list")
			return parsed
		try:
			return base64.b64decode(value, validate=True).decode("utf-8").split("\x1f")
		except (ValueError, UnicodeDecodeError) as error:
			raise ValueError(f"self-test answer is neither JSON nor base64: {value!r}") from error
	from lxml import html as html_parser
	root = html_parser.fromstring(path.read_text(encoding="utf-8"))
	choices = []
	for match in re.finditer(r"<input\b(?P<input>[^>]*)>\s*<label\b[^>]*>(?P<label>.*?)</label>", text, flags=re.I | re.S):
		attributes = match.group("input")
		correct = attribute(attributes, "data-correct")
		answers = attribute(attributes, "data-answers")
		if correct or answers:
			label = re.sub(r"<[^>]+>", " ", match.group("label"))
			choices.append({
				"text": collapse(label),
				"correct": correct or "",
				"answers": answer_values(answers) if answers else [],
			})
	result: dict[str, object] = {}
	if choices:
		result["choices"] = choices

	blanks = []
	for match in re.finditer(r"<input\b(?P<input>[^>]*)>", text, flags=re.I | re.S):
		attributes = match.group("input")
		answers = attribute(attributes, "data-answers")
		name = attribute(attributes, "name") or attribute(attributes, "aria-label")
		if answers and re.search(r"\bfib-blank\b", attribute(attributes, "class") or ""):
			blanks.append({
				"name": name or "",
				"answers": answer_values(answers),
			})
	if blanks:
		result["blanks"] = blanks
	plain_fib = []
	for match in re.finditer(r"<input\b(?P<input>[^>]*)>", text, flags=re.I | re.S):
		attributes = match.group("input")
		answers = attribute(attributes, "data-answers")
		if answers and re.search(r"\bqti-fib-input\b", attribute(attributes, "class") or ""):
			plain_fib.extend(answer.lower() for answer in answer_values(answers))
	if plain_fib:
		result["fib_answers"] = plain_fib

	fib_answers = re.search(r"\bconst\s+fibAnswers_[A-Za-z0-9_]+\s*=\s*(\[[^;]+\]);", text)
	if fib_answers:
		result["fib_answers"] = json.loads(fib_answers.group(1))

	answer = re.search(r"\bconst\s+numAnswer_[A-Za-z0-9_]+\s*=\s*([^;]+);", text)
	tolerance = re.search(r"\bconst\s+numTolerance_[A-Za-z0-9_]+\s*=\s*([^;]+);", text)
	if answer and tolerance:
		result["numeric"] = [float(answer.group(1)), float(tolerance.group(1))]
	for item in root.xpath('//*[@data-kind="num"]'):
		answer_value = item.get("data-answer")
		tolerance_value = item.get("data-tolerance")
		if answer_value is not None and tolerance_value is not None:
			result["numeric"] = [float(answer_value), float(tolerance_value)]

	match_choices = []
	for choice in re.finditer(r"<button\b(?P<attrs>[^>]*)>(?P<body>.*?)</button>", text, flags=re.I | re.S):
		attributes = choice.group("attrs")
		if not re.search(r"\bqti-match-choice\b", attribute(attributes, "class") or ""):
			continue
		value = attribute(attributes, "data-value")
		if value is None:
			raise ValueError("self-test MATCH choice lacks data-value")
		# Grading maps the visible authored choice; an explicit accessible name is
		# validated separately against that choice by the source-selection check.
		label = re.sub(r"<[^>]+>", " ", choice.group("body"))
		if any(existing == value for existing, _ in match_choices):
			raise ValueError(f"self-test MATCH has duplicate choice token {value}")
		match_choices.append((value, collapse(label.removeprefix("Select "))))
	match_values = dict(match_choices)
	if all(
		label.startswith(f"{chr(ord('A') + index)}. ")
		for index, (_, label) in enumerate(match_choices)
	):
		match_values = {
			value: label.removeprefix(f"{chr(ord('A') + index)}. ")
			for index, (value, label) in enumerate(match_choices)
		}
	match_pairs = []
	for slot in root.xpath('//button[contains(concat(" ", normalize-space(@class), " "), " qti-match-slot ")]'):
		rows = slot.xpath('ancestor::tr[1]')
		cells = rows[0].xpath('./td') if rows else []
		correct = slot.get("data-correct")
		if correct is None or not cells:
			raise ValueError("self-test MATCH row lacks its answer token or prompt")
		if correct not in match_values:
			raise ValueError(f"self-test MATCH row references missing choice token {correct}")
		prompt = collapse(" ".join(cells[-1].itertext()))
		match_pairs.append({"prompt": re.sub(r"^\d+\.\s*", "", prompt), "answer": match_values[correct]})
	if match_pairs:
		result["match"] = match_pairs
	return result


def normalized_html(path: pathlib.Path) -> str:
	from lxml import etree, html
	# Compare browser-decoded entities without dropping markup or answer marks.
	root = html.document_fromstring(path.read_text(encoding="utf-8"))
	from xtask.support.parity_human_tables import normalize
	for pre in root.xpath("//pre"):
		if len(pre) == 0 and pre.text is not None:
			pre.text = normalize(pre.text)
	text = etree.tostring(root, encoding="unicode", method="html")
	def normalize_style(match: re.Match[str]) -> str:
		css = collapse(match.group(1))
		css = re.sub(r"\s*([{}:;,])\s*", r"\1", css)
		return f"<style>{css}</style>"
	text = re.sub(r"<style\b[^>]*>(.*?)</style>", normalize_style, text, flags=re.I | re.S)
	return collapse(re.sub(r">\s+<", "><", text))


def html_to_image_structure(path: pathlib.Path) -> object:
	text = path.read_text(encoding="utf-8")
	images = re.findall(r"<img\b[^>]*>", text, flags=re.I)
	result = []
	for image in images:
		src = re.search(r"\bsrc=[\"']([^\"']+)", image, flags=re.I)
		alt = re.search(r"\balt=[\"']([^\"']*)", image, flags=re.I)
		result.append({"src": src.group(1) if src else "", "alt": collapse(alt.group(1)) if alt else ""})
	return result


def text_path(text: str) -> pathlib.Path:
	"""Present decoded XML to the same structural parser without retaining a temp file."""
	# Regex uses text only; this tiny Path-like carrier keeps call sites readable.
	class TextPath:
		def read_text(self, encoding: str = "utf-8") -> str:
			return text
	return TextPath()  # type: ignore[return-value]
