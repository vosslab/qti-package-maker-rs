"""Reproduce frozen MC output before correcting its nested display-label defect."""

import collections.abc
import hashlib
import json
import pathlib
import re
import tempfile

PREFIX = re.compile(r'(?is)^(?P<leading>(?:\s*<[^/!][^>]*>\s*)*)(?P<label>[A-Za-z0-9][):.])\s*')


def corrected(value: str, frozen_strip: collections.abc.Callable) -> str | None:
	match = PREFIX.match(value)
	if not match or frozen_strip(value) != value:
		return None
	if match.group('label').endswith('.') and value[match.end('label'):match.end('label') + 1].isdigit():
		return None
	return match.group('leading') + value[match.end():]


def selftest() -> None:
	unchanged = lambda value: value
	assert corrected('<div style="color:red"><span>A. Alpha</span></div>', unchanged) == '<div style="color:red"><span>Alpha</span></div>'
	assert corrected('<div><span>1.5 units</span></div>', unchanged) is None
	assert corrected('<div><span>Alpha</span></div>', unchanged) is None
	assert corrected('<div><span>A. Alpha</span></div>', lambda value: 'already stripped') is None
	assert corrected('<div><span>A. Alpha A. content</span></div>', unchanged) == '<div><span>Alpha A. content</span></div>'


def compare(engine: str, python_path: pathlib.Path, rust_path: pathlib.Path, original_compare: collections.abc.Callable) -> tuple | None:
	from qti_package_maker import package_interface
	from qti_package_maker.common import string_functions
	from xtask.support import parity_oracle as oracle
	from xtask.support.parity_selftest_selection import authored_images, validate_control_associations
	from xtask.support.parity_selftest_structure import statement_structure

	binding_path = python_path.parent / 'source_input_receipt.json'
	if not binding_path.is_file():
		return None
	binding = json.loads(binding_path.read_text())
	source = pathlib.Path(binding['path'])
	if hashlib.sha256(source.read_bytes()).hexdigest() != binding['sha256']:
		raise ValueError('choice-label repair source changed after export')
	name = re.fullmatch(r'bbq-(.+?)-questions\.txt', source.name)
	if name is None:
		return None
	interface = package_interface.QTIPackageInterface(name.group(1), verbose=False, allow_mixed=True)
	interface.read_package(str(source), 'bbq_text_upload')
	items = list(interface.item_bank)
	if len(items) != 1 or items[0].item_type != 'MC':
		return None
	item = items[0]
	choices = [corrected(choice, string_functions.strip_prefix_from_string) for choice in item.choices_list]
	answer = corrected(item.answer_text, string_functions.strip_prefix_from_string)
	if any(choice is None for choice in choices) or answer is None:
		return None
	if answer not in choices or len(choices) != len(set(choices)):
		raise ValueError('choice-label repair changes authored answer identity')

	def frozen_value(path: pathlib.Path) -> object:
		if engine == 'blackboard_export_zip':
			from xtask.support.parity_blackboard import xml_projection
			return {'semantic': xml_projection(path, allow_frozen_source_repairs=True), 'readback': oracle.readback_outcome(path, engine)}
		if engine == 'html_selftest':
			return {'grading': oracle.selftest_projection(path), 'statement': statement_structure(path), 'images': authored_images(path)}
		return original_compare(engine, path, path, False)[0]

	with tempfile.TemporaryDirectory(dir=python_path.parent) as temporary:
		folder = pathlib.Path(temporary)
		original = folder / python_path.name
		(folder / 'source_input_receipt.json').write_text(json.dumps(binding))
		interface.save_package(engine, str(original))
		if frozen_value(python_path) != frozen_value(original):
			raise ValueError('choice-label repair output differs from pinned source rendering')
		item.choices_list = choices
		item.answer_text = answer
		corrected_folder = folder / 'corrected'
		corrected_folder.mkdir()
		expected = corrected_folder / python_path.name
		interface.save_package(engine, str(expected))
		if engine == 'html_selftest':
			ids = re.findall(r'\bid=["\']question_html_([^"\']+)["\']', rust_path.read_text())
			if ids != [item.item_crc16]:
				raise ValueError('choice-label repair self-test selected an unknown source item')
			validate_control_associations(rust_path)
			values = frozen_value(expected), frozen_value(rust_path)
		else:
			values = original_compare(engine, expected, rust_path, False)
	receipt = {'source': binding, 'engine': engine, 'raw_output_sha256': hashlib.sha256(python_path.read_bytes()).hexdigest(), 'choices_corrected': len(choices)}
	(python_path.parent / 'choice_label_source_repair_receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
	return values
