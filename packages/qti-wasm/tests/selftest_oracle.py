"""Emit current Python QPM self-test HTML for the browser parity test."""

from __future__ import annotations

import base64
import json
import pathlib
import sys
import tempfile


def main() -> None:
	python_root = pathlib.Path(sys.argv[1]).resolve()
	if not (python_root / "qti_package_maker").is_dir():
		raise SystemExit(f"not a qti-package-maker checkout: {python_root}")
	sys.path.insert(0, str(python_root))
	from qti_package_maker import package_interface
	from qti_package_maker.engines.html_selftest import write_item

	source = base64.b64decode(sys.argv[2]).decode("utf-8")
	with tempfile.TemporaryDirectory(prefix="qti-selftest-oracle-") as temporary:
		input_path = pathlib.Path(temporary) / "input.txt"
		input_path.write_text(source, encoding="utf-8")
		interface = package_interface.QTIPackageInterface("browser-selftest", verbose=False, allow_mixed=True)
		interface.read_package(str(input_path), "bbq_text_upload")
		items = list(interface.item_bank)
	if len(items) != 1:
		raise SystemExit(f"expected one parsed item, got {len(items)}")
	item = items[0]
	render = getattr(write_item, item.item_type)
	print(json.dumps({"crc": item.item_crc16, "html": render(item)}))


if __name__ == "__main__":
	main()
