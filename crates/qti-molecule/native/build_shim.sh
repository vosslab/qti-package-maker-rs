#!/usr/bin/env bash
set -euo pipefail

# Build ABI v1 of the optional RDKit renderer. This is a release-build action,
# separate from Cargo: qti-molecule must compile when no RDKit SDK is present.
readonly NATIVE_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly OUTPUT_DIR="${OUTPUT_DIR:-$NATIVE_DIR/../dist}"
readonly RDKIT_PREFIX="${RDKIT_PREFIX:?set RDKIT_PREFIX to the RDKit installation prefix}"
readonly RDKIT_LIBRARY_DIR="${RDKIT_LIBRARY_DIR:-$RDKIT_PREFIX/lib}"
readonly BOOST_PREFIX="${BOOST_PREFIX:-}"
readonly CAIRO_PREFIX="${CAIRO_PREFIX:-}"
readonly CXX_BIN="${CXX:-c++}"
readonly CXXFLAGS_TEXT="${CXXFLAGS:-}"
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

includes=(-I"$RDKIT_PREFIX/include/rdkit")
libraries=(-L"$RDKIT_LIBRARY_DIR" -Wl,-rpath,"$RDKIT_LIBRARY_DIR")
if [[ -n "$BOOST_PREFIX" ]]; then
  includes+=(-I"$BOOST_PREFIX/include")
fi
if [[ -n "$CAIRO_PREFIX" ]]; then
  includes+=(-I"$CAIRO_PREFIX/include/cairo")
  libraries+=(-L"$CAIRO_PREFIX/lib" -Wl,-rpath,"$CAIRO_PREFIX/lib")
fi

command=("$CXX_BIN" -std=c++23 -O2)
if [[ -n "$CXXFLAGS_TEXT" ]]; then
  command+=("${CXXFLAGS_ARRAY[@]}")
fi
command+=("${SHARED[@]}" "${includes[@]}" "$NATIVE_DIR/qti_rdkit_shim.cpp"
  -o "$OUTPUT_DIR/libqti_rdkit_shim.$EXT" "${libraries[@]}"
  -lRDKitMolDraw2D -lRDKitDepictor -lRDKitSubstructMatch -lRDKitSmilesParse
  -lRDKitGraphMol -lRDKitRDGeometryLib -lRDKitDataStructs -lRDKitRDGeneral -lcairo)
"${command[@]}"
