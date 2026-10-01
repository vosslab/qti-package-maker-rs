"""Preserve authored self-test markup while comparing equivalent line layout."""

import re


def text_value(value):
	value = value or ''
	if '\n' in value and not value.strip():
		return ''
	return re.sub(r'[\t\n\r\f ]+', ' ', value)


def statement_structure(path):
	from lxml import html

	root = html.fromstring(path.read_bytes())
	nodes = root.xpath('//*[@id and starts-with(@id,"statement_text_")] | //*[contains(concat(" ",normalize-space(@class)," ")," qti-statement ")]')
	if len(nodes) != 1:
		raise ValueError('self-test must contain one structured statement')
	tokens = []
	table_containers = {'table', 'tr', 'thead', 'tbody', 'tfoot', 'colgroup', 'ul', 'ol'}

	def add_text(value, parent=None, raw=False):
		value = value or ''
		if not raw:
			value = text_value(value)
		if parent in table_containers and not value.strip(' '):
			return
		if value:
			tokens.append(['text', value])

	def walk(node):
		parent = node.getparent().tag
		if not isinstance(node.tag, str):
			tokens.append(['comment', node.text or ''])
			add_text(node.tail, parent)
			return
		if node.tag == 'input' and 'fib-blank' in node.get('class', '').split():
			name = node.get('name') or node.get('aria-label')
			if not name or list(node) or node.text:
				raise ValueError('self-test blank lacks an authored name or has child content')
			generated = {'name', 'aria-label', 'id', 'class', 'autocomplete', 'data-answers', 'placeholder', 'type'}
			tokens.append(['blank', name, node.get('type', 'text'), sorted((key, value) for key, value in node.attrib.items() if key not in generated)])
			add_text(node.tail, parent)
			return
		attributes = sorted(node.attrib.items())
		layout = node.tag in {'p', 'br'} and not attributes
		if layout:
			tokens.append(['break'])
		else:
			tokens.append(['start', node.tag, attributes])
		add_text(node.text, node.tag, node.tag in {'script', 'style'})
		for child in node:
			walk(child)
		if layout:
			if node.tag == 'p':
				tokens.append(['break'])
		else:
			tokens.append(['end', node.tag])
		add_text(node.tail, parent)

	add_text(nodes[0].text)
	for child in nodes[0]:
		walk(child)
	for i, token in enumerate(tokens):
		if token[0] != 'text':
			continue
		if i == 0 or tokens[i - 1] == ['break']:
			token[1] = token[1].lstrip(' ')
		if i == len(tokens) - 1 or tokens[i + 1] == ['break']:
			token[1] = token[1].rstrip(' ')
	result = []
	for token in tokens:
		if token == ['text', '']:
			continue
		if token == ['break'] and (not result or result[-1] == token):
			continue
		result.append(token)
	while result and result[-1] == ['break']:
		result.pop()
	return result


def selftest():
	import pathlib
	import tempfile

	with tempfile.TemporaryDirectory() as directory:
		path = pathlib.Path(directory) / 'statement.html'
		def project(value):
			path.write_text('<div class="qti-statement">' + value + '</div>')
			return statement_structure(path)
		assert project('<p>First<br>Second</p>') == project('<p>First</p><p>Second</p>')
		assert project('<strong>word</strong> next') != project('<strong>word</strong>next')
		assert project('<strong>word</strong>\u2009next') != project('<strong>word</strong> next')
		assert project('<i>Note</i>') != project('<span>Note</span>')
		assert project('<p>Text</p>') != project('<p style="color:red">Text</p>')
		assert project('<script src="one.js">x()</script>') != project('<script src="two.js">x()</script>')
		assert project('<script>x()</script>') != project('<script>y()</script>')
		assert project('<script>x()</script><strong>Text</strong>') != project('<strong>Text</strong><script>x()</script>')
		assert project('<table style="border:1px"><tr><td>A</td></tr></table>') != project('<table style="border:2px"><tr><td>A</td></tr></table>')
		assert project('<table><tr><td>A</td><td>B</td></tr></table>') != project('<table><tr><td>B</td><td>A</td></tr></table>')
		assert project('<input class="fib-blank" name="gene">') == project('<input class="qti-input fib-blank" aria-label="gene" type="text">')
		assert project('<input class="fib-blank" name="gene">') != project('<input class="fib-blank" name="other">')
		assert project('<input class="fib-blank" name="gene">') != project('<input class="fib-blank" name="gene" type="hidden">')
		assert project('<input class="fib-blank" name="gene">') != project('<input class="fib-blank" name="gene" required>')
