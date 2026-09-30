#!/usr/bin/env bash
# Run Playwright; build dist/ first when requested or required.
set -euo pipefail

usage() {
	printf '%s\n' "Usage: run_playwright_tests.sh [-h|--help] [--build] [PLAYWRIGHT_ARGS...]" \
		"" "  -h, --help  Print this help and exit 0." \
		"  --build      Force a dist/ rebuild before running tests."
}

FORCE_BUILD=0
PLAYWRIGHT_ARGS=()

while [ "$#" -gt 0 ]; do
	case "$1" in
		-h|--help)
			usage
			exit 0
			;;
		--build)
			FORCE_BUILD=1
			shift
			;;
		*)
			PLAYWRIGHT_ARGS+=("$1")
			shift
			;;
	esac
done

cd "$(git rev-parse --show-toplevel)"
if ! command -v node >/dev/null 2>&1; then
	echo "ERROR: node not found on PATH. Install Node.js first." >&2
	exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
	echo "ERROR: npm not found on PATH. Install Node.js first." >&2
	exit 1
fi

if [ ! -d node_modules ]; then
	echo "ERROR: node_modules/ missing. Run 'npm install' first." >&2
	exit 1
fi

if [ ! -f playwright.config.ts ]; then
	echo "ERROR: playwright.config.ts not found at repo root." >&2
	exit 1
fi

if [ "$FORCE_BUILD" -eq 1 ]; then
	echo "==> --build flag set: rebuilding dist/..."
	bash build_github_pages.sh
elif [ ! -f dist/index.html ] || [ ! -f dist/main.js ]; then
	echo "==> dist/index.html or dist/main.js missing: running build_github_pages.sh..."
	bash build_github_pages.sh
fi

echo "==> npx playwright test ${PLAYWRIGHT_ARGS[*]+"${PLAYWRIGHT_ARGS[*]}"}"
if npx playwright test ${PLAYWRIGHT_ARGS[@]+"${PLAYWRIGHT_ARGS[@]}"}; then
	echo "PASS: playwright tests passed."
else
	PW_EXIT=$?
	echo "FAIL: playwright tests failed (exit code $PW_EXIT)."
	exit "$PW_EXIT"
fi
