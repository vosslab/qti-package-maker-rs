#!/usr/bin/env bash
# Run the complete local gate in a native-architecture Debian container; macOS uses Podman.
# No tag or GitHub contact. Source clone, Cargo/target caches, shim and Python oracle stay under ignored output_tables/.
set -euo pipefail
readonly REPOSITORY="$(git rev-parse --show-toplevel)"
readonly PINNED_ORACLE=55e5f368777f7809fe2e91b5d070caf6df0cb581
readonly RUST_VERSION=1.98.1
# Debian 13 multi-architecture index verified 2026-10-01.
readonly RUST_IMAGE="${QTI_LINUX_RELEASE_IMAGE:-docker.io/library/rust@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546}"
readonly EVIDENCE_ROOT="$REPOSITORY/output_tables/linux_release_check/run_$$"
readonly SOURCE_DIR="$EVIDENCE_ROOT/source"
readonly ORACLE_REPOSITORY="${QTI_LINUX_RELEASE_ORACLE_SOURCE:-$REPOSITORY/../qti-package-maker}"
readonly ORACLE_SOURCE="$EVIDENCE_ROOT/oracle_source"
require_directory() {
	local path="$1"
	local description="$2"
	if [[ ! -d "$path" ]]; then
		echo "missing $description: $path" >&2
		exit 2
	fi
}

require_directory "$REPOSITORY/output_tables/corpus" "harvested corpus"
require_directory "$ORACLE_REPOSITORY" "Python oracle Git checkout"
git -C "$ORACLE_REPOSITORY" rev-parse "${PINNED_ORACLE}^{commit}" >/dev/null
command -v podman >/dev/null

mkdir -p "$EVIDENCE_ROOT"
git clone --no-hardlinks --depth 1 "$REPOSITORY" "$SOURCE_DIR"
git clone --no-hardlinks "$ORACLE_REPOSITORY" "$ORACLE_SOURCE"
git -C "$ORACLE_SOURCE" checkout --detach "$PINNED_ORACLE"
mkdir -p "$SOURCE_DIR/output_tables"
cp -R "$REPOSITORY/output_tables/corpus" "$SOURCE_DIR/output_tables/corpus"

podman run --rm --workdir /src \
	-v "$SOURCE_DIR:/src" \
	-v "$EVIDENCE_ROOT:/work" \
	-e "RUST_RELEASE_PYTHON_QTI=/work/oracle_source" \
	-e "QTI_ORACLE_ROOT=/work/oracle_source" \
	-e "RUST_RELEASE_HTML_TO_IMAGE=1" \
	-e "QTI_RDKIT_SHIM=/work/shim/libqti_rdkit_shim.so" \
	-e "CARGO_HOME=/work/cargo-home" \
	-e "CARGO_TARGET_DIR=/work/target" \
	-e "QTI_CHROMIUM=/usr/bin/chromium" \
	-e "PLAYWRIGHT_BROWSERS_PATH=/work/playwright" \
	-e "QTI_LINUX_RELEASE_SMOKE_ONLY=${QTI_LINUX_RELEASE_SMOKE_ONLY:-0}" \
	"$RUST_IMAGE" bash -ceu '
		test -f /root/.bashrc || : > /root/.bashrc
		apt-get update
		apt-get install --yes --no-install-recommends \
			ca-certificates chromium chromium-sandbox file g++ git libboost-dev libcairo2-dev librdkit-dev \
			python3 python3-crcmod python3-defusedxml python3-lxml python3-num2words \
			python3-numpy python3-pip python3-rdkit python3-tabulate python3-yaml
		source source_me.sh
		# Current lxml is needed by the pinned Python oracle; upgrades stay in this disposable container.
		python3 -m pip install --upgrade --break-system-packages lxml playwright
		python3 -m playwright install chromium
		test "$(rustc --version | awk '\''{print $2}'\'')" = "'"$RUST_VERSION"'"
		test "$(python3 -c '\''import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")'\'')" = "3.13"
		test "$(g++ -dumpversion | cut -d. -f1)" -ge 14
		rustup component add rustfmt clippy
		RDKIT_PREFIX=/usr \
		RDKIT_LIBRARY_DIR="/usr/lib/$(gcc -dumpmachine)" \
		CAIRO_PREFIX=/usr \
		OUTPUT_DIR=/work/shim \
		bash crates/qti-molecule/native/build_shim.sh
		{
			uname -m
			rustc -Vv
			python3 --version
			python3 -m pip list --format=freeze
			python3 - <<PY
import subprocess
import playwright.sync_api

with playwright.sync_api.sync_playwright() as playwright:
	print("Playwright Chromium:", playwright.chromium.executable_path, flush=True)
	subprocess.run([playwright.chromium.executable_path, "--version"], check=True)
PY
			chromium --version
			g++ --version | head -1
			dpkg-query --show --showformat='\''${Package} ${Version} ${Architecture}\n'\'' \
				librdkit-dev librdkit1t64 libcairo2-dev
			sha256sum /work/shim/libqti_rdkit_shim.so
			git -C /src rev-parse HEAD
			git -C /work/oracle_source rev-parse HEAD
		} | tee /work/prerequisites.txt
		# Chromium needs a non-root sandbox user; only disposable source/cache mounts are chowned.
		useradd --create-home qti
		chown -R qti:qti /work /src
		runuser -u qti -- chromium --headless --dump-dom about:blank > /work/chromium_smoke.html
		runuser -u qti -- python3 -c "from qti_package_maker import package_interface; from qti_package_maker.html_to_image import render_table, render_canvas; import rdkit"
		if [ "${QTI_LINUX_RELEASE_SMOKE_ONLY:-0}" = 1 ]; then
			exit 0
		fi
		runuser -u qti -- bash devel/rust_release_check.sh
	'

printf 'Linux release evidence: %s\n' "$EVIDENCE_ROOT"
