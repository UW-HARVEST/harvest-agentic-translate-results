#!/usr/bin/env bash
# Phase D driver: symbol parity + full B/C suite under every feature
# combination and both profiles. Run from translation/.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "===================== rebuild C ====================="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# ---- enumerate feature combinations from Cargo.toml ----------------------
FEATS=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

COMBOS=()
if [ -z "$FEATS" ]; then
  echo "No [features] in Cargo.toml -> only the default (empty) feature set."
  COMBOS+=("__default__" "__nodefault__")
else
  FARR=($FEATS)
  N=${#FARR[@]}
  COMBOS+=("__default__" "__nodefault__")
  for ((m=1; m<(1<<N); m++)); do
    c=""
    for ((i=0; i<N; i++)); do
      if (( m & (1<<i) )); then c="${c:+$c,}${FARR[$i]}"; fi
    done
    COMBOS+=("$c")
  done
fi
echo "feature combinations to verify: ${COMBOS[*]}"

run_combo() {
  local combo="$1" profile="$2" flags=() pdir
  case "$combo" in
    __default__)   flags=() ;;
    __nodefault__) flags=(--no-default-features) ;;
    *)             flags=(--no-default-features --features "$combo") ;;
  esac
  if [ "$profile" = release ]; then flags+=(--release); pdir=release; else pdir=debug; fi

  echo
  echo "----- combo=$combo profile=$profile -----"
  timeout 300 cargo build "${flags[@]}" >/dev/null 2>&1 \
    || { echo "  BUILD FAILED"; FAIL=1; return; }

  local R_SO="target/$pdir/libget_predict_func_lib.so"
  [ -f "$R_SO" ] || { echo "  MISSING $R_SO"; FAIL=1; return; }

  # symbol parity for THIS build
  local d
  d=$(comm -3 \
        <(nm -D --defined-only --format=posix "$C_SO" | awk '$2=="T"{print $1}' | sort) \
        <(nm -D --defined-only --format=posix "$R_SO" | awk '$2=="T"{print $1}' | sort))
  if [ -n "$d" ]; then
    echo "  SYMBOL DIFF NON-EMPTY:"; echo "$d" | sed 's/^/    /'; FAIL=1
  else
    echo "  symbol diff: EMPTY (ok)"
  fi

  local out
  out=$(timeout 600 cargo test "${flags[@]}" 2>&1)
  if [ $? -ne 0 ]; then
    echo "  TESTS FAILED"; echo "$out" | tail -30 | sed 's/^/    /'; FAIL=1
  else
    echo "$out" | grep -E '^test result:' | sed 's/^/  /'
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    run_combo "$combo" "$profile"
  done
done

echo
if [ $FAIL -eq 0 ]; then echo "===== PHASE D: ALL COMBINATIONS PASS ====="; else echo "===== PHASE D: FAILURES PRESENT ====="; fi
exit $FAIL
