#!/usr/bin/env bash
# Run local Rust validation before preparing a manual source release.
#
# Rust release checks are self-contained and do not require the Python implementation.
set -euo pipefail

readonly REPOSITORY="$(git rev-parse --show-toplevel)"
cd "$REPOSITORY"
source source_me.sh

native_renderers=false
wasm=false
for argument in "$@"; do
	case "$argument" in
		--native-renderers) native_renderers=true ;;
		--wasm) wasm=true ;;
		-h|--help)
			echo "Usage: $0 [--native-renderers] [--wasm]"
			exit 0 ;;
		*) echo "unknown option: $argument" >&2; exit 2 ;;
	esac
done

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
cargo build -p qpm-cli --bin bbq-converter --locked

if $native_renderers; then
	cargo test -p qpm-molecule --test rdkit_shim --locked -- --ignored
fi

# Browser/Wasm acceptance is an explicit release lane. Native builds need no Node.
if $wasm; then
	require_command node
	require_command npm
	require_command rustup
	node --version
	if [[ " $(rustup target list --installed | tr '\n' ' ') " != *" wasm32-unknown-unknown "* ]]; then
		echo "install wasm32-unknown-unknown with rustup, then rerun this lane" >&2
		exit 2
	fi
	cargo check -p qpm-core -p qpm-engines -p qpm-integrity -p qpm-wasm \
		--target wasm32-unknown-unknown --locked
	(
		cd packages/qpm-wasm
		npm ci
		require_file node_modules/.bin/wasm-pack "package-local wasm-pack"
		npm run build
		npm run typecheck
		npm test
		npm run test:browser
	)
fi
