"""Compare resolved package images, placement, and authored markup semantically."""

from __future__ import annotations

import base64
import html
import pathlib
import posixpath
import urllib.parse
import zipfile
from html.parser import HTMLParser
from defusedxml import ElementTree as element_tree
from xml.etree import ElementTree as xml_tree


REFERENCE_LIST_ATTRIBUTES = frozenset({
	"aria-describedby",
	"aria-labelledby",
	"aria-controls",
	"aria-owns",
	"headers",
})
REFERENCE_SINGLE_ATTRIBUTES = frozenset({"aria-activedescendant", "for", "form", "list"})


def raw_attributes(attrs: list[tuple[str, str | None]], *, image: bool) -> list[tuple[str, str]]:
	"""Preserve authored attributes, excluding only image source names."""
	return sorted(
		(name.lower(), value or "")
		for name, value in attrs
		if not (image and name.lower() == "src")
	)


def identifier_graph(tokens: list[dict[str, object]]) -> dict[str, str]:
	"""Build an injective normalization only for local identifier values."""
	identifiers: list[str] = []
	for token in tokens:
		for name, value in token.get("attributes", []):
			if name == "id":
				identifiers.append(value)
	if any(not identifier for identifier in identifiers) or len(set(identifiers)) != len(identifiers):
		raise ValueError("markup has an empty or duplicate identifier")
	return {identifier: f"id_{index:03}" for index, identifier in enumerate(identifiers, start=1)}


def normalize_reference(value: str, graph: dict[str, str], *, multiple: bool, fragment: bool = False) -> str:
	if fragment:
		if not value.startswith("#"):
			return value
		identifier = value[1:]
		if identifier not in graph:
			raise ValueError(f"markup references undeclared identifier: {identifier}")
		return f"#{graph[identifier]}"
	identifiers = value.split() if multiple else [value]
	if any(identifier not in graph for identifier in identifiers):
		raise ValueError(f"markup references undeclared identifier: {value}")
	return " ".join(graph[identifier] for identifier in identifiers)


def normalized_attributes(attrs: list[tuple[str, str]], graph: dict[str, str]) -> list[tuple[str, str]]:
	result = []
	for name, value in attrs:
		if name == "id":
			result.append((name, normalize_reference(value, graph, multiple=False)))
		elif name in REFERENCE_LIST_ATTRIBUTES:
			result.append((name, normalize_reference(value, graph, multiple=True)))
		elif name in REFERENCE_SINGLE_ATTRIBUTES:
			result.append((name, normalize_reference(value, graph, multiple=False)))
		elif name == "href":
			result.append((name, normalize_reference(value, graph, multiple=False, fragment=True)))
		else:
			result.append((name, value))
	return result


def resolve(member: str, source: str, names: set[str]) -> str:
	if not source or source.startswith("data:"):
		raise ValueError(f"image source is not a package member: {source}")
	source = urllib.parse.unquote(source)
	prefix = "@X@EmbeddedFile.requestUrlStub@X@bbcswebdav/"
	if source.startswith(prefix):
		identifier = source.removeprefix(prefix)
		candidates = [name for name in names if name.startswith(f"csfiles/home_dir/__{identifier}") and not name.endswith(".xml")]
		if len(candidates) != 1:
			raise ValueError(f"Blackboard image source is unresolved or ambiguous: {source}")
		return candidates[0]
	resolved = posixpath.normpath(posixpath.join(posixpath.dirname(member), source))
	if resolved.startswith("../") or resolved not in names:
		raise ValueError(f"image source is unresolved: {source}")
	return resolved


def local(node: element_tree.Element) -> str:
	return node.tag.rsplit("}", 1)[-1].lower()


def qualified(value: str) -> str:
	"""Retain namespace identity in HTML-safe canonical names."""
	if not value.startswith("{"):
		return value
	uri, name = value[1:].split("}", maxsplit=1)
	if uri == "http://www.w3.org/XML/1998/namespace":
		return f"xml:{name}"
	return f"ns_{uri.encode().hex()}:{name}"


QTI_INTERACTIONS = frozenset({"choiceinteraction", "matchinteraction", "textentryinteraction", "orderinteraction"})
QTI_CHOICES = frozenset({"simplechoice", "simpleassociablechoice"})


def xml_escape(value: str, *, attribute: bool = False) -> str:
	return html.escape(value, quote=attribute)


def fragment(node: element_tree.Element, *, wrapper: bool) -> str:
	"""Serialize visible XML as HTML without namespace or scoring wrappers."""
	if local(node) in {"script", "style"}:
		payload = base64.b64encode((node.text or "").encode("utf-8")).decode("ascii")
		attributes = "".join(f' {qualified(name)}="{xml_escape(value, attribute=True)}"' for name, value in node.attrib.items())
		return f'<parity-raw data-kind="{local(node)}" data-payload="{payload}"{attributes}></parity-raw>'
	content = xml_escape(node.text or "")
	for child in node:
		content += fragment(child, wrapper=True)
		content += xml_escape(child.tail or "")
	if not wrapper:
		return content
	attributes = "".join(f' {qualified(name)}="{xml_escape(value, attribute=True)}"' for name, value in node.attrib.items())
	name = qualified(node.tag)
	return f"<{name}{attributes}>{content}</{name}>"


def slot(owner: str, content: str) -> str:
	return f'<parity-slot data-owner="{owner}">{content}</parity-slot>'


def qti2_markup(item_body: element_tree.Element) -> list[str]:
	"""Extract authored prompt and choice slots, not QTI response-processing XML."""
	entries = []
	prompt = xml_escape(item_body.text or "")
	for child in item_body:
		if local(child) in QTI_INTERACTIONS:
			prompt += xml_escape(child.tail or "")
			continue
		prompt += fragment(child, wrapper=True)
		prompt += xml_escape(child.tail or "")
	if prompt.strip():
		entries.append(slot("itembody/prompt", prompt))
	for interaction_index, interaction in enumerate((node for node in item_body.iter() if local(node) in QTI_INTERACTIONS), start=1):
		for choice_index, choice in enumerate((node for node in interaction.iter() if local(node) in QTI_CHOICES), start=1):
			entries.append(slot(f"itembody/interaction[{interaction_index}]/choice[{choice_index}]", fragment(choice, wrapper=False)))
	return entries


def semantic_markup(member: str, payload: bytes) -> list[tuple[str, bool]]:
	"""Extract authored presentation HTML, excluding generated XML scoring wrappers."""
	if member.endswith(".html"):
		return [(payload.decode("utf-8"), False)]
	root = element_tree.fromstring(payload)
	item_bodies = [node for node in root.iter() if local(node) == "itembody"]
	if item_bodies:
		return [(markup, False) for owner in item_bodies for markup in qti2_markup(owner)]
	presentations = [node for node in root.iter() if local(node) == "presentation"]
	return [(xml_tree.tostring(material, encoding="unicode"), True) for owner in presentations for material in owner.iter() if local(material) == "mat_formattedtext"]


class Placement(HTMLParser):
	"""Keep image order relative to visible markup and its identifier graph."""

	def __init__(self) -> None:
		super().__init__(convert_charrefs=True)
		self.tokens: list[dict[str, object]] = []
		self.images: list[dict[str, object]] = []

	def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
		name = tag.lower()
		if name != "img" and not name.endswith(":img"):
			self.tokens.append({"start": name, "attributes": raw_attributes(attrs, image=False)})
			return
		values = {name.lower(): value or "" for name, value in attrs}
		if not values.get("src") or "alt" not in values:
			raise ValueError("image lacks src or alt")
		image_attributes = raw_attributes(attrs, image=True)
		self.tokens.append({"image": len(self.images), "attributes": image_attributes})
		self.images.append({"alt": values["alt"], "attributes": image_attributes, "source": values["src"]})

	def handle_endtag(self, tag: str) -> None:
		name = tag.lower()
		if name != "img" and not name.endswith(":img"):
			self.tokens.append({"end": name})

	def handle_data(self, value: str) -> None:
		value = " ".join(value.split())
		if value:
			self.tokens.append({"text": value})

	def normalized(self) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
		graph = identifier_graph(self.tokens)
		for token in self.tokens:
			token["attributes"] = normalized_attributes(token.get("attributes", []), graph)
		for image in self.images:
			image["attributes"] = normalized_attributes(image["attributes"], graph)
		return self.tokens, self.images


def contract(path: pathlib.Path) -> list[dict[str, object]]:
	with zipfile.ZipFile(path) as archive:
		names = set(archive.namelist())
		result = []
		document_index = 0
		for member in sorted(name for name in names if name.endswith((".html", ".xml", ".dat"))):
			for markup, decode_mattext in semantic_markup(member, archive.read(member)):
				content = html.unescape(markup) if decode_mattext else markup
				parser = Placement()
				parser.feed(content)
				if parser.images:
					placement, images = parser.normalized()
					for image in images:
						image["resolved"] = bool(resolve(member, image.pop("source"), names))
					result.append({"document": document_index, "occurrences": images, "placement": placement})
				document_index += 1
		return result


def resolution_provenance(path: pathlib.Path) -> list[dict[str, object]]:
	"""Record resolved member names for audit without making generated names equality keys."""
	with zipfile.ZipFile(path) as archive:
		names = set(archive.namelist())
		result = []
		for member in sorted(name for name in names if name.endswith((".html", ".xml", ".dat"))):
			for markup, decode_mattext in semantic_markup(member, archive.read(member)):
				content = html.unescape(markup) if decode_mattext else markup
				parser = Placement()
				parser.feed(content)
				for index, image in enumerate(parser.images):
					source = image["source"]
					result.append({"document_member": member, "position": index, "source": source, "resolved_member": resolve(member, source, names)})
		return result

