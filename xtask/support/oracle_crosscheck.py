"""Build independent package-integrity corpora and emit Python oracle findings.

This development helper deliberately constructs its malformed packages from
format rules.  It never calls Rust writers or derives an expected result from
Rust checker output.
"""

import argparse
import base64
import json
import pathlib
import sys
import zipfile

from qti_package_maker.assessment_items import item_bank
from qti_package_maker.common import package_integrity
from qti_package_maker.engines.blackboard_export_zip import engine_class as bb_export
from qti_package_maker.engines.blackboard_qti_v2_1 import engine_class as bb_qti21
from qti_package_maker.engines.canvas_qti_v1_2 import engine_class as canvas_qti12


PNG_8X8 = base64.b64decode(
	"iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAFElEQVR4nGNkaPjPgA0wYRUdtBIALlIBjzTK6JgAAAAASUVORK5CYII="
)
PNG_1X1 = base64.b64decode(
	"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)


def violation_code(message: str) -> str:
	"""Translate stable Python diagnostic shapes into Rust's public code API."""
	mappings = (
		("resource href ", "dangling-resource-href"),
		("<file href=", "dangling-file-href"),
		("dependency identifierref ", "dangling-dependency"),
		("scored varequal ", "qti12-dangling-varequal"),
		("correctResponse token ", "qti21-dangling-correct-response"),
		("has no outcomeDeclaration identifier=\"SCORE\"", "missing-score-outcome"),
		("<img src=", "dangling-image-source"),
		("could not read image dimensions", "unreadable-raster"),
		("image dimensions ", "invisible-raster"),
		("is not an id-safe token", "unsafe-identifier"),
		("has no csfiles binary", "orphaned-xid-token"),
		("has no matching CSResourceLinks", "orphaned-xid-resource-link"),
		("CSResourceLinks parentId ", "orphaned-cs-parent"),
		("is missing its LOM sidecar", "missing-lom-sidecar"),
		("bb:file ", "dangling-bb-file"),
	)
	for needle, code in mappings:
		if needle in message:
			return code
	raise ValueError(f"unmapped Python integrity diagnostic: {message}")


def diagnostic_path(message: str) -> str:
	"""Return the package location used by Python's human-readable diagnostic."""
	if ": " in message:
		return message.split(": ", 1)[0]
	if message.startswith("xid token") or message.startswith("CSResourceLinks"):
		return "blackboard-export"
	if message.startswith("csfiles binary '"):
		return message.split("'", 2)[1]
	raise ValueError(f"Python diagnostic has no location prefix: {message}")


def canonical_violations(path: pathlib.Path) -> list[dict]:
	"""Return sorted, wording-independent Python checker findings."""
	result = []
	for message in package_integrity.check_package(str(path)):
		code = violation_code(message)
		result.append({
			"code": code,
			"path": diagnostic_path(message),
			"severity": "advisory" if code == "invisible-raster" else "error",
		})
	return sorted(result, key=lambda item: (item["code"], item["path"], item["severity"]))


def write_zip(path: pathlib.Path, entries: dict[str, bytes]) -> None:
	"""Write one explicit independent test package."""
	with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
		for name, data in entries.items():
			archive.writestr(name, data)


def manifest(resources: str = "") -> bytes:
	return (
		"<?xml version='1.0' encoding='UTF-8'?>"
		"<manifest xmlns='http://www.imsglobal.org/xsd/imscp_v1p1' identifier='m'>"
		f"<resources>{resources}</resources></manifest>"
	).encode()


def qti12_item(answer: str = "choice_999", ident: str = "mc1") -> bytes:
	return (
		"<questestinterop xmlns='http://www.imsglobal.org/xsd/ims_qtiasiv1p2'><assessment><section><item ident='" + ident + "'><presentation>"
		"<response_lid ident='response1'><render_choice>"
		"<response_label ident='choice_001'/></render_choice></response_lid></presentation>"
		"<resprocessing><respcondition><conditionvar><varequal respident='response1'>"
		+ answer + "</varequal></conditionvar></respcondition></resprocessing>"
		"</item></section></assessment></questestinterop>"
	).encode()


def qti21_item(answer: str = "answer_002", with_score: bool = True) -> bytes:
	outcome = "<outcomeDeclaration identifier='SCORE'/>" if with_score else ""
	return (
		"<assessmentItem xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1' identifier='qti21'><responseDeclaration identifier='RESPONSE'>"
		"<correctResponse><value>" + answer + "</value></correctResponse></responseDeclaration>"
		+ outcome + "<itemBody><choiceInteraction responseIdentifier='RESPONSE'>"
		"<simpleChoice identifier='answer_001'>A</simpleChoice></choiceInteraction></itemBody>"
		"</assessmentItem>"
	).encode()


def blackboard_manifest() -> bytes:
	return (
		"<manifest xmlns:bb='http://www.blackboard.com/content-packaging/' identifier='m'><resources>"
		"<resource bb:file='res00002.dat' identifier='res00002'/><resource bb:file='res00005.dat' "
		"identifier='res00005'/></resources></manifest>"
	).encode()


def negative_cases() -> dict[str, dict[str, bytes]]:
	"""Return M6's format-specification negative corpus and four regression canaries."""
	item_resource = "<resource identifier='item' href='item.xml'><file href='item.xml'/></resource>"
	image_item = qti21_item(answer="answer_001").replace(
		b"</itemBody>", b'<img src="images/missing.png"/></itemBody>')
	pool = b"<questestinterop><bbmd_asi_object_id>_111_1</bbmd_asi_object_id></questestinterop>"
	links = b"<cms_resource_link_list><cms_resource_link><parentId>_999_9</parentId><resourceId>other</resourceId></cms_resource_link></cms_resource_link_list>"
	return {
		"dangling_resource_href": {"imsmanifest.xml": manifest("<resource identifier='item' href='ghost.xml'/>")},
		"dangling_file_href": {"imsmanifest.xml": manifest("<resource identifier='item'><file href='ghost.xml'/></resource>")},
		"dangling_dependency": {"imsmanifest.xml": manifest("<resource identifier='item'><dependency identifierref='ghost'/></resource>")},
		"qti12_varequal": {"imsmanifest.xml": manifest(item_resource), "item.xml": qti12_item()},
		"qti21_correct_response": {"imsmanifest.xml": manifest(item_resource), "item.xml": qti21_item()},
		"missing_score_outcome": {"imsmanifest.xml": manifest(item_resource), "item.xml": qti21_item(answer='answer_001', with_score=False)},
		"dangling_image": {"imsmanifest.xml": manifest(item_resource), "item.xml": image_item},
		"unused_raster": {"imsmanifest.xml": manifest("<resource identifier='image'><file href='images/orphan.png'/></resource>"), "images/orphan.png": PNG_8X8},
		"truncated_raster": {"imsmanifest.xml": manifest("<resource identifier='image'><file href='images/broken.png'/></resource>"), "images/broken.png": b"not a png"},
		"single_pixel_raster": {"imsmanifest.xml": manifest("<resource identifier='image'><file href='images/tiny.png'/></resource>"), "images/tiny.png": PNG_1X1},
		"unsafe_identifier": {"imsmanifest.xml": manifest("<resource identifier='bad id'/>")},
		"orphaned_xid": {"imsmanifest.xml": blackboard_manifest(), "res00002.dat": b"<pool>bbcswebdav/xid-77</pool>", "res00005.dat": b"<links/>"},
		"orphaned_parent": {"imsmanifest.xml": blackboard_manifest(), "res00002.dat": pool, "res00005.dat": links},
		"missing_lom_sidecar": {"imsmanifest.xml": blackboard_manifest(), "res00002.dat": b"<pool/>", "res00005.dat": b"<links/>", "csfiles/home_dir/__xid-77.png": PNG_8X8},
		"canary_qti12": {"imsmanifest.xml": manifest(item_resource), "item.xml": qti12_item()},
		"canary_qti21": {"imsmanifest.xml": manifest(item_resource), "item.xml": qti21_item()},
		"canary_parent": {"imsmanifest.xml": blackboard_manifest(), "res00002.dat": pool, "res00005.dat": links},
		"canary_manifest_media": {"imsmanifest.xml": manifest("<resource identifier='item' href='item.xml'><file href='item.xml'/><file href='ghost.xml'/><dependency identifierref='nope'/></resource>"), "item.xml": image_item},
	}


def representative_bank(include_order: bool = True) -> object:
	"""Build all seven core question kinds and one actual media asset."""
	bank = item_bank.ItemBank(allow_mixed=True)
	bank.add_image("images/figure.png", PNG_8X8)
	bank.add_item("MC", ('MC question <img src="images/figure.png" alt="figure"/>', ["choice a", "choice b", "choice c", "choice d"], "choice b"))
	bank.add_item("MA", ("MA question", ["choice a", "choice b", "choice c"], ["choice a"]))
	bank.add_item("MATCH", ("MATCH question", ["prompt one", "prompt two", "prompt three"], ["choice one", "choice two", "choice three"]))
	bank.add_item("NUM", ("NUM question", 4.0, 0.1, True))
	bank.add_item("FIB", ("FIB question", ["answer"]))
	bank.add_item("MULTI_FIB", ("MULTI [one] [two]", {"one": ["one"], "two": ["two"]}))
	if include_order:
		bank.add_item("ORDER", ("ORDER question", ["first", "second", "third"]))
	bank.renumber_items()
	return bank


def write_python_outputs(destination: pathlib.Path) -> tuple[list[tuple[str, pathlib.Path]], list[dict]]:
	"""Write clean packages through the three Python ZIP writers."""
	engines = {
		"canvas_qti_v1_2": (canvas_qti12.EngineClass("oracle", verbose=False), False),
		"blackboard_qti_v2_1": (bb_qti21.EngineClass("oracle", verbose=False), True),
		"blackboard_export_zip": (bb_export.EngineClass("oracle", verbose=False), False),
	}
	paths, coverage = [], []
	for name, (engine, supports_order) in engines.items():
		bank = representative_bank(include_order=supports_order)
		path = destination / f"python_{name}.zip"
		engine.save_package(bank, outfile=str(path))
		shape = "MC,MA,MATCH,NUM,FIB,MULTI_FIB,ORDER" if supports_order else "MC,MA,MATCH,NUM,FIB,MULTI_FIB"
		paths.append((f"python:{name}", path))
		coverage.append({"engine": name, "kinds": shape, "unsupported": "ORDER" if not supports_order else ""})
	return paths, coverage


def main() -> None:
	parser = argparse.ArgumentParser()
	parser.add_argument("--output", required=True, type=pathlib.Path)
	parser.add_argument("--existing", action="append", default=[], type=pathlib.Path)
	args = parser.parse_args()
	args.output.mkdir(parents=True, exist_ok=True)
	records = []
	for name, entries in negative_cases().items():
		path = args.output / f"negative_{name}.zip"
		write_zip(path, entries)
		records.append({"name": f"negative:{name}", "path": str(path), "python": canonical_violations(path)})
	python_outputs, producer_coverage = write_python_outputs(args.output)
	for name, path in python_outputs:
		records.append({"name": name, "path": str(path), "python": canonical_violations(path)})
	for path in args.existing:
		records.append({"name": f"real:{path.name}", "path": str(path), "python": canonical_violations(path)})
	json.dump({"producer_coverage": producer_coverage, "records": records}, sys.stdout, sort_keys=True)


if __name__ == "__main__":
	main()
