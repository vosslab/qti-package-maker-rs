"""Git finish, validation, and terminal reporting for repository reset."""

# Standard Library
import datetime
import os
import subprocess
import sys

# local repo modules
import repolib.model


# Template-owned locations removed from every consumer reset.
TEMPLATE_OWNED_PREFIXES = [
	"templates/",
	"repolib/",
	"LICENSES/",
	"meta/",
	"tests/meta/",
]

# Type-specific paths that prove propagation created a usable scaffold.
SCAFFOLD_SENTINELS: dict[str, str] = {
	"typescript": "eslint.config.js",
	"python": "docs/PYTHON_STYLE.md",
	"website": "mkdocs.yml",
}


#============================================
def get_git_dir(repo_root: str) -> str:
	"""Return the absolute Git directory for repo_root."""
	result = subprocess.run(
		["git", "rev-parse", "--git-dir"],
		check=True, capture_output=True, text=True, cwd=repo_root,
	)
	git_dir = result.stdout.strip()
	if not os.path.isabs(git_dir):
		git_dir = os.path.join(repo_root, git_dir)
	return os.path.abspath(git_dir)


#============================================
def preflight_finish(repo_root: str, commit: bool, push: bool) -> bool:
	"""Check Git finish prerequisites before reset mutates a consumer.

	Returns:
		True when Git has an origin remote configured.
	"""
	git_dir = get_git_dir(repo_root)
	if not os.access(git_dir, os.W_OK):
		sys.exit(f"Git directory is not writable: {git_dir}")
	lock_path = os.path.join(git_dir, "index.lock")
	if os.path.exists(lock_path):
		lock_stat = os.stat(lock_path)
		lock_age = datetime.datetime.now().timestamp() - lock_stat.st_mtime
		lock_mode = oct(lock_stat.st_mode & 0o777)
		lock_holder = subprocess.run(
			["lsof", lock_path], check=False, capture_output=True, text=True,
		)
		holder_detail = "no process detected"
		if lock_holder.returncode == 0:
			holder_detail = lock_holder.stdout.strip()
		sys.exit(
			f"Git index lock exists: {lock_path} (owner uid {lock_stat.st_uid}, "
			f"mode {lock_mode}, age {lock_age:.0f}s; {holder_detail}). Close "
			"the active Git operation or remove a confirmed stale lock."
		)
	if commit:
		identity = subprocess.run(
			["git", "var", "GIT_AUTHOR_IDENT"],
			check=False, capture_output=True, text=True, cwd=repo_root,
		)
		if identity.returncode != 0:
			sys.exit("Git author identity is required before reset can commit.")
	remote = subprocess.run(
		["git", "remote", "get-url", "origin"],
		check=False, capture_output=True, text=True, cwd=repo_root,
	)
	has_origin = remote.returncode == 0 and bool(remote.stdout.strip())
	if push and has_origin:
		print("Preflight: origin remote configured; requested publication can be attempted.")
	elif push:
		print("Preflight: no origin remote configured; requested publication will be degraded.")
	elif has_origin:
		print("Preflight: origin remote configured; publication not requested.")
	else:
		print("Preflight: no origin remote configured; publication not requested.")
	return has_origin


#============================================
def verify_clean_end_state(repo_root: str, dry_run: bool) -> tuple[int, str | None]:
	"""Return an incomplete-reset detail when template-owned paths remain."""
	if dry_run:
		print("DRY-RUN: verify: would check for leftover template-owned paths")
		return 1, None
	ls_result = subprocess.run(
		["git", "ls-files"], check=False, capture_output=True, text=True, cwd=repo_root,
	)
	if ls_result.returncode != 0:
		failure_text = ls_result.stderr.strip() or ls_result.stdout.strip()
		return 1, f"git ls-files validation failed: {failure_text}"
	leftover_tracked: list[str] = []
	for tracked_path in ls_result.stdout.splitlines():
		for prefix in TEMPLATE_OWNED_PREFIXES:
			if tracked_path.startswith(prefix) or tracked_path == prefix.rstrip("/"):
				leftover_tracked.append(f"tracked: {tracked_path}")
				break
	leftover_disk: list[str] = []
	for prefix in TEMPLATE_OWNED_PREFIXES:
		check_path = os.path.join(repo_root, prefix.rstrip("/"))
		if os.path.isdir(check_path):
			leftover_disk.append(f"on disk: {prefix}")
	all_leftovers = leftover_tracked + leftover_disk
	if all_leftovers:
		return 1, "template-owned paths remain after cleanup:\n  " + "\n  ".join(all_leftovers)
	return 1, None


#============================================
def verify_scaffold_sentinel(repo_root: str, project_type: str) -> str | None:
	"""Return an incomplete-reset detail when propagation missed its sentinel."""
	for chain_type in repolib.model.effective_type_chain(project_type):
		sentinel = SCAFFOLD_SENTINELS.get(chain_type)
		if sentinel is not None:
			break
	else:
		return None
	sentinel_path = os.path.join(repo_root, sentinel)
	if not os.path.isfile(sentinel_path):
		return (
			f"propagation completed but required scaffold path is missing: {sentinel}\n"
			f"Expected at: {sentinel_path}\n"
			"process_repo returned success but may have written nothing."
		)
	return None


#============================================
def print_next_steps(project_type: str) -> None:
	"""Print the first setup command appropriate for the selected project type."""
	type_chain = repolib.model.effective_type_chain(project_type)
	print("\nNext steps:")
	if "python" in type_chain:
		print("  pip install -r pip_requirements.txt && pip install -r pip_requirements-dev.txt")
	elif "typescript" in type_chain:
		print("  npm install && bash devel/setup_playwright.sh")
		print("  pip install -r pip_requirements-dev.txt")
	elif project_type == "rust":
		print("  cargo build")
		print("  pip install -r pip_requirements-dev.txt")
	else:
		print("  pip install -r pip_requirements-dev.txt")


#============================================
def report_incomplete_reset(project_type: str, detail: str) -> int:
	"""Print the terminal incomplete-reset outcome and return exit status 4."""
	print(f"Reset finish incomplete: {detail}")
	print_next_steps(project_type)
	print("RESET OUTCOME: incomplete; post-mutation operation failed (exit 4).")
	return 4


#============================================
def complete_reset(
	repo_root: str,
	project_type: str,
	dry_run: bool,
	stage: bool,
	commit: bool,
	push: bool,
	has_origin: bool,
	action_count: int,
	validation_problem: str | None = None,
) -> int:
	"""Finish reset with ordered Git actions and one explicit terminal outcome."""
	clean_action_count, clean_problem = verify_clean_end_state(repo_root, dry_run)
	action_count += clean_action_count
	if validation_problem is not None:
		return report_incomplete_reset(project_type, validation_problem)
	if clean_problem is not None:
		return report_incomplete_reset(project_type, clean_problem)
	if stage:
		action_count += 1
		if dry_run:
			print("DRY-RUN: git add -A")
		else:
			stage_result = subprocess.run(
				["git", "add", "-A"], check=False, capture_output=True, text=True,
				cwd=repo_root,
			)
			if stage_result.returncode != 0:
				failure_text = stage_result.stderr.strip() or stage_result.stdout.strip()
				return report_incomplete_reset(project_type, f"git add -A failed: {failure_text}")
	if commit:
		action_count += 1
		commit_msg = f"initial commit: reset repo to base template ({project_type})"
		if dry_run:
			print(f"DRY-RUN: git commit -m {repr(commit_msg)}")
		else:
			commit_result = subprocess.run(
				["git", "commit", "-m", commit_msg], check=False, capture_output=True,
				text=True, cwd=repo_root,
			)
			if commit_result.returncode != 0:
				failure_text = commit_result.stderr.strip() or commit_result.stdout.strip()
				return report_incomplete_reset(project_type, f"git commit failed: {failure_text}")
	if dry_run:
		if push:
			print("DRY-RUN: git push origin HEAD")
		print(f"DRY-RUN: {action_count} actions planned. No files changed.")
		print_next_steps(project_type)
		print("RESET OUTCOME: dry-run preview complete (exit 0).")
		return 0
	if not push:
		if commit:
			finish_state = "staged and committed"
		elif stage:
			finish_state = "staged but not committed"
		else:
			# Earlier cleanup uses git rm, which stages its own deletions. This
			# status describes only the optional finish steps, not the full index.
			finish_state = "finish stage not run; finish commit not run"
		subprocess.run(["git", "status", "--short"], check=False, cwd=repo_root)
		print_next_steps(project_type)
		print(f"RESET OUTCOME: complete; publication not requested ({finish_state}) (exit 0).")
		return 0
	if not has_origin:
		print_next_steps(project_type)
		print("RESET OUTCOME: complete; publication requested but no origin remote configured (exit 2).")
		return 2
	push_result = subprocess.run(
		["git", "push", "origin", "HEAD"],
		check=False, capture_output=True, text=True, cwd=repo_root,
	)
	if push_result.returncode == 0:
		print_next_steps(project_type)
		print("RESET OUTCOME: complete and published to origin (exit 0).")
		return 0
	failure_text = push_result.stderr.strip() or push_result.stdout.strip()
	print(f"Push failed: {failure_text}")
	print("Manual publication command: git push origin HEAD")
	print_next_steps(project_type)
	print("RESET OUTCOME: complete; push attempted and failed (exit 3).")
	return 3
