#!/usr/bin/env python3
"""Single-repo interactive tool for propagating canonical docs and styles."""

import os
import argparse
import datetime

import repolib.console
import repolib.propagation_changelog
import repolib.process


CHANGELOG_CATEGORY = "Fixes and Maintenance"
CHANGELOG_TITLE = (
	"Synchronized shared style guides, tests, and repository support files from "
	"the starter template."
)


#============================================
def parse_args() -> argparse.Namespace:
	"""Parse CLI flags and return the argparse Namespace."""
	parser = argparse.ArgumentParser(
		description=(
			"Propagate shared style guides and docs into a single target repo."
		)
	)
	parser.add_argument(
		'-n', '--dry-run', dest='dry_run',
		help='Only display planned changes', action='store_true'
	)
	parser.add_argument(
		'-R', '--repo', dest='repo_path',
		required=True,
		help='Path to the target repo (relative or absolute, e.g. ../vosslab-skills or .)'
	)
	parser.set_defaults(dry_run=False)
	args = parser.parse_args()
	return args


#============================================
def record_propagation_changelog(repo_dir: str, date_str: str) -> str:
	"""Add the canonical propagation entry and return the changelog path."""
	changelog_path = repolib.propagation_changelog.record_entry(
		repo_dir, date_str, CHANGELOG_CATEGORY, CHANGELOG_TITLE,
	)
	result = changelog_path
	return result


#============================================
def main() -> int:
	"""Build context for a single repo and run propagation."""
	args = parse_args()
	repo_dir = os.path.abspath(os.path.expanduser(args.repo_path))
	changelog_path = os.path.join(repo_dir, 'docs', 'CHANGELOG.md')
	changelog_existed = os.path.isfile(changelog_path)
	context = repolib.process.build_context_for_repo(
		repo_path=repo_dir, dry_run=args.dry_run,
		initial_setup=False, auto_discover=True, write_marker=True)
	counters = repolib.console.init_counters()
	result = repolib.process.process_repo(repo_dir, context, counters, emit_per_repo_summary=False)
	repo_results = []
	if result is not None:
		repo_results.append(result)
	if (
			result is not None and result['changed'] and counters['errors'] == 0
			and not context.dry_run
			):
		date_str = datetime.date.today().isoformat()
		recorded_path = record_propagation_changelog(repo_dir, date_str)
		action = 'merge' if changelog_existed else 'create'
		repolib.console.log_action(action, recorded_path)
		if changelog_existed:
			counters['merged_count'] += 1
	repolib.console.validate_counters(counters)
	repolib.console.print_summary(counters, repo_results=repo_results, dry_run=context.dry_run)

	# Final completion line: success (green) or failure (bold red)
	if counters['errors'] == 0:
		repolib.console.CONSOLE.print("[green]done[/]")
	else:
		repolib.console.CONSOLE.print(f"[bold red]failed ({counters['errors']} errors)[/]")

	exit_code = repolib.process.exit_code_for(counters)
	return exit_code


if __name__ == '__main__':
	raise SystemExit(main())
