#!/usr/bin/env bash
# Builds the C reference .so and the Rust cdylib, then runs the differential
# test suite for every valid feature combination declared in Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/parity

mkdir -p "$LOG"
rc=0

echo "== enumerating feature combinations =="
# Collect feature names from the [features] table (ignoring "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  echo "   no [features] table -> single configuration"
  COMBOS=("")
else
  # Power set of all declared features.
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "   ${#COMBOS[@]} combination(s): ${COMBOS[*]@Q}"

echo "== building C reference library =="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" &&
  timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
  timeout 600 cmake --build .) >"$LOG/cmake.log" 2>&1 ||
  { echo "   C build FAILED (see $LOG/cmake.log)"; tail -20 "$LOG/cmake.log"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "   $C_SO"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  slug="${combo//,/_}"; slug="${slug:-default}"
  echo
  echo "########## features: $label ##########"

  featargs=(--no-default-features)
  [ -n "$combo" ] && featargs+=(--features "$combo")

  echo "-- cargo check"
  if ! (cd "$CRATE" && timeout 600 cargo check "${featargs[@]}" --all-targets) \
      >"$LOG/check-$slug.log" 2>&1; then
    echo "   CHECK FAILED"; tail -40 "$LOG/check-$slug.log"; rc=1; continue
  fi

  echo "-- cargo build (both profiles: dlopen targets)"
  if ! (cd "$CRATE" && timeout 600 cargo build --release "${featargs[@]}" &&
        timeout 600 cargo build "${featargs[@]}") \
      >"$LOG/build-$slug.log" 2>&1; then
    echo "   BUILD FAILED"; tail -40 "$LOG/build-$slug.log"; rc=1; continue
  fi

  for profile in release debug; do
    RUST_SO="$CRATE/target/$profile/libupdate_md5_lib.so"
    if [ ! -f "$RUST_SO" ]; then echo "   no cdylib for $profile, skipping"; continue; fi
    export PARITY_RUST_SO="$RUST_SO"

    echo "-- [$profile] nm -D symbol comparison"
    syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TtDdBbRr]$/ {print $3}' | sort -u; }
    missing=$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO"))
    if [ -n "$missing" ]; then
      echo "   MISSING EXPORTS IN RUST .so:"; echo "$missing" | sed 's/^/     /'; rc=1
    else
      echo "   all C exports present ($(syms "$C_SO" | tr '\n' ' '))"
    fi

    echo "-- [$profile] cargo test"
    if (cd "$CRATE" && timeout 600 cargo test "${featargs[@]}" -- --test-threads=4) \
        >"$LOG/test-$slug-$profile.log" 2>&1; then
      grep -E '^test result:' "$LOG/test-$slug-$profile.log" | sed 's/^/   /'
    else
      echo "   TESTS FAILED (see $LOG/test-$slug-$profile.log)"
      grep -E 'panicked|mismatch|^test .* FAILED|^failures:|^test result:' \
        "$LOG/test-$slug-$profile.log" | head -40 | sed 's/^/   /'
      rc=1
    fi
    unset PARITY_RUST_SO
  done
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$rc"
