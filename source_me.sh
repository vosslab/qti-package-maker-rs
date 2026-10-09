# source_me.sh - shell environment for running this repo's Python.
# Usage: source source_me.sh && python3 ...
# This is a bash script sourced into your shell, not run directly.

# Require bash: the checks below and the repo's tab-indented shell style are
# bash-specific. Fail loudly rather than misbehave under another shell.
set | grep -q '^BASH_VERSION=' || echo "use bash for your shell"
set | grep -q '^BASH_VERSION=' || exit 1

# Source ~/.bashrc FIRST, before any repo-specific environment extension below.
# ~/.bashrc applies local shell setup (PATH, etc.) and resets some variables --
# it clears PYTHONPATH (verified). Anything that sets PYTHONPATH must run after
# this line, or ~/.bashrc would wipe it.
source ~/.bashrc

# Python runtime defaults: unbuffered stdout/stderr, and no .pyc/__pycache__
# files written on import.
export PYTHONUNBUFFERED=1
export PYTHONDONTWRITEBYTECODE=1

# Qualified development helper imports must work from any caller directory.
# Derive the repository from this script after ~/.bashrc clears PYTHONPATH.
QTI_DEVEL_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PYTHONPATH="$QTI_DEVEL_REPO_ROOT${PYTHONPATH:+:$PYTHONPATH}"
unset QTI_DEVEL_REPO_ROOT

# Corpus subprocesses run in an isolated directory with source helpers on their import path.
if [[ -n "${QTI_CORPUS_IMPORT_PATH:-}" ]]; then
	export PYTHONPATH="$QTI_CORPUS_IMPORT_PATH${PYTHONPATH:+:$PYTHONPATH}"
fi
# This host installs the Python 3.12 development-oracle modules here (see AGENTS.md).
if [[ -d /opt/homebrew/lib/python3.12/site-packages ]]; then
	export PYTHONPATH="/opt/homebrew/lib/python3.12/site-packages${PYTHONPATH:+:$PYTHONPATH}"
fi
