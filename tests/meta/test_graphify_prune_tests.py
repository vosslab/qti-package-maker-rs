"""Behavior tests for the propagated Graphify Rust test-symbol pruner."""

# Standard Library
import sys
import pathlib

# local repo modules
import file_utils


# OWASP ASVS 5.0.0 V5.3.2: this fixed path uses trusted, internal components.
# pytest.ini is template-meta and never propagates, but this test does, so the
# devel/ directory is placed on sys.path here rather than relying on the
# template's pythonpath setting.
DEVEL_DIR: pathlib.Path = pathlib.Path(file_utils.get_repo_root()) / "devel"
if str(DEVEL_DIR) not in sys.path:
	sys.path.insert(0, str(DEVEL_DIR))

import graphify_prune_tests


RUST_SOURCE = """\
pub fn resolve_policy() -> bool {
	true
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn denied_membership_never_produces_preview_data() {
		assert!(resolve_policy());
	}
}
"""


#============================================


def parse_spans(source_text: str) -> list[tuple[int, int]]:
	"""Return the test-only line spans found in one Rust source string."""
	parser = graphify_prune_tests.build_rust_parser()
	spans = graphify_prune_tests.test_spans_in_source(source_text.encode("utf-8"), parser)
	return spans


#============================================


def lines_covered(spans: list[tuple[int, int]]) -> set[int]:
	"""Return every line number the given spans cover."""
	covered = set()
	for start_line, end_line in spans:
		covered.update(range(start_line, end_line + 1))
	return covered


#============================================


def test_cfg_test_module_is_spanned_and_production_code_is_not() -> None:
	"""The inline test module is covered; the function above it is untouched."""
	covered = lines_covered(parse_spans(RUST_SOURCE))
	# Line 1 holds resolve_policy(); line 6 holds the test module body.
	assert 1 not in covered
	assert 6 in covered


#============================================


def test_bare_test_function_is_spanned() -> None:
	"""A `#[test]` function outside any cfg(test) module is still test-only."""
	source_text = "#[test]\nfn checks_something() {\n\tassert!(true);\n}\n"
	covered = lines_covered(parse_spans(source_text))
	assert 2 in covered


#============================================


def test_namespaced_test_function_is_spanned() -> None:
	"""Namespaced runners count too, so async test suites are not missed."""
	source_text = "#[tokio::test]\nasync fn checks_something() {}\n"
	covered = lines_covered(parse_spans(source_text))
	assert 2 in covered


#============================================


def test_ordinary_attribute_does_not_mark_code_as_test_only() -> None:
	"""A production attribute must never cause code to be pruned."""
	source_text = "#[derive(Debug)]\nstruct Policy;\n"
	assert parse_spans(source_text) == []


#============================================


def build_graph_data() -> dict:
	"""Return a graph holding one production node, one test node, and a stub."""
	graph_data = {
		"nodes": [
			{
				"id": "policy",
				"label": "resolve_policy()",
				"source_file": "src/policy.rs",
				"source_location": "L1",
			},
			{
				"id": "test_case",
				"label": "denied_membership_never_produces_preview_data()",
				"source_file": "src/policy.rs",
				"source_location": "L10",
			},
			{
				"id": "stub",
				"label": "ExternalType",
				"source_file": "",
				"source_location": "",
			},
		],
		"edges": [
			{"source": "test_case", "target": "policy", "relation": "calls"},
		],
	}
	return graph_data


#============================================


def test_pruning_removes_test_nodes_and_their_links(tmp_path: pathlib.Path) -> None:
	"""Test symbols leave the graph itself, not just the printed orientation."""
	source_dir = tmp_path / "src"
	source_dir.mkdir()
	(source_dir / "policy.rs").write_text(RUST_SOURCE, encoding="utf-8")
	graph_data = build_graph_data()
	summary = graphify_prune_tests.prune_graph_data(graph_data, tmp_path)
	remaining_ids = [node["id"] for node in graph_data["nodes"]]
	assert summary == {"removed_nodes": 1, "removed_links": 1}
	assert remaining_ids == ["policy", "stub"]


# Vendored pytest file. Local changes can and will be overwritten.
