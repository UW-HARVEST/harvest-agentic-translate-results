#!/usr/bin/env bash
# Full differential verification sweep.
#
# Runs tests/differential.rs (which loads BOTH the C and the Rust .so through
# libloading) for every combination of:
#   * Rust cdylib profile     : debug, release
#   * C .so optimisation level : -O0 (the CMakeLists default) and -O2
#   * cargo feature combination: enumerated from Cargo.toml (see below)
#
# Usage: ./verify_all.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
TMP="${TMPDIR:-/tmp}"
CARGO_FLAGS="--offline"

fail=0
run() { # run <label> <env assignments...> -- <cargo args...>
  local label="$1"; shift
  echo "=============================================================="
  echo "== $label"
  echo "=============================================================="
  if env "$@" ; then
    echo "-- PASS: $label"
  else
    echo "-- FAIL: $label"
    fail=1
  fi
}

# ---------------------------------------------------------------- C libraries
echo "### Building C shared libraries"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
C_SO_DEFAULT="$ROOT/c_src/build/libdriver.so"

# A second, optimised C build (kept outside c_src so nothing there is modified
# beyond the prescribed build/ directory).
C_SO_O2="$TMP/libdriver_O2.so"
cc -O2 -DNDEBUG -fPIC -shared -I"$ROOT/c_src/include" \
   "$ROOT/c_src/src/driver.c" -o "$C_SO_O2" || { echo "C -O2 build failed"; exit 1; }

# ------------------------------------------------------------- Rust libraries
echo "### Building Rust cdylibs"
( cd "$CRATE_DIR" && cargo build $CARGO_FLAGS ) || exit 1
( cd "$CRATE_DIR" && cargo build $CARGO_FLAGS --release ) || exit 1
R_SO_DEBUG="$CRATE_DIR/target/debug/libdriver.so"
R_SO_RELEASE="$CRATE_DIR/target/release/libdriver.so"

# --------------------------------------------------------------- feature list
# Enumerate the crate's features mechanically; if there are none, the only
# configurations are the default build and --no-default-features.
FEATURES_JSON=$(cd "$CRATE_DIR" && cargo metadata $CARGO_FLAGS --no-deps --format-version 1 \
                 | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["packages"][0]["features"]))')
echo "### Declared features: [${FEATURES_JSON:-none}]"

COMBOS=("")                       # default features
COMBOS+=("--no-default-features")
if [ -n "$FEATURES_JSON" ]; then
  # power set of the declared features
  read -ra FEATS <<< "$FEATURES_JSON"
  n=${#FEATS[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATS[$i]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

# ----------------------------------------------------------------- symbol diff
echo "### Symbol parity (nm -D)"
diff <(nm -D --defined-only "$C_SO_DEFAULT"  | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$R_SO_RELEASE"  | awk '{print $NF}' | sort) \
  && echo "-- PASS: exported symbol sets identical" \
  || { echo "-- FAIL: exported symbol sets differ"; fail=1; }

# --------------------------------------------------------------------- matrix
for combo in "${COMBOS[@]}"; do
  for c_variant in "O0:$C_SO_DEFAULT" "O2:$C_SO_O2"; do
    for r_variant in "debug:$R_SO_DEBUG" "release:$R_SO_RELEASE"; do
      c_name="${c_variant%%:*}"; c_path="${c_variant#*:}"
      r_name="${r_variant%%:*}"; r_path="${r_variant#*:}"
      label="features=[${combo:-default}] C=$c_name Rust=$r_name"
      log="$TMP/verify_$(echo "$label" | tr ' =[],' '_').log"
      ( cd "$CRATE_DIR" && \
        DRIVER_C_SO="$c_path" DRIVER_RUST_SO="$r_path" \
        timeout 600 cargo test $CARGO_FLAGS $combo --test differential ) >"$log" 2>&1
      status=$?
      grep -E '^(test result|test .* \.\.\. FAILED)' "$log" | tail -n 5
      if [ "$status" -eq 0 ]; then
        echo "-- PASS: $label"
      else
        echo "-- FAIL: $label  (log: $log)"
        tail -n 25 "$log"
        fail=1
      fi
    done
  done
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
