"""Compare Blackboard HTML through its browser-implied paragraph structure."""

import html


def canonical(tokens: list[dict]) -> list:
	from lxml import html as dom
	from xtask.support.parity_qti21 import QTI21
	from xtask.support.parity_qti21_dom import normalize

	parts = []
	for token in tokens:
		if 'start' in token:
			attributes = ''.join(' ' + key + '="' + html.escape(value, quote=True) + '"' for key, value in token['attributes'])
			parts.append('<' + token['start'] + attributes + '>')
		elif 'end' in token:
			parts.append('</' + token['end'] + '>')
		elif 'text' in token:
			parts.append(html.escape(token['text']))
	root = dom.fragment_fromstring(''.join(parts), create_parent='div')
	for node in root.iter():
		node.tag = '{' + QTI21 + '}' + node.tag
	normalize(root)

	def tree(node: object) -> list:
		return [node.tag, sorted(node.attrib.items()), node.text, [tree(child) for child in node], node.tail]
	return tree(root)


def selftest() -> None:
	from xtask.support.parity_blackboard import fragment

	base = '<p>Question</p><table><tr><td>A</td><td>B</td></tr></table><img src="figure.png" alt="Tree"><script>x()</script>'
	expected = fragment(base)
	for changed in (
		base.replace('Question', 'Other'),
		base.replace('<td>A</td><td>B</td>', '<td>B</td><td>A</td>'),
		base.replace('figure.png', 'other.png'),
		base.replace('alt="Tree"', 'alt="Other"'),
		base.replace('<p>', '<p style="color:red">'),
		base.replace('x()', 'y()'),
		base.replace('<script>x()</script>', '').replace('<table>', '<script>x()</script><table>'),
	):
		assert fragment(changed) != expected
	assert fragment('<p><table><tr><td>A</td></tr></table></p>') == fragment('<table><tr><td>A</td></tr></table>')
