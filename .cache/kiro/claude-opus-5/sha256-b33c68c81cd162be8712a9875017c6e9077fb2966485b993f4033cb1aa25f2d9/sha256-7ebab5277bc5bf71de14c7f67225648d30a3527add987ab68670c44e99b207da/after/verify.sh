#!/usr/bin/env bash
# Full verification sweep: enumerate every build-time configuration, check that
# each compiles, and for each one confirm the Rust .so exports every symbol the
# C .so exports and that the differential tests pass.
#
# Usage: ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
FAIL=0
TIMEOUT=600

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
#
# The [features] table is read out of Cargo.toml rather than hard-coded, so this
# keeps working if features are added later. With no [features] table the only
# valid configuration is the empty one.
# --------------------------------------------------------------------------
note "Enumerating feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "")
      if ($0 != "default") print
    }
  ' "$CRATE/Cargo.toml"
)
printf 'features found: %d %s\n' "${#FEATURES[@]}" "${FEATURES[*]:-(none)}"

# Power set of FEATURES as comma-separated strings ("" == no features).
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"

# --------------------------------------------------------------------------
# 2. Build the C reference library.
# --------------------------------------------------------------------------
note "Building C reference library"
mkdir -p "$CBUILD"
( cd "$CBUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
  && cmake --build . >>/tmp/cmake.log 2>&1 ) || { fail "C build"; tail -20 /tmp/cmake.log; exit 1; }
C_SO="$(find "$CBUILD" -maxdepth 1 -name '*.so' | head -1)"
printf 'C .so: %s\n' "$C_SO"

# --------------------------------------------------------------------------
# 3. For every combination x profile: check, build, compare symbols, run tests.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FEATFLAGS=(--no-default-features)
    label="<no features>"
  else
    FEATFLAGS=(--no-default-features --features "$combo")
    label="$combo"
  fi

  note "cargo check [$label]"
  ( cd "$CRATE" && timeout $TIMEOUT cargo check "${FEATFLAGS[@]}" ) \
    || fail "cargo check [$label]"

  for profile in dev release; do
    if [ "$profile" = release ]; then
      PROFFLAGS=(--release); outdir="$CRATE/target/release"
    else
      PROFFLAGS=();          outdir="$CRATE/target/debug"
    fi

    note "cargo build [$label / $profile]"
    ( cd "$CRATE" && timeout $TIMEOUT cargo build "${FEATFLAGS[@]}" "${PROFFLAGS[@]}" ) \
      || { fail "cargo build [$label / $profile]"; continue; }

    # ---- symbol parity: every symbol the C .so exports must be exported ----
    note "symbol parity [$label / $profile]"
    R_SO="$outdir/libagglom_lib.so"
    [ -f "$R_SO" ] || R_SO="$outdir/deps/libagglom_lib.so"
    syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TDBRWiI]$/ {print $3}' | sort -u; }
    missing="$(comm -23 <(syms "$C_SO") <(syms "$R_SO"))"
    if [ -n "$missing" ]; then
      fail "symbols missing from Rust .so [$label / $profile]:"
      printf '  %s\n' $missing
    else
      printf 'all %d C symbols exported by Rust .so\n' "$(syms "$C_SO" | wc -l)"
    fi

    note "cargo test [$label / $profile]"
    ( cd "$CRATE" && timeout $TIMEOUT cargo test "${FEATFLAGS[@]}" "${PROFFLAGS[@]}" \
        --no-fail-fast 2>&1 | grep -E 'test result|FAILED|panicked' ) \
      || fail "cargo test [$label / $profile]"
  done
done

note "SUMMARY"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS VERIFIED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
