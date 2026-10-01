"""Extract selector-backed table and RDKit canvas records from generated BBQ files.

This development harness is deliberately outside shipping Rust crates.  It uses the
current Python reader and selectors as the oracle for the corpus that later Rust
work must reproduce.
"""

import dataclasses
import json
import pathlib
import sys

from qti_package_maker.html_to_image import selectors
from qti_package_maker.package_interface import QTIPackageInterface


def html_fields(item: object) -> object:
	"""Yield every string field that may contain generated HTML."""
	yield "question_text", item.question_text
	for field_name in ("choices_list", "prompts_list", "answers_list", "ordered_answers_list"):
		for index, value in enumerate(getattr(item, field_name, ())):
			yield f"{field_name}[{index}]", value
	for value in getattr(item, "answer_map", {}).values():
		yield "answer_map.value", value


def records_for_file(filename: pathlib.Path) -> list[dict]:
	"""Return all outermost tables and supported RDKit canvases from one BBQ file."""
	package = QTIPackageInterface("corpus", allow_mixed=True)
	package.read_package(str(filename), "bbq_text_upload")
	records = []
	for item_index, item in enumerate(package.item_bank, start=1):
		for field_name, html in html_fields(item):
			if not isinstance(html, str):
				continue
			for field_index, fragment in enumerate(selectors.find_table_fragments(html), start=1):
				records.append({
					"family": "table", "item_index": item_index, "item_type": item.item_type,
					"item_crc16": item.item_crc16, "field": field_name,
					"field_index": field_index, "fragment": fragment,
				})
			for field_index, canvas in enumerate(selectors.find_canvas_fragments(html), start=1):
				records.append({
					"family": "canvas", "item_index": item_index, "item_type": item.item_type,
					"item_crc16": item.item_crc16, "field": field_name,
					"field_index": field_index, "fragment": dataclasses.asdict(canvas),
				})
	return records


def main() -> None:
	"""Print JSON, retaining a per-file error as corpus evidence."""
	output = {"files": []}
	for filename in map(pathlib.Path, sys.argv[1:]):
		try:
			output["files"].append({"path": str(filename), "records": records_for_file(filename)})
		except Exception as error:
			output["files"].append({"path": str(filename), "error": str(error), "records": []})
	json.dump(output, sys.stdout, ensure_ascii=True, sort_keys=True)


if __name__ == "__main__":
	main()
