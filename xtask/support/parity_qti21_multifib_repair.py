"""Bounded repair for frozen QTI 2.1 MULTI_FIB source grammar defects."""

from __future__ import annotations

import hashlib
import pathlib
import tempfile
import zipfile
from xml.etree import ElementTree as xml_tree

from defusedxml import ElementTree as element_tree

from xtask.support import parity_qti21


QTI21 = "http://www.imsglobal.org/xsd/imsqti_v2p1"


def local(node: xml_tree.Element) -> str:
	return node.tag.rsplit("}", 1)[-1]


def qti(name: str) -> str:
	return f"{{{QTI21}}}{name}"


def qti_direct(node: xml_tree.Element, name: str) -> list[xml_tree.Element]:
	return [child for child in node if child.tag == qti(name)]


def one(nodes: list[xml_tree.Element], description: str) -> xml_tree.Element:
	if len(nodes) != 1:
		raise ValueError(f"frozen QTI2 MULTI_FIB repair requires one {description}, found {len(nodes)}")
	return nodes[0]


def leaf_text(node: xml_tree.Element, description: str) -> str:
	if list(node):
		raise ValueError(f"frozen QTI2 MULTI_FIB {description} must not contain child markup")
	return node.text or ""


def finite_decimal(value: str, description: str) -> str:
	try:
		parsed = float(value)
	except ValueError as error:
		raise ValueError(f"frozen QTI2 MULTI_FIB {description} is not numeric") from error
	if not parsed == parsed or parsed in {float("inf"), float("-inf")}:
		raise ValueError(f"frozen QTI2 MULTI_FIB {description} is non-finite")
	return value


def repair_item(root: xml_tree.Element, raw_xml: bytes) -> dict[str, object] | None:
	"""Repair exactly the archived lower-case interaction and QTI 1.2 processing form."""
	if root.tag != qti("assessmentItem"):
		return None
	allowed_root_children = {
		qti("responseDeclaration"), qti("outcomeDeclaration"), qti("itemBody"), qti("responseProcessing"),
	}
	if any(child.tag not in allowed_root_children for child in root):
		raise ValueError("frozen QTI2 MULTI_FIB item has an unknown root child")
	item_body = one(qti_direct(root, "itemBody"), "itemBody")
	for node in item_body.iter():
		if local(node) == "textentryinteraction" and node.tag != qti("textentryinteraction"):
			raise ValueError("frozen QTI2 MULTI_FIB interaction must use the QTI namespace")
	interactions = [node for node in item_body.iter() if node.tag == qti("textentryinteraction")]
	if not interactions:
		return None
	if any(set(node.attrib) != {"responseidentifier"} for node in interactions):
		raise ValueError("frozen QTI2 MULTI_FIB interaction attributes differ from the bounded source defect")
	declarations = qti_direct(root, "responseDeclaration")
	declaration_values: dict[str, list[str]] = {}
	for declaration in declarations:
		identifier = declaration.attrib.get("identifier")
		if set(declaration.attrib) != {"identifier", "baseType", "cardinality"} or not identifier or declaration.attrib.get("baseType") != "string" or declaration.attrib.get("cardinality") != "single":
			raise ValueError("frozen QTI2 MULTI_FIB declaration shape differs from the bounded source defect")
		correct = one(qti_direct(declaration, "correctResponse"), "correct response")
		if list(declaration) != [correct] or correct.attrib:
			raise ValueError("frozen QTI2 MULTI_FIB declaration has unknown children or attributes")
		value_nodes = qti_direct(correct, "value")
		if not value_nodes or list(correct) != value_nodes or any(node.attrib for node in value_nodes):
			raise ValueError("frozen QTI2 MULTI_FIB correct response has unknown children or attributes")
		values = [leaf_text(node, "correct response value") for node in value_nodes]
		if not all(values) or len(values) != len(set(values)) or identifier in declaration_values:
			raise ValueError("frozen QTI2 MULTI_FIB declarations are not unique authored blanks and alternatives")
		declaration_values[identifier] = values
	interaction_ids = [node.attrib["responseidentifier"] for node in interactions]
	if len(interaction_ids) != len(set(interaction_ids)) or set(interaction_ids) != set(declaration_values):
		raise ValueError("frozen QTI2 MULTI_FIB interaction and declaration identifiers differ")
	outcome = one(qti_direct(root, "outcomeDeclaration"), "SCORE outcome declaration")
	if outcome.attrib != {"identifier": "SCORE", "baseType": "float", "cardinality": "single"} or list(outcome):
		raise ValueError("frozen QTI2 MULTI_FIB SCORE outcome differs from the bounded source defect")
	processing = one(qti_direct(root, "responseProcessing"), "responseProcessing")
	if processing.attrib:
		raise ValueError("frozen QTI2 MULTI_FIB responseProcessing attributes differ")
	conditions = qti_direct(processing, "respcondition")
	if len(conditions) != len(interaction_ids) or list(processing) != conditions:
		raise ValueError("frozen QTI2 MULTI_FIB processing differs from the bounded source defect")
	contributions = []
	for condition in conditions:
		if set(condition.attrib) or len(condition) != 2:
			raise ValueError("frozen QTI2 MULTI_FIB legacy condition shape differs")
		conditionvar, setvar = condition
		if [child.tag for child in condition] != [qti("conditionvar"), qti("setvar")]:
			raise ValueError("frozen QTI2 MULTI_FIB legacy processing nodes differ")
		match = one(qti_direct(conditionvar, "match"), "legacy match")
		if conditionvar.attrib or [child.tag for child in conditionvar] != [qti("match")] or match.attrib:
			raise ValueError("frozen QTI2 MULTI_FIB legacy match has unknown attributes or children")
		variable = one(qti_direct(match, "variable"), "legacy match variable")
		correct = one(qti_direct(match, "correct"), "legacy match correct")
		if [child.tag for child in match] != [qti("variable"), qti("correct")] or set(variable.attrib) != {"identifier"} or set(correct.attrib) != {"identifier"}:
			raise ValueError("frozen QTI2 MULTI_FIB legacy match operands differ")
		if leaf_text(variable, "legacy match variable") or leaf_text(correct, "legacy match correct"):
			raise ValueError("frozen QTI2 MULTI_FIB legacy match operands must be empty leaves")
		identifier = variable.attrib.get("identifier")
		if identifier not in declaration_values or correct.attrib.get("identifier") != identifier:
			raise ValueError("frozen QTI2 MULTI_FIB legacy match does not target an authored blank")
		if list(setvar) or set(setvar.attrib) != {"varname", "action"} or setvar.attrib != {"varname": "SCORE", "action": "Add"}:
			raise ValueError("frozen QTI2 MULTI_FIB score contribution differs from the bounded source defect")
		contributions.append({"identifier": identifier, "score": finite_decimal(leaf_text(setvar, "score contribution"), "score contribution")})
	if [entry["identifier"] for entry in contributions] != interaction_ids:
		raise ValueError("frozen QTI2 MULTI_FIB contribution order differs from authored blanks")
	for interaction, identifier in zip(interactions, interaction_ids, strict=True):
		interaction.tag = qti("textEntryInteraction")
		interaction.attrib = {"responseIdentifier": identifier}
	new_processing = xml_tree.Element(qti("responseProcessing"))
	for contribution in contributions:
		condition = xml_tree.SubElement(new_processing, qti("responseCondition"))
		response_if = xml_tree.SubElement(condition, qti("responseIf"))
		identifier = contribution["identifier"]
		values = declaration_values[identifier]
		if len(values) == 1:
			match = xml_tree.SubElement(response_if, qti("match"))
			xml_tree.SubElement(match, qti("variable"), {"identifier": identifier})
			xml_tree.SubElement(match, qti("correct"), {"identifier": identifier})
		else:
			alternatives = xml_tree.SubElement(response_if, qti("or"))
			for value in values:
				match = xml_tree.SubElement(alternatives, qti("match"))
				xml_tree.SubElement(match, qti("variable"), {"identifier": identifier})
				xml_tree.SubElement(match, qti("baseValue"), {"baseType": "string"}).text = value
		set_outcome = xml_tree.SubElement(response_if, qti("setOutcomeValue"), {"identifier": "SCORE"})
		sum_node = xml_tree.SubElement(set_outcome, qti("sum"))
		xml_tree.SubElement(sum_node, qti("variable"), {"identifier": "SCORE"})
		base_value = xml_tree.SubElement(sum_node, qti("baseValue"), {"baseType": "float"})
		base_value.text = contribution["score"]
	for declaration in declarations:
		correct = one(qti_direct(declaration, "correctResponse"), "correct response")
		for extra in list(correct)[1:]:
			correct.remove(extra)
	root.remove(processing)
	root.append(new_processing)
	return {
		"repair": "frozen_qti21_multifib_legacy_grammar",
		"raw_item_sha256": hashlib.sha256(raw_xml).hexdigest(),
		"authored_blanks": [{"identifier": identifier, "answer": declaration_values[identifier]} for identifier in interaction_ids],
		"contributions": contributions,
		"source_forms": {
			"lowercase_textentryinteraction": len(interactions),
			"lowercase_responseidentifier": len(interactions),
			"legacy_respcondition": len(conditions),
		},
	}


def frozen_projection(path: pathlib.Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
	"""Require strict rejection, repair only the exact source grammar, then project it."""
	try:
		return parity_qti21.xml_projection(path), []
	except ValueError as strict_error:
		strict_message = str(strict_error)
	if "responseIdentifier" not in strict_message:
		raise ValueError(f"QTI2 strict projection failed outside the bounded MULTI_FIB repair: {strict_message}")
	with tempfile.TemporaryDirectory(prefix="qti21-multifib-repair-") as temporary:
		repaired_path = pathlib.Path(temporary) / "repaired.zip"
		repairs = []
		with zipfile.ZipFile(path) as source, zipfile.ZipFile(repaired_path, "w") as repaired:
			for name in source.namelist():
				payload = source.read(name)
				if name.endswith(".xml"):
					root = element_tree.fromstring(payload)
					repair = repair_item(root, payload)
					if repair is not None:
						repairs.append(repair)
						payload = xml_tree.tostring(root, encoding="utf-8", xml_declaration=True)
				repaired.writestr(name, payload)
		if not repairs:
			raise ValueError("QTI2 strict projection failed but no bounded MULTI_FIB source grammar was repaired")
		return parity_qti21.xml_projection(repaired_path), repairs


def selftest() -> dict[str, object]:
	"""Prove the repair accepts only its exact frozen source grammar."""
	source = f'''<assessmentItem xmlns="{QTI21}">
<responseDeclaration identifier="one" baseType="string" cardinality="single"><correctResponse><value>one</value></correctResponse></responseDeclaration>
<responseDeclaration identifier="two" baseType="string" cardinality="single"><correctResponse><value>two</value></correctResponse></responseDeclaration>
<outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single"/>
<itemBody><div><textentryinteraction responseidentifier="one"/> <textentryinteraction responseidentifier="two"/></div></itemBody>
<responseProcessing><respcondition><conditionvar><match><variable identifier="one"/><correct identifier="one"/></match></conditionvar><setvar varname="SCORE" action="Add">50.00</setvar></respcondition><respcondition><conditionvar><match><variable identifier="two"/><correct identifier="two"/></match></conditionvar><setvar varname="SCORE" action="Add">50.00</setvar></respcondition></responseProcessing>
</assessmentItem>'''.encode()
	with tempfile.TemporaryDirectory(prefix="qti21-multifib-repair-selftest-") as temporary:
		directory = pathlib.Path(temporary)
		archive_path = directory / "source.zip"
		with zipfile.ZipFile(archive_path, "w") as archive:
			archive.writestr("item.xml", source)
		projection, repairs = frozen_projection(archive_path)
		if len(projection) != 1 or len(repairs) != 1:
			raise ValueError("QTI2 MULTI_FIB bounded repair did not produce one item and one receipt")
		if repairs[0]["authored_blanks"] != [{"identifier": "one", "answer": ["one"]}, {"identifier": "two", "answer": ["two"]}]:
			raise ValueError("QTI2 MULTI_FIB bounded repair changed authored blank order")
		mutations = {
			"interaction_case": source.replace(b"textentryinteraction", b"textEntryInteraction", 1),
			"response_attribute": source.replace(b"responseidentifier", b"responseIdentifier", 1),
			"score_action": source.replace(b'action="Add"', b'action="Set"', 1),
			"declared_answer": source.replace(b'<correct identifier="two"/>', b'<correct identifier="one"/>', 1),
			"extra_interaction_attribute": source.replace(b'responseidentifier="one"', b'responseidentifier="one" class="extra"', 1),
			"foreign_match": source.replace(b"<match>", b'<x:match xmlns:x="urn:evil">', 1).replace(b"</match>", b"</x:match>", 1),
			"foreign_setvar": source.replace(b"<setvar ", b'<x:setvar xmlns:x="urn:evil" ', 1).replace(b"</setvar>", b"</x:setvar>", 1),
			"extra_match_child": source.replace(b"<correct identifier=\"one\"/>", b'<correct identifier="one"/><extra/>', 1),
			"extra_conditionvar_child": source.replace(b"</conditionvar>", b"<extra/></conditionvar>", 1),
		}
		for name, mutated in mutations.items():
			with zipfile.ZipFile(archive_path, "w") as archive:
				archive.writestr("item.xml", mutated)
			try:
				frozen_projection(archive_path)
			except ValueError:
				continue
			raise ValueError(f"QTI2 MULTI_FIB bounded repair accepted mutation: {name}")
	return {"status": "passed", "mutations": sorted(mutations)}
