#!/usr/bin/env python3
"""Current-Python migration half of the M13 differential parity harness.

It is deliberately development tooling.  The Rust binary never imports this
module, and this script only imports the run-scoped current source selected by its caller.
"""

import argparse
import contextlib
import json
import pathlib
import sys
from xtask.support.parity_oracle_writer import (
	READBACK_ENGINES as READBACK_ENGINES,
	ZIP_ENGINES as ZIP_ENGINES,
	extension as extension,
	selected_engines as selected_engines,
	write_outputs as write_outputs,
	write_via_registered_adapter as write_via_registered_adapter,
	order_receipt as order_receipt,
	readback_fingerprint as readback_fingerprint,
	readback_outcome as readback_outcome,
)
from xtask.support.parity_oracle_qti12 import xml_projection as xml_projection
from xtask.support.parity_oracle_projection import (
	yaml_projection as yaml_projection,
	aiken_projection as aiken_projection,
	selftest_projection as selftest_projection,
	normalized_html as normalized_html,
	text_path as text_path,
)

__all__ = ["text_path"]



def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser(description="Current Python M13 migration parity support.")
	parser.add_argument("mode", choices=("write", "compare", "order-receipt", "readback", "selftest"))
	parser.add_argument("--oracle-root", type=pathlib.Path)
	parser.add_argument("--input", type=pathlib.Path)
	parser.add_argument("--output", type=pathlib.Path)
	parser.add_argument("--python-output", type=pathlib.Path)
	parser.add_argument("--rust-output", type=pathlib.Path)
	parser.add_argument("--html-to-image", action="store_true")
	parser.add_argument("--zip-only", action="store_true")
	parser.add_argument("--engines", help="comma-separated fixed engine subset")
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


def compare_one(engine: str, python_path: pathlib.Path, rust_path: pathlib.Path, html_to_image: bool) -> tuple[object, object]:
	if html_to_image and engine == "blackboard_export_zip":
		return html_to_image_structure_from_zip(python_path), html_to_image_structure_from_zip(rust_path)
	if engine == "blackboard_export_zip":
		from xtask.support import parity_blackboard
		return (
			{"semantic": parity_blackboard.xml_projection(python_path), "readback": readback_outcome(python_path, engine)},
			{"semantic": parity_blackboard.xml_projection(rust_path), "readback": readback_outcome(rust_path, engine)},
		)
	if engine == "text2qti":
		return readback_fingerprint(python_path, engine), readback_fingerprint(rust_path, engine)
	if engine == "okla_chrst_bqgen":
		from xtask.support import parity_okla
		return parity_okla.compare(python_path, rust_path, readback_fingerprint, write_via_registered_adapter)
	if engine in READBACK_ENGINES:
		return readback_fingerprint(python_path, engine), readback_fingerprint(rust_path, engine)
	if engine == "blackboard_qti_v2_1":
		from xtask.support import parity_qti21
		python_value = parity_qti21.xml_projection(python_path)
		rust_value = parity_qti21.xml_projection(rust_path)
		if html_to_image:
			for items in (python_value, rust_value):
				for item in items:
					item["scripts"] = [script for script in item["scripts"] if not (
						script["attributes"] == [("src", "https://unpkg.com/@rdkit/rdkit/dist/RDKit_minimal.js")]
						and not script["text"]
					)]
		return python_value, rust_value
	if engine == "canvas_qti_v1_2":
		if html_to_image:
			return xml_projection(python_path, True), xml_projection(rust_path, True)
		return xml_projection(python_path), xml_projection(rust_path)
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
			# Python readers can print malformed embedded-markup diagnostics while
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
				"engine": engine, "item": "package",
				"field": f"comparator error: {type(error).__name__}: {error}",
				"python": "not compared", "rust": "not compared",
			})
	print(json.dumps({"divergences": divergences}, sort_keys=True))


def readback_outputs(args: argparse.Namespace) -> None:
	"""Emit the current reader's complete normalized item fields for one package."""
	engines = selected_engines(args)
	if len(engines) != 1:
		raise ValueError("readback requires exactly one engine")
	print(json.dumps(readback_fingerprint(args.input, engines[0]), sort_keys=True))


def html_to_image_structure_from_zip(path: pathlib.Path) -> object:
	from xtask.support import parity_images
	return parity_images.contract(path)


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
