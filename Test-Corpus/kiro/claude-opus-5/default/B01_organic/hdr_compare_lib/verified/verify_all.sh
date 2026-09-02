#!/usr/bin/env bash
# Phase D driver: rebuild both shared objects, then run the whole differential
# suite once per Cargo feature combination.
#
# Usage: ./verify_all.sh            (from translation/)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

step "Build the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C  .so: $C_SO"

step "Enumerate feature combinations from Cargo.toml"
# Features declared under [features], excluding the implicit `default`.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}
' Cargo.toml)

COMBOS=()
if [[ -z "$FEATURES" ]]; then
  echo "no [features] declared -> the only configuration is the default build"
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  echo "declared features: $FEATURES"
  # Full power set of declared features, plus the default build.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  COMBOS+=("default:")
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FARR[$i]}"); done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("no-default[${joined}]:--no-default-features --features=${joined}")
  done
fi

for entry in "${COMBOS[@]}"; do
  name=${entry%%:*}
  flags=${entry#*:}
  step "cargo check   [$name] $flags"
  # shellcheck disable=SC2086
  timeout 600 cargo check $flags 2>&1 | tail -3 || { echo "check FAILED [$name]"; FAIL=1; continue; }

  step "cargo build --release   [$name] $flags"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $flags 2>&1 | tail -3 || { echo "build FAILED [$name]"; FAIL=1; continue; }

  step "nm -D symbol parity   [$name]"
  R_SO="target/release/libhdr_compare_lib.so"
  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [[ -n "$missing" ]]; then
    echo "MISSING from Rust .so [$name]:"; echo "$missing"; FAIL=1
  else
    echo "symbol diff empty ($(echo "$c_syms" | wc -l) C symbol(s), all present in Rust)"
  fi

  step "cargo test   [$name] $flags"
  # shellcheck disable=SC2086
  timeout 600 cargo test $flags 2>&1 | grep -E '^(test result|error|failures:|running)' \
    || { echo "test FAILED [$name]"; FAIL=1; }
  # shellcheck disable=SC2086
  timeout 600 cargo test $flags >/dev/null 2>&1 || { echo "test FAILED [$name]"; FAIL=1; }
done

step "RESULT"
if [[ $FAIL -eq 0 ]]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
