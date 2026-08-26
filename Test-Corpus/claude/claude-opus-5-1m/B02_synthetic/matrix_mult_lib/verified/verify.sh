#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration, check it compiles,
# compare exported symbols against the C .so and run the full differential test
# suite (Phases B + C) for each configuration, against BOTH the dev-profile and
# the release-profile Rust shared library.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOG="${TMPDIR:-/tmp}/verify-$$"
mkdir -p "$LOG"
rc_total=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (single CMake configuration, no options).
# ---------------------------------------------------------------------------
say "building C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG/cmake.log" 2>&1 \
  || { echo "C build FAILED (see $LOG/cmake.log)"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate the Cargo feature combinations (the [features] table is empty ⇒
#    the only combination is "no features", which equals the default build).
# ---------------------------------------------------------------------------
FEATURES=$(awk '/^\[features\]/{flag=1;next} /^\[/{flag=0} flag && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  COMBOS=("")            # only the empty combination exists
else
  # power set of all declared features
  COMBOS=("")
  for f in $FEATURES; do
    new=()
    for c in "${COMBOS[@]}"; do
      new+=("$c")
      if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
  done
fi
say "feature combinations: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  [${c:-<none>}]"; done

# ---------------------------------------------------------------------------
# 3. For every combination: check, build (dev + release), symbol-diff, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  say "cargo check --no-default-features --features '$combo'  ($label)"
  cargo check --offline --no-default-features --features "$combo" --all-targets \
      > "$LOG/check-$label.log" 2>&1 \
    || { echo "CHECK FAILED for [$label] (see $LOG/check-$label.log)"; rc_total=1; continue; }
  echo "check OK"

  for profile in dev release; do
    if [ "$profile" = release ]; then
      cargo build --offline --release --no-default-features --features "$combo" \
        > "$LOG/build-$label-$profile.log" 2>&1
      RUST_SO="$ROOT/target/release/libdriver.so"
    else
      cargo build --offline --no-default-features --features "$combo" \
        > "$LOG/build-$label-$profile.log" 2>&1
      RUST_SO="$ROOT/target/debug/libdriver.so"
    fi
    [ $? -eq 0 ] || { echo "BUILD FAILED [$label/$profile]"; rc_total=1; continue; }

    say "symbol parity [$label/$profile]"
    nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u > "$LOG/c.syms"
    nm -D --defined-only "$RUST_SO"| awk '{print $3}' | sort -u > "$LOG/rust.syms"
    missing=$(comm -23 "$LOG/c.syms" "$LOG/rust.syms")
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST .so:"; echo "$missing"; rc_total=1
    else
      echo "all $(wc -l < "$LOG/c.syms") C symbols are exported by the Rust .so"
    fi
    undef=$(ldd -r "$RUST_SO" 2>&1 | grep -i "undefined symbol")
    if [ -n "$undef" ]; then
      echo "UNRESOLVED SYMBOLS IN RUST .so:"; echo "$undef"; rc_total=1
    else
      echo "no unresolved symbols in the Rust .so"
    fi

    say "differential tests [$label/$profile]"
    DRIVER_RUST_SO="$RUST_SO" DRIVER_C_SO="$C_SO" \
      timeout 600 cargo test --offline --no-default-features --features "$combo" \
        -- --test-threads=1 > "$LOG/test-$label-$profile.log" 2>&1
    if [ $? -ne 0 ]; then
      echo "TESTS FAILED [$label/$profile] (see $LOG/test-$label-$profile.log)"
      grep -E "^(test .* FAILED|failures:|thread)" -A 5 "$LOG/test-$label-$profile.log" | head -60
      rc_total=1
    else
      grep -E "^test result:" "$LOG/test-$label-$profile.log"
    fi
  done
done

say "SUMMARY"
if [ "$rc_total" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES DETECTED (logs in $LOG)"
fi
exit "$rc_total"
