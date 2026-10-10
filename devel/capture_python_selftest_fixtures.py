#!/usr/bin/env python3
"""Regenerate the proposed self-test fixtures through the current Python QPM writer.

Run with the Python reference checkout's source_me.sh environment; see the
fixture README. This explicit development command updates captured output.
"""

# Standard Library
import json
import re
import random
import pathlib
import hashlib
import subprocess

# Python reference checkout
from qti_package_maker import package_interface


#============================================
def main() -> None:
	root = pathlib.Path(subprocess.check_output(
		["git", "-C", str(pathlib.Path(__file__).parent), "rev-parse", "--show-toplevel"],
		text=True).strip())
	python_root = pathlib.Path(package_interface.__file__).parent.parent
	directory = root / "tests/fixtures/python_selftest"
	manifest_path = directory / "manifest.json"
	manifest = json.loads(manifest_path.read_text())
	manifest["python_commit"] = subprocess.check_output(
		["git", "-C", str(python_root), "rev-parse", "HEAD"], text=True).strip()
	manifest["python_worktree"] = subprocess.check_output(
		["git", "-C", str(python_root), "status", "--short"], text=True).splitlines()
	manifest["python_source_sha256"] = {}
	for path in sorted((python_root / "qti_package_maker/engines/html_selftest").glob("*.py")):
		manifest["python_source_sha256"][str(path.relative_to(python_root))] = hashlib.sha256(
			path.read_bytes()).hexdigest()
	for case in manifest["cases"]:
		input_path = directory / case["input"]
		output_path = directory / case["output"]
		interface = package_interface.QTIPackageInterface(
			case["name"], verbose=False, allow_mixed=True)
		interface.read_package(str(input_path), "bbq_text_upload")
		items = list(interface.item_bank)
		if len(items) != 1:
			raise ValueError(f"Expected one question in {input_path}")
		item = items[0]
		random.seed(manifest["random_seed"])
		interface.save_package("html_selftest", str(output_path))
		case["crc"] = item.item_crc16
		case["kind"] = item.item_type
		case["answer_key"] = {
			name: getattr(item, name) for name in (
				"answer_text", "answers_list", "answer_float", "tolerance_float",
				"answer_map", "prompts_list", "choices_list", "ordered_answers_list")
			if hasattr(item, name)
		}
		case["external_scripts"] = re.findall(
			r'<script[^>]+src=["\x27]([^"\x27]+)', output_path.read_text())
		case["input_sha256"] = hashlib.sha256(input_path.read_bytes()).hexdigest()
		case["output_sha256"] = hashlib.sha256(output_path.read_bytes()).hexdigest()
		print(case["output"])
	manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
	main()
