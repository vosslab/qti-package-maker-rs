#!/usr/bin/env bash
set -euo pipefail

# Build ABI v1 of the optional RDKit renderer. This release-build action is
# separate from Cargo: qpm-molecule compiles without an RDKit SDK.
readonly NATIVE_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly DEFAULT_PREFIX="$([[ -d /opt/homebrew/include/rdkit ]] && printf /opt/homebrew || printf /usr)"
readonly CXX_BIN="${CXX:-c++}"
readonly CXXFLAGS_TEXT="${CXXFLAGS:-}"
RDKIT_PREFIX="$DEFAULT_PREFIX"
RDKIT_LIBRARY_DIR=""
BOOST_PREFIX="$DEFAULT_PREFIX"
CAIRO_PREFIX="$DEFAULT_PREFIX"
OUTPUT_DIR="$NATIVE_DIR/../dist"

usage() {
  cat <<'EOF'
usage: build_shim.sh [options]

Build the optional QPM RDKit native shim.

  --rdkit-prefix PATH       RDKit SDK prefix (default: /opt/homebrew or /usr)
  --rdkit-library-dir PATH  RDKit library directory (default: prefix/lib or Debian multiarch)
  --boost-prefix PATH       Boost SDK prefix (default: RDKit prefix)
  --cairo-prefix PATH       Cairo SDK prefix (default: RDKit prefix)
  --output-dir PATH         Shim destination (default: crates/qpm-molecule/dist)
  -h, --help                Show this help

CXX and CXXFLAGS remain available as standard compiler environment variables.
EOF
}

while (($#)); do
  case "$1" in
    --rdkit-prefix) RDKIT_PREFIX="$2"; shift 2 ;;
    --rdkit-library-dir) RDKIT_LIBRARY_DIR="$2"; shift 2 ;;
    --boost-prefix) BOOST_PREFIX="$2"; shift 2 ;;
    --cairo-prefix) CAIRO_PREFIX="$2"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ ! -f "$RDKIT_PREFIX/include/rdkit/GraphMol/MolDraw2D/MolDraw2DCairo.h" ]]; then
  echo "RDKit C++ SDK header missing: $RDKIT_PREFIX/include/rdkit/GraphMol/MolDraw2D/MolDraw2DCairo.h" >&2
  echo 'Install SDKs: macOS: brew install rdkit cairo boost' >&2
  echo '              Debian: sudo apt-get install librdkit-dev libcairo2-dev g++' >&2
  echo 'For a nonstandard SDK, pass --rdkit-prefix PATH to build_shim.sh.' >&2
  exit 1
fi

if [[ -z "$RDKIT_LIBRARY_DIR" ]]; then
  if [[ -d /usr/lib/x86_64-linux-gnu && "$RDKIT_PREFIX" == /usr ]]; then
    RDKIT_LIBRARY_DIR=/usr/lib/x86_64-linux-gnu
  else
    RDKIT_LIBRARY_DIR="$RDKIT_PREFIX/lib"
  fi
fi
if [[ -n "$CXXFLAGS_TEXT" ]]; then
  read -r -a CXXFLAGS_ARRAY <<< "$CXXFLAGS_TEXT"
else
  CXXFLAGS_ARRAY=()
fi

mkdir -p "$OUTPUT_DIR"
case "$(uname -s)" in
  Darwin)
    readonly EXT=dylib
    readonly SHARED=(-dynamiclib -fPIC)
    ;;
  Linux)
    readonly EXT=so
    readonly SHARED=(-shared -fPIC)
    ;;
  *)
    echo "unsupported platform: $(uname -s)" >&2
    exit 2
    ;;
esac

includes=(-I"$RDKIT_PREFIX/include/rdkit" -I"$BOOST_PREFIX/include" -I"$CAIRO_PREFIX/include/cairo")
libraries=(-L"$RDKIT_LIBRARY_DIR" -Wl,-rpath,"$RDKIT_LIBRARY_DIR")
libraries+=(-L"$CAIRO_PREFIX/lib")
if [[ "$CAIRO_PREFIX/lib" != "$RDKIT_LIBRARY_DIR" ]]; then
  libraries+=(-Wl,-rpath,"$CAIRO_PREFIX/lib")
fi

command=("$CXX_BIN" -std=c++23 -O2)
if [[ -n "$CXXFLAGS_TEXT" ]]; then
  command+=("${CXXFLAGS_ARRAY[@]}")
fi
command+=("${SHARED[@]}" "${includes[@]}" "$NATIVE_DIR/qpm_rdkit_shim.cpp"
  -o "$OUTPUT_DIR/libqpm_rdkit_shim.$EXT" "${libraries[@]}"
  -lRDKitMolDraw2D -lRDKitDepictor -lRDKitSubstructMatch -lRDKitSmilesParse
  -lRDKitGraphMol -lRDKitRDGeometryLib -lRDKitDataStructs -lRDKitRDGeneral -lcairo)
"${command[@]}"

# Shared libraries are loaded by the renderer, not executed as programs.
chmod 644 "$OUTPUT_DIR/libqpm_rdkit_shim.$EXT"
