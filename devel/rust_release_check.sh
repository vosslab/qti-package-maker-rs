#!/usr/bin/env bash
# Run local Rust validation before preparing a manual source release.
#
# This script deliberately requires its cross-language prerequisites.  A missing
# Python oracle, native shim, or harness is a release failure, not a skipped lane.
set -euo pipefail

readonly REPOSITORY="$(git rev-parse --show-toplevel)"
cd "$REPOSITORY"

require_directory() {
	local path="$1"
	local description="$2"
	if [[ ! -d "$path" ]]; then
		echo "missing $description: $path" >&2
		exit 2
	fi
}

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

readonly PYTHON_QTI="${RUST_RELEASE_PYTHON_QTI:?set RUST_RELEASE_PYTHON_QTI to the pinned Python checkout}"
require_directory "$PYTHON_QTI" "pinned Python checkout"
require_file "$PYTHON_QTI/source_me.sh" "Python checkout bootstrap"
require_file "$PYTHON_QTI/qti_package_maker/__init__.py" "Python package"

# The repository bootstrap adds the selected checkout and local Python modules
# after ~/.bashrc.  It is required for every Python-backed xtask invocation.
export QTI_ORACLE_ROOT="$PYTHON_QTI"
source source_me.sh

python3 devel/check_rust_release_metadata.py
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
cargo build -p qti-cli --bin bbq-converter --locked

# oracle-crosscheck materializes the immutable pin used by parity.  Both commands
# execute the actual Rust and Python implementations; neither has a fallback mode.
cargo run --locked -p xtask -- oracle-crosscheck --python-qti "$PYTHON_QTI"
cargo run --locked -p xtask -- parity --fixtures
cargo run --locked -p xtask -- parity

if [[ "${RUST_RELEASE_HTML_TO_IMAGE:-0}" == "1" ]]; then
	: "${QTI_RDKIT_SHIM:?set QTI_RDKIT_SHIM for the native HTML-to-image release lane}"
	require_file "$QTI_RDKIT_SHIM" "native RDKit shim"
	cargo test -p qti-molecule --test rdkit_shim --locked -- --ignored
	cargo run --locked -p xtask -- parity --fixtures --html-to-image
	cargo run --locked -p xtask -- parity --html-to-image
fi

# Browser/Wasm acceptance is an explicit release lane. Native builds need no Node.
if [[ "${RUST_RELEASE_WASM:-0}" == "1" ]]; then
	require_command node
	require_command npm
	require_command rustup
	if [[ "$(node -p 'process.versions.node.split(".")[0]')" != "24" ]]; then
		echo "the Wasm release lane requires Node 24; select it, then rerun" >&2
		exit 2
	fi
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
