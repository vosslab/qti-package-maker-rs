"""Record a narrowly identified, source-bound frozen MATCH reader defect."""

import hashlib
import json
import pathlib
import tempfile


def compare(python_path: pathlib.Path, rust_path: pathlib.Path, readback: object, writer: object) -> tuple:
	from lxml.etree import XMLSyntaxError
	from qti_package_maker import package_interface
	from qti_package_maker.engines.okla_chrst_bqgen import read_package

	try:
		return readback(python_path, "okla_chrst_bqgen"), readback(rust_path, "okla_chrst_bqgen")
	except XMLSyntaxError:
		binding_path = python_path.parent / "source_input_receipt.json"
		if not binding_path.is_file() or python_path.read_bytes() != rust_path.read_bytes():
			raise
		binding = json.loads(binding_path.read_text())
		source = pathlib.Path(binding["path"])
		if hashlib.sha256(source.read_bytes()).hexdigest() != binding["sha256"]:
			raise ValueError("Okla source binding changed")
		bad_closer = False
		for block in read_package._split_blocks(python_path.read_text()):
			lines = block.splitlines()
			if not lines[0].lower().startswith("match"):
				continue
			for line in lines[1:]:
				parsed = read_package._parse_choice_line(line)
				if parsed and "/" in parsed[1]:
					text = parsed[1]
					slash = text.index("/")
					if slash > 0 and text[slash - 1] == "<":
						bad_closer = True
		if not bad_closer:
			raise ValueError("Okla rejection is outside the known MATCH closing-tag defect")
		with tempfile.TemporaryDirectory(dir=python_path.parent) as temporary:
			regenerated = pathlib.Path(temporary) / "okla_chrst_bqgen.txt"
			writer(package_interface, source, regenerated, "okla_chrst_bqgen")
			if regenerated.read_bytes() != python_path.read_bytes():
				raise ValueError("Okla rejected output does not reproduce from its bound source")
		try:
			readback(rust_path, "okla_chrst_bqgen")
		except XMLSyntaxError:
			pass
		else:
			raise ValueError("Okla rejection does not occur on both identical outputs")
		value = {"outcome": "frozen_source_equivalent_rejection", "defect": "MATCH reader splits HTML closing tag at first slash", "output_sha256": hashlib.sha256(python_path.read_bytes()).hexdigest(), "source": binding}
		(python_path.parent / "okla_source_defect_receipt.json").write_text(json.dumps(value, indent=2) + "\n")
		return value, value
