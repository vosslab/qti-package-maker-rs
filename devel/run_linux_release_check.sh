#!/usr/bin/env bash
# Run the complete local release gate in an isolated Debian amd64 Podman container.
#
# This is a local, emulated-linux receipt on an arm64 macOS host. It neither creates a
# tag nor contacts GitHub. The source clone, Cargo cache, target directory, native shim,
# and Python oracle copy are all ignored evidence below output_tables/.
set -euo pipefail

readonly REPOSITORY="$(git rev-parse --show-toplevel)"
readonly PINNED_ORACLE=55e5f368777f7809fe2e91b5d070caf6df0cb581
readonly RUST_VERSION=1.98.1
# The amd64 manifest of rust:1.98.1-trixie (Debian 13) inspected on 2026-09-30.
readonly RUST_IMAGE="${QTI_LINUX_RELEASE_IMAGE:-docker.io/library/rust@sha256:f31fa9eaac4e417505e10009786ec0c636558e02dff3fa662cc92c10894ba065}"
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

podman run --rm --arch amd64 --workdir /src \
	-v "$SOURCE_DIR:/src" \
	-v "$EVIDENCE_ROOT:/work" \
	-e "RUST_RELEASE_PYTHON_QTI=/work/oracle_source" \
	-e "RUST_RELEASE_HTML_TO_IMAGE=1" \
	-e "QTI_RDKIT_SHIM=/work/shim/libqti_rdkit_shim.so" \
	-e "CARGO_HOME=/work/cargo-home" \
	-e "CARGO_TARGET_DIR=/work/target" \
	-e "QTI_LINUX_RELEASE_SMOKE_ONLY=${QTI_LINUX_RELEASE_SMOKE_ONLY:-0}" \
	"$RUST_IMAGE" bash -ceu '
		test -f /root/.bashrc || : > /root/.bashrc
		apt-get update
		apt-get install --yes --no-install-recommends \
			ca-certificates file g++ git libboost-dev libcairo2-dev librdkit-dev \
			python3 python3-defusedxml python3-lxml python3-numpy python3-yaml
		test "$(rustc --version | awk '\''{print $2}'\'')" = "'"$RUST_VERSION"'"
		test "$(python3 -c '\''import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")'\'')" = "3.13"
		test "$(g++ -dumpversion | cut -d. -f1)" -ge 14
		RDKIT_PREFIX=/usr \
		RDKIT_LIBRARY_DIR=/usr/lib/x86_64-linux-gnu \
		CAIRO_PREFIX=/usr \
		OUTPUT_DIR=/work/shim \
		bash crates/qti-molecule/native/build_shim.sh
		{
			uname -m
			rustc -Vv
			python3 --version
			g++ --version | head -1
			dpkg-query --show --showformat='\''${Package} ${Version} ${Architecture}\n'\'' \
				librdkit-dev librdkit1t64 libcairo2-dev
			sha256sum /work/shim/libqti_rdkit_shim.so
			git -C /src rev-parse HEAD
			git -C /work/oracle_source rev-parse HEAD
		} | tee /work/prerequisites.txt
		if [ "${QTI_LINUX_RELEASE_SMOKE_ONLY:-0}" = 1 ]; then
			exit 0
		fi
		bash devel/rust_release_check.sh
	'

printf 'Linux release evidence: %s\n' "$EVIDENCE_ROOT"
