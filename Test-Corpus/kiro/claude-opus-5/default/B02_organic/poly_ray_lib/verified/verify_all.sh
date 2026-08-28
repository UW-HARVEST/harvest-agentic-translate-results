#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth across every
# build-time configuration.
#
#   1. enumerate the [features] table in Cargo.toml -> every valid combination
#   2. cargo check each combination
#   3. cargo test each combination (against the C .so)
#   4. compare `nm -D` exports for each combination
#
# Usage: ./verify_all.sh            (from translation/)
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOGDIR="${TMPDIR:-/tmp}/poly_ray_verify"
mkdir -p "$LOGDIR"
FAILED=0

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library
# ---------------------------------------------------------------------------
note "building C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOGDIR/cmake.log" 2>&1 \
  || { fail "C build (see $LOGDIR/cmake.log)"; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | tail -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
# Read feature names from the [features] section (ignoring "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
if [ "$N" -eq 0 ]; then
  note "no [features] declared in Cargo.toml -- single configuration"
  COMBOS=("")
else
  note "features: ${FEATURES[*]}"
  COMBOS=()
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2-4. check / test / symbol-parity per combination, per profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ "$N" -eq 0 ]; then
    FLAGS=()
    label="default"
  else
    FLAGS=(--no-default-features)
    [ -n "$combo" ] && FLAGS+=(--features "$combo")
    label="${combo:-<none>}"
  fi

  for profile in dev release; do
    tag="$(echo "$label-$profile" | tr ',<> ' '____')"
    PFLAGS=()
    [ "$profile" = release ] && PFLAGS=(--release)

    note "cargo check [$label] [$profile]"
    if ! timeout 600 cargo check "${FLAGS[@]}" "${PFLAGS[@]}" \
         > "$LOGDIR/check-$tag.log" 2>&1; then
      fail "cargo check [$label] [$profile] -- see $LOGDIR/check-$tag.log"
      tail -20 "$LOGDIR/check-$tag.log"
      continue
    fi

    note "cargo build (cdylib) [$label] [$profile]"
    if ! timeout 600 cargo build "${FLAGS[@]}" "${PFLAGS[@]}" \
         > "$LOGDIR/build-$tag.log" 2>&1; then
      fail "cargo build [$label] [$profile] -- see $LOGDIR/build-$tag.log"
      tail -20 "$LOGDIR/build-$tag.log"
      continue
    fi

    pdir=$([ "$profile" = release ] && echo release || echo debug)
    RS_SO="target/$pdir/libpoly_ray_lib.so"
    if [ ! -f "$RS_SO" ]; then
      fail "no Rust .so at $RS_SO [$label] [$profile]"
      continue
    fi

    note "nm -D parity [$label] [$profile]"
    nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > "$LOGDIR/c.syms"
    nm -D --defined-only "$RS_SO" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > "$LOGDIR/rs-$tag.syms"
    missing="$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/rs-$tag.syms")"
    if [ -n "$missing" ]; then
      fail "symbols missing from Rust .so [$label] [$profile]:"
      echo "$missing"
    else
      echo "all $(wc -l < "$LOGDIR/c.syms") C symbols present"
    fi

    # `cargo test` always builds the test harness with the dev profile; point
    # it explicitly at the .so for the profile under verification so that both
    # dev and release codegen are compared against C.
    note "cargo test [$label] [$profile cdylib]"
    if ! RUST_SO_PATH="$PWD/$RS_SO" C_SO_PATH="$C_SO" \
         timeout 600 cargo test --no-fail-fast "${FLAGS[@]}" \
         > "$LOGDIR/test-$tag.log" 2>&1; then
      fail "cargo test [$label] [$profile] -- see $LOGDIR/test-$tag.log"
      grep -E '^(test result|---- |thread )' "$LOGDIR/test-$tag.log" | head -40
    else
      grep -E '^test result' "$LOGDIR/test-$tag.log"
    fi
  done
done

note "summary"
if [ "$FAILED" -eq 0 ]; then
  echo "PASS: all ${#COMBOS[@]} feature combination(s) verified against C, dev + release."
else
  echo "FAILURES present -- see $LOGDIR"
fi
exit "$FAILED"
