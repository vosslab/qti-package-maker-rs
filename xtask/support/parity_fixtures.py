#!/usr/bin/env python3
"""Create deterministic, ignored M13 parity inputs below an explicit directory."""

import argparse
import base64
import pathlib


PNG = base64.b64decode(
	"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)
SVG = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1'/></svg>"


def write(root: pathlib.Path, relative: str, data: bytes) -> None:
	path = root / relative
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_bytes(data)


def main() -> None:
	parser = argparse.ArgumentParser(description="Create ignored M13 parity fixtures.")
	parser.add_argument("--output", required=True, type=pathlib.Path)
	args = parser.parse_args()
	root = args.output
	root.mkdir(parents=True, exist_ok=True)
	write(root, "images/a/shared.png", PNG)
	write(root, "images/b/shared.png", PNG)
	write(root, "nested/mixedCase.svg", SVG)
	write(root, "images/with space.PNG", PNG)
	all_kinds = [
		"MC\tWhich base pairs with A?\tT\tcorrect\tC\tincorrect",
		"MA\tChoose all base pairs.\tA\tcorrect\tC\tcorrect\tU\tincorrect",
		"MAT\tMatch bases.\tA\tT\tC\tG",
		"NUM\tHow many chromatids follow replication?\t4\t0.01",
		"FIB\tThe hereditary material is ____.\tDNA",
		"FIB_PLUS\tA [animal] is a [class].\tanimal\tdog\t\tclass\tmammal\t",
		"ORD\tOrder mitosis.\tProphase\tMetaphase\tAnaphase\tTelophase",
	]
	supported = {
		"bbq_text_upload": range(7),
		"text2qti": (0, 1, 3, 4),
		"okla_chrst_bqgen": (0, 1, 2, 4),
		"blackboard_export_zip": (0, 1, 2, 3, 4, 5),
		"canvas_qti_v1_2": (0, 1, 2, 3, 4, 5),
		"blackboard_qti_v2_1": range(7),
		"exam_yaml": range(7),
		"moodle_aiken": (0,),
		"human_readable": range(7),
		"html_selftest": range(7),
	}
	kind_names = ("mc", "ma", "match", "num", "fib", "multi_fib", "order")
	for engine, indexes in supported.items():
		(root / f"bbq-parity-{engine}-questions.txt").write_text(
			"\n".join(all_kinds[index] for index in indexes) + "\n",
			encoding="utf-8",
		)
		for index in indexes:
			(root / f"bbq-parity-projection-{engine}--{kind_names[index]}-questions.txt").write_text(
				all_kinds[index] + "\n",
				encoding="utf-8",
			)
	(root / "bbq-parity-all-kinds-questions.txt").write_text("\n".join(all_kinds) + "\n", encoding="utf-8")
	(root / "bbq-parity-mc-only-questions.txt").write_text(all_kinds[0] + "\n", encoding="utf-8")
	(root / "bbq-parity-text2qti-multiblock-delimiter-repair-questions.txt").write_text(
		"\n".join(all_kinds[index] for index in (0, 1, 3, 4)) + "\n",
		encoding="utf-8",
	)
	(root / "bbq-parity-html-table-questions.txt").write_text(
		"MC\tWhich base pairs with A? <table border='1'><tr><td>A</td><td>T</td></tr></table>\tT\tcorrect\tC\tincorrect\n",
		encoding="utf-8",
	)
	(root / "bbq-parity-order-only-questions.txt").write_text(all_kinds[-1] + "\n", encoding="utf-8")
	local = (
		"MC\tLocal images <img src='images/a/shared.png' alt='A'/>"
		"<img src='images/b/shared.png' alt='B'/>"
		"<img src='nested/mixedCase.svg' alt='SVG'/>\tfirst\tcorrect\tsecond\tincorrect\n"
	)
	(root / "bbq-parity-media-local-questions.txt").write_text(local, encoding="utf-8")
	repeated = (
		"MC\tRepeated <img src='images/a/shared.png' alt='first'/>"
		"<img src='images/a/shared.png' alt='again'/>"
		"<img src='images/b/shared.png' alt='same bytes, other path'/>\tfirst\tcorrect\tsecond\tincorrect\n"
	)
	(root / "bbq-parity-media-repeated-questions.txt").write_text(repeated, encoding="utf-8")
	spaced = "MC\tSpace <img src='images/with space.PNG' alt='space'/>\tfirst\tcorrect\tsecond\tincorrect\n"
	(root / "bbq-parity-media-spaces-questions.txt").write_text(spaced, encoding="utf-8")
	data = base64.b64encode(PNG).decode("ascii")
	nonpackageable = (
		"MC\tReferences <img src='https://example.invalid/remote.png' alt='external'/>"
		f"<img src='data:image/png;base64,{data}' alt='embedded'/>\tfirst\tcorrect\tsecond\tincorrect\n"
	)
	(root / "bbq-parity-media-nonpackageable-questions.txt").write_text(nonpackageable, encoding="utf-8")
	(root / "bbq-parity-media-noimage-questions.txt").write_text(
		"MC\tNo image baseline.\tfirst\tcorrect\tsecond\tincorrect\n",
		encoding="utf-8",
	)
	print(root)


if __name__ == "__main__":
	main()
