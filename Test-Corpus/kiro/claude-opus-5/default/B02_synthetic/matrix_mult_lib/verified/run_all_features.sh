#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature combinations are derived mechanically from Cargo.toml rather than
# hard-coded: if a [features] table is ever added, every subset is exercised.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
FAIL=0

# --- 1. (re)build the C ground truth ---------------------------------------
echo "== building C shared library =="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/../c_src/build/libdriver.so"
ls -l "$C_SO"

# --- 2. enumerate feature combinations ------------------------------------
# Names under [features], excluding "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] != "default" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== no [features] in Cargo.toml: the complete combination set is {default} =="
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  N=${#FEATURES[@]}
  COMBOS+=("default:")
  for ((mask = 0; mask < (1 << N); mask++)); do
    sel=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then sel="$sel,${FEATURES[$i]}"; fi
    done
    sel="${sel#,}"
    COMBOS+=("no-default[$sel]:--no-default-features --features $sel")
  done
fi

# --- 3. cargo check + full test run per combination ------------------------
for entry in "${COMBOS[@]}"; do
  label="${entry%%:*}"
  flags="${entry#*:}"
  echo
  echo "=============================================================="
  echo "== combination: $label   (flags: ${flags:-<none>})"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --release $flags 2>&1 | tail -5; then
    echo "!! cargo check FAILED for $label"; FAIL=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $flags 2>&1 | tail -3; then
    echo "!! cargo build FAILED for $label"; FAIL=1; continue
  fi

  echo "-- symbol diff (C -> Rust) --"
  diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort) \
       > /tmp/symdiff.$$ 
  # Lines starting with '<' are symbols present in C but missing from Rust.
  if grep -q '^<' /tmp/symdiff.$$; then
    echo "!! MISSING SYMBOLS in Rust .so for $label:"; grep '^<' /tmp/symdiff.$$; FAIL=1
  else
    echo "   OK: 0 C symbols missing from the Rust .so"
  fi
  rm -f /tmp/symdiff.$$

  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $flags 2>&1 | grep -E "^test result|^error|FAILED|panicked"; then
    echo "!! cargo test FAILED for $label"; FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
