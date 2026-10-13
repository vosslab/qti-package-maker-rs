#!/usr/bin/env bash
# Run the complete local gate in a native-architecture Debian container; macOS uses Podman.
# No tag or GitHub contact. This recipe checks a local committed source clone.
set -euo pipefail
readonly REPOSITORY="$(git rev-parse --show-toplevel)"
readonly RUST_VERSION="$(awk -F '\"' '/^rust-version = / { print $2; exit }' "$REPOSITORY/Cargo.toml")"
: "${RUST_VERSION:?workspace Cargo.toml must declare rust-version}"
# Use the workspace requirement; the build enforces it and prerequisites.txt records the toolchain.
rust_image="docker.io/library/rust:${RUST_VERSION}-trixie"
smoke_only=false
while [[ $# -gt 0 ]]; do
	case "$1" in
		--image) rust_image="${2:?--image requires a value}"; shift 2 ;;
		--smoke-only) smoke_only=true; shift ;;
		-h|--help)
			echo "Usage: $0 [--image IMAGE] [--smoke-only]"
			exit 0 ;;
		*) echo "unknown option: $1" >&2; exit 2 ;;
	esac
done
readonly RUST_IMAGE="$rust_image"
readonly EVIDENCE_ROOT="$REPOSITORY/output_tables/linux_release_check/run_$$"
readonly SOURCE_DIR="$EVIDENCE_ROOT/source"
command -v podman >/dev/null

mkdir -p "$EVIDENCE_ROOT"
git clone --no-hardlinks --depth 1 "$REPOSITORY" "$SOURCE_DIR"

podman run --rm --workdir /src \
	-v "$SOURCE_DIR:/src" \
	-v "$EVIDENCE_ROOT:/work" \
	-e "CARGO_HOME=/work/cargo-home" \
	-e "CARGO_TARGET_DIR=/work/target" \
	"$RUST_IMAGE" bash -ceu '
		test -f /root/.bashrc || : > /root/.bashrc
		apt-get update
		apt-get install --yes --no-install-recommends \
			ca-certificates chromium chromium-sandbox file g++ git libboost-dev libcairo2-dev librdkit-dev \
			python3 python3-packaging
		source source_me.sh
		rustup component add rustfmt clippy
		bash crates/qpm-molecule/native/build_shim.sh --rdkit-prefix /usr \
			--rdkit-library-dir "/usr/lib/$(gcc -dumpmachine)" --cairo-prefix /usr
		{
			uname -m
			rustc -Vv
			python3 --version
			chromium --version
			g++ --version | head -1
			dpkg-query --show --showformat='\''${Package} ${Version} ${Architecture}\n'\'' \
				librdkit-dev librdkit1t64 libcairo2-dev
			sha256sum crates/qpm-molecule/dist/libqpm_rdkit_shim.so
			git -C /src rev-parse HEAD
		} | tee /work/prerequisites.txt
		# Chromium needs a non-root sandbox user; only disposable source/cache mounts are chowned.
		useradd --create-home qpm
		chown -R qpm:qpm /work /src
		runuser -u qpm -- chromium --headless --dump-dom about:blank > /work/chromium_smoke.html
		if [ "$1" = true ]; then
			exit 0
		fi
		runuser -u qpm -- bash devel/rust_release_check.sh --native-renderers
	' bash "$smoke_only"

printf 'Linux release evidence: %s\n' "$EVIDENCE_ROOT"
