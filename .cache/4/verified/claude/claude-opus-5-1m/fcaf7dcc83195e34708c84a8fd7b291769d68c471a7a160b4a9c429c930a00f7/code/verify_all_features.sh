#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination mechanically from Cargo.toml
# and run `cargo check` + the full differential test suite for each one.
#
#   Phase A step 1-2 : enumerate combos, `cargo check` each
#   Phase D          : re-run Phases B and C for every combo
#
# Usage: ./verify_all_features.sh
set -uo pipefail
cd "$(dirname "$0")"

# ---------------------------------------------------------------- enumerate
# Every key in the [features] table, minus the implicit "default" entry.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { in_f = 1; next }
    /^\[/                { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "== [features] in Cargo.toml: $n ${FEATURES[*]:-(none)}"

# The power set of the feature list. With n = 0 this is exactly one combo: the
# empty set, i.e. --no-default-features on its own.
COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done
echo "== valid feature combinations: ${#COMBOS[@]}"

# ---------------------------------------------------------------- build C .so
echo
echo "== building the C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find c_src/build -maxdepth 1 -name 'lib*.so' | head -1)
echo "   C .so: $C_SO"

# ---------------------------------------------------------------- run combos
rc=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(no features / default)"}
  args=(--no-default-features)
  [[ -n $combo ]] && args+=(--features "$combo")

  echo
  echo "================================================================"
  echo "== feature combination: $label"
  echo "================================================================"

  echo "-- cargo check"
  if ! timeout 600 cargo check "${args[@]}" 2>&1 | tail -3; then
    echo "   CHECK FAILED for $label"; rc=1; continue
  fi

  echo "-- cargo build (cdylib, both profiles)"
  timeout 600 cargo build "${args[@]}" --target-dir target/cdylib-under-test >/dev/null 2>&1
  timeout 600 cargo build "${args[@]}" --release --target-dir target/cdylib-under-test >/dev/null 2>&1

  echo "-- nm -D symbol parity (C .so vs Rust .so)"
  for prof in debug release; do
    rust_so="target/cdylib-under-test/$prof/libhalf2float_lib.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u))
    if [[ -n $missing ]]; then
      echo "   MISSING from $prof .so: $missing"; rc=1
    else
      echo "   $prof: 0 missing symbols"
    fi
  done

  echo "-- differential tests (Phase B + Phase C)"
  # HALF2FLOAT_FEATURES tells the harness which features to build the cdylib with.
  if HALF2FLOAT_FEATURES="$combo" timeout 600 cargo test "${args[@]}" 2>&1 \
       | grep -E '^(test result|error|thread)'; then :; fi
  if ! HALF2FLOAT_FEATURES="$combo" timeout 600 cargo test "${args[@]}" >/dev/null 2>&1; then
    echo "   TESTS FAILED for $label"; rc=1
  else
    echo "   all tests passed for $label"
  fi
done

echo
if ((rc == 0)); then
  echo "== ALL ${#COMBOS[@]} feature combination(s) passed check + symbol parity + tests"
else
  echo "== FAILURES were reported above"
fi
exit $rc
