#!/usr/bin/env bash
# Phase D driver: build the C .so, then run the whole differential suite for
# every Cargo feature combination x every build profile, and diff `nm -D`
# between the two artefacts each time.
#
# Usage:  ./run_all_tests.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_BUILD="$ROOT/c_src/build"
CARGO_FLAGS="--offline --manifest-path $CRATE_DIR/Cargo.toml"
RUST_SO_NAME="libgaussian_kernel_lib.so"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
mkdir -p "$CRATE_DIR/.verify"

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth)
# ---------------------------------------------------------------------------
note "building C shared library"
mkdir -p "$C_BUILD"
cmake -S "$ROOT/c_src" -B "$C_BUILD" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  || { echo "cmake configure FAILED"; exit 1; }
cmake --build "$C_BUILD" >/dev/null || { echo "cmake build FAILED"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
[ -n "$C_SO" ] || { echo "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
COMBOS+=("default:")                       # implicit default features
COMBOS+=("no-default:--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "declared features: ${FEATURES[*]}"
  COMBOS+=("all-features:--all-features")
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel+=("${FEATURES[$i]}"); fi
    done
    joined="$(
      IFS=,
      echo "${sel[*]}"
    )"
    COMBOS+=("feat[$joined]:--no-default-features --features $joined")
  done
else
  echo "declared features: (none) -- only the default/no-default configurations exist"
fi

# ---------------------------------------------------------------------------
# 3. For each (combo x profile): build cdylib, diff nm -D, run the suite
# ---------------------------------------------------------------------------
for entry in "${COMBOS[@]}"; do
  label="${entry%%:*}"
  flags="${entry#*:}"
  for profile in debug release; do
    if [ "$profile" = "release" ]; then prof_flag="--release"; else prof_flag=""; fi
    note "combo=$label profile=$profile flags='$flags'"

    # shellcheck disable=SC2086
    if ! cargo build $CARGO_FLAGS $prof_flag $flags >/dev/null 2>&1; then
      echo "BUILD FAILED (cargo build)"
      fail=1
      continue
    fi
    RUST_SO="$CRATE_DIR/target/$profile/$RUST_SO_NAME"
    if [ ! -f "$RUST_SO" ]; then
      echo "missing $RUST_SO"
      fail=1
      continue
    fi

    # --- symbol parity -----------------------------------------------------
    c_syms="$(nm -D --defined-only "$C_SO" |
      awk '$2 ~ /^[TtDdBbWwVv]$/ {print $3}' |
      grep -vE '^(_init|_fini|__bss_start|_edata|_end|__.*)$' | sort -u)"
    r_syms="$(nm -D --defined-only "$RUST_SO" |
      awk '$2 ~ /^[TtDdBbWwVv]$/ {print $3}' |
      grep -vE '^(_init|_fini|__bss_start|_edata|_end|__.*)$' | sort -u)"
    missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
    if [ -n "$missing" ]; then
      echo "SYMBOL DIFF NOT EMPTY -- Rust is missing:"
      echo "$missing" | sed 's/^/    /'
      fail=1
    else
      echo "symbol diff: EMPTY ($(echo "$c_syms" | wc -l) C API symbol(s) all present)"
    fi

    # --- differential test suite -------------------------------------------
    # shellcheck disable=SC2086
    RUST_DIFF_SO="$RUST_SO" cargo test $CARGO_FLAGS $prof_flag $flags 2>&1 |
      tee "$CRATE_DIR/.verify/test-$label-$profile.log" |
      grep -E '^(test result|error|warning: unused)' | sed 's/^/    /'
    # shellcheck disable=SC2086
    if grep -qE '^test result: FAILED|^error' "$CRATE_DIR/.verify/test-$label-$profile.log"; then
      echo "TESTS FAILED for combo=$label profile=$profile"
      fail=1
    fi
  done
done

note "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL feature combinations x profiles: symbol diff empty, all tests passed."
else
  echo "FAILURES DETECTED -- see the logs in $CRATE_DIR/.verify/"
fi
exit "$fail"
