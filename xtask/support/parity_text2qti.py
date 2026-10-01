"""Bounded readback repair for the pinned writer's missing block separators."""

import hashlib
import json
import pathlib
import tempfile


def sha256(payload: bytes) -> str:
	return hashlib.sha256(payload).hexdigest()


def frozen_readback(path: pathlib.Path, readback: object) -> object:
	"""Insert only separators between exact, source-derived frozen writer blocks."""
	from qti_package_maker import package_interface
	from qti_package_maker.engines.text2qti.engine_class import EngineClass

	binding_path = path.parent / "source_input_receipt.json"
	if not binding_path.is_file():
		return readback(path, "text2qti")
	binding = json.loads(binding_path.read_text())
	source = pathlib.Path(binding["path"])
	if sha256(source.read_bytes()) != binding["sha256"]:
		raise ValueError("text2qti repair source changed after frozen export")
	interface = package_interface.QTIPackageInterface("parity", verbose=False, allow_mixed=True)
	interface.read_package(str(source), "bbq_text_upload")
	engine = EngineClass("parity", verbose=False)
	blocks = engine._render_items_with_media(interface.item_bank, str(path))
	raw = path.read_bytes()
	if "".join(blocks).encode("utf-8") != raw:
		raise ValueError("text2qti separator repair is not bound to exact frozen writer output")
	if len(blocks) <= 1:
		return readback(path, "text2qti")
	repaired = "\n".join(blocks).encode("utf-8")
	# Keep the frozen file and its adjacent media intact; the temporary reader file
	# shares that media base and is removed even when readback rejects the grammar.
	with tempfile.NamedTemporaryFile(dir=path.parent, suffix=".txt") as temporary:
		temporary.write(repaired)
		temporary.flush()
		result = readback(pathlib.Path(temporary.name), "text2qti")
	if len(result) != len(blocks):
		raise ValueError("text2qti repaired readback did not preserve every source-derived block")
	(path.parent / "text2qti_readback_repair.json").write_text(json.dumps({
		"contract": "pinned_text2qti_missing_separators",
		"source": binding,
		"raw_sha256": sha256(raw),
		"repaired_sha256": sha256(repaired),
		"block_sha256": [sha256(block.encode("utf-8")) for block in blocks],
		"readback_items": len(result),
	}, indent=2) + "\n")
	return result
