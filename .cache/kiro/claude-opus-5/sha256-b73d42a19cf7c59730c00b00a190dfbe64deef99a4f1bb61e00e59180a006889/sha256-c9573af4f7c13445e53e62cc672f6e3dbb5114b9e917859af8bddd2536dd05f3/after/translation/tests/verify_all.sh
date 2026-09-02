#!/usr/bin/env bash
# Phase D driver: symbol parity + the full differential suite under EVERY
# feature combination and both profiles.
#
# Feature combinations are enumerated MECHANICALLY from Cargo.toml rather than
# hardcoded, so a future [features] table is picked up automatically.
#
# Usage: tests/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"
TIMEOUT=${TIMEOUT:-600}

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$C_DIR/build" || exit 1
( cd "$C_DIR/build" \
  && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout "$TIMEOUT" cmake --build . >/dev/null ) \
  && ok "libdriver.so (C)" || bad "C build"
C_SO="$C_DIR/build/libdriver.so"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Grab the [features] table, drop the implicit "default" key, and list the rest.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features] table."
  echo "  cfg(feature ...) uses in src/: $(grep -rc 'cfg(feature' src/ | paste -sd, -)"
  # The only two distinct configurations are then: default, and
  # --no-default-features (which is identical, but exercised anyway).
  COMBOS=("" "--no-default-features")
else
  echo "  declared features: $FEATURES"
  COMBOS=("" "--no-default-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--features $f")
  done
  # All-features combination.
  ALL=$(echo "$FEATURES" | paste -sd, -)
  COMBOS+=("--no-default-features --features $ALL")
  COMBOS+=("--all-features")
fi
echo "  combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "cargo check under every combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if timeout "$TIMEOUT" cargo check $combo >/dev/null 2>&1; then
    ok "cargo check ${combo:-<default>}"
  else
    bad "cargo check ${combo:-<default>}"
  fi
done

# ---------------------------------------------------------------------------
step "Symbol parity: nm -D on C .so vs Rust .so, every combination x profile"
# ---------------------------------------------------------------------------
c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
echo "  C exports: $(echo "$c_syms" | paste -sd' ' -)"

for profile in debug release; do
  flag=""; [ "$profile" = release ] && flag="--release"
  for combo in "${COMBOS[@]}"; do
    timeout "$TIMEOUT" cargo build $flag $combo >/dev/null 2>&1 || { bad "build $profile ${combo:-<default>}"; continue; }
    r_so="target/$profile/libdriver.so"
    r_syms=$(nm -D --defined-only "$r_so" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -z "$missing" ]; then
      ok "symbol parity $profile ${combo:-<default>} (0 missing)"
    else
      bad "symbol parity $profile ${combo:-<default>} missing: $(echo "$missing" | paste -sd' ' -)"
    fi
    # Every remaining undefined symbol must resolve against libc/libgcc; a
    # successful RTLD_NOW dlopen proves it. ldd reports any that do not.
    if ldd -r "$r_so" 2>&1 | grep -q 'undefined symbol'; then
      bad "unresolved non-libc symbols in $profile ${combo:-<default>}"
      ldd -r "$r_so" 2>&1 | grep 'undefined symbol' | head -5
    else
      ok "all imports resolve $profile ${combo:-<default>}"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Differential suite (Phases B + C) under every combination x profile"
# ---------------------------------------------------------------------------
for profile in debug release; do
  flag=""; [ "$profile" = release ] && flag="--release"
  for combo in "${COMBOS[@]}"; do
    # Rebuild the cdylib first: this crate is cdylib-only, so `cargo test`
    # alone will NOT refresh it and the suite would load a stale .so. The
    # in-test freshness guard also catches this, but build explicitly anyway.
    timeout "$TIMEOUT" cargo build $flag $combo >/dev/null 2>&1
    out=$(timeout "$TIMEOUT" cargo test $flag $combo --test differential 2>&1)
    line=$(echo "$out" | grep 'test result' | tail -1)
    if echo "$line" | grep -q '^test result: ok'; then
      ok "differential $profile ${combo:-<default>} -> $line"
    else
      bad "differential $profile ${combo:-<default>} -> ${line:-<no result line>}"
      echo "$out" | grep -E 'FAILED|panicked|STALE|divergence' | head -10
    fi
  done
done

# ---------------------------------------------------------------------------
printf '\n=== SUMMARY ===\n'
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASE D CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$fail"
