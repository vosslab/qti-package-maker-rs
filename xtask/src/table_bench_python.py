"""Run the Python HTML-to-image baseline with temporary timing hooks."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import runpy
import shutil
import subprocess
import sys
import tempfile
import time
from collections import defaultdict
from pathlib import Path


def arguments() -> argparse.Namespace:
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--corpus", type=Path)
	parser.add_argument("--report-json", type=Path)
	parser.add_argument("--run-repaired-converter", nargs=argparse.REMAINDER)
	return parser.parse_args()


def inputs(corpus: Path) -> list[Path]:
	with (corpus / "manifest.json").open(encoding="utf-8") as handle:
		paths = [Path(value) for value in json.load(handle).get("bbq_files", [])]
	if not paths:
		raise ValueError("corpus manifest has no bbq_files; rerun cargo xtask table-corpus")
	if missing := next((path for path in paths if not path.is_file()), None):
		raise ValueError(f"corpus references missing BBQ input: {missing}")
	return paths


def converter() -> Path:
	# source_me.sh deliberately exposes the oracle through PYTHONPATH, not a durable
	# environment variable. Resolve the installed-or-checkout package that Python imports.
	import qti_package_maker
	path = Path(qti_package_maker.__file__).resolve().parent.parent / "tools" / "bbq_converter.py"
	if not path.is_file():
		raise ValueError(f"Python converter is unavailable: {path}")
	return path


def oracle_provenance() -> tuple[str, str]:
	"""Return the immutable snapshot root and commit selected by source_me.sh."""
	import qti_package_maker

	oracle_root = Path(qti_package_maker.__file__).resolve().parent.parent
	marker = oracle_root / "PINNED_ORACLE_PROVENANCE.txt"
	if not marker.is_file():
		raise ValueError(f"benchmark Python oracle lacks provenance marker: {marker}")
	commit = marker.read_text(encoding="utf-8").splitlines()[0]
	return str(oracle_root), commit


def stage_source_media(source_bank: object, converted_bank: object, metrics: dict[str, float] | None = None) -> None:
	"""Copy source local assets into the converted bank's owned media root.

	This is a development-only reproduction of WP-T5's safe media staging rule.
	It reads only assets already resolved by the source bank, retains their relative
	src paths, and writes only into the converted bank's new owned directory.
	"""
	from qti_package_maker.common import media_assets

	start = time.perf_counter()
	assets = source_bank.collect_assets().assets
	if not any(asset.kind == media_assets.KIND_LOCAL for asset in assets):
		return
	if converted_bank.media_base_dir is None:
		media_root = tempfile.mkdtemp(prefix="qti_table_bench_media_")
		converted_bank.set_media_base_dir(media_root, owned=True)
	for asset in assets:
		if asset.kind != media_assets.KIND_LOCAL:
			continue
		# resolve_local_path retains the package's traversal protection for every copy.
		destination = Path(media_assets.resolve_local_path(converted_bank.media_base_dir, asset.src))
		destination.parent.mkdir(parents=True, exist_ok=True)
		payload = asset.read_bytes()
		destination.write_bytes(payload)
		if metrics is not None:
			metrics["media_staging_asset_count"] += 1
			metrics["media_staging_bytes"] += len(payload)
	if metrics is not None:
		metrics["media_staging"] += time.perf_counter() - start


def repaired_convert_bank(metrics: dict[str, float] | None = None) -> None:
	"""Install the benchmark-only media staging wrapper around convert_bank."""
	from qti_package_maker.html_to_image import transform

	original_convert = getattr(transform, "_table_bench_original_convert_bank", transform.convert_bank)
	transform._table_bench_original_convert_bank = original_convert

	def convert_bank(*args: object, **kwargs: object) -> object:
		converted_bank = original_convert(*args, **kwargs)
		stage_source_media(args[0], converted_bank, metrics)
		return converted_bank

	transform.convert_bank = convert_bank


def run_repaired_converter(converter_args: list[str]) -> None:
	"""Run one CLI conversion in a fresh Python process with benchmark-only staging."""
	if not converter_args:
		raise ValueError("--run-repaired-converter needs a converter path")
	source, remaining_args = converter_args[0], converter_args[1:]
	repaired_convert_bank()
	sys.argv = [source, *remaining_args]
	runpy.run_path(source, run_name="__main__")


def plain_run(source: Path, bbq_files: list[Path], formats: list[str], work: Path) -> dict[str, object]:
	start = time.perf_counter()
	failures: list[dict[str, object]] = []
	for number, bbq in enumerate(bbq_files, start=1):
		output = work / "plain" / str(number)
		output.mkdir(parents=True)
		completed = subprocess.run(
			[sys.executable, str(Path(__file__).resolve()), "--run-repaired-converter",
				str(source), "-i", str(bbq), *formats, "--html-to-image", "--quiet"],
			cwd=output, text=True, capture_output=True, check=False)
		if completed.returncode:
			failures.append({"input": str(bbq), "exit_code": completed.returncode,
				"stderr": completed.stderr[-2000:], "stdout": completed.stdout[-2000:]})
	return {"wall_seconds": time.perf_counter() - start, "input_count": len(bbq_files), "failures": failures}


def hooks(metrics: dict[str, float]) -> None:
	"""Patch only this process's renderer methods; production source stays unchanged."""
	from qti_package_maker.html_to_image import render_table, transform

	def timed(stage: str, callback: object, *args: object, **kwargs: object) -> object:
		start = time.perf_counter()
		try:
			return callback(*args, **kwargs)
		finally:
			metrics[stage] += time.perf_counter() - start

	def ensure_started(self: object) -> None:
		if getattr(self, "_page", None) is not None:
			return
		self._playwright = timed("playwright_start", render_table.sync_playwright().start)
		metrics["browser_launch_count"] += 1
		self._browser = timed("browser_launch", self._playwright.chromium.launch)
		self._context = timed("browser_context", self._browser.new_context,
			device_scale_factor=render_table.DEVICE_SCALE_FACTOR)
		self._page = timed("page_creation", self._context.new_page)
		self._font_face_css = timed("font_css", render_table._font_face_css)
		font_styles = "<style>" + self._font_face_css + "</style>" if self._font_face_css else ""
		html = ("<html><head>" + font_styles
			+ "<style>#mathml-render-root math { font-size: 1.2em; }</style></head><body style='"
			+ render_table.WRAPPER_BODY_STYLE
			+ "'><div id='table-render-root'></div><div id='mathml-render-root'></div></body></html>")
		timed("static_page_setup", self._page.set_content, html)

	def render_png(self: object, table_html: str) -> bytes:
		self._ensure_started()
		timed("table_evaluate_and_font_ready", self._page.evaluate,
			render_table.TABLE_FONT_MAPPING_SCRIPT, table_html)
		metrics["table_count"] += 1
		return timed("table_screenshot", self._page.locator("#table-render-root table").first.screenshot, type="png")

	original_convert = getattr(transform, "_table_bench_original_convert_bank", transform.convert_bank)
	transform._table_bench_original_convert_bank = original_convert
	def convert_bank(*args: object, **kwargs: object) -> object:
		converted_bank = timed("conversion_total", original_convert, *args, **kwargs)
		stage_source_media(args[0], converted_bank, metrics)
		return converted_bank

	# The certified snapshot creates a page per table, while the live Python
	# checkout keeps a static page. Measure both real APIs without changing either.
	if hasattr(render_table.TableRenderer(), "_page"):
		render_table.TableRenderer._ensure_started = ensure_started
		render_table.TableRenderer.render_table_png = render_png
	else:
		def enter(self: object) -> object:
			self._playwright = timed("playwright_start", render_table.sync_playwright().start)
			metrics["browser_launch_count"] += 1
			self._browser = timed("browser_launch", self._playwright.chromium.launch)
			self._context = timed("browser_context", self._browser.new_context,
				device_scale_factor=render_table.DEVICE_SCALE_FACTOR)
			self._font_face_css = timed("font_css", render_table._font_face_css)
			return self

		def render_legacy_png(self: object, table_html: str) -> bytes:
			font_styles = ""
			if self._font_face_css:
				font_styles = "<style>" + self._font_face_css + "</style>"
			html = ("<html><head>" + font_styles + "</head><body style='"
				+ render_table.WRAPPER_BODY_STYLE + "'>" + table_html + "</body></html>")
			page = timed("page_creation", self._context.new_page)
			timed("static_page_setup", page.set_content, html)
			timed("table_evaluate_and_font_ready", page.evaluate,
				render_table.TABLE_FONT_MAPPING_SCRIPT)
			timed("table_evaluate_and_font_ready", page.evaluate,
				"async () => { await document.fonts.ready; }")
			metrics["table_count"] += 1
			png_bytes = timed("table_screenshot", page.locator("table").first.screenshot, type="png")
			page.close()
			return png_bytes

		render_table.TableRenderer.__enter__ = enter
		render_table.TableRenderer.render_table_png = render_legacy_png
	transform.convert_bank = convert_bank


def instrumented_run(source: Path, bbq_files: list[Path], formats: list[str], work: Path) -> dict[str, object]:
	metrics: dict[str, float] = defaultdict(float)
	hooks(metrics)
	failures: list[dict[str, object]] = []
	start = time.perf_counter()
	for number, bbq in enumerate(bbq_files, start=1):
		output = work / "instrumented" / str(number)
		output.mkdir(parents=True)
		old_argv, old_cwd = sys.argv, Path.cwd()
		try:
			os.chdir(output)
			sys.argv = [str(source), "-i", str(bbq), *formats, "--html-to-image", "--quiet"]
			try:
				runpy.run_path(str(source), run_name="__main__")
			except SystemExit as error:
				if error.code not in (0, None):
					failures.append({"input": str(bbq), "exit_code": error.code})
		except Exception as error:
			failures.append({"input": str(bbq), "exception": repr(error)})
		finally:
			sys.argv = old_argv
			os.chdir(old_cwd)
	metrics["instrumented_wall_seconds"] = time.perf_counter() - start
	render_seconds = sum(metrics[name] for name in (
		"playwright_start", "browser_launch", "browser_context", "page_creation", "font_css",
		"static_page_setup", "table_evaluate_and_font_ready", "table_screenshot"))
	metrics["conversion_bookkeeping"] = max(0.0, metrics["conversion_total"] - render_seconds)
	return {"stages_seconds": dict(metrics), "failures": failures}


def main() -> None:
	args = arguments()
	if args.run_repaired_converter:
		run_repaired_converter(args.run_repaired_converter)
		return
	if args.corpus is None or args.report_json is None:
		raise ValueError("--corpus and --report-json are required for a benchmark run")
	bbq_files, source = inputs(args.corpus), converter()
	input_provenance = [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
		for path in bbq_files]
	manifest_sha = hashlib.sha256((args.corpus / "manifest.json").read_bytes()).hexdigest()
	oracle_root, oracle_commit = oracle_provenance()
	work = Path(tempfile.mkdtemp(prefix="qti-table-bench-"))
	try:
		runs: dict[str, object] = {}
		for name, formats in (("three_format", ["-1", "-2", "-B"]), ("blackboard_export", ["-B"])):
			run_work = work / name
			runs[name] = {"formats": formats, "plain": plain_run(source, bbq_files, formats, run_work),
				"instrumented": instrumented_run(source, bbq_files, formats, run_work)}
		if hashlib.sha256((args.corpus / "manifest.json").read_bytes()).hexdigest() != manifest_sha:
			raise ValueError("corpus manifest changed during benchmark")
		for record in input_provenance:
			if hashlib.sha256(Path(record["path"]).read_bytes()).hexdigest() != record["sha256"]:
				raise ValueError(f"corpus input changed during benchmark: {record['path']}")
		result = {"manifest_sha256": manifest_sha, "input_provenance": input_provenance, "command": "cargo xtask table-bench", "corpus": str(args.corpus.resolve()),
			"oracle_commit": oracle_commit, "oracle_root": oracle_root,
			"python": sys.version, "platform": platform.platform(), "machine": platform.machine(), "runs": runs}
		args.report_json.parent.mkdir(parents=True, exist_ok=True)
		args.report_json.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
		print(json.dumps(result, indent=2, sort_keys=True))
		if any(run[mode]["failures"] for run in runs.values() for mode in ("plain", "instrumented")):
			raise RuntimeError("Python benchmark failed; retained receipt is not a performance baseline")
	finally:
		shutil.rmtree(work)


if __name__ == "__main__":
	main()
