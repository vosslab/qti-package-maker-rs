"""Bounded browser-implied paragraph closure for QTI HTML."""


def normalize(parent: object) -> None:
	from xtask.support.parity_qti21 import qti, collapse

	for child in list(parent):
		normalize(child)
	result = []
	for child in list(parent):
		if not qti(child, 'p'):
			result.append(child)
			continue
		children = list(child)
		first = next((i for i, node in enumerate(children) if any(qti(node, name) for name in ('table', 'ul', 'ol', 'div'))), None)
		if first is None:
			if child.attrib or children or collapse(child.text or ''):
				result.append(child)
			elif child.text or child.tail:
				spacing = (child.text or '') + (child.tail or '')
				if result:
					result[-1].tail = (result[-1].tail or '') + spacing
				else:
					parent.text = (parent.text or '') + spacing
			continue
		tail = child.tail
		child.tail = None
		promoted = children[first:]
		child[:] = children[:first]
		if child.attrib or list(child) or collapse(child.text or ''):
			result.append(child)
		elif child.text:
			if result:
				result[-1].tail = (result[-1].tail or '') + child.text
			else:
				parent.text = (parent.text or '') + child.text
		result.extend(promoted)
		result[-1].tail = (result[-1].tail or '') + (tail or '')
	parent[:] = result
