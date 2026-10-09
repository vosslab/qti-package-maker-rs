#!/usr/bin/env bash
# Run local Rust validation before preparing a manual source release.
#
# Rust release checks are self-contained. Current-Python migration comparisons
# are explicit `cargo xtask` commands and are not routine release requirements.
set -euo pipefail

readonly REPOSITORY="$(git rev-parse --show-toplevel)"
cd "$REPOSITORY"
source source_me.sh

require_file() {
	local path="$1"
	local description="$2"
	if [[ ! -f "$path" ]]; then
		echo "missing $description: $path" >&2
		exit 2
	fi
}

require_command() {
	local name="$1"
	if ! command -v "$name" >/dev/null 2>&1; then
		echo "missing command: $name; install it, then rerun this lane" >&2
		exit 2
	fi
}

python3 devel/check_rust_release_metadata.py
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
cargo build -p qti-cli --bin bbq-converter --locked

if [[ "${RUST_RELEASE_HTML_TO_IMAGE:-0}" == "1" ]]; then
	: "${QTI_RDKIT_SHIM:?set QTI_RDKIT_SHIM for the native HTML-to-image release lane}"
	require_file "$QTI_RDKIT_SHIM" "native RDKit shim"
	cargo test -p qti-molecule --test rdkit_shim --locked -- --ignored
fi

# Browser/Wasm acceptance is an explicit release lane. Native builds need no Node.
if [[ "${RUST_RELEASE_WASM:-0}" == "1" ]]; then
	require_command node
	require_command npm
	require_command rustup
	node --version
	if [[ " $(rustup target list --installed | tr '\n' ' ') " != *" wasm32-unknown-unknown "* ]]; then
		echo "install wasm32-unknown-unknown with rustup, then rerun this lane" >&2
		exit 2
	fi
	cargo check -p qti-core -p qti-engines -p qti-integrity -p qti-wasm \
		--target wasm32-unknown-unknown --locked
	(
		cd packages/qti-wasm
		npm ci
		require_file node_modules/.bin/wasm-pack "package-local wasm-pack"
		npm run build
		npm run typecheck
		npm test
		npm run test:browser
	)
fi
