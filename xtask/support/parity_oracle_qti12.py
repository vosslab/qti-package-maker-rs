"""Semantic projections of QTI 1.2 XML archives for parity comparison."""

import decimal
import pathlib
import re
import zipfile
from defusedxml import ElementTree as element_tree
from xtask.support.parity_oracle_writer import collapse



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


def xml_projection(path: pathlib.Path, html_to_image: bool = False) -> object:
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
					value = normalized_markup(text_content(child))
					response_label_groups.setdefault(response_id, []).append(value)
					if value:
						choices[(response_id, identifier)] = value
						ordered.append(value)
						response_choice_ids.setdefault(response_id, []).append(identifier)
				elif name in {"mattext", "prompt"} and not question:
					question = normalized_markup(text_content(child))
					if html_to_image:
						# The frozen writer leaves this unused loader after rasterization.
						question = question.replace(
							'<script src="https://unpkg.com/@rdkit/rdkit/dist/RDKit_minimal.js"></script>', ''
						)
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
