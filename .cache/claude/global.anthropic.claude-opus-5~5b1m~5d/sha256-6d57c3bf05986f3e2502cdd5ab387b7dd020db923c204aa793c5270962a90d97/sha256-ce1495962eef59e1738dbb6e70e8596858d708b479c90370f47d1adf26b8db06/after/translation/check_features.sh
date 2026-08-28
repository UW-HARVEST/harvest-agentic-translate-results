#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
#
#   ./check_features.sh
#
# 1. builds the C shared object and the Rust cdylib (release *and* dev)
# 2. diffs `nm -D` of both .so files in both directions
# 3. enumerates the crate's features from Cargo.toml, builds the power set and
#    runs `cargo check` + the whole differential suite for each combination
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="--offline"
TMP="${TMPDIR:-/tmp}"
fail=0

echo "=== 1. build the C shared object ============================================"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so)
echo "C  : $C_SO"

echo "=== 2. build the Rust cdylib (dev + release) ================================"
cargo build $CARGO_FLAGS          >/dev/null 2>&1 || { echo "dev build FAILED"; fail=1; }
cargo build $CARGO_FLAGS --release >/dev/null 2>&1 || { echo "release build FAILED"; fail=1; }
R_SO=translation_release
for R_SO in target/release/libhelxo_lib.so target/debug/libhelxo_lib.so; do
  echo "Rust: $R_SO"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort >"$TMP/.c.syms.$$"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort >"$TMP/.r.syms.$$"
  missing=$(comm -23 "$TMP/.c.syms.$$" "$TMP/.r.syms.$$")
  if [ -n "$missing" ]; then
    echo "  MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol diff: empty ($(wc -l <"$TMP/.c.syms.$$") exported symbols)"
  fi
  extra=$(comm -13 "$TMP/.c.syms.$$" "$TMP/.r.syms.$$" | grep -v -E '^(rust_eh_personality|__rust|_ZN|rust_)' )
  [ -n "$extra" ] && { echo "  extra Rust exports (informational):"; echo "$extra" | sed 's/^/    /'; }
  # undefined, non-libc symbols
  und=$(nm -D -u "$R_SO" | awk '{print $2}' | grep -v -E '^(_ITM_|__gmon_start__|__cxa_|_Unwind|__tls|__gcc)' |
        grep -v -E '@GLIBC|@GCC' )
  [ -n "$und" ] && { echo "  UNDEFINED non-libc symbols:"; echo "$und" | sed 's/^/    /'; fail=1; }
  rm -f "$TMP/.c.syms.$$" "$TMP/.r.syms.$$"
done

echo "=== 3. feature combinations ================================================="
# mechanically extract the [features] table
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");gsub(/"/,"");print}' Cargo.toml |
        grep -v '^default$' | tr -d ' ')
n=0
combos=()
if [ -z "$feats" ]; then
  echo "no [features] table -> exactly one configuration (default == no features)"
  combos=("<default>")
else
  arr=($feats)
  total=$((1 << ${#arr[@]}))
  for ((m = 0; m < total; m++)); do
    sel=""
    for ((i = 0; i < ${#arr[@]}; i++)); do
      (((m >> i) & 1)) && sel="$sel,${arr[$i]}"
    done
    combos+=("${sel#,}")
  done
fi

for combo in "${combos[@]}"; do
  n=$((n + 1))
  if [ "$combo" = "<default>" ]; then
    flags=""
    label="default"
  else
    flags="--no-default-features --features $combo"
    label="$combo"
  fi
  echo "--- combination $n: $label"
  cargo check $CARGO_FLAGS --tests $flags >/dev/null 2>&1 || { echo "  cargo check FAILED"; fail=1; }
  for prof in "" "--release"; do
    out=$(cargo test $CARGO_FLAGS $prof $flags 2>&1)
    res=$(echo "$out" | grep -c '^test result: ok')
    bad=$(echo "$out" | grep -E '^test result: FAILED|^error' | head -3)
    if [ -n "$bad" ]; then
      echo "  ${prof:---dev} FAILED:"; echo "$bad" | sed 's/^/    /'; fail=1
    else
      echo "  ${prof:---dev} ok ($res test binaries passed)"
    fi
  done
done

echo "============================================================================"
[ $fail -eq 0 ] && echo "PHASE D: OK" || echo "PHASE D: FAILURES"
exit $fail
