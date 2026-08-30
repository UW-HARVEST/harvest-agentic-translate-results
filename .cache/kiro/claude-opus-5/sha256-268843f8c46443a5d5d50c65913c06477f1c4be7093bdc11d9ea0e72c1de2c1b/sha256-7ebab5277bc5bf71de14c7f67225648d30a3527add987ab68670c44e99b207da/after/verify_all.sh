#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration: every feature combination declared in Cargo.toml, in both the
# dev and release profiles.
#
# Usage: ./verify_all.sh          (run from the repository root)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
C_SO="$ROOT/c_src/build/libStaticAlias.so"
FAILURES=0

step() { printf '\n=== %s ===\n' "$*"; }

# --- 1. Build the C reference shared library ------------------------------
step "Building C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }
echo "ok: $C_SO"

# --- 2. Enumerate every valid feature combination -------------------------
# Read the [features] table from Cargo.toml (excluding the implicit "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: the crate has exactly one configuration.
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

step "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done
# The `default` feature set is also verified explicitly below.

# --- 3. cargo check for every combination ---------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo check --no-default-features --features '${combo:-<none>}'"
  if [ -z "$combo" ]; then
    ( cd "$CRATE" && timeout 600 cargo check --no-default-features --all-targets )
  else
    ( cd "$CRATE" && timeout 600 cargo check --no-default-features --all-targets --features "$combo" )
  fi || { echo "CHECK FAILED for '${combo:-<none>}'"; FAILURES=$((FAILURES + 1)); }
done
step "cargo check (default features)"
( cd "$CRATE" && timeout 600 cargo check --all-targets ) \
  || { echo "CHECK FAILED for default"; FAILURES=$((FAILURES + 1)); }

# --- 4/5. Symbol parity + differential tests, per combination and profile --
compare_symbols() {
  local rust_so="$1" label="$2"
  local missing
  missing="$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))"
  if [ -n "$missing" ]; then
    echo "SYMBOL PARITY FAILED ($label); missing from Rust .so:"
    echo "$missing"
    return 1
  fi
  echo "symbol parity ok ($label)"
}

run_config() {
  local combo="$1" profile="$2"
  local -a fargs=() pargs=() 
  local profdir="debug"
  if [ -n "$combo" ]; then
    fargs=(--no-default-features --features "$combo")
  elif [ "$combo_is_default" = "yes" ]; then
    fargs=()
  else
    fargs=(--no-default-features)
  fi
  if [ "$profile" = "release" ]; then
    pargs=(--release)
    profdir="release"
  fi

  step "cargo build ${fargs[*]} ${pargs[*]}  (features='${combo:-<none>}', profile=$profile)"
  ( cd "$CRATE" && timeout 600 cargo build --lib --examples "${fargs[@]}" "${pargs[@]}" ) \
    || { echo "BUILD FAILED (features='${combo:-<none>}', profile=$profile)"; FAILURES=$((FAILURES + 1)); return; }

  step "cargo test ${fargs[*]} ${pargs[*]}  (features='${combo:-<none>}', profile=$profile)"
  ( cd "$CRATE" && timeout 600 cargo test "${fargs[@]}" "${pargs[@]}" ) \
    || { echo "TESTS FAILED (features='${combo:-<none>}', profile=$profile)"; FAILURES=$((FAILURES + 1)); }

  compare_symbols "$CRATE/target/$profdir/libStaticAlias.so" \
    "features='${combo:-<none>}', profile=$profile" \
    || FAILURES=$((FAILURES + 1))
}

for combo in "${COMBOS[@]}"; do
  combo_is_default="no"
  for profile in debug release; do
    run_config "$combo" "$profile"
  done
done

# Also exercise the crate exactly as a default consumer would build it.
combo_is_default="yes"
for profile in debug release; do
  run_config "" "$profile"
done

step "SUMMARY"
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "$FAILURES configuration(s) FAILED"
fi
exit "$FAILURES"
