#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff their exported symbols, and run
# every test suite under every feature combination and both build profiles.
#
# Usage:  ./scripts/verify_all.sh
# Run from anywhere; paths are resolved relative to the crate root.

set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(dirname "$CRATE_DIR")"
C_SRC="$WORK_DIR/c_src"
LOG_DIR="$CRATE_DIR/target/verify-logs"
mkdir -p "$LOG_DIR"

FAILURES=0
note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }
ok()   { printf 'ok:   %s\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$C_SRC/build"
if ! ( cd "$C_SRC/build" \
       && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       && timeout 600 cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1; then
  fail "C build (see $LOG_DIR/cmake.log)"
  tail -20 "$LOG_DIR/cmake.log"
  exit 1
fi
C_SO="$(ls "$C_SRC"/build/lib*.so)"
ok "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
#
# A crate with no [features] table has exactly one configuration; the loop below
# still exercises --no-default-features and the explicit default so that the
# same script keeps working if features are ever added.
# ---------------------------------------------------------------------------
FEATURES_DECLARED="$(awk '
  /^\[features\]/ { inblk = 1; next }
  /^\[/           { inblk = 0 }
  inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
' "$CRATE_DIR/Cargo.toml" | grep -v '^default$' | sort -u)"

COMBOS=()
if [ -z "$FEATURES_DECLARED" ]; then
  COMBOS+=("--default")            # the one and only configuration
  COMBOS+=("--no-default-features")
else
  COMBOS+=("--default")
  COMBOS+=("--no-default-features")
  # Every non-empty subset of the declared features, with defaults off.
  mapfile -t FEATS <<< "$FEATURES_DECLARED"
  n=${#FEATS[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then set="${set:+$set,}${FEATS[i]}"; fi
    done
    COMBOS+=("--no-default-features --features $set")
  done
  # And every subset on top of the defaults.
  for ((mask = 1; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then set="${set:+$set,}${FEATS[i]}"; fi
    done
    COMBOS+=("--features $set")
  done
fi

note "Feature combinations to verify: ${#COMBOS[@]}"
printf '  %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. For each combination x profile: build, diff symbols, run all tests
# ---------------------------------------------------------------------------
symbol_diff() {
  local rust_so="$1" label="$2"
  nm -D --defined-only "$C_SO"   | awk '$2=="T"||$2=="W"{print $3}' | sort > "$LOG_DIR/c_syms.txt"
  nm -D --defined-only "$rust_so" | awk '$2=="T"||$2=="W"{print $3}' | sort > "$LOG_DIR/r_syms.txt"
  local missing extra undef
  missing="$(comm -23 "$LOG_DIR/c_syms.txt" "$LOG_DIR/r_syms.txt")"
  extra="$(comm -13 "$LOG_DIR/c_syms.txt" "$LOG_DIR/r_syms.txt")"
  # Undefined symbols that are not libc / libgcc-unwind imports.
  undef="$(nm -D -u "$rust_so" | awk '{print $NF}' \
    | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^__cxa_|^__tls_get_addr$' \
    | sort -u)"
  printf '  C symbols: %s   Rust symbols: %s\n' \
    "$(wc -l < "$LOG_DIR/c_syms.txt")" "$(wc -l < "$LOG_DIR/r_syms.txt")"
  if [ -n "$missing" ]; then fail "$label: symbols missing from Rust:"; printf '    %s\n' $missing
  else ok "$label: 0 symbols missing from Rust"; fi
  if [ -n "$extra" ]; then printf '  note: Rust exports extra symbols:\n'; printf '    %s\n' $extra; fi
  if [ -n "$undef" ]; then fail "$label: undefined non-libc symbols:"; printf '    %s\n' $undef
  else ok "$label: 0 undefined non-libc symbols"; fi
}

TESTS=(phase_b_leaf phase_b_composed phase_c_errors)

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2206
  if [ "$combo" = "--default" ]; then FLAGS=(); else FLAGS=($combo); fi
  for profile in dev release; do
    if [ "$profile" = release ]; then PROF=(--release); OUT=release; else PROF=(); OUT=debug; fi
    label="[$combo | $profile]"
    note "$label"

    slug="$(echo "$combo-$profile" | tr -c 'A-Za-z0-9._-' '_')"

    if ! ( cd "$CRATE_DIR" && timeout 600 cargo build "${PROF[@]}" "${FLAGS[@]}" ) \
         > "$LOG_DIR/build-$slug.log" 2>&1; then
      fail "$label cargo build (see $LOG_DIR/build-$slug.log)"
      tail -20 "$LOG_DIR/build-$slug.log"
      continue
    fi
    ok "$label cargo build"

    RUST_SO="$CRATE_DIR/target/$OUT/libmathop_lib.so"
    if [ ! -f "$RUST_SO" ]; then fail "$label missing $RUST_SO"; continue; fi
    symbol_diff "$RUST_SO" "$label"

    for t in "${TESTS[@]}"; do
      if ( cd "$CRATE_DIR" && timeout 600 cargo test "${PROF[@]}" "${FLAGS[@]}" \
             --test "$t" -- --test-threads=1 ) \
           > "$LOG_DIR/test-$t-$slug.log" 2>&1; then
        ok "$label $t: $(grep -oE '[0-9]+ passed' "$LOG_DIR/test-$t-$slug.log" | head -1)"
      else
        fail "$label $t (see $LOG_DIR/test-$t-$slug.log)"
        grep -E '^(test .*FAILED|thread .* panicked|assertion|  left:|  right:|test result:)' \
          "$LOG_DIR/test-$t-$slug.log" | head -30
      fi
    done
  done
done

note "SUMMARY"
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
  exit 0
else
  echo "$FAILURES CHECK(S) FAILED"
  exit 1
fi
