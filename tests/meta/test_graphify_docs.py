"""Behavior tests for the propagated Graphify documentation builder."""

# Standard Library
import sys
import re
import json
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils


# OWASP ASVS 5.0.0 V5.3.2: this fixed path uses trusted, internal components.
# pytest.ini is template-meta and never propagates, but this test does, so the
# devel/ directory is placed on sys.path here.
DEVEL_DIR: pathlib.Path = pathlib.Path(file_utils.get_repo_root()) / "devel"
if str(DEVEL_DIR) not in sys.path:
	sys.path.insert(0, str(DEVEL_DIR))

import graphify_docs_lib


#============================================


def sample_graph_data() -> dict:
	"""Return a two-community graph with one relationship crossing between them."""
	graph_data = {
		"nodes": [
			{
				"id": "store",
				"label": "RunStore",
				"community": 0,
				"community_name": "Run Storage",
				"source_file": "src/store.py",
			},
			{
				"id": "writer",
				"label": "write_run()",
				"community": 0,
				"community_name": "Run Storage",
				"source_file": "src/store.py",
			},
			{
				"id": "client",
				"label": "GitHubClient",
				"community": 1,
				"community_name": "GitHub API Client",
				"source_file": "tools/github.py",
			},
		],
		"links": [
			{"source": "store", "target": "writer", "relation": "contains"},
			{"source": "writer", "target": "client", "relation": "calls"},
		],
	}
	return graph_data


#============================================


def test_page_places_figure_before_repository_derived_prose() -> None:
	"""The figure is the first content and summaries come from graph data."""
	page = graphify_docs_lib.format_page(sample_graph_data(), None)
	first_lines = page.splitlines()[:5]
	assert first_lines[:3] == [
		"# Repository map", "", "![Community-level repository graph](GRAPHIFY_map.svg)",
	]
	assert "3 symbols and 2 relationships into 2 communities" in page


#============================================


def test_page_describes_repository_groups_and_major_communities() -> None:
	"""Source areas and Graphify communities both appear in the generated prose."""
	page = graphify_docs_lib.format_page(sample_graph_data(), None)
	assert "| `src` | 2 | 1 | 1 |" in page
	assert "| Run Storage | 2 | `src/store.py`" in page


#============================================


@pytest.mark.parametrize(
	"unsafe_text",
	["<small>hidden</small>", "&amp;", '"quoted"', "] injected["],
)
def test_page_sanitizes_generated_community_names(unsafe_text: str) -> None:
	"""Generated community names cannot inject HTML or Markdown table structure."""
	graph_data = sample_graph_data()
	graph_data["nodes"][2]["community_name"] = f"GitHub {unsafe_text} Client"
	page = graphify_docs_lib.format_page(graph_data, None)
	assert unsafe_text not in page


#============================================


def test_svg_is_self_contained_unlabeled_and_membership_scaled() -> None:
	"""The compact figure uses circles and lines without a text legend or external data."""
	svg_text = graphify_docs_lib.format_community_svg(sample_graph_data(), None)
	radii = re.findall(r'<circle [^>]*r="([0-9.]+)"', svg_text)
	assert float(radii[0]) > float(radii[1])
	assert "<text" not in svg_text and "href=" not in svg_text


#============================================


def test_svg_output_is_deterministic() -> None:
	"""Identical graph data produces byte-identical community illustrations."""
	first = graphify_docs_lib.format_community_svg(sample_graph_data(), None)
	second = graphify_docs_lib.format_community_svg(sample_graph_data(), None)
	assert first == second


#============================================


def test_write_docs_writes_both_fixed_artifacts(tmp_path: pathlib.Path) -> None:
	"""One publication action writes the page and its compact SVG."""
	output_dir = tmp_path / "graphify-out"
	docs_dir = tmp_path / "docs"
	output_dir.mkdir()
	docs_dir.mkdir()
	(output_dir / "graph.json").write_text(
		json.dumps(sample_graph_data()), encoding="utf-8",
	)
	page_path, figure_path = graphify_docs_lib.write_docs(tmp_path)
	assert page_path.read_text(encoding="ascii").startswith(
		"# Repository map\n\n![Community-level repository graph](GRAPHIFY_map.svg)"
	)
	assert figure_path.read_text(encoding="ascii").startswith("<svg")


# Vendored pytest file. Local changes can and will be overwritten.
