#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 0 ]]; then
  echo 'Usage: ./build_all.sh' >&2
  echo 'Build native CLIs, the RDKit shim, and the BPW Wasm package.' >&2
  [[ "$1" == --help || "$1" == -h ]] && exit 0
  exit 2
fi

readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
cd "$REPO_ROOT"

for tool in cargo rustup node npm; do
  command -v "$tool" >/dev/null || { echo "Required tool missing: $tool" >&2; exit 1; }
done
node -e 'if (Number(process.versions.node.split(".")[0]) < 24) { console.error("Node 24 or newer is required"); process.exit(1); }'

echo 'Building the native RDKit shim (requires RDKit/Cairo SDKs)...'
bash crates/qpm-molecule/native/build_shim.sh
echo 'Building native release binaries...'
cargo build --locked --release -p qpm-cli --bins
echo 'Building the BPW browser package...'
rustup target add wasm32-unknown-unknown
cd packages/qpm-wasm
npm ci
npm run build

cat <<'EOF'
Build complete:
  target/release/bbq-converter
  target/release/qpm
  crates/qpm-molecule/dist/ (native RDKit shim)
  packages/qpm-wasm/dist/ (BPW browser package)

HTML table image export also requires Chromium at runtime.
Use BPW's devel/vendor_qpm_wasm.py to build and copy its browser assets.
EOF
