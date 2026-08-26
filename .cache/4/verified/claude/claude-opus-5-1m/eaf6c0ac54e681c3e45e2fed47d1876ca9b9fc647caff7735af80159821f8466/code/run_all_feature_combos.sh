#!/usr/bin/env bash
# Phase A/D driver: enumerate every valid feature combination from Cargo.toml
# and run `cargo check` + the full differential suite for each one.
#
# Usage: ./run_all_feature_combos.sh [extra cargo args...]
set -u

cd "$(dirname "$0")" || exit 1
CARGO_FLAGS=("--offline" "$@")

# --- build the C reference library (default configuration) ------------------
if [ ! -f c_src/build/libdriver.so ]; then
  echo "== building C reference library"
  (mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || {
    echo "C build FAILED"
    exit 1
  }
fi

# --- enumerate features from Cargo.toml ------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[ \t]*=/ { sub(/[ \t]*=.*/, ""); print }
  ' Cargo.toml
)

echo "== features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Every subset of the feature list (2^n combinations); with n == 0 this is just
# the single empty combination.
COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if ((mask & (1 << i))); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done

fail=0
run() { # run <label> <cargo-subcommand> <feature-args...>
  local label="$1"
  shift
  echo "-- $label"
  if ! timeout 600 cargo "$@" >"$TMPDIR/combo.log" 2>&1; then
    echo "   FAILED: $label"
    tail -n 30 "$TMPDIR/combo.log"
    fail=1
  else
    grep -E "test result|rows passed" "$TMPDIR/combo.log" | sed 's/^/   /'
  fi
}

for combo in "${COMBOS[@]}"; do
  label="--no-default-features --features '${combo}'"
  echo "=============================================================="
  echo "== combination: ${combo:-<none>}"
  run "cargo check  $label" check "${CARGO_FLAGS[@]}" --no-default-features --features "$combo"
  run "cargo build  $label" build "${CARGO_FLAGS[@]}" --no-default-features --features "$combo"
  run "cargo test   $label" test "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" -- --nocapture
done

# The default configuration (with whatever default features exist) as well.
echo "=============================================================="
echo "== combination: <default features>"
run "cargo check  (default)" check "${CARGO_FLAGS[@]}"
run "cargo build  (default)" build "${CARGO_FLAGS[@]}"
run "cargo test   (default)" test "${CARGO_FLAGS[@]}" -- --nocapture

# --- release profile of the Rust cdylib, tested against the same C .so ------
echo "=============================================================="
echo "== extra: release-profile Rust cdylib (panic=abort, optimised)"
if timeout 600 cargo build --offline --release >"$TMPDIR/combo.log" 2>&1; then
  DRIVER_RUST_SO="$PWD/target/release/libdriver.so" \
    run "cargo test (rust .so = release build)" test "${CARGO_FLAGS[@]}" -- --nocapture
else
  echo "   release build FAILED"
  tail -n 20 "$TMPDIR/combo.log"
  fail=1
fi

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
