"""Frozen writer and readback operations for the parity oracle."""

import argparse
import contextlib
import hashlib
import json
import os
import pathlib
import re
# The development oracle invokes a fixed archived Python executable.
import subprocess  # nosec B404
import sys
from datetime import date, datetime

ENGINES = (
	"bbq_text_upload",
	"text2qti",
	"okla_chrst_bqgen",
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
	"exam_yaml",
	"moodle_aiken",
	"human_readable",
	"html_selftest",
)
READBACK_ENGINES = {
	"bbq_text_upload",
	"text2qti",
	"okla_chrst_bqgen",
	"blackboard_export_zip",
}
ZIP_ENGINES = {"blackboard_export_zip", "canvas_qti_v1_2", "blackboard_qti_v2_1"}
ZIP_ENGINE_SEQUENCE = (
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
)
CLI_ENGINES = {
	"bbq_text_upload",
	"blackboard_export_zip",
	"canvas_qti_v1_2",
	"blackboard_qti_v2_1",
	"moodle_aiken",
	"human_readable",
	"html_selftest",
}
REGISTERED_ONLY_ENGINES = {"text2qti", "okla_chrst_bqgen", "exam_yaml"}



def extension(engine: str) -> str:
	if engine in ZIP_ENGINES:
		return "zip"
	if engine == "exam_yaml":
		return "yaml"
	if engine in {"human_readable", "html_selftest"}:
		return "html"
	return "txt"


def selected_engines(args: argparse.Namespace) -> tuple[str, ...]:
	if args.engines:
		engines = tuple(args.engines.split(","))
		unknown = set(engines).difference(ENGINES)
		if unknown:
			raise ValueError(f"unknown parity engine(s): {', '.join(sorted(unknown))}")
		return engines
	if args.zip_only:
		return ZIP_ENGINE_SEQUENCE
	return ENGINES


def write_outputs(args: argparse.Namespace) -> None:
	from qti_package_maker import package_interface

	args.output.mkdir(parents=True, exist_ok=True)
	(args.output / "source_input_receipt.json").write_text(json.dumps({"path": str(args.input.resolve()), "sha256": hashlib.sha256(args.input.read_bytes()).hexdigest()}, sort_keys=True) + "\n")
	result: dict[str, object] = {"outputs": {}, "errors": {}, "no_output": {}}
	for engine in selected_engines(args):
		try:
			destination = args.output / f"{engine}.{extension(engine)}"
			# Frozen engines print recoverable media warnings directly.  Preserve them on
			# stderr so this development protocol has exactly one JSON stdout document.
			with contextlib.redirect_stdout(sys.stderr):
				if engine in CLI_ENGINES:
					write_via_cli(args.oracle_root, args.input, destination, engine, args.html_to_image)
				else:
					write_via_registered_adapter(
						package_interface, args.input, destination, engine
					)
			if not destination.is_file():
				result["no_output"][engine] = "writer completed successfully without an output artifact"
			else:
				result["outputs"][engine] = str(destination)
		except Exception as error:  # The error itself is observable parity data.
			result["errors"][engine] = f"{type(error).__name__}: {error}"
	print(json.dumps(result, sort_keys=True))


def write_via_cli(
	oracle_root: pathlib.Path,
	input_path: pathlib.Path,
	destination: pathlib.Path,
	engine: str,
	html_to_image: bool,
) -> None:
	"""Run frozen `bbq_converter.py` for the seven formats it actually exposes."""
	command = [
		sys.executable,
		str(oracle_root / "tools" / "bbq_converter.py"),
		"-i", str(input_path), "-f", engine, "-o", str(destination), "-q",
		"--allow-mixed",
	]
	if html_to_image and engine in ZIP_ENGINES:
		command.append("--html-to-image")
	environment = os.environ.copy()
	environment["PYTHONPATH"] = str(oracle_root) + os.pathsep + environment.get("PYTHONPATH", "")
	# The executable and arguments are fixed; this call never invokes a shell.
	result = subprocess.run(  # nosec B603
		command, cwd=destination.parent, env=environment, capture_output=True,
		text=True, check=False,
	)
	if result.returncode != 0:
		raise RuntimeError(
			f"frozen CLI exit {result.returncode}: {result.stderr.strip() or result.stdout.strip()}"
		)


def write_via_registered_adapter(
	package_interface: object,
	input_path: pathlib.Path,
	destination: pathlib.Path,
	engine: str,
) -> None:
	"""Write only the three frozen registered engines absent from the CLI parser."""
	if engine not in REGISTERED_ONLY_ENGINES:
		raise ValueError(f"not a registered-only engine: {engine}")
	# Mirror the frozen CLI's extract_core_name() exactly.  Registered-only
	# engines bypass its argparse table, but must receive the same package name.
	match = re.fullmatch(r"bbq-(.+?)-questions\.txt", input_path.name)
	if match is None:
		raise ValueError(f"Filename '{input_path}' does not match expected pattern.")
	interface = package_interface.QTIPackageInterface(
		match.group(1), verbose=False, allow_mixed=True
	)
	interface.read_package(str(input_path), "bbq_text_upload")
	written = interface.save_package(engine, str(destination))
	if written is None:
		raise RuntimeError("registered writer completed without an output path")


def order_receipt(args: argparse.Namespace) -> None:
	"""Record the deliberately asymmetric ORDER behavior of the two frozen CLI writers."""
	args.output.mkdir(parents=True, exist_ok=True)
	result = {}
	for engine in ("canvas_qti_v1_2", "blackboard_export_zip"):
		prefix = "qti12" if engine == "canvas_qti_v1_2" else "bez"
		destination = args.output / f"{prefix}-parity-order-only.zip"
		command = [
			sys.executable, str(args.oracle_root / "tools" / "bbq_converter.py"),
			"-i", str(args.input), "-f", engine, "-q",
		]
		environment = os.environ.copy()
		environment["PYTHONPATH"] = str(args.oracle_root) + os.pathsep + environment.get("PYTHONPATH", "")
		# The executable and arguments are fixed; this call never invokes a shell.
		completed = subprocess.run(  # nosec B603
			command, cwd=args.output, env=environment, text=True, capture_output=True, check=False
		)
		result[engine] = {
			"exit": completed.returncode,
			"file": destination.is_file(),
			"diagnostic": collapse(completed.stdout + " " + completed.stderr),
		}
	print(json.dumps(result, sort_keys=True))


def collapse(value: str) -> str:
	return " ".join(value.split())


def normalized_value(value: object) -> object:
	if isinstance(value, (date, datetime)):
		return value.isoformat()
	if isinstance(value, str):
		return collapse(value)
	if isinstance(value, list):
		return [normalized_value(child) for child in value]
	if isinstance(value, tuple):
		return [normalized_value(child) for child in value]
	if isinstance(value, dict):
		return {str(key): normalized_value(value[key]) for key in sorted(value)}
	return value


def readback_fingerprint(path: pathlib.Path, engine: str) -> object:
	from qti_package_maker import package_interface

	interface = package_interface.QTIPackageInterface("parity_read", allow_mixed=True)
	interface.read_package(str(path), engine)
	items = []
	for item in interface.item_bank:
		fields = {"kind": item.item_type, "question": collapse(item.question_text)}
		for field in item.get_supporting_field_names():
			fields[field] = normalized_value(getattr(item, field))
		items.append(fields)
	return items


def readback_outcome(path: pathlib.Path, engine: str) -> object:
	try:
		return {"outcome": "success", "items": readback_fingerprint(path, engine)}
	except Exception as error:
		return {"outcome": "rejected", "error_type": type(error).__name__, "message": str(error)}
