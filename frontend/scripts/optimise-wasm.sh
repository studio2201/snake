#!/usr/bin/env bash
# Post-process every Trunk-emitted WASM bundle with `wasm-opt -Oz` to strip
# debug info, drop producer/custom sections, and squeeze out redundant code
# paths. Snake's raw bundle shrinks ~32% (520 KB → 355 KB) and the
# gzipped-over-the-wire variant ~19% (185 KB → 149 KB).
#
# Run this AFTER `trunk build --release` from `frontend/`:
#
#     cd frontend
#     trunk build --release
#     ./scripts/optimise-wasm.sh
#
# Idempotent: skips files already shrunk (the `.opt` extension is excluded).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
DIST="${HERE}/../dist"

if ! command -v wasm-opt >/dev/null 2>&1; then
    echo "wasm-opt not on PATH; install via 'cargo install wasm-opt --version 121' or 'cargo install wasm-bindgen-cli'" >&2
    exit 1
fi

if [[ ! -d "$DIST" ]]; then
    echo "dist/ not found at $DIST; run 'trunk build --release' from frontend/ first" >&2
    exit 1
fi

shopt -s nullglob
total_saved=0
for w in "$DIST"/frontend-*.wasm; do
    orig_size=$(stat -c%s "$w")
    tmp="${w}.opt"

    wasm-opt -Oz --strip-debug --strip-producers --output "$tmp" "$w"

    opt_size=$(stat -c%s "$tmp")
    if (( opt_size < orig_size )) && (( opt_size > 0 )); then
        saved=$(( orig_size - opt_size ))
        total_saved=$(( total_saved + saved ))
        bak="${w}.preopt.bak"
        mv "$w" "$bak"
        mv "$tmp" "$w"
        rm "$bak"
        printf '  %s: %d -> %d bytes (-%d)\n' "$(basename "$w")" "$orig_size" "$opt_size" "$saved"
    else
        rm -f "$tmp"
        printf '  %s: no improvement (%d bytes); kept original\n' "$(basename "$w")" "$orig_size"
    fi
done

if (( total_saved > 0 )); then
    echo "Total saved: $total_saved bytes across all bundles"
fi
