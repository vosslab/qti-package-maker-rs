"""Source-bound correction of frozen Canvas MULTI_FIB text references."""

import collections.abc
import hashlib
import json
import pathlib
import tempfile
import zipfile
from defusedxml import ElementTree as xml
from xml.etree import ElementTree as serializer

QTI12 = 'http://www.imsglobal.org/xsd/ims_qtiasiv1p2'


def named(root: serializer.Element, name: str) -> list:
	return [node for node in root.iter() if node.tag == '{' + QTI12 + '}' + name]


def structure(node: serializer.Element, qualify: bool = False) -> list:
	tag = node.tag
	if qualify and not tag.startswith('{'):
		tag = '{' + QTI12 + '}' + tag
	text = node.text or ''
	if tag != '{' + QTI12 + '}mattext' and not text.strip():
		text = ''
	tail = node.tail or ''
	return [tag, sorted(node.attrib.items()), text, [structure(child, qualify) for child in node], tail if tail.strip() else '']


def frozen_projection(path: pathlib.Path, projector: collections.abc.Callable) -> object:
	try:
		return projector(path)
	except ValueError as original:
		failure = original
	from qti_package_maker import package_interface
	from qti_package_maker.engines.canvas_qti_v1_2 import write_item

	binding_path = path.parent / 'source_input_receipt.json'
	if not binding_path.is_file():
		raise failure
	binding = json.loads(binding_path.read_text())
	source = pathlib.Path(binding['path'])
	if hashlib.sha256(source.read_bytes()).hexdigest() != binding['sha256']:
		raise ValueError('Canvas repair source changed after export')
	interface = package_interface.QTIPackageInterface('parity', verbose=False, allow_mixed=True)
	interface.read_package(str(source), 'bbq_text_upload')
	items = list(interface.item_bank)
	if not items or any(item.item_type != 'MULTI_FIB' for item in items):
		raise failure
	repairs = []
	seen_items = 0
	with tempfile.TemporaryDirectory(dir=path.parent) as directory:
		output = pathlib.Path(directory) / 'repaired.zip'
		with zipfile.ZipFile(path) as old, zipfile.ZipFile(output, 'w') as new:
			for name in old.namelist():
				payload = old.read(name)
				if name == 'canvas_qti12_questions/canvas_qti12_questions.xml':
					root = xml.fromstring(payload)
					nodes = named(root, 'item')
					if len(nodes) != len(items):
						raise ValueError('Canvas repair item count differs from source')
					for item, node in zip(items, nodes):
						if structure(node) != structure(write_item.MULTI_FIB(item), qualify=True):
							raise ValueError('Canvas repair item differs from pinned source rendering')
						seen_items += 1
						labels = {response.get('ident'): {label.get('ident'): ''.join(label.itertext()).strip() for label in named(response, 'response_label')} for response in named(node, 'response_lid')}
						for predicate in named(node, 'varequal'):
							choices = labels.get(predicate.get('respident'), {})
							if predicate.text in choices:
								continue
							matches = [key for key, text in choices.items() if text == predicate.text]
							if len(matches) != 1:
								raise ValueError('Canvas repair answer has no unique declared choice')
							repairs.append({'response': predicate.get('respident'), 'answer': predicate.text, 'identifier': matches[0]})
							predicate.text = matches[0]
					payload = serializer.tostring(root)
				new.writestr(name, payload)
		if seen_items != len(items) or not repairs:
			raise failure
		value = projector(output)
	receipt = {'source': binding, 'raw_zip_sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'items': seen_items, 'repairs': repairs}
	(path.parent / 'canvas_multifib_source_repair_receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
	return value
