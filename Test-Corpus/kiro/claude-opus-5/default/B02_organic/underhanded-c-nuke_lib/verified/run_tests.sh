#!/usr/bin/env bash
# Full verification run: build both shared objects, then run every test suite
# under every feature combination.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target, so the
# `cargo build --release` before each `cargo test` is load-bearing -- without it
# the tests silently dlopen a stale .so. `tests/common/mod.rs` also guards
# against this, but belt and braces.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
TIMEOUT="${TIMEOUT:-600}"

echo "== building the C shared library =="
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "   $C_SO"

cd "$CRATE"

# --- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys
m=json.load(sys.stdin)
for p in m["packages"]:
    for f in p["features"]:
        if f != "default": print(f)'
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: the default (empty) set is the only configuration.
  COMBOS+=("--no-default-features")
  COMBOS+=("")
else
  COMBOS+=("--no-default-features")
  COMBOS+=("")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

echo "== ${#COMBOS[@]} feature combination(s) to verify: ${FEATURES[*]:-<none declared>} =="

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "===================================================================="
  echo "== feature combination: $label"
  echo "===================================================================="

  # shellcheck disable=SC2086
  timeout "$TIMEOUT" cargo build --release $combo

  nm -D --defined-only target/release/libunderhanded_c_nuke_lib.so |
    awk '{print $2, $3}' | sort >/tmp/rust_syms.txt
  nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort >/tmp/c_syms.txt
  echo "-- symbol diff (C vs Rust), must be empty --"
  if diff /tmp/c_syms.txt /tmp/rust_syms.txt; then
    echo "   OK: identical export sets"
  else
    echo "   MISMATCH"
    FAIL=1
  fi

  for suite in symbols configs errors nan_payload_search; do
    echo "-- cargo test --release --test $suite $label --"
    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo test --release $combo --test "$suite" -- --test-threads=4; then
      FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
  exit 1
fi
