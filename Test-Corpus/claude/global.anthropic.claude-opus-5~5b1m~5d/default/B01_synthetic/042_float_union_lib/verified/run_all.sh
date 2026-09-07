#!/usr/bin/env bash
# Full verification run: builds the C .so and the Rust cdylib, then runs the
# Phase B / C / D differential suites under every feature combination and both
# profiles.
#
# Usage:  ./run_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_FLAGS="--offline"          # crates.io is unreachable here; libloading is vendored in ~/.cargo
FAIL=0

hdr() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth)
# ---------------------------------------------------------------------------
hdr "Building C libdriver.so"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
# `Cargo.toml` declares no [features] table, so the feature power set is the
# single empty set. Each spelling below is still exercised, because a future
# feature addition must not silently skip a configuration.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' "$HERE/Cargo.toml")
if [ -n "$FEATURES" ]; then
  echo "Declared features: $FEATURES"
else
  echo "Declared features: (none — Cargo.toml has no [features] table)"
fi

COMBOS=("" "--no-default-features" "--all-features" "--no-default-features --all-features")
# Add every individual declared feature, and the power set if any exist.
for f in $FEATURES; do
  COMBOS+=("--no-default-features --features $f")
done

# ---------------------------------------------------------------------------
# 3. Run every (profile x feature-combo) cell
# ---------------------------------------------------------------------------
for PROFILE in dev release; do
  if [ "$PROFILE" = "release" ]; then PFLAG="--release"; else PFLAG=""; fi
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"
    hdr "$LABEL"

    # The tests dlopen the cdylib built for the same profile, so build it first.
    # shellcheck disable=SC2086
    if ! ( cd "$HERE" && timeout 600 cargo build $CARGO_FLAGS $PFLAG $COMBO ); then
      echo "BUILD FAILED: $LABEL"; FAIL=1; continue
    fi
    # shellcheck disable=SC2086
    if ! ( cd "$HERE" && timeout 600 cargo test $CARGO_FLAGS $PFLAG $COMBO -- --test-threads=1 ); then
      echo "TESTS FAILED: $LABEL"; FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Raw symbol diff (belt and braces alongside tests/phase_d_symbols.rs)
# ---------------------------------------------------------------------------
hdr "Symbol diff: C .so vs Rust .so"
TD="$(mktemp -d "${TMPDIR:-/tmp}/driver-syms-XXXXXX")"
strip_syms() {
  nm -D --defined-only --format=posix "$1" 2>/dev/null \
    | awk '{print $1}' | sed 's/@.*//' \
    | grep -vE '^(_init|_fini|__bss_start|_edata|_end|_etext)$' \
    | grep -vE '^(_ZN|_R|rust_|__rust)' | sort -u
}
strip_syms "$ROOT/c_src/build/libdriver.so" > "$TD/c.syms"
for RSO in "$HERE/target/debug/libdriver.so" "$HERE/target/release/libdriver.so"; do
  [ -f "$RSO" ] || continue
  strip_syms "$RSO" > "$TD/r.syms"
  MISSING=$(comm -23 "$TD/c.syms" "$TD/r.syms")
  if [ -n "$MISSING" ]; then
    echo "MISSING from $RSO:"; echo "$MISSING"; FAIL=1
  else
    echo "OK: $RSO exports every C symbol ($(wc -l < "$TD/c.syms") symbol(s)); diff is empty"
  fi
done
rm -rf "$TD"

hdr "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
