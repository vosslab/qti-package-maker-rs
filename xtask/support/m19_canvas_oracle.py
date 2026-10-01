"""Emit pinned-Python static CanvasSource outcomes for the M19 differential receipt."""

import dataclasses
import hashlib
import json
import pathlib
import sys

from qti_package_maker.html_to_image import selectors
from qti_package_maker.package_interface import QTIPackageInterface

PIN = "55e5f368777f7809fe2e91b5d070caf6df0cb581"


def string_leaves(field: str, value: object) -> object:
	"""Yield each string leaf with its supporting-field path."""
	if isinstance(value, str):
		yield field, value
	elif isinstance(value, (list, tuple)):
		for index, child in enumerate(value):
			yield from string_leaves(f"{field}[{index}]", child)
	elif isinstance(value, dict):
		for key, child in value.items():
			yield from string_leaves(f"{field}[{key!r}]", child)


def html_fields(item: object) -> object:
	"""Yield the exact question-plus-supporting-field traversal used in conversion."""
	yield "question_text", item.question_text
	for name, value in zip(item.get_supporting_field_names(), item.get_tuple()):
		yield from string_leaves(name, value)


def outcome(script: str, width: str | None, height: str | None) -> dict:
	"""Return a Python acceptance result without evaluating the script."""
	try:
		width_attribute = "" if width is None else f" width='{width}'"
		height_attribute = "" if height is None else f" height='{height}'"
		root = selectors.parse_html_fragment(f"<canvas id='canvas_x'{width_attribute}{height_attribute}></canvas><script>{script}</script>")
		canvas, _script, source = selectors.iter_canvas_targets(root)[0]
		return {"accepted": True, "source": dataclasses.asdict(source), "canvas_id": canvas.get("id")}
	except ValueError as error:
		return {"accepted": False, "error": str(error)}


def hostile_cases() -> list[dict]:
	"""Cover every static grammar boundary and the approved receiver spelling."""
	valid = 'let smiles="CCO";let mol=RDKitModule.get_mol(smiles);let mdetails={};mdetails["atoms"]=[0];mdetails["highlightColour"]=[0,1,.5];mol.draw_to_canvas(canvas,JSON.stringify(mdetails));'
	changes = [
		("missing_dimension", valid, None, "80"), ("invalid_dimension", valid, "12x", "80"),
		("zero_dimension", valid, "0", "80"), ("maximum_dimension", valid, "4096", "80"),
		("large_dimension", valid, "4097", "80"),
		("get_mol_count", valid + 'mol.get_mol(smiles);', "120", "80"),
		("draw_count", valid + 'mol.draw_to_canvas(canvas);', "120", "80"),
		("empty_smiles", valid.replace('smiles="CCO"', 'smiles=""'), "120", "80"),
		("dynamic_smiles", valid.replace('smiles="CCO"', 'smiles=value'), "120", "80"),
		("long_smiles", valid.replace('CCO', 'C' * 4097), "120", "80"),
		("details_init", valid.replace('mdetails={}', 'mdetails={"x":1}'), "120", "80"),
		("dynamic_key", valid.replace('mdetails["atoms"]', 'mdetails.atoms'), "120", "80"),
		("duplicate_key", valid.replace('[0];', '[0];mdetails["atoms"]=[1];'), "120", "80"),
		("unsupported_key", valid.replace('mdetails["atoms"]', 'mdetails["bad"]'), "120", "80"),
		("list_bound", valid.replace('[0]', '[-1]'), "120", "80"),
		("bond_literal", valid.replace('[0]', 'getBonds(mol)'), "120", "80"),
		("legend_literal", valid.replace('mdetails["atoms"]=[0];', 'mdetails["legend"]=value;'), "120", "80"),
		("explicit_boolean", valid.replace('mdetails["atoms"]=[0];', 'mdetails["explicitMethyl"]=1;'), "120", "80"),
		("rgb_shape", valid.replace('[0,1,.5]', '[0,1]'), "120", "80"),
		("rgb_negative", valid.replace('[0,1,.5]', '[-.1,0,0]'), "120", "80"),
		("rgb_bound", valid.replace('[0,1,.5]', '[1.1,0,0]'), "120", "80"),
		("rgb_length", valid.replace('[0,1,.5]', '[0,1,.5,0]'), "120", "80"),
		("receiver_agnostic", valid.replace('mol.draw_to_canvas(canvas,JSON.stringify(mdetails))', 'attacker.draw_to_canvas(evil())'), "120", "80"),
	]
	return [{"name": name, "script": script, "width": width, "height": height, "python": outcome(script, width, height)} for name, script, width, height in changes]


def main() -> None:
	"""Emit all real static scripts and the hostile grammar matrix as JSON."""
	pinned_root = pathlib.Path(__file__).resolve().parents[2] / "output_tables" / "oracle_snapshot" / PIN
	selector_path = pathlib.Path(selectors.__file__).resolve()
	if not selector_path.is_relative_to(pinned_root):
		raise ValueError(f"selectors loaded outside pinned oracle: {selector_path}")
	manifest_path = pathlib.Path(sys.argv[1])
	manifest_bytes = manifest_path.read_bytes()
	files = json.loads(manifest_bytes)["bbq_files"]
	records = []
	for filename in map(pathlib.Path, files):
		package = QTIPackageInterface("m19_canvas", allow_mixed=True)
		package.read_package(str(filename), "bbq_text_upload")
		for item in package.item_bank:
			for field, html in html_fields(item):
				if not isinstance(html, str):
					continue
				root = selectors.parse_html_fragment(html)
				for canvas, script, source in selectors.iter_canvas_targets(root):
					records.append({"file": str(filename), "item_crc": item.item_crc16, "field": field,
						"script": script.text_content(), "width": canvas.get("width"), "height": canvas.get("height"),
						"python": {"accepted": True, "source": dataclasses.asdict(source)}})
	json.dump({"pin": PIN, "oracle_snapshot_path": str(pinned_root), "selectors_sha256": hashlib.sha256(selector_path.read_bytes()).hexdigest(), "files": files, "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(), "file_sha256": {str(path): hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest() for path in files}, "real": records, "hostile": hostile_cases()}, sys.stdout, sort_keys=True)


if __name__ == "__main__":
	main()
