"""Emit Python assessment-item construction records for the Rust CRC oracle.

The emitted JSON is intentionally a development-only, typed interchange format.  It preserves
the raw constructor fields that define Python's secondary CRC and separately records Python's
normalized stored fields, so the Rust harness can test both identities and the complete item.
"""

import json
import pathlib
import sys

from qti_package_maker.assessment_items import item_types


def item_fields(item: object) -> dict:
	"""Return the normalized Python fields comparable to Rust's stored Item body."""
	fields = {"question_text": item.question_text}
	for name in (
		"choices_list", "answer_text", "answers_list", "prompts_list",
		"ordered_answers_list", "answer_float", "tolerance_float", "tolerance_message",
		"min_answers_required", "allow_all_correct", "answer_map",
	):
		if hasattr(item, name):
			fields[name] = getattr(item, name)
	return fields


def body_from_parts(parts: list[str]) -> tuple[str, dict, object]:
	"""Build raw Python constructor fields and the matching externally-tagged Rust body."""
	kind = parts[0].strip()
	question = parts[1].strip()
	if kind == "MC":
		choices = parts[2::2]
		answer = choices[[value.lower() for value in parts[3::2]].index("correct")]
		return question, {"MC": {"choices": choices, "answer": answer}}, item_types.MC(question, choices, answer)
	if kind == "MA":
		choices = parts[2::2]
		answers = [choice for choice, state in zip(choices, parts[3::2]) if state.lower() == "correct"]
		return question, {"MA": {"choices": choices, "answers": answers, "min_answers_required": 1, "allow_all_correct": True}}, item_types.MA(question, choices, answers)
	if kind == "MAT":
		prompts, choices = parts[2::2], parts[3::2]
		return question, {"MATCH": {"prompts": prompts, "choices": choices}}, item_types.MATCH(question, prompts, choices)
	if kind == "NUM":
		answer = float(parts[2].strip())
		tolerance = 0.0 if len(parts) <= 3 or not parts[3].strip() else float(parts[3].strip())
		return question, {"NUM": {"answer": answer, "tolerance": tolerance, "tolerance_message": True}}, item_types.NUM(question, answer, tolerance)
	if kind == "FIB":
		answers = parts[2:]
		return question, {"FIB": {"answers": answers}}, item_types.FIB(question, answers)
	if kind == "FIB_PLUS":
		answer_map, index = {}, 2
		while index < len(parts):
			key = parts[index].strip()
			index += 1
			if not key:
				continue
			values = []
			while index < len(parts) and parts[index].strip():
				values.append(parts[index].strip())
				index += 1
			answer_map[key] = values
			index += 1
		return question, {"MULTI_FIB": {"answers": answer_map}}, item_types.MULTI_FIB(question, answer_map)
	if kind == "ORD":
		answers = parts[2:]
		return question, {"ORDER": {"answers": answers}}, item_types.ORDER(question, answers)
	raise ValueError(f"Unsupported question type: {kind!r}")


def record(path: pathlib.Path | str, line_number: int, line: str) -> dict:
	"""Construct one item through Python and return all parity-relevant information."""
	parts = line.strip().split("\t")
	question, body, item = body_from_parts(parts)
	return {
		"path": str(path), "line": line_number, "input": {"question": question, "body": body},
		"python": {
			"item_type": item.item_type, "item_crc16": item.item_crc16,
			"question_crc16": item.question_crc16, "secondary_crc16": item.secondary_crc16,
			"fields": item_fields(item),
		},
	}


def synthetic_lines() -> list[str]:
	"""Guarantee all seven raw secondary-string forms, including prefix and repr edge cases."""
	return [
		"MC\tPrefix choice question?\tA. first\tCorrect\tB. second\tIncorrect",
		"MA\tPrefix answers question?\tA. one\tCorrect\tB. two\tIncorrect\tC. three\tCorrect",
		"MAT\tMatch these terms?\tA. prompt one\t1. choice one\tB. prompt two\t2. choice two",
		"NUM\tNumerical exponent question?\t0.0125\t0.00025",
		"FIB\tFill this blank?\tA. alpha\tB. beta",
		"FIB_PLUS\tFill [a] and [z]?\tz\tZed's\\path\t\ta\tfirst\tsecond\t",
		"ORD\tOrder these three?\tA. first\tB. second\tC. third",
	]


def main() -> None:
	"""Read files, retain every parse failure, and print one deterministic JSON document."""
	output = {"records": [], "errors": [], "synthetic_records": 0}
	for filename in map(pathlib.Path, sys.argv[1:]):
		try:
			with filename.open(encoding="ascii") as handle:
				for line_number, line in enumerate(handle, start=1):
					if not line.strip():
						continue
					try:
						output["records"].append(record(filename, line_number, line))
					except Exception as error:
						output["errors"].append({"path": str(filename), "line": line_number, "error": str(error)})
		except Exception as error:
			output["errors"].append({"path": str(filename), "line": 0, "error": str(error)})
	for line_number, line in enumerate(synthetic_lines(), start=1):
		output["records"].append(record("<synthetic>", line_number, line))
		output["synthetic_records"] += 1
	json.dump(output, sys.stdout, ensure_ascii=True, sort_keys=True)


if __name__ == "__main__":
	main()
