"""Verify every whole-file overwrite source identifies itself as vendored."""

# Standard Library
import pathlib

# local repo modules
import file_utils
import repolib.model
import repolib.plan


VENDORED_NOTICE = (
	"This file is vendored. Local changes can and will be overwritten by propagation."
)
VENDORED_HEADER_COMMENTS = frozenset({
	f"# {VENDORED_NOTICE}",
	f"// {VENDORED_NOTICE}",
	f"/* {VENDORED_NOTICE} */",
	f"<!-- {VENDORED_NOTICE} -->",
	f"> {VENDORED_NOTICE}",
	f"-- {VENDORED_NOTICE}",
	f"; {VENDORED_NOTICE}",
	f"% {VENDORED_NOTICE}",
})
FULL_OVERWRITE_BUCKETS = ("overwrite_files", "devel_files", "test_files")
HEADER_LINE_LIMIT = 5
REPO_ROOT = pathlib.Path(file_utils.get_repo_root())


#============================================

def overwrite_source_paths() -> list[pathlib.Path]:
	"""Return every canonical source copied with whole-file overwrite semantics."""
	source_paths: set[pathlib.Path] = set()
	repo_types = [
		repo_type for repo_type in repolib.model.REPO_TYPE_ORDER
		if repo_type != repolib.model.LANG_ALL
	]
	for repo_type in repo_types:
		plan = repolib.plan.compute_propagation_plan(str(REPO_ROOT), repo_type)
		for bucket in FULL_OVERWRITE_BUCKETS:
			for file_rel in plan[bucket]:
				source = repolib.model.source_path_for_bucket(
					str(REPO_ROOT), bucket, file_rel, repo_type,
				)
				source_paths.add(pathlib.Path(source))
	return sorted(source_paths)


#============================================
def test_whole_file_overwrite_sources_include_vendored_header() -> None:
	"""Every whole-file overwrite source warns editors near the top of the file."""
	missing = []
	for source_path in overwrite_source_paths():
		header_lines = source_path.read_text(encoding="utf-8").splitlines()[:HEADER_LINE_LIMIT]
		if not any(line.strip() in VENDORED_HEADER_COMMENTS for line in header_lines):
			missing.append(source_path.relative_to(REPO_ROOT).as_posix())
	message = (
		"Whole-file overwrite sources missing the exact vendored header comment within "
		f"their first {HEADER_LINE_LIMIT} lines: {missing}. Add the format's native comment "
		"after a required shebang. Noexist and partial-ownership buckets are excluded."
	)
	assert not missing, message
