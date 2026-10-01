#!/usr/bin/env python3
"""Project one BBQ bank into each writer's declared supported item kinds.

The M6 corpus is deliberately heterogeneous.  Each derived bank retains the
source record order, so parity checks each representable record without asking a
writer to silently discard an unrelated kind.  Outputs live below tests/_temp.
"""

import argparse
import hashlib
import html.parser
import json
import pathlib
import shutil


SUPPORTED = {
	"bbq_text_upload": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS", "ORD"},
	"text2qti": {"MC", "MA", "NUM", "FIB"},
	"okla_chrst_bqgen": {"MC", "MA", "MAT", "FIB"},
	"blackboard_export_zip": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS"},
	"canvas_qti_v1_2": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS"},
	"blackboard_qti_v2_1": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS", "ORD"},
	"exam_yaml": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS", "ORD"},
	"moodle_aiken": {"MC"},
	"human_readable": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS", "ORD"},
	"html_selftest": {"MC", "MA", "MAT", "NUM", "FIB", "FIB_PLUS", "ORD"},
}
KNOWN_KINDS = set().union(*SUPPORTED.values())


class ImageSources(html.parser.HTMLParser):
	"""Collect local image spellings without rewriting authored question text."""

	def __init__(self) -> None:
		super().__init__(convert_charrefs=True)
		self.sources: set[str] = set()

	def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
		if tag == "img":
			for name, value in attrs:
				if name == "src" and value:
					self.sources.add(value)


def preserve_media(source: pathlib.Path, destination: pathlib.Path, records: list[str]) -> None:
	"""Retain existing relative assets in an isolated projected-bank media base."""
	parser = ImageSources()
	for record in records:
		parser.feed(record)
	base = source.parent.resolve()
	for spelling in sorted(parser.sources):
		if spelling.startswith(("data:", "http:", "https:", "//")):
			continue
		relative = pathlib.Path(spelling)
		if relative.is_absolute() or ".." in relative.parts:
			continue
		original = base / relative
		if not original.is_file():
			continue
		# Match the production boundary: do not copy symlinks escaping the source base.
		if not original.resolve().is_relative_to(base):
			continue
		target = destination / relative
		target.parent.mkdir(parents=True, exist_ok=True)
		shutil.copyfile(original, target)


def main() -> None:
	parser = argparse.ArgumentParser(description="Project a heterogeneous BBQ bank by writer support.")
	parser.add_argument("--input", required=True, type=pathlib.Path)
	parser.add_argument("--output", required=True, type=pathlib.Path)
	parser.add_argument("--identity", required=True)
	args = parser.parse_args()
	lines = args.input.read_text(encoding="utf-8").splitlines()
	records: list[tuple[str, str]] = []
	for number, line in enumerate(lines, start=1):
		if not line.strip():
			continue
		kind = line.partition("\t")[0]
		if kind not in KNOWN_KINDS:
			raise ValueError(f"{args.input}:{number}: unknown BBQ item kind {kind!r}")
		records.append((kind, line))
	if not records:
		raise ValueError(f"{args.input}: no BBQ records")
	# Equal record bytes from different source directories may refer to different assets.
	origin = hashlib.sha256(str(args.input.resolve()).encode()).hexdigest()[:16]
	media_base = args.output / f"{args.identity}-{origin}"
	media_base.mkdir(parents=True, exist_ok=True)
	preserve_media(args.input, media_base, [line for _, line in records])
	result = []
	for engine, supported in SUPPORTED.items():
		selected = [line for kind, line in records if kind in supported]
		if not selected:
			continue
		path = media_base / f"bbq-parity-corpus-{args.identity}-{engine}-questions.txt"
		path.write_text("\n".join(selected) + "\n", encoding="utf-8")
		result.append({"engine": engine, "path": str(path), "records": len(selected)})
	print(json.dumps({"source_records": len(records), "projections": result}, sort_keys=True))


if __name__ == "__main__":
	main()
