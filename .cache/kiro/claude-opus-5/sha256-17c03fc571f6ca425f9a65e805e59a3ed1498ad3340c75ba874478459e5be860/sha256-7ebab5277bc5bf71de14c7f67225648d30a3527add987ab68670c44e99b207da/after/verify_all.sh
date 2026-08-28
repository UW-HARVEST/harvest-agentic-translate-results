#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs `cargo check` and
# `cargo test` for each, against both the debug and release Rust cdylib.
#
# Usage: ./verify_all.sh          (from the repo root)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR="${TMPDIR:-/tmp}/harvest-verify"
mkdir -p "$LOGDIR"

fail=0

# --- 1. Build the C reference shared library -------------------------------
echo "== building C reference =="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) > "$LOGDIR/cmake.log" 2>&1 || { echo "C build FAILED (see $LOGDIR/cmake.log)"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -name '*.so' | sort | tail -1)"
echo "   C .so: $C_SO"

# --- 2. Enumerate feature combinations -------------------------------------
# Read the [features] table from Cargo.toml; skip "default" (covered by the
# empty combination plus an explicit default run).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, p, "="); gsub(/[[:space:]]/, "", p[1]);
      if (p[1] != "default") print p[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  echo "== no [features] declared: single configuration =="
  COMBOS=("")
else
  # Power set of the declared features.
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]@Q} =="

# --- 3. cargo check / build / test for every combination --------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"
  echo
  echo "===== combination: $label ====="

  featargs=(--no-default-features)
  [[ -n "$combo" ]] && featargs+=(--features "$combo")

  for step in check build; do
    if ! (cd "$CRATE" && timeout 600 cargo "$step" "${featargs[@]}") \
          > "$LOGDIR/$step-$slug.log" 2>&1; then
      echo "  cargo $step (debug)   FAILED -> $LOGDIR/$step-$slug.log"
      fail=1; continue 2
    fi
    echo "  cargo $step (debug)   ok"
  done

  if ! (cd "$CRATE" && timeout 600 cargo build --release "${featargs[@]}") \
        > "$LOGDIR/build-rel-$slug.log" 2>&1; then
    echo "  cargo build (release) FAILED -> $LOGDIR/build-rel-$slug.log"
    fail=1; continue
  fi
  echo "  cargo build (release) ok"

  # Run the differential + symbol suites against each profile's cdylib.
  for profile in debug release; do
    so="$CRATE/target/$profile/libmemchra2_lib.so"
    [[ -f "$so" ]] || { echo "  $profile .so missing"; fail=1; continue; }
    if (cd "$CRATE" && RUST_LIB_PATH="$so" C_LIB_PATH="$C_SO" \
          timeout 600 cargo test --release "${featargs[@]}") \
          > "$LOGDIR/test-$profile-$slug.log" 2>&1; then
      echo "  cargo test vs $profile .so  ok"
    else
      echo "  cargo test vs $profile .so  FAILED -> $LOGDIR/test-$profile-$slug.log"
      fail=1
    fi
  done

  # Symbol parity, independent of the test harness.
  missing="$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $3}' | sort -u) \
    <(nm -D --defined-only "$CRATE/target/release/libmemchra2_lib.so" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $3}' | sort -u))"
  if [[ -n "$missing" ]]; then
    echo "  symbol parity FAILED; Rust .so missing: $(echo "$missing" | tr '\n' ' ')"
    fail=1
  else
    echo "  symbol parity         ok"
  fi
done

echo
if (( fail )); then
  echo "RESULT: FAILURES (logs in $LOGDIR)"
  exit 1
fi
echo "RESULT: all combinations match the C reference"
