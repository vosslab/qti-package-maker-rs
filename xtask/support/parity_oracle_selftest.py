"""Mutation checks for the parity comparator entry point."""

import pathlib
import tempfile
import time
import zipfile

def score_program_selftest() -> dict[str, object]:
	"""Exercise the bounded grading-program certificate without score enumeration."""
	from xtask.support.parity_oracle import selftest_projection, text_path, xml_projection
	from xtask.support import parity_qti21_multifib_repair
	repair_receipt = parity_qti21_multifib_repair.selftest()
	if repair_receipt["status"] != "passed":
		raise ValueError("QTI2 MULTI_FIB bounded repair selftest did not pass")
	labels = []
	predicates = []
	for response_index in range(1, 6):
		response_labels = []
		for choice_index in range(1, 6):
			response_labels.append(
				"<response_label ident='response_"
				+ str(response_index)
				+ "_choice_"
				+ f"{choice_index:03d}"
				+ "'><mattext>choice "
				+ str(choice_index)
				+ "</mattext></response_label>"
			)
		labels.append(
			"<response_lid rcardinality='Single' ident='response_"
			+ str(response_index)
			+ "'><render_choice>"
			+ "".join(response_labels)
			+ "</render_choice></response_lid>"
		)
		predicates.append(
			"<varequal respident='response_"
			+ str(response_index)
			+ "'>response_"
			+ str(response_index)
			+ "_choice_001</varequal>"
		)
	condition = "<and>" + "".join(predicates) + "</and>"
	source = (
		"<questestinterop><item ident='score-program-proof'><presentation>"
		"<material><mattext>five by five match</mattext></material>"
		+ "".join(labels)
		+ "</presentation><resprocessing><outcomes><decvar varname='SCORE' vartype='Decimal' minvalue='0' maxvalue='100'/></outcomes><respcondition><conditionvar>"
		+ condition
		+ "</conditionvar><setvar varname='SCORE' action='Set'>100.00</setvar>"
		"<setvar varname='SCORE' action='Add'>1</setvar>"
		"</respcondition></resprocessing></item></questestinterop>"
	)

	def certificate(xml_text: str, directory: pathlib.Path, name: str) -> object:
		archive_path = directory / f"{name}.zip"
		with zipfile.ZipFile(archive_path, "w") as archive:
			archive.writestr("item.xml", xml_text)
		projection = xml_projection(archive_path)
		item = projection[0]
		return {
			"responses": item["responses"],
			"score_declaration": item["score_declaration"],
			"score_program": item["score_program"],
		}

	with tempfile.TemporaryDirectory(prefix="qti-score-program-") as temporary:
		directory = pathlib.Path(temporary)
		started = time.monotonic()
		baseline = certificate(source, directory, "baseline")
		elapsed_ms = round((time.monotonic() - started) * 1000, 3)
		mutations = {
			"continue": source.replace("<respcondition>", "<respcondition continue='No'>"),
			"action_order": source.replace(
				"<setvar varname='SCORE' action='Set'>100.00</setvar>"
				"<setvar varname='SCORE' action='Add'>1</setvar>",
				"<setvar varname='SCORE' action='Add'>1</setvar>"
				"<setvar varname='SCORE' action='Set'>100.00</setvar>",
			),
			"action_value": source.replace("action='Add'>1</setvar>", "action='Add'>2</setvar>"),
			"predicate": source.replace(
				"response_1_choice_001</varequal>", "response_1_choice_002</varequal>", 1
			),
			"response_cardinality": source.replace("rcardinality='Single'", "rcardinality='Multiple'", 1),
			"outcome_maxvalue": source.replace("maxvalue='100'", "maxvalue='200'"),
			"dereferenced_label": source.replace("<mattext>choice 1</mattext>", "<mattext>other choice</mattext>", 1),
		}
		for name, mutated_source in mutations.items():
			if certificate(mutated_source, directory, name) == baseline:
				raise ValueError(f"grading-program mutation was not detected: {name}")
		match_source = (
			"<table><tr><td></td><td><button class='qti-match-slot' data-correct='token_t'></button></td><td>1. A</td></tr>"
			"<tr><td></td><td><button class='qti-match-slot' data-correct='token_g'></button></td><td>2. C</td></tr></table>"
			"<button class='qti-match-choice' data-value='token_g' aria-label='Select A. G'>A. G</button>"
			"<button class='qti-match-choice' data-value='token_t' aria-label='Select B. T'>B. T</button>"
		)
		match_expected = {"match": [{"prompt": "A", "answer": "T"}, {"prompt": "C", "answer": "G"}]}
		if selftest_projection(text_path(match_source)) != match_expected:
			raise ValueError("self-test MATCH generated-label dereference changed authored pairs")
		for name, mutated_source in {
			"html_match_wrong_pair": match_source.replace("data-correct='token_t'", "data-correct='token_g'", 1),
			"html_match_unknown_token": match_source.replace("token_t'", "token_missing'", 1),
			"html_match_visible_answer": match_source.replace(">A. G</button>", ">A. X</button>", 1),
		}.items():
			try:
				changed = selftest_projection(text_path(mutated_source))
			except ValueError:
				continue
			if changed == match_expected:
				raise ValueError(f"self-test MATCH mutation was not detected: {name}")
		from xtask.support.parity_selftest_selection import control_selftest, verify_accessible_choices
		control_selftest()
		from xtask.support.parity_selftest_structure import selftest as structure_selftest
		structure_selftest()
		from xtask.support.parity_blackboard_dom import selftest as blackboard_dom_selftest
		blackboard_dom_selftest()
		from xtask.support.parity_qti21 import selftest as qti21_selftest
		qti21_selftest()
		from xtask.support.parity_choice_label_repair import selftest as choice_label_selftest
		choice_label_selftest()
		from xtask.support.parity_human_tables import selftest as human_table_selftest
		human_table_selftest()
		accessible_expected = directory / "match-accessible-expected.html"
		accessible_actual = directory / "match-accessible-mutated.html"
		accessible_expected.write_text(match_source)
		accessible_actual.write_text(match_source.replace("Select A. G", "Select unknown answer", 1))
		try:
			verify_accessible_choices(accessible_actual, accessible_expected)
		except ValueError:
			pass
		else:
			raise ValueError("self-test MATCH accessible-name mutation was not detected")
	result = {
		"status": "passed",
		"responses": 5,
		"labels_per_response": 5,
		"elapsed_ms": elapsed_ms,
		"mutations": sorted([*mutations, "html_match_wrong_pair", "html_match_unknown_token", "html_match_visible_answer", "html_match_accessible_name"]),
	}
	return result

