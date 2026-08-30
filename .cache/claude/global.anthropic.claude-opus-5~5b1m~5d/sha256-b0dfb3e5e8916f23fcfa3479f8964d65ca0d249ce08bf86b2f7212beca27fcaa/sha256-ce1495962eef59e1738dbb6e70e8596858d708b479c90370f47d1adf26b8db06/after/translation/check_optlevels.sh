#!/usr/bin/env bash
# Extra robustness gate: re-run the whole differential suite against the C
# library compiled at every optimisation level.
#
# Rationale: `sum += update` and `i * stride` in staticloop.c are signed-overflow
# UB. Matching the default CMake build (which passes no -O flag, i.e. -O0) does
# not by itself prove the Rust matches an optimised C build, and many of the
# ERRORS.md rows sit exactly on those overflow boundaries. This builds the C
# source out-of-tree (c_src is never modified) and points the suite at each
# result via STATICLOOP_C_SO.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
SRC="$ROOT/c_src/src/staticloop.c"
INC="$ROOT/c_src/include"
OUT="${TMPDIR:-/tmp}/staticloop_optlevels"
mkdir -p "$OUT"

cd "$HERE"
cargo build --offline 2>/dev/null

overall=0
for opt in -O0 -O1 -O2 -O3 -Os; do
  so="$OUT/libStaticLoop${opt}.so"
  if ! cc -shared -fPIC "$opt" -I"$INC" -o "$so" "$SRC" 2>&1; then
    echo "### cc $opt FAILED to build" >&2
    overall=1
    continue
  fi

  echo
  echo "=============================================================="
  echo "### C built with $opt  ($so)"
  echo "=============================================================="
  if STATICLOOP_C_SO="$so" timeout 600 cargo test --offline --no-fail-fast 2>&1 \
      | grep -E "^(test result|error\[|error:)"; then
    :
  fi
  if ! STATICLOOP_C_SO="$so" timeout 600 cargo test --offline --no-fail-fast >/dev/null 2>&1; then
    echo "DIVERGENCE at $opt" >&2
    overall=1
  fi
done

echo
if [[ "$overall" -eq 0 ]]; then
  echo "ALL C OPTIMISATION LEVELS AGREE WITH THE RUST .so: PASS"
else
  echo "SOME OPTIMISATION LEVELS DIVERGED" >&2
fi
exit "$overall"
