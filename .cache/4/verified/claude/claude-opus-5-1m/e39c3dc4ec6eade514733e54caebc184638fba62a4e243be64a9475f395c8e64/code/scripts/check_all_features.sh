#!/usr/bin/env bash
# Phase D: enumerate EVERY build-time configuration and run `cargo check` plus
# the full differential test suite against each one.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so the script stays correct if features are ever added.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
LOG=${TMPDIR:-/tmp}/verify_$$
mkdir -p "$LOG"
fail=0

# --------------------------------------------------------------------------
# 1. Enumerate the [features] section of Cargo.toml
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "=== Build-time configuration surface ==="
if [ ${#FEATURES[@]} -eq 0 ]; then
  echo "Cargo.toml declares NO [features] -> exactly one configuration."
  COMBOS=("")
else
  echo "features: ${FEATURES[*]}"
  # full power set
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "total combinations: ${#COMBOS[@]}"

# The C side has no configuration axis either -- verify that mechanically.
echo
echo "=== C build-time configuration surface ==="
if grep -qE '^[[:space:]]*(option|add_definitions|target_compile_definitions|if)\b' c_src/CMakeLists.txt; then
  echo "NOTE: CMakeLists.txt contains conditionals; inspect manually:"
  grep -nE '^[[:space:]]*(option|add_definitions|target_compile_definitions|if)\b' c_src/CMakeLists.txt
else
  echo "c_src/CMakeLists.txt declares no option()/conditional -> one configuration."
fi
ifdefs=$(grep -cE '^[[:space:]]*#[[:space:]]*(ifdef|ifndef|if[[:space:]])' c_src/src/lib.c c_src/include/lib.h | paste -sd' ')
echo "conditional-compilation directives in the C sources: $ifdefs"

# --------------------------------------------------------------------------
# 2. Build the C shared library
# --------------------------------------------------------------------------
echo
echo "=== Building the C shared library ==="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .) \
  > "$LOG/cmake.log" 2>&1
if [ $? -ne 0 ]; then
  echo "FAIL: C build"; tail -30 "$LOG/cmake.log"; exit 1
fi
C_SO=c_src/build/libtranslated_rust.so
echo "ok -> $C_SO"

# --------------------------------------------------------------------------
# 3. cargo check + symbol parity + full test suite, per combination
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    args=(--no-default-features)
    label="<no features>"
  else
    args=(--no-default-features --features "$combo")
    label="$combo"
  fi
  echo
  echo "############################################################"
  echo "# configuration: $label"
  echo "############################################################"

  echo "--- cargo check ${args[*]}"
  if ! timeout 600 cargo check "${args[@]}" > "$LOG/check.log" 2>&1; then
    echo "FAIL: cargo check [$label]"; tail -40 "$LOG/check.log"; fail=1; continue
  fi
  # warnings are worth surfacing but are not failures
  grep -E '^(warning|error)' "$LOG/check.log" | sort -u | head -10
  echo "ok"

  echo "--- cargo build ${args[*]} (dev + release cdylib)"
  if ! timeout 600 cargo build "${args[@]}" > "$LOG/build.log" 2>&1 \
     || ! timeout 600 cargo build --release "${args[@]}" >> "$LOG/build.log" 2>&1; then
    echo "FAIL: cargo build [$label]"; tail -40 "$LOG/build.log"; fail=1; continue
  fi
  echo "ok"

  echo "--- symbol parity (nm -D)"
  for RS_SO in target/debug/libarr_push_lib.so target/release/libarr_push_lib.so; do
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u))
    nc=$(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u | wc -l)
    nr=$(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u | wc -l)
    if [ -n "$missing" ]; then
      echo "FAIL: $RS_SO is missing symbols exported by the C .so:"
      echo "$missing"
      fail=1
    else
      echo "ok: $RS_SO exports all $nc C symbols (has $nr defined symbols)"
    fi
  done

  echo "--- cargo test ${args[*]}"
  if ! timeout 600 cargo test "${args[@]}" > "$LOG/test.log" 2>&1; then
    echo "FAIL: cargo test [$label]"
    grep -E "^(test |error|thread|failures:|DIVERG)" "$LOG/test.log" | tail -60
    fail=1; continue
  fi
  grep -E "^test result:" "$LOG/test.log"
  total=$(grep -Eo '^test result: ok\. [0-9]+' "$LOG/test.log" | awk '{s+=$4} END{print s}')
  echo "ok: $total tests passed"
done

echo
echo "############################################################"
if [ "$fail" -eq 0 ]; then
  echo "# ALL CONFIGURATIONS PASSED"
else
  echo "# FAILURES DETECTED"
fi
echo "############################################################"
exit "$fail"
