"""Verify random self-test selections against their individual authored items."""

import base64
import hashlib
import json
import pathlib
import re
import tempfile
from urllib.parse import unquote_to_bytes


def statement(path: pathlib.Path) -> str:
	from lxml import html

	root = html.fromstring(path.read_bytes())
	nodes = root.xpath('//*[@id and starts-with(@id, "statement_text_")] | //*[contains(concat(" ", normalize-space(@class), " "), " qti-statement ")]')
	if len(nodes) != 1:
		raise ValueError("self-test must contain exactly one question statement")
	return " ".join(" ".join(nodes[0].itertext()).split())


def authored_images(path: pathlib.Path) -> list[dict[str, object]]:
	"""Bind each selected-item image to its content and authored alternative text."""
	from lxml import html

	root = html.fromstring(path.read_bytes())
	items = root.xpath('//*[@id and starts-with(@id, "question_html_")]')
	if len(items) != 1:
		raise ValueError("self-test must contain exactly one image-owning item")
	result = []
	for node in items[0].xpath('.//img'):
		source = node.get("src", "")
		if source.startswith("data:"):
			header, separator, payload = source.partition(",")
			if not separator or not header.startswith("data:image/"):
				raise ValueError("self-test image data URI is invalid")
			data = base64.b64decode(payload, validate=True) if header.endswith(";base64") else unquote_to_bytes(payload)
		else:
			if not source or ":" in source or source.startswith(("/", "\\")):
				raise ValueError("self-test image must resolve locally")
			target = (path.parent / source).resolve()
			if not target.is_relative_to(path.parent.resolve()):
				raise ValueError("self-test image escapes its output directory")
			data = target.read_bytes()
		if not data:
			raise ValueError("self-test image is empty")
		result.append({"sha256": hashlib.sha256(data).hexdigest(), "alt": node.get("alt"), "title": node.get("title")})
	return result


def accessible_choices(path: pathlib.Path) -> dict[str, set[str]]:
	from lxml import html

	root = html.fromstring(path.read_bytes())
	result: dict[str, set[str]] = {}
	for button in root.xpath('//button[contains(concat(" ", normalize-space(@class), " "), " qti-match-choice ")]'):
		visible = re.sub(r"^[A-Z]\.\s*", "", " ".join(" ".join(button.itertext()).split()))
		name = button.get("aria-label")
		if name is None:
			name = visible
		else:
			name = re.sub(r"^(?:Select\s+)?[A-Z]\.\s*", "", " ".join(name.split()))
			name = name.removeprefix("Select ")
		result.setdefault(visible, set()).add(name)
	return result


def verify_accessible_choices(actual: pathlib.Path, expected: pathlib.Path) -> None:
	aliases = accessible_choices(expected)
	for visible, names in accessible_choices(actual).items():
		if visible not in aliases or not names <= (aliases[visible] | {visible}):
			raise ValueError("self-test MATCH accessible name does not identify its authored choice")


def validate_answer_control(node) -> None:
	classes = set(node.get('class', '').split())
	if not classes.intersection({'qti-input', 'fib-blank', 'qti-match-slot', 'qti-match-choice', 'qti-order-move', 'qti-btn'}) and not node.get('data-correct'):
		return
	if 'disabled' in node.attrib:
		raise ValueError('self-test answer control is disabled')
	for fieldset in node.xpath('ancestor::fieldset[@disabled]'):
		legends = fieldset.xpath('./legend')
		if not legends or legends[0] not in node.iterancestors():
			raise ValueError('self-test answer control is disabled by fieldset')
	if node.tag in {'input', 'textarea'} and 'readonly' in node.attrib:
		raise ValueError('self-test answer input is read-only')


def control_selftest() -> None:
	from lxml import html

	for value in ('<button class="qti-match-slot" disabled>Answer</button>', '<fieldset disabled><input class="qti-input"></fieldset>', '<input class="qti-input" readonly>'):
		root = html.fromstring(value)
		try:
			for node in root.xpath('descendant-or-self::input | descendant-or-self::button'):
				validate_answer_control(node)
		except ValueError:
			pass
		else:
			raise AssertionError('unusable self-test answer control was accepted')
	root = html.fromstring('<fieldset disabled><legend><input class="qti-input"></legend></fieldset>')
	validate_answer_control(root.xpath('.//input')[0])


def validate_control_associations(path: pathlib.Path) -> bool:
	"""Require named native controls and uniquely resolved accessible references."""
	from lxml import html
	root = html.fromstring(path.read_bytes())
	ids = {}
	for node in root.xpath('//*[@id]'):
		key = node.get('id')
		if key in ids:
			raise ValueError('duplicate accessible target')
		ids[key] = node
	for node in root.xpath('//input[not(@type="hidden")] | //button | //select | //textarea'):
		validate_answer_control(node)
		for attribute in ('aria-labelledby', 'aria-describedby'):
			for target in node.get(attribute, '').split():
				if target not in ids:
					raise ValueError('unresolved accessible reference')
		labelled = node.get('aria-labelledby', '').split()
		if labelled:
			name = ' '.join(' '.join(ids[key].itertext()) for key in labelled)
		elif node.get('aria-label') is not None:
			name = node.get('aria-label')
		else:
			labels = root.xpath('//label[@for=$identifier]', identifier=node.get('id', ''))
			labels += node.xpath('ancestor::label')
			name = ' '.join(' '.join(label.itertext()) for label in labels)
			if node.tag == 'button' and not name:
				name = ' '.join(node.itertext())
		if not name.strip():
			raise ValueError('unnamed accessible control')
	return True

def compare(python_path: pathlib.Path, rust_path: pathlib.Path, projection: object) -> tuple:
	from xtask.support.parity_selftest_structure import statement_structure
	from qti_package_maker import package_interface
	from qti_package_maker.engines.html_selftest import write_item
	from qti_package_maker.engines.html_selftest.engine_class import EngineClass

	binding_path = python_path.parent / "source_input_receipt.json"
	if not binding_path.is_file():
		return projection(python_path), projection(rust_path)
	binding = json.loads(binding_path.read_text())
	source = pathlib.Path(binding["path"])
	if hashlib.sha256(source.read_bytes()).hexdigest() != binding["sha256"]:
		raise ValueError("self-test source changed after export")
	interface = package_interface.QTIPackageInterface("parity", verbose=False, allow_mixed=True)
	interface.read_package(str(source), "bbq_text_upload")
	items = list(interface.item_bank)
	engine = EngineClass("parity", verbose=False)
	selections = []
	for side, path in (("python", python_path), ("rust", rust_path)):
		ids = re.findall(r"\bid=[\"']question_html_([^\"']+)[\"']", path.read_text())
		if len(ids) != 1:
			raise ValueError("self-test must select exactly one identifiable source item")
		candidates = [item for item in items if item.item_crc16 == ids[0]]
		if len(candidates) != 1:
			raise ValueError("self-test selected an unknown or ambiguous source item")
		if side == "rust":
			validate_control_associations(path)
		item = candidates[0]
		render = getattr(write_item, item.item_type, None)
		if render is None:
			raise ValueError("self-test selected an unsupported source item")
		with tempfile.TemporaryDirectory(dir=python_path.parent) as temporary:
			expected_path = pathlib.Path(temporary) / "selected.html"
			expected_path.write_text(engine._embed_images_as_data_uris(interface.item_bank, item, render(item)))
			if projection(path) != projection(expected_path):
				raise ValueError(f"{side} self-test answers do not match its selected authored item")
			if authored_images(path) != authored_images(expected_path):
				raise ValueError(f"{side} self-test authored images differ from its selected item")
			verify_accessible_choices(path, expected_path)
			if statement(path) != statement(expected_path):
				raise ValueError(f"{side} self-test question differs from its selected authored item")
			if statement_structure(path) != statement_structure(expected_path):
				raise ValueError(f"{side} self-test question markup differs from its selected authored item")
		selections.append({"side": side, "crc": ids[0], "kind": item.item_type, "output_sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
	(python_path.parent / "selftest_selection_receipt.json").write_text(json.dumps({"source": binding, "selections": selections}, indent=2) + "\n")
	value = {"outcome": "valid_random_source_selection", "source_sha256": binding["sha256"]}
	return value, value
