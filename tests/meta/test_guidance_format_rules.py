"""Adversarial input tests for the vendored guidance-format parsing helpers.

The rules in tests/test_guidance_doc_format.py ship to every consumer repo, and
they run there against real, varied Markdown rather than the template's near-empty
stubs. Exercising them end-to-end against the stubs proves almost nothing, so the
parsing helpers get their edge cases here instead: this file is template-meta and
never propagates, and the logic it covers is identical in every repo that receives
it.

The blank-line continuation case below is a regression test. The first version of
the prose rule cleared its in-bullet state on any blank line, so a bullet written
with a blank-line-separated continuation -- ordinary Markdown -- was reported as a
stray prose paragraph in every consumer repo.
"""

# Standard Library
import os
import sys

# PIP3 modules
import pytest

# local repo modules
import file_utils

# The module under test ships from tests/, which is on the path for the template's
# own suite; import it by module name the same way a consumer's run would.
sys.path.insert(0, os.path.join(file_utils.get_repo_root(), 'tests'))
import test_guidance_doc_format


GUIDANCE = "docs/HUMAN_GUIDANCE.md"
DECISIONS = "docs/DESIGN_DECISIONS.md"


#============================================
def guidance_violations(body: str) -> list[str]:
	"""
	Run both HUMAN_GUIDANCE rules over a Markdown fragment.

	Args:
		body: Markdown text, without the vendored header.

	Returns:
		list[str]: Violation lines from the bullet-length and prose rules.
	"""
	lines = body.splitlines()
	violations = test_guidance_doc_format.check_guidance_bullets(GUIDANCE, lines)
	violations += test_guidance_doc_format.check_guidance_is_bulleted(GUIDANCE, lines)
	return violations


#============================================
def test_blank_line_continuation_stays_one_bullet() -> None:
	"""A bullet continued after a blank line is still that bullet."""
	body = (
		"# Human guidance\n"
		"\n"
		"## Decision priority\n"
		"\n"
		"- Keep it simple: avoid speculative machinery.\n"
		"\n"
		"  A blank-line-separated continuation of the same bullet.\n"
	)
	assert guidance_violations(body) == []


#============================================
def test_prose_paragraph_under_a_section_is_reported() -> None:
	"""An unindented paragraph under a section heading is agent narration."""
	body = (
		"# Human guidance\n"
		"\n"
		"## Decision priority\n"
		"\n"
		"The repository favors a measured approach, balancing concerns as needed.\n"
	)
	violations = guidance_violations(body)
	assert len(violations) == 1 and "prose paragraph" in violations[0]


#============================================
def test_prose_above_the_first_section_is_left_alone() -> None:
	"""A repository's own scope note sits above the first section heading."""
	body = (
		"# Human guidance\n"
		"\n"
		"This file records only what I state or approve.\n"
		"\n"
		"## Decision priority\n"
		"\n"
		"- Keep it simple.\n"
	)
	assert guidance_violations(body) == []


#============================================
@pytest.mark.parametrize(
	"entry",
	[
		"1. An ordered entry that carries a preference.",
		"* A star bullet rather than a dash.",
		"| column | column |",
	],
	ids=["ordered_item", "star_bullet", "table_row"],
)
def test_structured_entries_are_not_prose(entry: str) -> None:
	"""Ordered items, star bullets, and table rows are structure, not narration."""
	body = f"# Human guidance\n\n## Decision priority\n\n{entry}\n"
	assert guidance_violations(body) == []


#============================================
def test_long_bullet_is_reported_once() -> None:
	"""A bullet past the line limit is reported, with its length named."""
	body = (
		"# Human guidance\n"
		"\n"
		"## Decision priority\n"
		"\n"
		"- An entry that starts here,\n"
		"  runs on to a second line,\n"
		"  reaches a third,\n"
		"  and keeps going to a fourth.\n"
	)
	violations = guidance_violations(body)
	assert len(violations) == 1 and "runs 4 lines" in violations[0]


#============================================
def test_decision_entry_missing_a_field_is_reported() -> None:
	"""An entry without its Why field is an assertion, not a decision."""
	body = (
		"# Design decisions\n"
		"\n"
		"## Software design\n"
		"\n"
		"### Grading stays on the server\n"
		"\n"
		"**Decision.** Keep grading server-side.\n"
	)
	violations = test_guidance_doc_format.check_decision_entries(DECISIONS, body.splitlines())
	assert len(violations) == 1 and "**Why.**" in violations[0]


#============================================
def test_decision_skeleton_inside_a_fence_is_not_an_entry() -> None:
	"""The seeded stub keeps its example skeleton in a fence, so it is not checked."""
	body = (
		"# Design decisions\n"
		"\n"
		"```markdown\n"
		"### <decision title>\n"
		"\n"
		"**Decision.** <the durable direction>\n"
		"```\n"
	)
	assert test_guidance_doc_format.check_decision_entries(DECISIONS, body.splitlines()) == []
