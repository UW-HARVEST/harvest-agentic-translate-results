#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination from Cargo.toml and run
# the full differential suite under each one, plus the symbol diff.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"
C_BUILD="$ROOT/c_src/build"
FAIL=0

echo "=== 1. build the C shared library ==="
( mkdir -p "$C_BUILD" && cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$C_BUILD"/lib*.so | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== 2. enumerate feature combinations ==="
# Every feature name declared under [features] (excluding "default").
FEATURES=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the only configuration is the default build"
  COMBOS+=("DEFAULT")
  COMBOS+=("NO_DEFAULT")
else
  # Full power set of the declared features, plus the plain default build.
  FARR=($FEATURES)
  N=${#FARR[@]}
  COMBOS+=("DEFAULT")
  for ((mask=0; mask<(1<<N); mask++)); do
    combo=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("NO_DEFAULT${combo:+:$combo}")
  done
fi
printf 'combination: %s\n' "${COMBOS[@]}"

echo
echo "=== 3. cargo check + build + test per combination ==="
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)        ARGS=() ;;
    NO_DEFAULT)     ARGS=(--no-default-features) ;;
    NO_DEFAULT:*)   ARGS=(--no-default-features --features "${combo#NO_DEFAULT:}") ;;
  esac

  echo
  echo "--- [$combo] cargo check ---"
  timeout 600 cargo check --release "${ARGS[@]}" 2>&1 | tail -3 || FAIL=1

  echo "--- [$combo] cargo build --release ---"
  timeout 600 cargo build --release "${ARGS[@]}" 2>&1 | tail -3 || FAIL=1

  RS_SO="target/release/libmodeselect_lib.so"
  echo "--- [$combo] symbol diff (C vs Rust) ---"
  diff <(nm -D --defined-only "$C_SO"  | grep -vE ' [a-z] ' | awk '{print $NF}' \
          | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort) \
       <(nm -D --defined-only "$RS_SO" | grep -vE ' [a-z] ' | awk '{print $NF}' | sort) \
    && echo "symbol diff: EMPTY (parity OK)" \
    || { echo "symbol diff: NON-EMPTY -> FAIL"; FAIL=1; }

  echo "--- [$combo] cargo test (debug + release) ---"
  for prof in "" "--release"; do
    timeout 600 cargo test $prof "${ARGS[@]}" 2>&1 \
      | grep -E '^(test result:|error)' || FAIL=1
    timeout 600 cargo test $prof "${ARGS[@]}" >/dev/null 2>&1 \
      || { echo "  ${prof:-debug} FAILED"; FAIL=1; }
  done
done

echo
echo "=== 4. mutation check (does the suite actually detect divergence?) ==="
if command -v python3 >/dev/null; then
  timeout 3000 python3 mutation_check.py 2>&1 | tail -5 || FAIL=1
else
  echo "python3 unavailable; skipping"
fi

echo
if [ "$FAIL" -eq 0 ]; then
  echo "=== ALL COMBINATIONS PASSED ==="
else
  echo "=== FAILURES DETECTED (see above) ==="
fi
exit "$FAIL"
