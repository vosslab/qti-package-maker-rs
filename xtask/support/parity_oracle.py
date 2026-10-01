#!/usr/bin/env python3
"""Pinned-Python half of the M13 differential parity harness.

It is deliberately development tooling.  The Rust binary never imports this
module, and this script only imports the archived oracle selected by its caller.
"""

import argparse
import base64
import contextlib
import decimal
import html
import hashlib
import json
import os
import pathlib
import re
# The development oracle invokes a fixed archived Python executable.
import subprocess  # nosec B404
import sys
import tempfile
import time
import zipfile
from datetime import date, datetime
from defusedxml import ElementTree as element_tree


ENGINES = (
	"bbq_text_upload",
	"text2qti",
	"okla_chrst_bqgen",
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
	"exam_yaml",
	"moodle_aiken",
	"human_readable",
	"html_selftest",
)
READBACK_ENGINES = {
	"bbq_text_upload",
	"text2qti",
	"okla_chrst_bqgen",
	"blackboard_export_zip",
}
ZIP_ENGINES = {"blackboard_export_zip", "canvas_qti_v1_2", "blackboard_qti_v2_1"}
ZIP_ENGINE_SEQUENCE = (
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
)
CLI_ENGINES = {
	"bbq_text_upload",
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
	"moodle_aiken",
	"human_readable",
	"html_selftest",
}
REGISTERED_ONLY_ENGINES = {"text2qti", "okla_chrst_bqgen", "exam_yaml"}


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser(description="Pinned Python M13 parity support.")
	parser.add_argument("mode", choices=("write", "compare", "order-receipt", "readback", "selftest"))
	parser.add_argument("--oracle-root", type=pathlib.Path)
	parser.add_argument("--input", type=pathlib.Path)
	parser.add_argument("--output", type=pathlib.Path)
	parser.add_argument("--python-output", type=pathlib.Path)
	parser.add_argument("--rust-output", type=pathlib.Path)
	parser.add_argument("--html-to-image", action="store_true")
	parser.add_argument("--zip-only", action="store_true")
	parser.add_argument("--engines", help="comma-separated fixed engine subset")
	parser.add_argument("--canvas-multifib-repair", action="store_true")
	args = parser.parse_args()
	if args.mode != "selftest" and args.oracle_root is None:
		parser.error("--oracle-root is required outside selftest mode")
	if args.mode in {"write", "order-receipt"} and (args.input is None or args.output is None):
		parser.error(f"{args.mode} requires --input and --output")
	if args.mode == "readback" and args.input is None:
		parser.error("readback requires --input")
	if args.mode == "compare" and (args.python_output is None or args.rust_output is None):
		parser.error("compare requires --python-output and --rust-output")
	return args


def install_oracle(root: pathlib.Path) -> None:
	if not (root / "qti_package_maker").is_dir():
		raise ValueError(f"oracle package unavailable: {root}")
	sys.path.insert(0, str(root))


def extension(engine: str) -> str:
	if engine in ZIP_ENGINES:
		return "zip"
	if engine == "exam_yaml":
		return "yaml"
	if engine in {"human_readable", "html_selftest"}:
		return "html"
	return "txt"


def selected_engines(args: argparse.Namespace) -> tuple[str, ...]:
	if args.engines:
		engines = tuple(args.engines.split(","))
		unknown = set(engines).difference(ENGINES)
		if unknown:
			raise ValueError(f"unknown parity engine(s): {', '.join(sorted(unknown))}")
		return engines
	if args.zip_only:
		return ZIP_ENGINE_SEQUENCE
	return ENGINES


def write_outputs(args: argparse.Namespace) -> None:
	from qti_package_maker import package_interface

	args.output.mkdir(parents=True, exist_ok=True)
	(args.output / "source_input_receipt.json").write_text(json.dumps({"path": str(args.input.resolve()), "sha256": hashlib.sha256(args.input.read_bytes()).hexdigest()}, sort_keys=True) + "\n")
	result: dict[str, object] = {"outputs": {}, "errors": {}, "no_output": {}}
	for engine in selected_engines(args):
		try:
			destination = args.output / f"{engine}.{extension(engine)}"
			# Frozen engines print recoverable media warnings directly.  Preserve them on
			# stderr so this development protocol has exactly one JSON stdout document.
			with contextlib.redirect_stdout(sys.stderr):
				if engine in CLI_ENGINES:
					write_via_cli(args.oracle_root, args.input, destination, engine, args.html_to_image)
				else:
					write_via_registered_adapter(
						package_interface, args.input, destination, engine
					)
			if not destination.is_file():
				result["no_output"][engine] = "writer completed successfully without an output artifact"
			else:
				result["outputs"][engine] = str(destination)
		except Exception as error:  # The error itself is observable parity data.
			result["errors"][engine] = f"{type(error).__name__}: {error}"
	print(json.dumps(result, sort_keys=True))


def write_via_cli(
	oracle_root: pathlib.Path,
	input_path: pathlib.Path,
	destination: pathlib.Path,
	engine: str,
	html_to_image: bool,
) -> None:
	"""Run frozen `bbq_converter.py` for the seven formats it actually exposes."""
	command = [
		sys.executable,
		str(oracle_root / "tools" / "bbq_converter.py"),
		"-i", str(input_path), "-f", engine, "-o", str(destination), "-q",
		"--allow-mixed",
	]
	if html_to_image and engine in ZIP_ENGINES:
		command.append("--html-to-image")
	environment = os.environ.copy()
	environment["PYTHONPATH"] = str(oracle_root) + os.pathsep + environment.get("PYTHONPATH", "")
	# The executable and arguments are fixed; this call never invokes a shell.
	result = subprocess.run(  # nosec B603
		command, cwd=destination.parent, env=environment, capture_output=True,
		text=True, check=False,
	)
	if result.returncode != 0:
		raise RuntimeError(
			f"frozen CLI exit {result.returncode}: {result.stderr.strip() or result.stdout.strip()}"
		)


def write_via_registered_adapter(
	package_interface: object,
	input_path: pathlib.Path,
	destination: pathlib.Path,
	engine: str,
) -> None:
	"""Write only the three frozen registered engines absent from the CLI parser."""
	if engine not in REGISTERED_ONLY_ENGINES:
		raise ValueError(f"not a registered-only engine: {engine}")
	# Mirror the frozen CLI's extract_core_name() exactly.  Registered-only
	# engines bypass its argparse table, but must receive the same package name.
	match = re.fullmatch(r"bbq-(.+?)-questions\.txt", input_path.name)
	if match is None:
		raise ValueError(f"Filename '{input_path}' does not match expected pattern.")
	interface = package_interface.QTIPackageInterface(
		match.group(1), verbose=False, allow_mixed=True
	)
	interface.read_package(str(input_path), "bbq_text_upload")
	written = interface.save_package(engine, str(destination))
	if written is None:
		raise RuntimeError("registered writer completed without an output path")


def order_receipt(args: argparse.Namespace) -> None:
	"""Record the deliberately asymmetric ORDER behavior of the two frozen CLI writers."""
	args.output.mkdir(parents=True, exist_ok=True)
	result = {}
	for engine in ("canvas_qti_v1_2", "blackboard_export_zip"):
		prefix = "qti12" if engine == "canvas_qti_v1_2" else "bez"
		destination = args.output / f"{prefix}-parity-order-only.zip"
		command = [
			sys.executable, str(args.oracle_root / "tools" / "bbq_converter.py"),
			"-i", str(args.input), "-f", engine, "-q",
		]
		environment = os.environ.copy()
		environment["PYTHONPATH"] = str(args.oracle_root) + os.pathsep + environment.get("PYTHONPATH", "")
		# The executable and arguments are fixed; this call never invokes a shell.
		completed = subprocess.run(  # nosec B603
			command, cwd=args.output, env=environment, text=True, capture_output=True, check=False
		)
		result[engine] = {
			"exit": completed.returncode,
			"file": destination.is_file(),
			"diagnostic": collapse(completed.stdout + " " + completed.stderr),
		}
	print(json.dumps(result, sort_keys=True))


def collapse(value: str) -> str:
	return " ".join(value.split())


def normalized_value(value: object) -> object:
	if isinstance(value, (date, datetime)):
		return value.isoformat()
	if isinstance(value, str):
		return collapse(value)
	if isinstance(value, list):
		return [normalized_value(child) for child in value]
	if isinstance(value, tuple):
		return [normalized_value(child) for child in value]
	if isinstance(value, dict):
		return {str(key): normalized_value(value[key]) for key in sorted(value)}
	return value


def readback_fingerprint(path: pathlib.Path, engine: str) -> object:
	from qti_package_maker import package_interface

	interface = package_interface.QTIPackageInterface("parity_read", allow_mixed=True)
	interface.read_package(str(path), engine)
	items = []
	for item in interface.item_bank:
		fields = {"kind": item.item_type, "question": collapse(item.question_text)}
		for field in item.get_supporting_field_names():
			fields[field] = normalized_value(getattr(item, field))
		items.append(fields)
	return items


def readback_outcome(path: pathlib.Path, engine: str) -> object:
	try:
		return {"outcome": "success", "items": readback_fingerprint(path, engine)}
	except Exception as error:
		return {"outcome": "rejected", "error_type": type(error).__name__, "message": str(error)}


def zip_xml_members(path: pathlib.Path) -> list[tuple[str, bytes]]:
	with zipfile.ZipFile(path) as archive:
		return [
			(name, archive.read(name))
			for name in sorted(archive.namelist())
			if name.lower().endswith((".xml", ".dat"))
		]


def assessment_resource(name: str) -> bool:
	"""Identify package members that must parse as item XML, not manifests."""
	return pathlib.PurePosixPath(name).name.lower() != "imsmanifest.xml"


def text_content(element: object) -> str:
	return collapse("".join(element.itertext()))


def normalized_markup(value: str) -> str:
	"""Normalize serialization-only empty-element whitespace in embedded item HTML."""
	value = re.sub(r"\s+/>", "/>", collapse(value))
	def normalize_tag(match: re.Match[str]) -> str:
		return re.sub(
			r"(['\"])([^'\"]*)\1",
			lambda attribute: f'"{attribute.group(2)}"',
			match.group(0),
		)
	return re.sub(r"<[^>]+>", normalize_tag, value)


def xml_projection(path: pathlib.Path) -> object:
	"""Extract meaning, dereferencing choice IDs before comparison.

	The two QTI writers use different generated identifiers.  This projection only
	keeps question text, ordered choice text, and correct choice text, which is the
	format-level semantic contract in the M13 plan.
	"""
	items = []
	for name, payload in zip_xml_members(path):
		try:
			root = element_tree.fromstring(payload)
		except element_tree.ParseError as error:
			if assessment_resource(name):
				raise ValueError(f"assessment XML member {name} is malformed: {error}") from error
			continue
		for node in root.iter():
			local = node.tag.rsplit("}", 1)[-1].lower()
			if local not in {"item", "assessmentitem"}:
				continue
			choices: dict[tuple[str, str], str] = {}
			ordered = []
			response_kinds: dict[str, str] = {}
			response_forms: dict[str, dict[str, str]] = {}
			response_choice_ids: dict[str, list[str]] = {}
			response_label_groups: dict[str, list[str]] = {}
			question = ""

			def collect_presentation(child: object, response_id: str = "") -> None:
				nonlocal question
				name = child.tag.rsplit("}", 1)[-1].lower()
				identifier = child.attrib.get("ident") or child.attrib.get("identifier")
				if name in {"response_lid", "response_str", "response_num"} and identifier:
					response_id = identifier
					response_kinds[response_id] = name
					rcardinality = child.attrib.get("rcardinality", "Single")
					if rcardinality not in {"Single", "Multiple"}:
						raise ValueError(
							f"unsupported QTI response cardinality {rcardinality!r}"
						)
					response_forms[response_id] = {
						"kind": name,
						"rcardinality": rcardinality,
					}
				if name == "render_fib" and response_id:
					response_forms.setdefault(response_id, {})["fibtype"] = child.attrib.get("fibtype", "")
				if name in {"response_label", "simplechoice"} and identifier:
					value = text_content(child)
					response_label_groups.setdefault(response_id, []).append(value)
					if value:
						choices[(response_id, identifier)] = value
						ordered.append(value)
						response_choice_ids.setdefault(response_id, []).append(identifier)
				elif name in {"mattext", "prompt"} and not question:
					question = normalized_markup(text_content(child))
				for grandchild in child:
					collect_presentation(grandchild, response_id)

			presentation = next((child for child in node.iter() if child.tag.rsplit("}", 1)[-1].lower() == "presentation"), node)
			collect_presentation(presentation)
			for response_id, form in response_forms.items():
				form["labels"] = response_label_groups.get(response_id, [])
			answers: list[tuple[str, str]] = []
			lower_bounds: dict[str, float] = {}
			upper_bounds: dict[str, float] = {}

			def numeric_response(response_id: str) -> bool:
				form = response_forms.get(response_id, {})
				return (
					response_kinds.get(response_id) == "response_num"
					or form.get("fibtype") == "Decimal"
				)

			def collect_conditions(child: object, negated: bool = False) -> None:
				name = child.tag.rsplit("}", 1)[-1].lower()
				negated = negated or name == "not"
				response_id = child.attrib.get("respident", "")
				value = text_content(child).strip()
				if not negated and name == "varequal" and value:
					choice = choices.get((response_id, value))
					if choice is not None:
						answers.append((response_id, choice))
					elif response_kinds.get(response_id) in {"response_str", "response_num"}:
						answers.append((response_id, value))
					elif response_kinds.get(response_id) == "response_lid":
						raise ValueError(
							f"unresolved QTI response label {response_id}:{value}"
						)
				if not negated and name == "vargte" and value and numeric_response(response_id):
					lower_bounds[response_id] = float(value)
				if not negated and name == "varlte" and value and numeric_response(response_id):
					upper_bounds[response_id] = float(value)
				for grandchild in child:
					collect_conditions(grandchild, negated)

			for condition in node.iter():
				if condition.tag.rsplit("}", 1)[-1].lower() == "conditionvar":
					collect_conditions(condition)
			numeric = []
			for response_id, answer in answers:
				if numeric_response(response_id) and ":" in answer:
					value, tolerance = answer.split(":", 1)
					numeric.append([float(value), float(tolerance)])
			for response_id, lower in lower_bounds.items():
				if response_id in upper_bounds:
					value = next(
						(
							float(answer)
							for answer_response_id, answer in answers
							if answer_response_id == response_id and ":" not in answer
						),
						None,
					)
					if value is not None:
						numeric.append([value, round((upper_bounds[response_id] - lower) / 2, 12)])

			score_program = []
			score_declaration: object = {}
			response_conditions = []
			if root.tag.rsplit("}", 1)[-1].lower() == "questestinterop":
				response_conditions = [
					condition for condition in node.iter()
					if condition.tag.rsplit("}", 1)[-1].lower() == "respcondition"
				]
			if response_conditions:
				decvars = [
					candidate
					for candidate in node.iter()
					if candidate.tag.rsplit("}", 1)[-1].lower() == "decvar"
				]
				if len(decvars) != 1:
					raise ValueError(
						f"QTI score program requires exactly one decvar, found {len(decvars)}"
					)
				decvar = decvars[0]
				if decvar.attrib.get("varname") != "SCORE":
					raise ValueError("QTI score declaration must target SCORE")
				score_declaration = {
					"varname": "SCORE",
					"vartype": decvar.attrib.get("vartype", ""),
					"defaultval": decvar.attrib.get("defaultval", ""),
					"minvalue": decvar.attrib.get("minvalue", ""),
					"maxvalue": decvar.attrib.get("maxvalue", ""),
				}
				response_positions = {
					response_id: index
					for index, response_id in enumerate(response_kinds)
				}

				def exact_decimal(value: str) -> str:
					"""Return one finite decimal spelling for a grading value."""
					try:
						parsed = decimal.Decimal(value)
					except decimal.InvalidOperation as error:
						raise ValueError(f"invalid QTI score decimal {value!r}") from error
					if not parsed.is_finite():
						raise ValueError(f"non-finite QTI score decimal {value!r}")
					if parsed.is_zero():
						return "0"
					return format(parsed.normalize(), "f")

				def dereference_label(response_id: str, label_id: str) -> str:
					"""Bind a QTI label ID to one authored value without ambiguity."""
					choice = choices.get((response_id, label_id))
					if choice is None:
						raise ValueError(
							f"unresolved QTI score label {response_id}:{label_id}"
						)
					matching_ids = [
						identifier
						for identifier in response_choice_ids[response_id]
						if choices[(response_id, identifier)] == choice
					]
					if len(matching_ids) != 1:
						raise ValueError(
							f"ambiguous QTI score label {response_id}:{label_id} -> {choice!r}"
						)
					return choice

				def condition_tree(child: object) -> object:
					name = child.tag.rsplit("}", 1)[-1].lower()
					children = list(child)
					if name in {"conditionvar", "and", "or"}:
						return [name, [condition_tree(grandchild) for grandchild in children]]
					if name == "not":
						if len(children) != 1:
							raise ValueError("QTI not condition must have exactly one child")
						return [name, condition_tree(children[0])]
					response_id = child.attrib.get("respident", "")
					if name in {"varequal", "vargte", "varlte"}:
						if response_id not in response_positions:
							raise ValueError(
								f"QTI score condition references unknown response {response_id!r}"
							)
						value = text_content(child).strip()
						if response_kinds.get(response_id) == "response_lid":
							value = dereference_label(response_id, value)
						elif response_kinds[response_id] == "response_num":
							value = exact_decimal(value)
						else:
							if not value:
								raise ValueError("empty QTI string score predicate")
						return [name, response_positions[response_id], value]
					raise ValueError(f"unsupported QTI score condition {name}")

				for condition in response_conditions:
					children = list(condition)
					condition_var = next((child for child in children if child.tag.rsplit("}", 1)[-1].lower() == "conditionvar"), None)
					if condition_var is None:
						raise ValueError("QTI response condition lacks conditionvar")
					actions = []
					for child in children:
						name = child.tag.rsplit("}", 1)[-1].lower()
						if child is condition_var:
							continue
						if name != "setvar":
							raise ValueError(f"unsupported QTI score-program element {name}")
						if child.attrib.get("varname") != "SCORE":
							raise ValueError("QTI score action must target SCORE")
						action = child.attrib.get("action")
						if action not in {"Set", "Add"}:
							raise ValueError(f"unsupported QTI SCORE action {action!r}")
						actions.append({
							"varname": "SCORE",
							"action": action,
							"value": exact_decimal(text_content(child).strip()),
						})
					if not actions:
						raise ValueError("QTI response condition has no SCORE action")
					continue_value = condition.attrib.get("continue", "Yes")
					if continue_value not in {"Yes", "No"}:
						raise ValueError(f"unsupported QTI response condition continue={continue_value!r}")
					score_program.append({
						"continue": continue_value,
						"predicate": condition_tree(condition_var),
						"actions": actions,
					})

			semantic_answers = []
			for response_id, answer in answers:
				if numeric_response(response_id):
					value = answer.partition(":")[0]
					semantic_answers.append(str(float(value)))
				else:
					semantic_answers.append(answer)
			items.append({
				"question": question,
				"choices": ordered,
				"answers": semantic_answers,
				"numeric": numeric,
				"responses": list(response_forms.values()),
				"score_declaration": score_declaration,
				"score_program": score_program,
			})
	return items


def canvas_multifib_repair_projection(path: pathlib.Path) -> object:
	"""Read the repaired Canvas MULTI_FIB answers against the authored fixture meaning."""
	for name, payload in zip_xml_members(path):
		try:
			root = element_tree.fromstring(payload)
		except element_tree.ParseError as error:
			if assessment_resource(name):
				raise ValueError(f"assessment XML member {name} is malformed: {error}") from error
			continue
		for item in root.iter():
			if item.tag.rsplit("}", 1)[-1].lower() != "item":
				continue
			if "A [animal] is a [class]." not in text_content(item):
				continue
			labels: dict[str, dict[str, str]] = {}
			for response in item.iter():
				if response.tag.rsplit("}", 1)[-1].lower() != "response_lid":
					continue
				response_id = response.attrib.get("ident", "")
				labels[response_id] = {
					label.attrib.get("ident", ""): text_content(label)
					for label in response.iter()
					if label.tag.rsplit("}", 1)[-1].lower() == "response_label"
				}
			if not labels:
				raise ValueError("Canvas MULTI_FIB has no response_lid choice labels")
			answers = []
			for value in item.iter():
				if value.tag.rsplit("}", 1)[-1].lower() != "varequal":
					continue
				response_id = value.attrib.get("respident", "")
				choice_id = text_content(value)
				if response_id not in labels or choice_id not in labels[response_id]:
					raise ValueError(
						f"unresolved Canvas MULTI_FIB answer {response_id}:{choice_id}"
					)
				answers.append(labels[response_id][choice_id])
			return sorted(answers)
	raise ValueError("Canvas MULTI_FIB item was not found")


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
	for section in re.finditer(r"<section\b(?P<attrs>[^>]*)>(?P<body>.*?)</section>", text, flags=re.I | re.S):
		if attribute(section.group("attrs"), "data-kind") != "num":
			continue
		answer_value = attribute(section.group("attrs"), "data-answer")
		tolerance_value = attribute(section.group("attrs"), "data-tolerance")
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
	from lxml import html as html_parser
	match_pairs = []
	root = html_parser.fromstring(path.read_text(encoding="utf-8"))
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


def compare_original(engine: str, python_path: pathlib.Path, rust_path: pathlib.Path, html_to_image: bool) -> tuple[object, object]:
	if html_to_image and engine == "blackboard_export_zip":
		return html_to_image_structure_from_zip(python_path), html_to_image_structure_from_zip(rust_path)
	if engine == "blackboard_export_zip":
		from xtask.support import parity_blackboard
		return (
			{"semantic": parity_blackboard.xml_projection(python_path, allow_frozen_source_repairs=True), "readback": readback_outcome(python_path, engine)},
			{"semantic": parity_blackboard.xml_projection(rust_path), "readback": readback_outcome(rust_path, engine)},
		)
	if engine == "text2qti":
		from xtask.support import parity_text2qti
		return parity_text2qti.frozen_readback(python_path, readback_fingerprint), readback_fingerprint(rust_path, engine)
	if engine == "okla_chrst_bqgen":
		from xtask.support import parity_okla
		return parity_okla.compare(python_path, rust_path, readback_fingerprint, write_via_registered_adapter)
	if engine in READBACK_ENGINES:
		return readback_fingerprint(python_path, engine), readback_fingerprint(rust_path, engine)
	if engine == "blackboard_qti_v2_1":
		from xtask.support import parity_qti21
		from xtask.support import parity_qti21_multifib_repair
		python_value, _ = parity_qti21_multifib_repair.frozen_projection(python_path)
		return python_value, parity_qti21.xml_projection(rust_path)
	if engine == "canvas_qti_v1_2":
		from xtask.support.parity_canvas_multifib_repair import frozen_projection
		return frozen_projection(python_path, xml_projection), xml_projection(rust_path)
	if engine == "exam_yaml":
		return yaml_projection(python_path), yaml_projection(rust_path)
	if engine == "moodle_aiken":
		return aiken_projection(python_path), aiken_projection(rust_path)
	if engine == "human_readable":
		return normalized_html(python_path), normalized_html(rust_path)
	if engine == "html_selftest":
		from xtask.support import parity_selftest_selection
		return parity_selftest_selection.compare(python_path, rust_path, selftest_projection)
	raise ValueError(f"unhandled parity engine {engine}")


def compare_one(engine: str, python_path: pathlib.Path, rust_path: pathlib.Path, html_to_image: bool) -> tuple[object, object]:
	try:
		values = compare_original(engine, python_path, rust_path, html_to_image)
	except ValueError as failure:
		if html_to_image:
			raise
		from xtask.support.parity_choice_label_repair import compare
		repaired = compare(engine, python_path, rust_path, compare_original)
		if repaired is None:
			raise failure
		return repaired
	if values[0] != values[1] and not html_to_image:
		from xtask.support.parity_choice_label_repair import compare
		repaired = compare(engine, python_path, rust_path, compare_original)
		if repaired is not None:
			return repaired
	return values


def compare_outputs(args: argparse.Namespace) -> None:
	divergences = []
	for engine in selected_engines(args):
		python_path = args.python_output / f"{engine}.{extension(engine)}"
		rust_path = args.rust_output / f"{engine}.{extension(engine)}"
		if not python_path.is_file() or not rust_path.is_file():
			divergences.append({
				"engine": engine, "item": "package", "field": "writer outcome",
				"python": "present" if python_path.is_file() else "missing",
				"rust": "present" if rust_path.is_file() else "missing",
			})
			continue
		try:
			if args.canvas_multifib_repair and engine == "canvas_qti_v1_2":
				expected = ["dog", "mammal"]
				rust_value = canvas_multifib_repair_projection(rust_path)
				if rust_value != expected:
					divergences.append({
						"engine": engine, "item": "MULTI_FIB validity repair",
						"field": "authored blank-answer mapping",
						"python": json.dumps(expected),
						"rust": json.dumps(rust_value),
					})
				continue
			# Archived readers can print malformed embedded-markup diagnostics while
			# continuing to return a usable item bank.  This protocol reserves stdout
			# for its final JSON receipt, so retain those diagnostics on stderr.
			with contextlib.redirect_stdout(sys.stderr):
				python_value, rust_value = compare_one(
					engine, python_path, rust_path, args.html_to_image
				)
			if python_value != rust_value:
				divergences.append({
					"engine": engine, "item": "all", "field": "semantic projection",
					"python": json.dumps(python_value, sort_keys=True),
					"rust": json.dumps(rust_value, sort_keys=True),
				})
			if args.html_to_image and engine in ZIP_ENGINES:
				python_images = html_to_image_structure_from_zip(python_path)
				rust_images = html_to_image_structure_from_zip(rust_path)
				if python_images != rust_images:
					divergences.append({
						"engine": engine, "item": "all", "field": "html-to-image image names and alt text",
						"python": json.dumps(python_images, sort_keys=True),
						"rust": json.dumps(rust_images, sort_keys=True),
					})
		except Exception as error:
			divergences.append({
				"engine": engine, "item": "package", "field": "comparator error",
				"python": "comparison completed", "rust": f"{type(error).__name__}: {error}",
			})
	print(json.dumps({"divergences": divergences}, sort_keys=True))


def readback_outputs(args: argparse.Namespace) -> None:
	"""Emit the pinned reader's complete normalized item fields for one package."""
	engines = selected_engines(args)
	if len(engines) != 1:
		raise ValueError("readback requires exactly one engine")
	print(json.dumps(readback_fingerprint(args.input, engines[0]), sort_keys=True))


def html_to_image_structure_from_zip(path: pathlib.Path) -> object:
	from xtask.support import parity_images
	return parity_images.contract(path)


def text_path(text: str) -> pathlib.Path:
	"""Present decoded XML to the same structural parser without retaining a temp file."""
	# Regex uses text only; this tiny Path-like carrier keeps call sites readable.
	class TextPath:
		def read_text(self, encoding: str = "utf-8") -> str:
			return text
	return TextPath()  # type: ignore[return-value]


def score_program_selftest() -> dict[str, object]:
	from xtask.support.parity_oracle_selftest import score_program_selftest as run_selftest
	return run_selftest()


def main() -> None:
	args = parse_args()
	if args.mode == "selftest":
		print(json.dumps(score_program_selftest(), sort_keys=True))
		return
	args.oracle_root = args.oracle_root.resolve()
	if args.input is not None:
		args.input = args.input.resolve()
	if args.output is not None:
		args.output = args.output.resolve()
	if args.python_output is not None:
		args.python_output = args.python_output.resolve()
	if args.rust_output is not None:
		args.rust_output = args.rust_output.resolve()
	install_oracle(args.oracle_root)
	if args.mode == "write":
		write_outputs(args)
	elif args.mode == "order-receipt":
		order_receipt(args)
	elif args.mode == "readback":
		readback_outputs(args)
	else:
		compare_outputs(args)


if __name__ == "__main__":
	main()
