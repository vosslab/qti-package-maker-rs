"""Behavior tests for the propagated Graphify repository mapping tool."""

# Standard Library
import sys
import json
import pathlib
import datetime

# PIP3 modules
import pytest

# local repo modules
import file_utils


# OWASP ASVS 5.0.0 V5.3.2: this fixed path uses trusted, internal components.
# pytest.ini is template-meta and never propagates, but this test does, so the
# devel/ directory is placed on sys.path here rather than relying on the
# template's pythonpath setting. graphify_map_repo imports its sibling
# graphify_context_lib, so both must resolve the same way a direct
# `python3 devel/graphify_map_repo.py` run resolves them.
DEVEL_DIR: pathlib.Path = pathlib.Path(file_utils.get_repo_root()) / "devel"
if str(DEVEL_DIR) not in sys.path:
	sys.path.insert(0, str(DEVEL_DIR))

import graphify_context_lib
import graphify_map_repo


#============================================


def sample_graph_data() -> dict:
	"""Return a small multi-community graph for orientation behavior tests."""
	graph_data = {
		"nodes": [
			{
				"id": "app",
				"label": "App()",
				"_callable": True,
				"community_name": "Game Logic",
				"source_file": "src/app.tsx",
				"source_location": "L10",
			},
			{
				"id": "helper",
				"label": "beginWave()",
				"_callable": True,
				"community_name": "Game Logic",
				"source_file": "src/game.ts",
				"source_location": "L40",
			},
			{
				"id": "tick",
				"label": "tickGame()",
				"_callable": True,
				"community_name": "Game Simulation",
				"source_file": "src/simulation.ts",
				"source_location": "L20",
			},
			{
				"id": "settings",
				"label": "SettingsState",
				"_callable": True,
				"community_name": "Game Settings",
				"source_file": "src/settings.ts",
				"source_location": "L30",
			},
			{
				"id": "compiler",
				"label": "compilerOptions",
				"community_name": "TypeScript Configuration",
				"source_file": "tsconfig.json",
			},
		],
		"links": [
			{"source": "app", "target": "tick"},
			{"source": "app", "target": "settings"},
			{"source": "app", "target": "helper"},
			{"source": "tick", "target": "helper"},
		],
	}
	return graph_data


#============================================


def sample_mapped_at() -> datetime.datetime:
	"""Return the human-requested mapping-time example with an explicit timezone."""
	timezone = datetime.timezone(datetime.timedelta(hours=-5), name="CDT")
	mapped_at = datetime.datetime(2026, 8, 30, 21, 41, tzinfo=timezone)
	return mapped_at


#============================================


def sample_analysis_data() -> dict:
	"""Return structured Graphify analysis matching the sample graph."""
	return {
		"communities": {
			"0": ["app", "helper"],
			"1": ["tick"],
			"2": ["settings"],
			"3": ["compiler"],
		},
		"cohesion": {},
		"gods": [{"id": "app", "label": "App()", "degree": 3}],
		"questions": [
			{
				"type": "bridge_node",
				"question": (
					"Why does `App()` connect `Game Logic` to `Game Settings`, "
					"`Game Simulation`?"
				),
				"why": "cross-community bridge",
			},
		],
		"surprises": [
			{
				"source": "App()",
				"target": "tickGame()",
				"relation": "calls",
			},
		],
	}


#============================================


def sample_labels_data() -> dict:
	"""Return stable community labels matching the sample analysis."""
	return {
		"0": "Game Logic",
		"1": "Game Simulation",
		"2": "Game Settings",
		"3": "TypeScript Configuration",
	}


#============================================


def test_existing_graph_selects_real_update(tmp_path: pathlib.Path) -> None:
	"""An existing graph uses Graphify's incremental code-map update."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	(output_dir / "graph.json").write_text("{}", encoding="utf-8")
	operation, command, is_fresh = graphify_map_repo.graph_build_command(
		"graphify",
		tmp_path,
		graphify_map_repo.MODE_AUTO,
	)
	assert (operation, command) == ("UPDATING GRAPHIFY CODE MAP", ["graphify", "update", "."])
	assert is_fresh is False


#============================================


def test_missing_graph_selects_code_extraction(tmp_path: pathlib.Path) -> None:
	"""A missing graph performs a fresh code-only extraction."""
	operation, command, is_fresh = graphify_map_repo.graph_build_command(
		"graphify",
		tmp_path,
		graphify_map_repo.MODE_AUTO,
	)
	expected = ("EXTRACTING GRAPHIFY CODE MAP", ["graphify", "extract", ".", "--code-only"])
	assert (operation, command) == expected
	assert is_fresh is True


#============================================


def test_fresh_mode_forces_code_extraction(tmp_path: pathlib.Path) -> None:
	"""Fresh mode extracts even when an existing graph could be updated."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	(output_dir / "graph.json").write_text("{}", encoding="utf-8")
	operation, command, is_fresh = graphify_map_repo.graph_build_command(
		"graphify",
		tmp_path,
		graphify_map_repo.MODE_FRESH,
	)
	expected = ("EXTRACTING GRAPHIFY CODE MAP", ["graphify", "extract", ".", "--code-only"])
	assert (operation, command) == expected
	assert is_fresh is True


#============================================


def test_update_mode_extracts_when_graph_is_missing(tmp_path: pathlib.Path) -> None:
	"""Update mode announces the fresh-extraction fallback from the retired tool."""
	operation, command, is_fresh = graphify_map_repo.graph_build_command(
		"graphify",
		tmp_path,
		graphify_map_repo.MODE_UPDATE,
	)
	expected = (
		"NO EXISTING GRAPH; EXTRACTING FRESH GRAPHIFY CODE MAP",
		["graphify", "extract", ".", "--code-only"],
	)
	assert (operation, command) == expected
	assert is_fresh is True


#============================================


@pytest.mark.parametrize(
	("flag", "mode"),
	[
		("-F", graphify_map_repo.MODE_FRESH),
		("--fresh", graphify_map_repo.MODE_FRESH),
		("-U", graphify_map_repo.MODE_UPDATE),
		("--update", graphify_map_repo.MODE_UPDATE),
		("-C", graphify_map_repo.MODE_CONTEXT),
		("--context", graphify_map_repo.MODE_CONTEXT),
	],
)
def test_explicit_mode_flags(flag: str, mode: str) -> None:
	"""Each documented flag selects its matching lifecycle mode."""
	args = graphify_map_repo.parse_args([flag])
	assert args.mode == mode


#============================================


@pytest.mark.parametrize(
	"argv",
	[
		["--svg"],
		["--fresh", "--svg"],
		["--update", "--svg"],
		["--context", "--svg"],
	],
)
def test_svg_is_independent_of_the_lifecycle_mode(argv: list[str]) -> None:
	"""Publishing composes with explicit modes and works alone on existing data."""
	args = graphify_map_repo.parse_args(argv)
	assert args.write_svg is True


#============================================


def test_lifecycle_modes_remain_mutually_exclusive() -> None:
	"""Fresh, update, and context still cannot select conflicting graph actions."""
	with pytest.raises(SystemExit):
		graphify_map_repo.parse_args(["--fresh", "--update", "--svg"])


#============================================


def test_fresh_ollama_flag_selects_local_backend() -> None:
	"""The allowance fallback is available for the fresh-build workflow."""
	args = graphify_map_repo.parse_args(["--fresh", "--ollama"])
	assert args.label_backend == graphify_map_repo.OLLAMA_BACKEND


#============================================


def test_ollama_requires_fresh_mode() -> None:
	"""The local label fallback cannot silently change an update."""
	with pytest.raises(SystemExit):
		graphify_map_repo.parse_args(["--ollama"])


#============================================


def test_fresh_claude_labeling_uses_configured_model(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Fresh Claude labeling selects its model independently of the interactive default."""
	commands = []

	def record_command(command: list[str], repo_root: pathlib.Path) -> None:
		commands.append((command, repo_root))

	monkeypatch.setattr(graphify_map_repo, "run_command", record_command)
	graphify_map_repo.label_graph(
		"graphify",
		tmp_path,
		graphify_map_repo.LABEL_BACKEND,
	)
	expected = [
		"graphify",
		"label",
		".",
		"--backend=claude-cli",
		f"--model={graphify_map_repo.CLAUDE_LABEL_MODEL}",
	]
	assert commands == [(expected, tmp_path)]


#============================================


def test_fresh_ollama_labeling_uses_configured_model(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The local fallback uses its maintained model without contacting Claude."""
	commands = []

	def record_command(command: list[str], repo_root: pathlib.Path) -> None:
		commands.append((command, repo_root))

	monkeypatch.setattr(graphify_map_repo, "run_command", record_command)
	graphify_map_repo.label_graph(
		"graphify",
		tmp_path,
		graphify_map_repo.OLLAMA_BACKEND,
	)
	expected = [
		"graphify",
		"label",
		".",
		"--backend=ollama",
		f"--model={graphify_map_repo.OLLAMA_MODEL}",
	]
	assert commands == [(expected, tmp_path)]


#============================================


def test_context_prints_help_before_first_map(
	tmp_path: pathlib.Path,
	capsys: pytest.CaptureFixture,
) -> None:
	"""Context explains the missing map and prints normal help before first build."""
	graphify_map_repo.print_context(tmp_path)
	output = capsys.readouterr().out
	assert "No Graphify map exists" in output
	assert "usage:" in output and "--fresh" in output


#============================================


@pytest.mark.parametrize("artifact_name", ["manifest.json", "GRAPH_REPORT.md"])
def test_context_prints_help_for_incomplete_map(
	tmp_path: pathlib.Path,
	capsys: pytest.CaptureFixture,
	artifact_name: str,
) -> None:
	"""Internal or visible partial output is not presented as a usable graph."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	(output_dir / artifact_name).write_text("partial", encoding="utf-8")
	graphify_map_repo.print_context(tmp_path)
	output = capsys.readouterr().out
	assert "No Graphify map exists" in output
	assert "usage:" in output


#============================================


def test_svg_writes_the_page_and_community_figure(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture,
) -> None:
	"""SVG publication delegates both fixed artifacts to the documentation writer."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	(output_dir / "graph.json").write_text(json.dumps(sample_graph_data()), encoding="utf-8")
	docs_dir = tmp_path / "docs"
	docs_dir.mkdir()
	page_path = docs_dir / "GRAPHIFY.md"
	figure_path = docs_dir / "GRAPHIFY_map.svg"
	monkeypatch.setattr(
		graphify_map_repo.graphify_docs_lib,
		"write_docs",
		lambda _root: (page_path, figure_path),
	)
	graphify_map_repo.write_graphify_docs(tmp_path)
	output = capsys.readouterr().out
	assert "docs/GRAPHIFY.md" in output
	assert "docs/GRAPHIFY_map.svg" in output


#============================================


def test_structured_orientation_uses_mapping_time_not_commit() -> None:
	"""Manager context leads with the working-tree map's local timestamp."""
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(),
		sample_graph_data(),
		analysis_data=sample_analysis_data(),
		labels_data=sample_labels_data(),
	)
	assert orientation.startswith("GRAPHIFY CONTEXT\nGraph mapped at 9:41 PM CDT Aug 30 2026")
	assert "commit" not in orientation


#============================================


def test_bridge_is_cross_community_instead_of_high_degree() -> None:
	"""Graphify's bridge result is used instead of its within-area god node."""
	graph_data = {
		"nodes": [
			{"id": "hub", "label": "Hub()", "_callable": True,
				"community_name": "Area A"},
			{"id": "a1", "label": "a1()", "_callable": True,
				"community_name": "Area A"},
			{"id": "a2", "label": "a2()", "_callable": True,
				"community_name": "Area A"},
			{"id": "a3", "label": "a3()", "_callable": True,
				"community_name": "Area A"},
			{"id": "bridge", "label": "Bridge()", "_callable": True,
				"community_name": "Area A"},
			{"id": "b", "label": "b()", "_callable": True,
				"community_name": "Area B"},
			{"id": "c", "label": "c()", "_callable": True,
				"community_name": "Area C"},
		],
		"links": [
			{"source": "hub", "target": "a1"},
			{"source": "hub", "target": "a2"},
			{"source": "hub", "target": "a3"},
			{"source": "bridge", "target": "b"},
			{"source": "bridge", "target": "c"},
		],
	}
	analysis_data = {
		"communities": {
			"0": ["hub", "a1", "a2", "a3", "bridge"],
			"1": ["b"],
			"2": ["c"],
		},
		"gods": [{"id": "hub", "label": "Hub()", "degree": 3}],
		"questions": [
			{
				"type": "bridge_node",
				"question": "Why does `Bridge()` connect `Area A` to `Area B`, `Area C`?",
			},
		],
		"surprises": [],
	}
	labels_data = {"0": "Area A", "1": "Area B", "2": "Area C"}
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(),
		graph_data,
		analysis_data=analysis_data,
		labels_data=labels_data,
	)
	assert "Bridge() - connects Area A, Area B, and Area C" in orientation
	# The god node may appear as an architectural hub; what it must never do is
	# take the cross-area connector slot that belongs to the real bridge.
	assert "Hub() - connects" not in orientation


#============================================


def test_cross_area_connector_output_is_bounded() -> None:
	"""One large connector summarizes communities beyond the display bound.

	The map needs enough communities that a connector spanning this many of them
	is still a real bridge rather than a repository-wide utility type, otherwise
	the spread filter rejects it before the display bound is ever reached.
	"""
	community_names = tuple(
		f"Area {index:02d}"
		for index in range(graphify_context_lib.MAX_CONNECTOR_COMMUNITIES + 2)
	)
	quoted_names = ", ".join(f"`{name}`" for name in community_names)
	spanning_community_count = len(community_names)
	total_community_count = int(
		spanning_community_count / graphify_context_lib.MAX_CONNECTOR_SPREAD_RATIO
	)
	analysis_data = {
		"communities": {
			str(index): ["bridge"] for index in range(total_community_count)
		},
		"questions": [
			{
				"type": "bridge_node",
				"question": f"Why does `Bridge()` connect {quoted_names}?",
			},
		],
		"surprises": [],
		"gods": [],
	}
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None,
		analysis_data=analysis_data,
		labels_data={"0": "Bridge Area"},
	)
	visible_names = ", ".join(
		community_names[:graphify_context_lib.MAX_CONNECTOR_COMMUNITIES]
	)
	assert f"- Bridge() - connects {visible_names}, and 2 more" in orientation


#============================================


def test_large_analysis_hard_caps_major_areas() -> None:
	"""A large graph cannot expand manager context past the configured area cap."""
	communities = {}
	labels = {}
	for index in range(graphify_context_lib.MAX_COMMUNITIES + 2):
		community_id = str(index)
		communities[community_id] = [f"node-{index}-a", f"node-{index}-b"]
		labels[community_id] = f"Area {index:02d}"
	analysis_data = {
		"communities": communities,
		"questions": [],
		"surprises": [],
		"gods": [],
	}
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None,
		analysis_data=analysis_data,
		labels_data=labels,
	)
	assert "- Area 07" in orientation
	assert "- Area 08" not in orientation


#============================================


def test_small_graph_without_sidecars_still_produces_context() -> None:
	"""Graph JSON alone is sufficient for useful deterministic context."""
	first_output = graphify_context_lib.format_orientation(
		sample_mapped_at(), sample_graph_data()
	)
	second_output = graphify_context_lib.format_orientation(
		sample_mapped_at(), sample_graph_data()
	)
	assert "Major repository areas:" in first_output
	assert "- Game Logic" in first_output
	assert "Cross-area connectors:" not in first_output
	assert first_output == second_output


#============================================


def test_report_is_last_resort_context_source(tmp_path: pathlib.Path) -> None:
	"""A report alone supplies minimal orientation when structured files are absent."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	report_text = """# Graph Report

## Graph Freshness
- Built from commit: `abc12345`

## Communities

### Community 0 - "Scene Linting"
Cohesion: 0.20
Nodes (12): Finding

### Community 1 - "State Management"
Cohesion: 0.30
Nodes (8): StateMap

## Suggested Questions
- **Why does `Finding` connect `Scene Linting` to `State Management`?**
"""
	(output_dir / "GRAPH_REPORT.md").write_text(report_text, encoding="utf-8")
	orientation = graphify_context_lib.manager_context(tmp_path)
	assert orientation is not None
	assert "Finding - connects Scene Linting and State Management" in orientation


#============================================


def test_orientation_omits_graphify_diagnostics() -> None:
	"""Context contains repository structure, not artifact or maintenance diagnostics."""
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(),
		sample_graph_data(),
		analysis_data=sample_analysis_data(),
		labels_data=sample_labels_data(),
	)
	for unwanted_text in (
		"Corpus Check",
		"Graph scope excludes",
		"graph.html",
		"graph.json",
		"Token cost",
		"git ignored",
	):
		assert unwanted_text not in orientation


#============================================


def test_manager_context_file_matches_terminal_context(tmp_path: pathlib.Path) -> None:
	"""Build output saves the exact deterministic context shown to managers."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	context = graphify_context_lib.format_orientation(
		sample_mapped_at(), sample_graph_data()
	)
	context_path = graphify_context_lib.write_manager_context(tmp_path, context)
	assert context_path.name == "MANAGER_CONTEXT.md"
	assert context_path.read_text(encoding="utf-8") == f"{context}\n"


#============================================


def test_graph_data_loader_rejects_missing_links(tmp_path: pathlib.Path) -> None:
	"""A partial graph JSON fails before producing misleading orientation."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	graph_text = json.dumps({"nodes": []})
	(output_dir / "graph.json").write_text(graph_text, encoding="utf-8")
	with pytest.raises(RuntimeError, match="no links list"):
		graphify_context_lib.load_graph_data(tmp_path)


#============================================


def bridge_analysis_data(community_count: int, spans: dict[str, int]) -> dict:
	"""Build analysis data whose bridges span a chosen number of communities."""
	area_names = [f"Area {index:02d}" for index in range(community_count)]
	questions = []
	for label, span in spans.items():
		quoted = ", ".join(f"`{name}`" for name in area_names[:span])
		questions.append({
			"type": "bridge_node",
			"question": f"Why does `{label}` connect {quoted}?",
		})
	return {
		"communities": {str(index): ["node"] for index in range(community_count)},
		"questions": questions,
		"surprises": [],
		"gods": [],
	}


#============================================


def test_repository_wide_type_is_not_a_connector() -> None:
	"""A symbol spanning most of the map is a utility type, not a bridge."""
	analysis_data = bridge_analysis_data(
		community_count=40,
		spans={"Timestamp": 34, "GitHubClient": 3},
	)
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None, analysis_data=analysis_data
	)
	assert "GitHubClient - connects" in orientation
	assert "Timestamp" not in orientation


#============================================


def test_small_map_still_reports_its_connectors() -> None:
	"""The spread floor keeps a tiny map from rejecting every connector it has."""
	analysis_data = bridge_analysis_data(
		community_count=4,
		spans={"SharedClient": 3},
	)
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None, analysis_data=analysis_data
	)
	assert "SharedClient - connects" in orientation


#============================================


def test_notable_relationships_exclude_test_symbols() -> None:
	"""Notable relationships report architecture, not the test suite."""
	analysis_data = {
		"communities": {"0": ["a"]},
		"questions": [],
		"gods": [],
		"surprises": [
			{
				"source": "schedule_defaults_are_kept()",
				"target": "SchedulePolicy",
				"relation": "uses",
				"source_files": ["tests/test_schedule.py", "src/policy.py"],
			},
			{
				"source": "publish_report_date()",
				"target": "PublicationRuntime",
				"relation": "references",
				"source_files": ["src/publish.py", "src/runtime.py"],
			},
		],
	}
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None, analysis_data=analysis_data
	)
	assert "publish_report_date() references PublicationRuntime." in orientation
	assert "SchedulePolicy" not in orientation


#============================================


def test_test_symbols_are_not_offered_as_connectors() -> None:
	"""The same test predicate guards the connector list, not only relationships."""
	analysis_data = bridge_analysis_data(
		community_count=40,
		spans={"test_fixture_registry": 3, "GitHubClient": 3},
	)
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), None, analysis_data=analysis_data
	)
	assert "GitHubClient - connects" in orientation
	assert "test_fixture_registry" not in orientation


#============================================


def test_architectural_hubs_reach_the_output() -> None:
	"""Architectural hubs give managers a source path and omit test scaffolding."""
	analysis_data = sample_analysis_data()
	analysis_data["gods"].append(
		{"id": "fixture", "label": "test_fixture()", "degree": 30}
	)
	graph_data = sample_graph_data()
	graph_data["nodes"].append({
		"id": "fixture",
		"label": "test_fixture()",
		"source_file": "tests/test_fixture.py",
	})
	orientation = graphify_context_lib.format_orientation(
		sample_mapped_at(), graph_data, analysis_data=analysis_data
	)
	assert "- App() (src/app.tsx)" in orientation
	assert "test_fixture()" not in orientation


#============================================


def test_stale_map_still_prints_full_orientation(
	tmp_path: pathlib.Path, capsys: pytest.CaptureFixture
) -> None:
	"""Staleness is advisory: orientation is the point of context mode."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	graph_text = json.dumps(sample_graph_data())
	(output_dir / "graph.json").write_text(graph_text, encoding="utf-8")
	(output_dir / "needs_update").write_text("", encoding="utf-8")
	graphify_map_repo.print_context(tmp_path)
	printed = capsys.readouterr().out
	assert "Major repository areas:" in printed
	assert "Map is stale" in printed


#============================================


def test_lessons_file_is_pointed_at_when_present(tmp_path: pathlib.Path) -> None:
	"""Manager context routes readers to prior query outcomes when they exist."""
	output_dir = tmp_path / "graphify-out"
	output_dir.mkdir()
	graph_text = json.dumps(sample_graph_data())
	(output_dir / "graph.json").write_text(graph_text, encoding="utf-8")
	without_lessons = graphify_context_lib.manager_context(tmp_path)
	reflections_dir = output_dir / "reflections"
	reflections_dir.mkdir()
	(reflections_dir / "LESSONS.md").write_text("# Lessons\n", encoding="utf-8")
	with_lessons = graphify_context_lib.manager_context(tmp_path)
	assert "Prior query outcomes" not in without_lessons
	assert "graphify-out/reflections/LESSONS.md" in with_lessons


# Vendored pytest file. Local changes can and will be overwritten.
