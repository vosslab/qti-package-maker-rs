"""Fail-closed Blackboard Pool XML semantic projection for M13."""

from __future__ import annotations

import math
import pathlib
import re
import tempfile
import zipfile
from html.parser import HTMLParser

from defusedxml import ElementTree as element_tree


PRIVATE_METADATA = (
	"bbmd_qti_package_maker_ma_min_answers_required",
	"bbmd_qti_package_maker_ma_allow_all_correct",
	"bbmd_qti_package_maker_num_tolerance",
	"bbmd_qti_package_maker_num_tolerance_message",
)
BLOCKS = {"br", "p", "div", "li", "tr", "td", "th"}


def local(node: element_tree.Element) -> str:
	return node.tag.rsplit("}", 1)[-1].lower()


def direct(node: element_tree.Element, name: str) -> list[element_tree.Element]:
	return [child for child in node if local(child) == name]


def descendants(node: element_tree.Element, name: str) -> list[element_tree.Element]:
	return [child for child in node.iter() if local(child) == name]


def one(nodes: list[element_tree.Element], what: str) -> element_tree.Element:
	if len(nodes) != 1:
		raise ValueError(f"Blackboard requires one {what}, found {len(nodes)}")
	return nodes[0]


def text(node: element_tree.Element) -> str:
	return "".join(node.itertext()).strip()


class Fragment(HTMLParser):
	"""Preserve visible DOM order, scripts, and styles without double-decoding."""

	def __init__(self) -> None:
		super().__init__(convert_charrefs=True)
		self.visible: list[str] = []
		self.tokens: list[dict[str, object]] = []
		self.scripts: list[dict[str, object]] = []
		self.styles: list[dict[str, object]] = []
		self.active: tuple[str, dict[str, str], list[str], tuple[int, ...]] | None = None
		self.path: list[int] = []
		self.child_counts: list[int] = [0]

	def child_path(self) -> tuple[int, ...]:
		self.child_counts[-1] += 1
		return tuple(self.path + [self.child_counts[-1] - 1])

	def visible_text(self, value: str) -> None:
		if not value:
			return
		self.visible.append(value)
		normalized = " ".join(value.split())
		if normalized:
			self.tokens.append({"text": normalized})

	def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
		path = self.child_path()
		attributes = {name: value or "" for name, value in attrs}
		if tag in {"script", "style"}:
			if self.active is not None:
				raise ValueError("nested script/style Blackboard payload")
			self.active = (tag, attributes, [], path)
			return
		if self.active is not None:
			self.active[2].append(self.get_starttag_text())
			return
		if tag in BLOCKS:
			self.visible.append("\n")
			self.tokens.append({"boundary": tag})
		self.tokens.append({"start": tag, "attributes": sorted(attributes.items())})
		if tag in {"br", "img", "hr", "input", "meta", "link", "source", "wbr"}:
			self.tokens.append({"end": tag})
			return
		self.path.append(path[-1])
		self.child_counts.append(0)

	def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
		self.handle_starttag(tag, attrs)
		if tag not in {"br", "img", "hr", "input", "meta", "link", "source", "wbr"}:
			self.handle_endtag(tag)

	def handle_endtag(self, tag: str) -> None:
		if self.active is not None:
			name, attributes, body, path = self.active
			if tag != name:
				self.active[2].append(f"</{tag}>")
				return
			entry = {"path": list(path), "attributes": sorted(attributes.items()), "text": "".join(body)}
			getattr(self, f"{name}s").append(entry)
			self.active = None
			return
		if tag not in {"br", "img", "hr", "input", "meta", "link", "source", "wbr"}:
			self.tokens.append({"end": tag})
			if not self.path:
				raise ValueError(f"unexpected Blackboard HTML closing tag {tag}")
			self.path.pop()
			self.child_counts.pop()
		if tag in BLOCKS:
			self.visible.append("\n")
			self.tokens.append({"boundary": tag})

	def handle_data(self, value: str) -> None:
		if self.active is not None:
			self.active[2].append(value)
		else:
			self.visible_text(value)

	def projection(self) -> dict[str, object]:
		from xtask.support.parity_blackboard_dom import canonical
		visible = "\n".join(" ".join(line.split()) for line in "".join(self.visible).splitlines() if line.split())
		return {"visible": visible, "tokens": canonical(self.tokens), "scripts": self.scripts, "styles": self.styles}


def fragment(value: str) -> dict[str, object]:
	parser = Fragment()
	parser.feed(value)
	parser.close()
	if parser.active:
		raise ValueError(f"unterminated Blackboard {parser.active[0]} payload")
	return parser.projection()


def metadata(item: element_tree.Element) -> dict[str, str]:
	container = one(direct(item, "itemmetadata"), "itemmetadata")
	result: dict[str, str] = {}
	for child in container:
		name = local(child)
		if name in result:
			raise ValueError(f"duplicate Blackboard item metadata {name}")
		result[name] = text(child)
	if not result.get("bbmd_questiontype"):
		raise ValueError("Blackboard item lacks bbmd_questiontype")
	for name in result:
		if name.startswith("bbmd_qti_package_maker_") and name not in PRIVATE_METADATA:
			raise ValueError(f"unknown Blackboard private metadata carrier {name}")
	return result


def source_carriers(item_metadata: dict[str, str]) -> dict[str, str]:
	"""Validate the four application-private carriers outside LMS semantics."""
	carriers = {name: item_metadata[name] for name in PRIVATE_METADATA if name in item_metadata}
	if "bbmd_qti_package_maker_ma_min_answers_required" in carriers:
		if not carriers["bbmd_qti_package_maker_ma_min_answers_required"].isdigit():
			raise ValueError("Blackboard MA minimum carrier must be an integer")
	if "bbmd_qti_package_maker_ma_allow_all_correct" in carriers:
		if carriers["bbmd_qti_package_maker_ma_allow_all_correct"] not in {"true", "false"}:
			raise ValueError("Blackboard MA all-correct carrier must be boolean")
	if "bbmd_qti_package_maker_num_tolerance" in carriers:
		try:
			value = float(carriers["bbmd_qti_package_maker_num_tolerance"])
		except ValueError as error:
			raise ValueError("Blackboard numeric tolerance carrier is invalid") from error
		if not math.isfinite(value) or value < 0:
			raise ValueError("Blackboard numeric tolerance carrier must be finite and nonnegative")
	if "bbmd_qti_package_maker_num_tolerance_message" in carriers:
		if carriers["bbmd_qti_package_maker_num_tolerance_message"] not in {"true", "false"}:
			raise ValueError("Blackboard numeric tolerance-message carrier must be boolean")
	return carriers


def match_choices(presentation: element_tree.Element) -> list[dict[str, object]]:
	"""Return MATCH's shared right-hand values in rendered order."""
	right_blocks = [flow for flow in descendants(presentation, "flow") if flow.attrib.get("class") == "RIGHT_MATCH_BLOCK"]
	if not right_blocks:
		return []
	blocks = direct(one(right_blocks, "RIGHT_MATCH_BLOCK"), "flow")
	values = []
	for block in blocks:
		values.append(fragment(text(one(descendants(block, "mat_formattedtext"), "MATCH right value"))))
	if not values:
		raise ValueError("Blackboard RIGHT_MATCH_BLOCK has no values")
	return values


def interaction_projection(item: element_tree.Element) -> tuple[list[dict[str, object]], dict[str, dict[str, dict[str, object]]], dict[str, int], dict[str, str], dict[str, tuple[int, int, dict[str, object]]]]:
	presentation = one(direct(item, "presentation"), "presentation")
	interactions = [child for child in presentation.iter() if local(child) in {"response_lid", "response_str", "response_num"}]
	if not interactions:
		raise ValueError("Blackboard item has no response interaction")
	shared_match_values = match_choices(presentation)
	result = []
	labels: dict[str, dict[str, dict[str, object]]] = {}
	positions: dict[str, int] = {}
	kinds: dict[str, str] = {}
	label_refs: dict[str, tuple[int, int, dict[str, object]]] = {}
	for position, interaction in enumerate(interactions):
		name = local(interaction)
		identifier = interaction.attrib.get("ident")
		if not identifier or identifier in labels:
			raise ValueError("Blackboard response interactions must have unique identifiers")
		values: dict[str, dict[str, object]] = {}
		choices = []
		label_nodes = descendants(interaction, "response_label")
		for label_index, label in enumerate(label_nodes):
			label_id = label.attrib.get("ident")
			if not label_id or label_id in values:
				raise ValueError("Blackboard response labels must be unique and identified")
			materials = descendants(label, "mat_formattedtext")
			if materials:
				value = fragment(text(one(materials, "choice mat_formattedtext")))
			elif shared_match_values:
				if label_index >= len(shared_match_values):
					raise ValueError("Blackboard MATCH label exceeds right-hand values")
				value = shared_match_values[label_index]
			else:
				raise ValueError("Blackboard response label lacks material")
			if label_id in label_refs:
				raise ValueError(f"Blackboard response label identifier is ambiguous: {label_id}")
			values[label_id] = value
			label_refs[label_id] = (position, label_index, value)
			choices.append({
				"value": value,
				"attributes": sorted((key, value) for key, value in label.attrib.items() if key != "ident"),
			})
		if name == "response_lid" and not choices:
			raise ValueError(f"Blackboard {identifier} has no response labels")
		if name != "response_lid" and choices:
			raise ValueError(f"Blackboard {identifier} has labels on non-choice response")
		render = direct(interaction, "render_choice") + direct(interaction, "render_fib")
		if len(render) != 1:
			raise ValueError(f"Blackboard {identifier} requires one renderer")
		labels[identifier] = values
		positions[identifier] = position
		kinds[identifier] = name
		result.append({
			"kind": name,
			"rcardinality": interaction.attrib.get("rcardinality", "Single"),
			"rtiming": interaction.attrib.get("rtiming", ""),
			"renderer": {"kind": local(render[0]), "attributes": sorted(render[0].attrib.items())},
			"choices": choices,
		})
	return result, labels, positions, kinds, label_refs


def predicate(node: element_tree.Element, labels: dict[str, dict[str, dict[str, object]]], positions: dict[str, int], kinds: dict[str, str], label_refs: dict[str, tuple[int, int, dict[str, object]]]) -> dict[str, object]:
	name = local(node)
	if name == "conditionvar":
		children = list(node)
		if len(children) == 1:
			return predicate(children[0], labels, positions, kinds, label_refs)
		if not children:
			raise ValueError("Blackboard conditionvar is empty")
		return {"conditionvar": [predicate(child, labels, positions, kinds, label_refs) for child in children]}
	if name in {"and", "or"}:
		children = list(node)
		if not children:
			raise ValueError(f"Blackboard {name} is empty")
		return {name: [predicate(child, labels, positions, kinds, label_refs) for child in children]}
	if name == "not":
		return {"not": predicate(one(list(node), "not child"), labels, positions, kinds, label_refs)}
	if name == "other":
		if list(node) or node.attrib or text(node):
			raise ValueError("Blackboard other predicate is not empty")
		return {"other": True}
	if name not in {"varequal", "vargte", "varlte"}:
		raise ValueError(f"unsupported Blackboard grading node {name}")
	response = node.attrib.get("respident")
	value = text(node)
	if not response:
		raise ValueError(f"Blackboard {name} lacks response identifier")
	allowed_attributes = {"respident", "case"} if name == "varequal" else {"respident"}
	if set(node.attrib) - allowed_attributes:
		raise ValueError(f"unsupported Blackboard {name} attributes {sorted(node.attrib)}")
	if response not in positions:
		label_ref = label_refs.get(response)
		if name != "varequal" or value or label_ref is None:
			raise ValueError(f"Blackboard grading references unknown response {response}")
		# MA emits one empty per-choice penalty condition. It is a typed
		# response-label reference, canonicalized to response and label ordinals.
		response_position, label_position, choice = label_ref
		return {"label_ref": {"response": response_position, "label": label_position, "case": node.attrib.get("case", ""), "value": choice}}
	if name == "varequal" and kinds[response] == "response_lid":
		if value not in labels[response]:
			raise ValueError(f"Blackboard unresolved response label {response}:{value}")
		value_object: object = labels[response][value]
	else:
		if not value:
			raise ValueError(f"Blackboard {name} has empty value")
		value_object = value
	return {name: {"response": positions[response], "case": node.attrib.get("case", ""), "value": value_object}}


def feedbacks(item: element_tree.Element) -> dict[str, dict[str, object]]:
	"""Dereference feedback identifiers so generated hashes cannot hide content changes."""
	result: dict[str, dict[str, object]] = {}
	for feedback in direct(item, "itemfeedback"):
		identifier = feedback.attrib.get("ident")
		if not identifier or identifier in result:
			raise ValueError("Blackboard itemfeedback identifiers must be unique")
		if set(feedback.attrib) - {"ident", "view"}:
			raise ValueError(f"unsupported Blackboard itemfeedback attributes {sorted(feedback.attrib)}")
		materials = descendants(feedback, "mat_formattedtext")
		result[identifier] = {
			"role": identifier if identifier in {"correct", "incorrect"} else "item_feedback",
			"view": feedback.attrib.get("view", ""),
			"content": [fragment(text(material)) for material in materials],
		}
	if "correct" not in result or "incorrect" not in result:
		raise ValueError("Blackboard item lacks correct or incorrect feedback targets")
	return result


def score_program(item: element_tree.Element, labels: dict[str, dict[str, dict[str, object]]], positions: dict[str, int], kinds: dict[str, str], label_refs: dict[str, tuple[int, int, dict[str, object]]], feedback_targets: dict[str, dict[str, object]]) -> tuple[dict[str, str], list[dict[str, object]]]:
	processing = one(direct(item, "resprocessing"), "resprocessing")
	outcomes = one(direct(processing, "outcomes"), "outcomes")
	decvar = one(direct(outcomes, "decvar"), "SCORE decvar")
	if decvar.attrib.get("varname") != "SCORE":
		raise ValueError("Blackboard score declaration must target SCORE")
	declaration = {key: decvar.attrib.get(key, "") for key in ("varname", "vartype", "defaultval", "minvalue", "maxvalue")}
	program = []
	reachable: set[str] = set()
	for condition in direct(processing, "respcondition"):
		condition_var = one(direct(condition, "conditionvar"), "conditionvar")
		actions = []
		for action in condition:
			name = local(action)
			if name == "conditionvar":
				continue
			if name == "displayfeedback":
				if set(action.attrib) != {"linkrefid", "feedbacktype"}:
					raise ValueError(f"unsupported Blackboard displayfeedback attributes {sorted(action.attrib)}")
				target = action.attrib["linkrefid"]
				if target not in feedback_targets:
					raise ValueError("Blackboard displayfeedback references missing target")
				reachable.add(target)
				actions.append({"displayfeedback": {"feedbacktype": action.attrib["feedbacktype"], "target": feedback_targets[target]}})
				continue
			if name != "setvar":
				raise ValueError(f"unsupported Blackboard grading action {name}")
			variable = action.attrib.get("variablename", action.attrib.get("varname"))
			if variable != "SCORE":
				raise ValueError("Blackboard score action must target SCORE")
			action_name = action.attrib.get("action", "Set")
			if action_name not in {"Set", "Add"}:
				raise ValueError(f"unsupported Blackboard SCORE action {action_name}")
			actions.append({"varname": "SCORE", "action": action_name, "value": text(action)})
		title = condition.attrib.get("title", "")
		if title in {"correct", "incorrect", ""}:
			title_value: object = title
		elif title in feedback_targets:
			reachable.add(title)
			title_value = {"feedback": feedback_targets[title]}
		elif re.fullmatch(r"[0-9a-f]{32}|[0-9a-f]{4}_[0-9a-f]{4}_(fib_answer|num_correct)_[0-9]+", title):
			# These writer-generated branch titles have no feedback edge.
			title_value = ""
		else:
			raise ValueError(f"Blackboard response condition title has no feedback target {title}")
		predicate_value = predicate(condition_var, labels, positions, kinds, label_refs)
		program.append({
			"title": title_value,
			"continue": condition.attrib.get("continue", "Yes"),
			"predicate": predicate_value,
			"actions": actions,
		})
	if not program:
		raise ValueError("Blackboard item has no score program")
	if set(feedback_targets).difference(reachable):
		raise ValueError("Blackboard item has unreachable feedback target")
	return declaration, program


def item_projection(item: element_tree.Element) -> dict[str, object]:
	item_metadata = metadata(item)
	question_blocks = [flow for flow in descendants(item, "flow") if flow.attrib.get("class") == "QUESTION_BLOCK"]
	question_flow = one(question_blocks, "QUESTION_BLOCK")
	question = fragment(text(one(descendants(question_flow, "mat_formattedtext"), "question mat_formattedtext")))
	responses, labels, positions, kinds, label_refs = interaction_projection(item)
	feedback_targets = feedbacks(item)
	declaration, program = score_program(item, labels, positions, kinds, label_refs, feedback_targets)
	source_carriers(item_metadata)
	return {
		"kind": item_metadata["bbmd_questiontype"],
		"question": question,
		"responses": responses,
		"score_declaration": declaration,
		"score_program": program,
	}


def xml_projection(path: pathlib.Path) -> list[dict[str, object]]:
	items = []
	with zipfile.ZipFile(path) as archive:
		for name in sorted(archive.namelist()):
			if not name.endswith(".dat"):
				continue
			root = element_tree.fromstring(archive.read(name))
			items.extend(item_projection(item) for item in descendants(root, "item"))
	if not items:
		raise ValueError("Blackboard ZIP has no item XML")
	return items


def source_carrier_projection(path: pathlib.Path) -> list[dict[str, str]]:
	"""Return only validated private carriers for the named paired receipt."""
	carriers = []
	with zipfile.ZipFile(path) as archive:
		for name in sorted(archive.namelist()):
			if name.endswith(".dat"):
				root = element_tree.fromstring(archive.read(name))
				carriers.extend(source_carriers(metadata(item)) for item in descendants(root, "item"))
	if not carriers:
		raise ValueError("Blackboard ZIP has no item XML")
	return carriers


def selftest() -> dict[str, object]:
	"""Prove semantic mutations survive even where readers reject embedded markup."""
	for invalid in ("nan", "inf", "-inf", "1e999", "-0.1"):
		try:
			source_carriers({"bbmd_qti_package_maker_num_tolerance": invalid})
		except ValueError:
			pass
		else:
			raise ValueError(f"invalid numeric carrier accepted: {invalid}")
	source = """<questestinterop><item><itemmetadata><bbmd_questiontype>Multiple Choice</bbmd_questiontype><bbmd_qti_package_maker_ma_min_answers_required>0</bbmd_qti_package_maker_ma_min_answers_required><bbmd_qti_package_maker_ma_allow_all_correct>false</bbmd_qti_package_maker_ma_allow_all_correct><bbmd_qti_package_maker_num_tolerance>0.01</bbmd_qti_package_maker_num_tolerance><bbmd_qti_package_maker_num_tolerance_message>true</bbmd_qti_package_maker_num_tolerance_message></itemmetadata><presentation><flow class='QUESTION_BLOCK'><material><mat_formattedtext>&lt;script&gt;x()&lt;/script&gt;Question</mat_formattedtext></material></flow><response_lid ident='response' rcardinality='Single'><render_choice shuffle='Yes'><response_label ident='a'><material><mat_formattedtext>Alpha</mat_formattedtext></material></response_label><response_label ident='b'><material><mat_formattedtext>Beta</mat_formattedtext></material></response_label></render_choice></response_lid></presentation><resprocessing><outcomes><decvar varname='SCORE' vartype='Decimal' minvalue='0' maxvalue='100'/></outcomes><respcondition><conditionvar><varequal respident='response'>a</varequal></conditionvar><setvar variablename='SCORE' action='Set'>100</setvar><displayfeedback linkrefid='correct' feedbacktype='Response'/></respcondition><respcondition><conditionvar><varequal respident='a' case='No'/></conditionvar><setvar variablename='SCORE' action='Set'>0</setvar><displayfeedback linkrefid='correct' feedbacktype='Response'/></respcondition><respcondition><conditionvar><other/></conditionvar><setvar variablename='SCORE' action='Set'>0</setvar><displayfeedback linkrefid='incorrect' feedbacktype='Response'/></respcondition></resprocessing><itemfeedback ident='correct' view='All'/><itemfeedback ident='incorrect' view='All'/></item></questestinterop>"""
	if fragment("<b>A</b>B") == fragment("A<b>B</b>"):
		raise ValueError("Blackboard structured fragment lost text ownership")
	malformed = element_tree.fromstring(source).find("item")
	penalty = malformed.find("resprocessing").findall("respcondition")[1]
	link = penalty.find("displayfeedback")
	link.set("linkrefid", "a")
	try:
		item_projection(malformed)
	except ValueError:
		pass
	else:
		raise ValueError("dangling feedback reference was accepted")
	with tempfile.TemporaryDirectory(prefix="bb-parity-") as temporary:
		archive_path = pathlib.Path(temporary) / "item.zip"
		with zipfile.ZipFile(archive_path, "w") as archive:
			archive.writestr("res00002.dat", source)
		baseline = xml_projection(archive_path)
		carrier_baseline = source_carrier_projection(archive_path)
		for name, changed in {
			"choice": source.replace("Alpha", "Changed Alpha"),
			"correct": source.replace(">a</varequal>", ">b</varequal>", 1),
			"renderer": source.replace("shuffle='Yes'", "shuffle='No'", 1),
			"feedback_target": source.replace("linkrefid='correct'", "linkrefid='incorrect'", 1),
		}.items():
			with zipfile.ZipFile(archive_path, "w") as archive:
				archive.writestr("res00002.dat", changed)
			try:
				projection = xml_projection(archive_path)
			except ValueError:
				continue
			if projection == baseline:
				raise ValueError(f"Blackboard semantic mutation was not detected: {name}")
		for name, changed in {
			"unresolved_label": source.replace(">a</varequal>", ">missing</varequal>", 1),
			"unresolved_response": source.replace("respident='response'", "respident='missing'", 1),
			"duplicate_private": source.replace("</itemmetadata>", "<bbmd_qti_package_maker_num_tolerance>0.02</bbmd_qti_package_maker_num_tolerance></itemmetadata>"),
			"missing_feedback": source.replace("linkrefid='correct'", "linkrefid='missing'", 1),
		}.items():
			with zipfile.ZipFile(archive_path, "w") as archive:
				archive.writestr("res00002.dat", changed)
			try:
				xml_projection(archive_path)
			except ValueError:
				continue
			raise ValueError(f"Blackboard invalid mutation was accepted: {name}")
		for name, changed in {
			"carrier_tamper": source.replace(">0.01</bbmd_qti_package_maker_num_tolerance>", ">0.02</bbmd_qti_package_maker_num_tolerance>"),
			"carrier_missing": source.replace("<bbmd_qti_package_maker_ma_allow_all_correct>false</bbmd_qti_package_maker_ma_allow_all_correct>", ""),
		}.items():
			with zipfile.ZipFile(archive_path, "w") as archive:
				archive.writestr("res00002.dat", changed)
			if source_carrier_projection(archive_path) == carrier_baseline:
				raise ValueError(f"Blackboard source-carrier mutation was not detected: {name}")
	return {"status": "passed", "mutations": ["choice", "correct", "renderer", "feedback_target", "unresolved_label", "unresolved_response", "duplicate_private", "missing_feedback", "carrier_tamper", "carrier_missing", "text_ownership"]}
