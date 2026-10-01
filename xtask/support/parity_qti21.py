"""Strict semantic projection for Blackboard QTI 2.1 packages.

This is deliberately separate from the QTI 1.2 projection in parity_oracle.py.
It compares rendered meaning and grading structure, never generated identifiers or XML
pretty-printing.  Unsupported QTI constructs are errors: a projection must not turn an
unknown interaction or scoring rule into an empty result.
"""

from __future__ import annotations

from decimal import Decimal, InvalidOperation
import copy
import hashlib
import json
import pathlib
import tempfile
import zipfile

from defusedxml import ElementTree as element_tree


QTI21 = "http://www.imsglobal.org/xsd/imsqti_v2p1"
QTI_TEMPLATES = {
    "http://www.imsglobal.org/question/qti_v2p1/rptemplates/map_response",
    "http://www.imsglobal.org/question/qti_v2p1/rptemplates/match_correct",
}
BLOCKS = {"p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr", "td", "th"}
INTERACTIONS = {
    "choiceinteraction", "matchinteraction", "orderinteraction", "textentryinteraction",
    "extendedtextinteraction", "hottextinteraction", "inlinechoiceinteraction",
    "associateinteraction", "gapmatchinteraction", "graphicgapmatchinteraction",
    "graphicorderinteraction", "sliderinteraction", "uploadinteraction",
}
SUPPORTED_INTERACTIONS = {"choiceinteraction", "matchinteraction", "orderinteraction", "textentryinteraction"}


def local(node: element_tree.Element) -> str:
    return node.tag.rsplit("}", 1)[-1]


def qti(node: element_tree.Element, expected: str) -> bool:
    return node.tag == f"{{{QTI21}}}{expected}"


def children(node: element_tree.Element, expected: str) -> list[element_tree.Element]:
    return [child for child in node if qti(child, expected)]


def one(node: element_tree.Element, expected: str, *, required: bool = True) -> element_tree.Element | None:
    found = children(node, expected)
    if len(found) == 1:
        return found[0]
    if not found and not required:
        return None
    raise ValueError(f"{local(node)} requires exactly one QTI {expected}, found {len(found)}")


def only_qti_children(node: element_tree.Element, allowed: set[str]) -> None:
    for child in node:
        if not child.tag.startswith(f"{{{QTI21}}}") or local(child) not in allowed:
            raise ValueError(f"unsupported child QName {child.tag!r} in {local(node)}")


def collapse(value: str) -> str:
    return " ".join(value.split())


def finite_decimal(value: str) -> str:
    """Return a canonical finite QTI decimal without changing string-valued responses."""
    try:
        decimal = Decimal(value.strip())
    except InvalidOperation as error:
        raise ValueError(f"invalid QTI float value {value!r}") from error
    if not decimal.is_finite():
        raise ValueError(f"QTI float value must be finite: {value!r}")
    if decimal.is_zero():
        return "0"
    return format(decimal.normalize(), "f")


def finite_tolerance(value: str) -> str:
    values = value.split()
    if not values:
        raise ValueError("QTI tolerance must contain finite decimal values")
    return " ".join(finite_decimal(part) for part in values)


def visible_fragment(node: element_tree.Element) -> str:
    """Return visible HTML text with explicit block and line-break separators."""
    parts: list[str] = []

    def add(value: str | None) -> None:
        # XML pretty-print indentation is not authored inline spacing.
        if value and (value.strip() or "\n" not in value):
            parts.append(value)

    def visit(current: element_tree.Element) -> None:
        name = local(current).lower()
        if name in {"script", "style"}:
            return
        if name in BLOCKS and parts and not parts[-1].endswith("\n"):
            parts.append("\n")
        add(current.text)
        for child in current:
            if local(child).lower() == "br":
                parts.append("\n")
            else:
                visit(child)
            add(child.tail)
        if name in BLOCKS:
            parts.append("\n")

    visit(node)
    return "\n".join(collapse(line) for line in "".join(parts).splitlines() if collapse(line))


def visible_markup(node: element_tree.Element) -> list[object]:
    """Canonical visible DOM tokens, retaining element names and presentation attributes."""
    tokens: list[object] = []
    def visit(current: element_tree.Element) -> None:
        name = local(current).lower()
        if name in {"script", "style"}:
            return
        if qti(current, "p") and not current.attrib and not collapse(current.text or "") and (
            not list(current) or any(qti(current[0], block) for block in {"ul", "ol", "table", "div"})
        ):
            # HTML closes empty paragraphs before block content; XML may nest it.
            # Keep all child content and tails, and preserve attributed paragraphs.
            for child in current:
                visit(child)
                if tail := collapse(child.tail or ""):
                    tokens.append(["text", tail])
            return
        if name in {"simplechoice", "simpleassociablechoice"} and not visible_fragment(current):
            tokens.append(["visual_choice", current.tag, visual_choice_identity(current)])
            return
        if name in BLOCKS:
            tokens.append(["boundary", "block"])
        if name == "br":
            tokens.append(["boundary", "br"])
            return
        tokens.append(["start", name, sorted((key, value) for key, value in current.attrib.items() if key != "identifier")])
        if text := collapse(current.text or ""):
            tokens.append(["text", text])
        for child in current:
            visit(child)
            if tail := collapse(child.tail or ""):
                tokens.append(["text", tail])
        tokens.append(["end", name])
        if name in BLOCKS:
            tokens.append(["boundary", "block"])
    visit(node)
    return tokens


def payload_blocks(node: element_tree.Element, tag: str) -> list[dict[str, object]]:
    blocks = []
    def visit(current: element_tree.Element, path: tuple[int, ...]) -> None:
        if local(current).lower() == tag:
            blocks.append({"path": list(path), "attributes": sorted(current.attrib.items()), "text": current.text or ""})
        for index, child in enumerate(current):
            visit(child, path + (index,))
    visit(node, ())
    return blocks


def require_identifier(node: element_tree.Element) -> str:
    identifier = node.attrib.get("identifier")
    if not identifier:
        raise ValueError(f"QTI {local(node)} lacks identifier")
    return identifier


def direct_interactions(body: element_tree.Element) -> list[element_tree.Element]:
    found = []
    for candidate in body.iter():
        name = local(candidate).lower()
        if not qti(candidate, local(candidate)) and (
            name.endswith("interaction") or name in INTERACTIONS
        ):
            raise ValueError(f"foreign-namespace QTI interaction {candidate.tag!r}")
        if qti(candidate, local(candidate)) and name.endswith("interaction") and name not in INTERACTIONS:
            raise ValueError(f"unsupported QTI 2.1 interaction {name}")
        if name in INTERACTIONS:
            if name not in SUPPORTED_INTERACTIONS:
                raise ValueError(f"unsupported QTI 2.1 interaction {name}")
            found.append(candidate)
    return found


def question_fragment(body: element_tree.Element) -> str:
    parts = [collapse(body.text or "")]
    for child in body:
        if local(child).lower() not in INTERACTIONS:
            text = visible_fragment(child)
            if text:
                parts.append(text)
        else:
            for prompt in children(child, "prompt"):
                text = visible_fragment(prompt)
                if text:
                    parts.append(text)
        tail = collapse(child.tail or "")
        if tail:
            parts.append(tail)
    return "\n".join(part for part in parts if part)



def visual_choice_identity(node: element_tree.Element) -> str:
    """Identify nontext diagrams without relying on generated outer identifiers."""
    def tree(current: element_tree.Element, outer: bool = False) -> list[object]:
        attributes = sorted((key, value) for key, value in current.attrib.items()
                            if not (outer and key == "identifier"))
        contents = []
        for child in current:
            contents.extend(tree(child))
            if tail := collapse(child.tail or ""):
                contents.append(["tail", tail])
        # Empty, unstyled paragraph wrappers carry no authored diagram content.
        # Flatten only in this nontext-choice identity, retaining all other nodes.
        if qti(current, "p") and not attributes and not collapse(current.text or ""):
            return contents
        return [[current.tag, attributes, collapse(current.text or ""), contents]]
    if not any(local(candidate).lower() in {"table", "svg", "img", "canvas"}
               for candidate in node.iter()):
        raise ValueError("nontext QTI choice has no supported visual content")
    payload = json.dumps(tree(node, True), sort_keys=True).encode()
    return "visual:" + hashlib.sha256(payload).hexdigest()


def choice_records(interaction: element_tree.Element) -> list[dict[str, str]]:
    records = []
    for choice in interaction.iter():
        if local(choice).lower() != "simplechoice":
            continue
        if not qti(choice, "simpleChoice"):
            raise ValueError(f"foreign-namespace simpleChoice {choice.tag!r}")
        identifier = require_identifier(choice)
        visible = visible_fragment(choice)
        if not visible:
            visible = visual_choice_identity(choice)
        records.append({
            "identifier": identifier,
            "visible": visible,
            "markup": visible_markup(choice),
            "fixed": choice.attrib.get("fixed", "false"),
        })
    # Distinct authored trees can have identical leaf labels but different edges.
    repeated = {record["visible"] for record in records
                if sum(other["visible"] == record["visible"] for other in records) > 1}
    nodes = {require_identifier(choice): choice for choice in interaction.iter()
             if qti(choice, "simpleChoice")}
    for record in records:
        if record["visible"] in repeated:
            record["visible"] = visual_choice_identity(nodes[record["identifier"]])
    identifiers = [record["identifier"] for record in records]
    visible = [record["visible"] for record in records]
    if len(identifiers) != len(set(identifiers)):
        raise ValueError("simpleChoice identifiers are not injective")
    if len(visible) != len(set(visible)):
        raise ValueError("simpleChoice visible values are not injective for semantic dereference")
    return records


def associable_records(interaction: element_tree.Element) -> dict[str, dict[str, str]]:
    records: dict[str, dict[str, str]] = {}
    for candidate in interaction.iter():
        if local(candidate).lower() != "simpleassociablechoice":
            continue
        if not qti(candidate, "simpleAssociableChoice"):
            raise ValueError(f"foreign-namespace simpleAssociableChoice {candidate.tag!r}")
        identifier = require_identifier(candidate)
        value = visible_fragment(candidate)
        if not value:
            value = visual_choice_identity(candidate)
        if identifier in records:
            raise ValueError(f"duplicate simpleAssociableChoice {identifier}")
        records[identifier] = {"visible": value, "fixed": candidate.attrib.get("fixed", "false")}
    if not records:
        raise ValueError("matchInteraction has no simpleAssociableChoice")
    for group in children(interaction, "simpleMatchSet"):
        nodes = children(group, "simpleAssociableChoice")
        values = [records[require_identifier(node)]["visible"] for node in nodes]
        for node in nodes:
            identifier = require_identifier(node)
            if values.count(records[identifier]["visible"]) > 1:
                records[identifier]["visible"] = visual_choice_identity(node)
        resolved = [records[require_identifier(node)]["visible"] for node in nodes]
        if len(resolved) != len(set(resolved)):
            raise ValueError("simpleMatchSet values are ambiguous for semantic dereference")
    return records


def interaction_projection(body: element_tree.Element) -> tuple[list[dict[str, object]], dict[str, dict[str, str]], list[str]]:
    projections = []
    choice_ids: dict[str, dict[str, str]] = {}
    all_choices: list[str] = []
    for interaction in direct_interactions(body):
        name = local(interaction).lower()
        response_id = interaction.attrib.get("responseIdentifier")
        if not response_id:
            raise ValueError(f"{name} lacks responseIdentifier")
        base = {
            "kind": name,
            "response_identifier": response_id,
            "shuffle": interaction.attrib.get("shuffle", "false"),
        }
        if name == "choiceinteraction":
            records = choice_records(interaction)
            if not records:
                raise ValueError("choiceInteraction has no simpleChoice")
            base["max_choices"] = interaction.attrib.get("maxChoices", "1")
            base["choices"] = records
            choice_ids[response_id] = {record["identifier"]: record["visible"] for record in records}
            all_choices.extend(record["visible"] for record in records)
        elif name == "orderinteraction":
            records = choice_records(interaction)
            if not records:
                raise ValueError("orderInteraction has no simpleChoice")
            base["choices"] = records
            choice_ids[response_id] = {record["identifier"]: record["visible"] for record in records}
            all_choices.extend(record["visible"] for record in records)
        elif name == "matchinteraction":
            records = associable_records(interaction)
            base["max_associations"] = interaction.attrib.get("maxAssociations", "")
            base["choices"] = [
                {"identifier": identifier, **record}
                for identifier, record in records.items()
            ]
            choice_ids[response_id] = {identifier: record["visible"] for identifier, record in records.items()}
        else:
            if list(interaction):
                raise ValueError("textEntryInteraction must not carry child markup")
        projections.append(base)
    return projections, choice_ids, all_choices


def resolve_value(value: str, base_type: str, response_id: str, identifiers: dict[str, str]) -> object:
    if base_type == "identifier":
        if value not in identifiers:
            raise ValueError(f"unresolved QTI choice identifier {response_id}:{value}")
        return identifiers[value]
    if base_type == "directedPair":
        pair = value.split()
        if len(pair) != 2 or any(part not in identifiers for part in pair):
            raise ValueError(f"unresolved QTI directedPair {response_id}:{value}")
        return [identifiers[pair[0]], identifiers[pair[1]]]
    return value


def response_projection(item: element_tree.Element, choice_ids: dict[str, dict[str, str]]) -> tuple[list[dict[str, object]], list[object]]:
    responses = []
    answers = []
    for declaration in children(item, "responseDeclaration"):
        identifier = require_identifier(declaration)
        base_type = declaration.attrib.get("baseType")
        cardinality = declaration.attrib.get("cardinality")
        if base_type not in {"identifier", "directedPair", "float", "string"} or cardinality not in {"single", "multiple", "ordered"}:
            raise ValueError(f"unsupported responseDeclaration {identifier}:{base_type}/{cardinality}")
        correct = one(declaration, "correctResponse")
        only_qti_children(declaration, {"correctResponse", "mapping"})
        if correct is not None:
            only_qti_children(correct, {"value"})
        values = []
        for value in [] if correct is None else children(correct, "value"):
            if value.attrib or list(value):
                raise ValueError("correctResponse value must have no attributes or child elements")
            values.append(value.text or "")
        if not values:
            raise ValueError(f"responseDeclaration {identifier} has no correctResponse value")
        resolved = [
            resolve_value(
                finite_decimal(value) if base_type == "float" else value,
                base_type,
                identifier,
                choice_ids.get(identifier, {}),
            )
            for value in values
        ]
        mapping = one(declaration, "mapping", required=False)
        map_entries = []
        mapping_attributes: list[tuple[str, str]] = []
        if mapping is not None:
            only_qti_children(mapping, {"mapEntry"})
            unsupported = set(mapping.attrib) - {"defaultValue", "lowerBound", "upperBound"}
            if unsupported:
                raise ValueError(f"unsupported mapping attributes {sorted(unsupported)}")
            mapping_attributes = sorted(mapping.attrib.items())
            for entry in children(mapping, "mapEntry"):
                key = entry.attrib.get("mapKey")
                score = entry.attrib.get("mappedValue")
                if key is None or score is None:
                    raise ValueError(f"mapping entry for {identifier} is incomplete")
                unsupported_entry = set(entry.attrib) - {"mapKey", "mappedValue", "caseSensitive"}
                if unsupported_entry:
                    raise ValueError(f"unsupported mapEntry attributes {sorted(unsupported_entry)}")
                map_entries.append({
                    "key": resolve_value(key, base_type, identifier, choice_ids.get(identifier, {})),
                    "mapped_value": score,
                    "case_sensitive": entry.attrib.get("caseSensitive", "true"),
                })
        response = {
            "identifier": identifier,
            "base_type": base_type,
            "cardinality": cardinality,
            "correct": resolved,
            "mapping_attributes": mapping_attributes,
            "mapping": map_entries,
        }
        responses.append(response)
        answers.extend(resolved)
    if not responses:
        raise ValueError("assessmentItem has no responseDeclaration")
    return responses, answers


def value_ast(node: element_tree.Element) -> object:
    name = local(node)
    if not node.tag.startswith(f"{{{QTI21}}}"):
        raise ValueError(f"foreign-namespace QTI value expression {node.tag!r}")
    if name == "variable" or name == "correct":
        identifier = node.attrib.get("identifier")
        if not identifier or node.attrib.keys() != {"identifier"}:
            raise ValueError(f"{name} must have exactly identifier")
        return [name, identifier]
    if name == "baseValue":
        base_type = node.attrib.get("baseType")
        if set(node.attrib) != {"baseType"} or list(node):
            raise ValueError("baseValue must contain only its baseType and text")
        value = node.text or ""
        if not base_type or not value.strip():
            raise ValueError("baseValue is incomplete")
        if base_type == "float":
            value = finite_decimal(value)
        return [name, base_type, value]
    raise ValueError(f"unsupported QTI 2.1 value expression {name}")


def predicate_ast(node: element_tree.Element) -> object:
    name = local(node)
    if not node.tag.startswith(f"{{{QTI21}}}"):
        raise ValueError(f"foreign-namespace QTI predicate {node.tag!r}")
    if name == "or":
        if node.attrib or len(node) < 2 or collapse(node.text or "") or any(collapse(child.tail or "") for child in node):
            raise ValueError("QTI or requires at least two predicates and no extra attributes or text")
        return [name, [predicate_ast(child) for child in node]]
    if name in {"match", "equal"}:
        values = [value_ast(child) for child in node]
        if len(values) != 2:
            raise ValueError(f"{name} requires exactly two operands")
        attributes = sorted(
            (key, finite_tolerance(value) if key == "tolerance" else value)
            for key, value in node.attrib.items()
        )
        return [name, attributes, values]
    raise ValueError(f"unsupported QTI 2.1 predicate {name}")


def action_ast(node: element_tree.Element) -> object:
    if not qti(node, "setOutcomeValue") or node.attrib.get("identifier") != "SCORE":
        raise ValueError(f"unsupported QTI 2.1 scoring action {local(node)}")
    if set(node.attrib) != {"identifier"}:
        raise ValueError("setOutcomeValue carries unsupported attributes")
    value = one(node, "baseValue", required=False)
    if value is not None:
        only_qti_children(node, {"baseValue"})
        return ["setOutcomeValue", value_ast(value)]
    total = one(node, "sum", required=False)
    if total is None:
        raise ValueError("setOutcomeValue requires baseValue or SCORE accumulation")
    only_qti_children(node, {"sum"})
    if total.attrib or len(total) != 2:
        raise ValueError("SCORE accumulation sum must have exactly two operands")
    score, contribution = list(total)
    if value_ast(score) != ["variable", "SCORE"]:
        raise ValueError("SCORE accumulation must begin with variable SCORE")
    contribution_ast = value_ast(contribution)
    if contribution_ast[0:2] != ["baseValue", "float"]:
        raise ValueError("SCORE accumulation contribution must be a float baseValue")
    return ["setOutcomeValue", ["sum", ["variable", "SCORE"], contribution_ast]]


def score_program(item: element_tree.Element) -> object:
    processing = one(item, "responseProcessing")
    if processing is None:
        raise ValueError("assessmentItem has no responseProcessing")
    template = processing.attrib.get("template")
    if template:
        if set(processing.attrib) != {"template"} or list(processing):
            raise ValueError("responseProcessing template cannot have children or extra attributes")
        if template not in QTI_TEMPLATES:
            raise ValueError(f"unsupported QTI 2.1 responseProcessing template {template!r}")
        return ["template", template]
    if processing.attrib:
        raise ValueError("responseProcessing has unsupported attributes")
    only_qti_children(processing, {"responseCondition"})
    rules = []
    for condition in children(processing, "responseCondition"):
        if set(condition.attrib):
            raise ValueError("responseCondition has unsupported attributes")
        only_qti_children(condition, {"responseIf", "responseElseIf", "responseElse"})
        branches = []
        for branch in condition:
            name = local(branch)
            if name in {"responseIf", "responseElseIf"}:
                predicate = next(iter(branch), None)
                if predicate is None:
                    raise ValueError(f"{name} lacks a predicate")
                actions = [action_ast(action) for action in list(branch)[1:]]
                branches.append([name, predicate_ast(predicate), actions])
            elif name == "responseElse":
                branches.append([name, [action_ast(action) for action in branch]])
            else:
                raise ValueError(f"unsupported QTI 2.1 responseCondition child {name}")
        if not branches or branches[0][0] != "responseIf":
            raise ValueError("responseCondition must begin with responseIf")
        rules.append(branches)
    if not rules:
        raise ValueError("responseProcessing has no template or responseCondition")
    return ["rules", rules]


def outcome_projection(item: element_tree.Element) -> dict[str, str]:
    score = [declaration for declaration in children(item, "outcomeDeclaration") if declaration.attrib.get("identifier") == "SCORE"]
    if len(score) != 1:
        raise ValueError(f"assessmentItem requires one SCORE outcomeDeclaration, found {len(score)}")
    declaration = score[0]
    if declaration.attrib.get("baseType") != "float" or declaration.attrib.get("cardinality") != "single":
        raise ValueError("SCORE outcomeDeclaration must be float/single")
    only_qti_children(declaration, {"defaultValue"})
    default = one(declaration, "defaultValue", required=False)
    default_value = "0"
    if default is not None:
        if default.attrib:
            raise ValueError("SCORE defaultValue carries unsupported attributes")
        only_qti_children(default, {"value"})
        value = one(default, "value")
        if value is None or value.attrib or list(value) or not (value.text or "").strip():
            raise ValueError("SCORE defaultValue must contain one plain value")
        default_value = value.text or ""
    return {
        "identifier": "SCORE",
        "base_type": "float",
        "cardinality": "single",
        "attributes": sorted(declaration.attrib.items()),
        "default_value": default_value,
    }


def validate_bindings(interactions: list[dict[str, object]], responses: list[dict[str, object]]) -> None:
    by_interaction = {interaction["response_identifier"]: interaction for interaction in interactions}
    by_response = {response["identifier"]: response for response in responses}
    if len(by_interaction) != len(interactions) or len(by_response) != len(responses):
        raise ValueError("QTI response identifiers are not injective")
    if set(by_interaction) != set(by_response):
        raise ValueError("QTI interactions and responseDeclarations are not a bijection")
    expected = {
        "choiceinteraction": ("identifier", {"single", "multiple"}),
        "orderinteraction": ("identifier", {"ordered"}),
        "matchinteraction": ("directedPair", {"multiple"}),
        "textentryinteraction": (None, {"single"}),
    }
    for identifier, interaction in by_interaction.items():
        response = by_response[identifier]
        base_type, cardinalities = expected[interaction["kind"]]
        if base_type is not None and response["base_type"] != base_type:
            raise ValueError(f"interaction {interaction['kind']} has incompatible {response['base_type']} declaration")
        if response["cardinality"] not in cardinalities:
            raise ValueError(f"interaction {interaction['kind']} has incompatible cardinality")


def validate_score_references(program: object, responses: list[dict[str, object]]) -> None:
    identifiers = {response["identifier"] for response in responses}
    def visit(value: object, allow_score: bool = False) -> None:
        if isinstance(value, list):
            if len(value) == 2 and isinstance(value[0], str) and value[0] in {"variable", "correct"}:
                if value[1] not in identifiers and not (allow_score and value == ["variable", "SCORE"]):
                    raise ValueError(f"scoring rule references unknown response {value[1]!r}")
            if value and value[0] == "sum":
                for child in value[1:]:
                    visit(child, allow_score=True)
                return
            for child in value:
                visit(child, allow_score)
    visit(program)


def has_score_accumulation(program: object) -> bool:
    if isinstance(program, list):
        return bool(program and program[0] == "sum") or any(has_score_accumulation(value) for value in program)
    return False


def assessment_projection(item: element_tree.Element) -> dict[str, object]:
    if not qti(item, "assessmentItem"):
        raise ValueError("QTI 2.1 item root must be assessmentItem in the QTI 2.1 namespace")
    only_qti_children(item, {"responseDeclaration", "outcomeDeclaration", "itemBody", "responseProcessing"})
    from xtask.support.parity_qti21_dom import normalize
    item = copy.deepcopy(item)
    body = one(item, "itemBody")
    normalize(body)
    interactions, choice_ids, choices = interaction_projection(body)
    responses, answers = response_projection(item, choice_ids)
    validate_bindings(interactions, responses)
    program = score_program(item)
    validate_score_references(program, responses)
    score_declaration = outcome_projection(item)
    if has_score_accumulation(program) and score_declaration["default_value"] != "0":
        raise ValueError("SCORE accumulation requires an effective default score of zero")
    return {
        "question": question_fragment(body),
        "question_markup": visible_markup(body),
        "choices": choices,
        "answers": answers,
        "numeric": [],
        "interactions": interactions,
        "responses": responses,
        "score_declaration": score_declaration,
        "score_program": program,
        "scripts": payload_blocks(item, "script"),
        "styles": payload_blocks(item, "style"),
    }


def xml_projection(path: pathlib.Path) -> list[dict[str, object]]:
    """Project every QTI 2.1 assessment item in a ZIP; malformed members fail loudly."""
    result = []
    with zipfile.ZipFile(path) as archive:
        for name in sorted(archive.namelist()):
            if not name.lower().endswith(".xml") or pathlib.PurePosixPath(name).name == "imsmanifest.xml":
                continue
            payload = archive.read(name)
            try:
                root = element_tree.fromstring(payload)
            except element_tree.ParseError as error:
                raise ValueError(f"assessment XML member {name} is malformed: {error}") from error
            if qti(root, "assessmentItem"):
                result.append(assessment_projection(root))
    if not result:
        raise ValueError(f"QTI 2.1 package {path} contains no assessmentItem")
    return result


def _item(question: str = "Stem", answer: str = "a", script: str = "draw()") -> bytes:
    return f'''<assessmentItem xmlns="{QTI21}"><responseDeclaration identifier="R" baseType="identifier" cardinality="single"><correctResponse><value>{answer}</value></correctResponse><mapping><mapEntry mapKey="a" mappedValue="1"/></mapping></responseDeclaration><outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single"/><itemBody><div>{question}<br/>next</div><choiceInteraction responseIdentifier="R" maxChoices="1" shuffle="true"><simpleChoice identifier="a" fixed="true">Alpha<script>{script}</script></simpleChoice><simpleChoice identifier="b" fixed="true">Beta<style>.x{{}}</style></simpleChoice></choiceInteraction></itemBody><responseProcessing><responseCondition><responseIf><match><variable identifier="R"/><correct identifier="R"/></match><setOutcomeValue identifier="SCORE"><baseValue baseType="float">100</baseValue></setOutcomeValue></responseIf><responseElse><setOutcomeValue identifier="SCORE"><baseValue baseType="float">0</baseValue></setOutcomeValue></responseElse></responseCondition></responseProcessing></assessmentItem>'''.encode()


def selftest() -> None:
    diagrams = element_tree.fromstring(f'<matchInteraction xmlns="{QTI21}"><simpleMatchSet><simpleAssociableChoice identifier="p">Prompt</simpleAssociableChoice></simpleMatchSet><simpleMatchSet><simpleAssociableChoice identifier="a"><table style="border:1px"><tr><td>X</td></tr></table></simpleAssociableChoice><simpleAssociableChoice identifier="b"><table style="border:2px"><tr><td>X</td></tr></table></simpleAssociableChoice></simpleMatchSet></matchInteraction>')
    visual_ids = {key: record["visible"] for key, record in associable_records(diagrams).items()}
    assert resolve_value("p a", "directedPair", "R", visual_ids) != resolve_value("p b", "directedPair", "R", visual_ids)
    duplicate = element_tree.fromstring(element_tree.tostring(diagrams).replace(b"border:2px", b"border:1px"))
    try:
        associable_records(duplicate)
    except ValueError:
        pass
    else:
        raise AssertionError("ambiguous MATCH diagrams were accepted")
    with tempfile.TemporaryDirectory() as directory:
        package = pathlib.Path(directory) / "item.zip"
        with zipfile.ZipFile(package, "w") as archive:
            archive.writestr("qti21_items/item.xml", _item())
        projection = xml_projection(package)[0]
        assert projection["question"] == "Stem\nnext"
        assert projection["answers"] == ["Alpha"]
        assert projection["scripts"][0]["text"] == "draw()"
        assert projection["scripts"][0]["path"] != projection["styles"][0]["path"]
        assert projection["styles"][0]["text"] == ".x{}"
        for bad in [_item(question="Wrong"), _item(answer="b"), _item(script="changed")]:
            with zipfile.ZipFile(package, "w") as archive:
                archive.writestr("qti21_items/item.xml", bad)
            assert xml_projection(package)[0] != projection
        visible_boundary = _item().replace(b"Stem<br/>next", b"Stem next")
        with zipfile.ZipFile(package, "w") as archive:
            archive.writestr("qti21_items/item.xml", visible_boundary)
        assert xml_projection(package)[0] != projection
        relocated_script = _item().replace(
            b"Alpha<script>draw()</script></simpleChoice><simpleChoice identifier=\"b\" fixed=\"true\">Beta",
            b"Alpha</simpleChoice><simpleChoice identifier=\"b\" fixed=\"true\">Beta<script>draw()</script>",
        )
        with zipfile.ZipFile(package, "w") as archive:
            archive.writestr("qti21_items/item.xml", relocated_script)
        assert xml_projection(package)[0] != projection
        hidden = _item().replace(b'<simpleChoice identifier="a"', b'<simpleChoice style="display:none" identifier="a"')
        with zipfile.ZipFile(package, "w") as archive:
            archive.writestr("qti21_items/item.xml", hidden)
        assert xml_projection(package)[0] != projection
        orphan = _item().replace(b'<outcomeDeclaration', b'<responseDeclaration identifier="orphan" baseType="string" cardinality="single"><correctResponse><value>x</value></correctResponse></responseDeclaration><outcomeDeclaration')
        with zipfile.ZipFile(package, "w") as archive:
            archive.writestr("qti21_items/item.xml", orphan)
        try:
            xml_projection(package)
        except ValueError:
            pass
        else:
            raise AssertionError("orphan responseDeclaration did not fail")
        foreign_choice = _item().replace(
            b'<simpleChoice identifier="a"',
            b'<x:simpleChoice xmlns:x="urn:evil" identifier="a"',
            1,
        ).replace(b"</simpleChoice>", b"</x:simpleChoice>", 1)
        foreign_interaction = _item().replace(
            b'<choiceInteraction responseIdentifier="R"',
            b'<x:choiceInteraction xmlns:x="urn:evil" responseIdentifier="R"',
            1,
        ).replace(b"</choiceInteraction>", b"</x:choiceInteraction>", 1)
        nested_value = _item().replace(b"<value>a</value>", b"<value><b>a</b></value>", 1)
        for mutation in [
            _item().replace(b'mapKey="a"', b'mapKey="missing"'),
            _item().replace(b'identifier="SCORE"', b'identifier="POINTS"', 1),
            _item().replace(b'setOutcomeValue', b'setOutcomeValue identifier="POINTS"', 1),
            _item().replace(b'<match>', b'<and>', 1),
            foreign_choice,
            foreign_interaction,
            nested_value,
        ]:
            with zipfile.ZipFile(package, "w") as archive:
                archive.writestr("qti21_items/item.xml", mutation)
            try:
                xml_projection(package)
            except ValueError:
                pass
            else:
                raise AssertionError("invalid QTI 2.1 mutation did not fail")
        _all_kind_selftest(package)


def _all_kind_selftest(package: pathlib.Path) -> None:
    """Exercise every writer interaction/declaration/scoring family, not just MC."""
    choice_body = '<choiceInteraction responseIdentifier="R" maxChoices="2" shuffle="true"><simpleChoice identifier="a" fixed="true">A</simpleChoice><simpleChoice identifier="b" fixed="true">B</simpleChoice></choiceInteraction>'
    choice_declaration = '<responseDeclaration identifier="R" baseType="identifier" cardinality="{cardinality}"><correctResponse>{values}</correctResponse></responseDeclaration>'
    rule = '<responseProcessing><responseCondition><responseIf><match><variable identifier="R"/><correct identifier="R"/></match></responseIf></responseCondition></responseProcessing>'
    variants = {
        "mc": (choice_declaration.format(cardinality="single", values="<value>a</value>"), choice_body, rule),
        "ma": (choice_declaration.format(cardinality="multiple", values="<value>a</value><value>b</value>"), choice_body, rule),
        "match": ('<responseDeclaration identifier="R" baseType="directedPair" cardinality="multiple"><correctResponse><value>p c</value></correctResponse><mapping><mapEntry mapKey="p c" mappedValue="1"/></mapping></responseDeclaration>', '<matchInteraction responseIdentifier="R" maxAssociations="1" shuffle="true"><simpleMatchSet><simpleAssociableChoice identifier="p" fixed="true">Prompt</simpleAssociableChoice></simpleMatchSet><simpleMatchSet><simpleAssociableChoice identifier="c" fixed="true">Choice</simpleAssociableChoice></simpleMatchSet></matchInteraction>', '<responseProcessing template="http://www.imsglobal.org/question/qti_v2p1/rptemplates/map_response"/>'),
        "num": ('<responseDeclaration identifier="R" baseType="float" cardinality="single"><correctResponse><value>4</value></correctResponse></responseDeclaration>', '<textEntryInteraction responseIdentifier="R"/>', '<responseProcessing><responseCondition><responseIf><equal tolerance="0.1 0.1" toleranceMode="absolute"><variable identifier="R"/><correct identifier="R"/></equal><setOutcomeValue identifier="SCORE"><baseValue baseType="float">100</baseValue></setOutcomeValue></responseIf><responseElse><setOutcomeValue identifier="SCORE"><baseValue baseType="float">0</baseValue></setOutcomeValue></responseElse></responseCondition></responseProcessing>'),
        "fib": ('<responseDeclaration identifier="R" baseType="string" cardinality="single"><correctResponse><value>answer</value></correctResponse><mapping><mapEntry mapKey="answer" mappedValue="100"/></mapping></responseDeclaration>', '<textEntryInteraction responseIdentifier="R"/>', '<responseProcessing template="http://www.imsglobal.org/question/qti_v2p1/rptemplates/map_response"/>'),
        "multi_fib": ('<responseDeclaration identifier="one" baseType="string" cardinality="single"><correctResponse><value>one</value></correctResponse></responseDeclaration><responseDeclaration identifier="two" baseType="string" cardinality="single"><correctResponse><value>two</value></correctResponse></responseDeclaration>', '<textEntryInteraction responseIdentifier="one"/><textEntryInteraction responseIdentifier="two"/>', '<responseProcessing><responseCondition><responseIf><match><variable identifier="one"/><correct identifier="one"/></match><setOutcomeValue identifier="SCORE"><sum><variable identifier="SCORE"/><baseValue baseType="float">50</baseValue></sum></setOutcomeValue></responseIf></responseCondition><responseCondition><responseIf><match><variable identifier="two"/><correct identifier="two"/></match><setOutcomeValue identifier="SCORE"><sum><variable identifier="SCORE"/><baseValue baseType="float">50</baseValue></sum></setOutcomeValue></responseIf></responseCondition></responseProcessing>'),
        "order": (choice_declaration.format(cardinality="ordered", values="<value>a</value><value>b</value>"), '<orderInteraction responseIdentifier="R" shuffle="true"><simpleChoice identifier="a" fixed="false">A</simpleChoice><simpleChoice identifier="b" fixed="false">B</simpleChoice></orderInteraction>', '<responseProcessing template="http://www.imsglobal.org/question/qti_v2p1/rptemplates/match_correct"/>'),
    }
    with zipfile.ZipFile(package, "w") as archive:
        for name, (declaration, body, processing) in variants.items():
            score_default = "<defaultValue><value>0</value></defaultValue>" if name == "multi_fib" else ""
            archive.writestr(
                f"qti21_items/{name}.xml",
                f'<assessmentItem xmlns="{QTI21}">{declaration}<outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single">{score_default}</outcomeDeclaration><itemBody><div>{name}</div>{body}</itemBody>{processing}</assessmentItem>',
            )
    projected = {item["question"]: item for item in xml_projection(package)}
    assert set(projected) == set(variants)
    assert projected["mc"]["responses"][0]["cardinality"] == "single"
    assert projected["ma"]["responses"][0]["correct"] == ["A", "B"]
    assert projected["match"]["responses"][0]["correct"] == [["Prompt", "Choice"]]
    assert projected["num"]["score_program"][0] == "rules"
    assert projected["num"]["score_program"][1][0][0][1][1][0] == ("tolerance", "0.1 0.1")
    assert projected["fib"]["responses"][0]["mapping"][0]["key"] == "answer"
    assert len(projected["multi_fib"]["responses"]) == 2
    assert projected["multi_fib"]["score_declaration"]["default_value"] == "0"
    assert projected["multi_fib"]["score_program"][1][0][0][2][0][1][0] == "sum"
    assert projected["order"]["interactions"][0]["kind"] == "orderinteraction"
    equivalent = dict(variants)
    declaration, body, processing = equivalent["num"]
    equivalent["num"] = (
        declaration.replace("<value>4</value>", "<value>4.0</value>"),
        body,
        processing.replace("0.1 0.1", "0.10 0.10").replace(
            ">100</baseValue>", ">100.0</baseValue>"
        ),
    )
    with zipfile.ZipFile(package, "w") as archive:
        for name, (declaration, body, processing) in equivalent.items():
            score_default = "<defaultValue><value>0</value></defaultValue>" if name == "multi_fib" else ""
            archive.writestr(
                f"qti21_items/{name}.xml",
                f'<assessmentItem xmlns="{QTI21}">{declaration}<outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single">{score_default}</outcomeDeclaration><itemBody><div>{name}</div>{body}</itemBody>{processing}</assessmentItem>',
            )
    assert {item["question"]: item for item in xml_projection(package)} == projected
    changed = dict(variants)
    declaration, body, processing = changed["num"]
    changed["num"] = (
        declaration.replace("<value>4</value>", "<value>4.01</value>"),
        body,
        processing,
    )
    with zipfile.ZipFile(package, "w") as archive:
        for name, (declaration, body, processing) in changed.items():
            score_default = "<defaultValue><value>0</value></defaultValue>" if name == "multi_fib" else ""
            archive.writestr(
                f"qti21_items/{name}.xml",
                f'<assessmentItem xmlns="{QTI21}">{declaration}<outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single">{score_default}</outcomeDeclaration><itemBody><div>{name}</div>{body}</itemBody>{processing}</assessmentItem>',
            )
    assert {item["question"]: item for item in xml_projection(package)} != projected
    declaration, body, processing = variants["num"]
    changed["num"] = (
        declaration,
        body,
        processing.replace('tolerance="0.1 0.1"', 'tolerance="0.2 0.2"'),
    )
    with zipfile.ZipFile(package, "w") as archive:
        for name, (declaration, body, processing) in changed.items():
            score_default = "<defaultValue><value>0</value></defaultValue>" if name == "multi_fib" else ""
            archive.writestr(
                f"qti21_items/{name}.xml",
                f'<assessmentItem xmlns="{QTI21}">{declaration}<outcomeDeclaration identifier="SCORE" baseType="float" cardinality="single">{score_default}</outcomeDeclaration><itemBody><div>{name}</div>{body}</itemBody>{processing}</assessmentItem>',
            )
    assert {item["question"]: item for item in xml_projection(package)} != projected


if __name__ == "__main__":
    selftest()
    print("parity_qti21 selftest: OK")
