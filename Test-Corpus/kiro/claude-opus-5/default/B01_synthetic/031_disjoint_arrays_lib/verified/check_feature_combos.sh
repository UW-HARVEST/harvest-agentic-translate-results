#!/usr/bin/env bash
# Phase D sweep: run the whole differential suite under every cargo feature
# combination AND against both the debug and the release build of the Rust
# cdylib, since the release profile turns on optimisation and `panic = "abort"`.
#
# Feature combinations are read out of Cargo.toml rather than hardcoded, so a
# feature added later is picked up automatically instead of silently skipped.
#
# Usage: ./check_feature_combos.sh
set -uo pipefail

cd "$(dirname "$0")"
FAILED=0
TIMEOUT=${TIMEOUT:-600}

# ---------------------------------------------------------------------------
# Make sure the C reference library exists.
# ---------------------------------------------------------------------------
C_SO="../c_src/build/libdriver.so"
if [[ ! -f "$C_SO" ]]; then
  echo "== building the C reference library =="
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
# Pull the feature names out of the [features] table, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

# Always cover the default build and the empty feature set. Then add the powerset
# of the declared features (capped, so a large feature table cannot explode).
COMBOS=("default:" "no-default-features:")
N=${#FEATURES[@]}
if (( N > 0 )); then
  if (( N > 12 )); then
    echo "note: $N features declared; testing singles and the all-features set only"
    for f in "${FEATURES[@]}"; do COMBOS+=("no-default-features:$f"); done
    COMBOS+=("no-default-features:$(IFS=,; echo "${FEATURES[*]}")")
  else
    for (( mask = 1; mask < (1 << N); mask++ )); do
      sel=()
      for (( i = 0; i < N; i++ )); do
        (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
      done
      COMBOS+=("no-default-features:$(IFS=,; echo "${sel[*]}")")
    done
  fi
fi

echo "== ${#COMBOS[@]} feature combination(s) to check =="
printf '   %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# Run cargo check + the full test suite for each combination.
# ---------------------------------------------------------------------------
run() {
  local label="$1"; shift
  echo
  echo "---- $label ----"
  if timeout "$TIMEOUT" "$@" >/tmp/combo.log 2>&1; then
    tail -n 3 /tmp/combo.log | sed 's/^/     /'
    echo "     PASS: $label"
  else
    echo "     FAIL: $label"
    tail -n 40 /tmp/combo.log | sed 's/^/     /'
    FAILED=1
  fi
}

for combo in "${COMBOS[@]}"; do
  mode="${combo%%:*}"
  feats="${combo#*:}"
  args=()
  [[ "$mode" == "no-default-features" ]] && args+=(--no-default-features)
  [[ -n "$feats" ]] && args+=(--features "$feats")
  desc="${mode}${feats:+ +$feats}"

  run "cargo check   [$desc]" cargo check "${args[@]}"
  run "cargo test    [$desc]" cargo test "${args[@]}"
done

# ---------------------------------------------------------------------------
# Re-run the suite against the RELEASE cdylib.
#
# `cargo test --release` cannot be used directly: [profile.release] sets
# `panic = "abort"`, which the test harness cannot work with. Instead build the
# release cdylib and point the harness at it with DRIVER_RUST_SO, so the exact
# optimised, panic=abort artifact is the one compared against the C.
# ---------------------------------------------------------------------------
echo
echo "== release-profile cdylib =="
for combo in "${COMBOS[@]}"; do
  mode="${combo%%:*}"
  feats="${combo#*:}"
  args=()
  [[ "$mode" == "no-default-features" ]] && args+=(--no-default-features)
  [[ -n "$feats" ]] && args+=(--features "$feats")
  desc="${mode}${feats:+ +$feats}"

  run "cargo build --release [$desc]" cargo build --release "${args[@]}"
  DRIVER_RUST_SO="$PWD/target/release/libdriver.so" \
    run "cargo test vs release .so [$desc]" cargo test "${args[@]}"
done

echo
if (( FAILED )); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi

# ---------------------------------------------------------------------------
# The C reference at several optimisation levels.
#
# `c_src/CMakeLists.txt` sets no CMAKE_BUILD_TYPE, so the reference build is
# unoptimised, and the translation's use of wrapping arithmetic rests on gcc
# emitting plain two's-complement imul/add for the signed overflow in
# `fma_array` (which is UB per the standard, so an optimiser is entitled to
# assume it never happens). Rebuilding the C out-of-tree at higher optimisation
# levels and re-running the suite checks that the translation does not silently
# depend on -O0. c_src/ itself is never touched: the build trees go to /tmp.
# ---------------------------------------------------------------------------
echo "== C reference at several optimisation levels =="
# Use the debug cdylib cargo just built, deterministically: the release leg above
# leaves DRIVER_RUST_SO set (bash keeps a `VAR=x func` assignment in the shell
# after the function returns), and relying on that would be accidental.
unset DRIVER_RUST_SO
C_SRC_DIR="$(cd .. && pwd)/c_src"
i=0
for opt in "-O0" "-O1" "-O2" "-O3" "-Os" "-O3 -march=native" "-Ofast"; do
  i=$((i + 1))
  d="/tmp/driver_c_opt_$i"
  rm -rf "$d"
  if ! cmake -S "$C_SRC_DIR" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >/dev/null 2>&1 \
     || ! cmake --build "$d" >/dev/null 2>&1; then
    echo "     SKIP: C would not build with '$opt' on this toolchain"
    continue
  fi
  export DRIVER_C_SO="$d/libdriver.so"
  run "cargo test vs C built with '$opt'" cargo test
  unset DRIVER_C_SO
done

echo
if (( FAILED )); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all configurations PASSED"
