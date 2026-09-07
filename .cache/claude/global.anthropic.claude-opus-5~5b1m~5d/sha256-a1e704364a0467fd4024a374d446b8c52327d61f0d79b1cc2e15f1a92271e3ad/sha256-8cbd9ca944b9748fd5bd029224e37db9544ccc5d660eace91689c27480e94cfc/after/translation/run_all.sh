#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust .so, diffs their
# exported symbols, then runs the differential test suite across every feature
# combination and both cargo profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
LOGDIR="${TMPDIR:-/tmp}/verify-logs"
mkdir -p "$LOGDIR"
FAILED=0

say() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------- C shared lib
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# ------------------------------------------------ feature combinations
# Cargo.toml declares no [features], so the only combinations are the default
# one and the explicitly-empty one. Enumerated mechanically so that adding a
# feature later is picked up automatically.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' \
    "$CRATE/Cargo.toml"
)
COMBOS=("" "--no-default-features" "--all-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
fi
echo "features declared: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# ------------------------------------------------------------------ test loop
for PROFILE in debug release; do
  PROFILE_FLAG=""
  [ "$PROFILE" = release ] && PROFILE_FLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    LABEL="$PROFILE ${COMBO:-<default features>}"
    TAG="$(echo "$PROFILE$COMBO" | tr -c 'A-Za-z0-9' '_')"
    say "Testing: $LABEL"

    # `cargo test` does not build cdylib targets, so build the .so explicitly.
    if ! ( cd "$CRATE" && cargo build --offline $PROFILE_FLAG $COMBO ) \
         > "$LOGDIR/build-$TAG.log" 2>&1; then
      echo "BUILD FAILED ($LABEL) -- see $LOGDIR/build-$TAG.log"; FAILED=1; continue
    fi
    R_SO="$CRATE/target/$PROFILE/libmodeselect_lib.so"
    echo "Rust .so: $R_SO"

    # --- symbol parity (Phase D) ---
    nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort -u > "$LOGDIR/c.syms"
    nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort -u > "$LOGDIR/r.syms"
    MISSING="$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/r.syms")"
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED ($LABEL): missing from Rust .so:"; echo "$MISSING"; FAILED=1
    else
      echo "symbol parity: OK ($(wc -l < "$LOGDIR/c.syms") C symbols, 0 missing)"
    fi
    # non-libc undefined symbols in the Rust .so
    UNDEF="$(nm -D --undefined-only --format=posix "$R_SO" | awk '{print $1}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__cxa_|^__gmon_start__|^_Unwind_|^statx$|^gettid$' || true)"
    if [ -n "$UNDEF" ]; then
      echo "UNRESOLVED non-libc symbols in Rust .so: $UNDEF"; FAILED=1
    fi

    # --- differential tests (Phases B + C + D) ---
    if ( cd "$CRATE" && cargo test --offline $PROFILE_FLAG $COMBO ) \
         > "$LOGDIR/test-$TAG.log" 2>&1; then
      grep -hE '^test result' "$LOGDIR/test-$TAG.log"
      echo "TESTS PASSED ($LABEL)"
    else
      echo "TESTS FAILED ($LABEL) -- see $LOGDIR/test-$TAG.log"
      grep -hE '^(test result|---- .* stdout|DIVERGENCE)' "$LOGDIR/test-$TAG.log" | head -40
      FAILED=1
    fi
  done
done

# ------------------------------------------- alternate C codegen (robustness)
# Re-run the whole suite against the same C source compiled at other
# optimisation levels, to show the Rust matches the C *semantics* and not one
# particular codegen accident. `-O3` is deliberately paired with `-fwrapv`
# because at plain `-O3` gcc exploits the signed-overflow UB in
# `hash *= 0x1F` (hash_time_value) and stops agreeing with its own -O0/-O2
# output -- see SYMBOLS.md / CONFIGS.md for the measurement.
ALT="${TMPDIR:-/tmp}/alt-c"
mkdir -p "$ALT"
if command -v gcc >/dev/null; then
  for spec in "O0:-O0" "O2:-O2" "O3wrapv:-O3 -fwrapv"; do
    name="${spec%%:*}"; flags="${spec#*:}"
    so="$ALT/libc_$name.so"
    # shellcheck disable=SC2086
    if ! gcc $flags -fPIC -shared -o "$so" "$ROOT/c_src/src/lib.c" \
         -I"$ROOT/c_src/include" 2>"$LOGDIR/altbuild-$name.log"; then
      echo "skipping alt C build $name (compile failed)"; continue
    fi
    say "Testing: release <default features> vs C built with $flags"
    if ( cd "$CRATE" && DIFF_C_SO="$so" cargo test --offline --release ) \
         > "$LOGDIR/test-alt-$name.log" 2>&1; then
      grep -hE '^test result' "$LOGDIR/test-alt-$name.log"
      echo "TESTS PASSED (C $flags)"
    else
      echo "TESTS FAILED (C $flags) -- see $LOGDIR/test-alt-$name.log"; FAILED=1
    fi
  done
fi

say "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (logs in $LOGDIR)"
fi
exit "$FAILED"
