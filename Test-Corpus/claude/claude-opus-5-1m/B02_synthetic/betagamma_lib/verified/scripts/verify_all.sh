#!/usr/bin/env bash
# Full verification sweep: builds the C reference .so, then for EVERY valid
# cargo feature combination rebuilds the Rust cdylib, diffs the exported symbol
# tables, and runs the whole differential test suite (Phases B, C and D).
#
# Usage: scripts/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
LOG=${TMPDIR:-/tmp}/verify_all.$$.log
: > "$LOG"
rc=0

say() { printf '%s\n' "$*"; }
hdr() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate the valid feature combinations straight out of Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{sub(/ *=.*/,"");print}' Cargo.toml
)
say "features declared in Cargo.toml: ${FEATURES[*]:-<none>}"

# Cross-product of all declared features (the crate declares only the empty
# `default`, so this is {} and {default}).  Kept generic on purpose.
COMBOS=("--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
say "feature combinations to verify: ${#COMBOS[@]}"
for cb in "${COMBOS[@]}"; do say "  * cargo <cmd> $cb"; done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library.
# ---------------------------------------------------------------------------
hdr "building the C reference .so"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >>"$LOG" 2>&1 || { say "C build FAILED (see $LOG)"; exit 1; }
C_SO=$(ls c_src/build/*.so | head -1)
say "C .so: $C_SO"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.$$"
say "C exports $(wc -l < "${TMPDIR:-/tmp}/c_syms.$$") symbols"

# ---------------------------------------------------------------------------
# 3. For every combination: cargo check, build, symbol diff, full test run.
# ---------------------------------------------------------------------------
for cb in "${COMBOS[@]}"; do
  for prof in dev release; do
    if [ "$prof" = release ]; then PFLAG="--release"; PDIR=release; else PFLAG=""; PDIR=debug; fi
    hdr "combination: cargo ... $cb  [profile: $prof]"

    say "-- cargo check"
    if ! timeout 600 cargo check --offline $PFLAG $cb >>"$LOG" 2>&1; then
      say "   cargo check FAILED"; rc=1; continue
    fi
    say "   ok"

    say "-- cargo build (cdylib)"
    if ! timeout 600 cargo build --offline $PFLAG $cb >>"$LOG" 2>&1; then
      say "   cargo build FAILED"; rc=1; continue
    fi
    R_SO=target/$PDIR/libbetagamma_lib.so
    say "   ok ($R_SO)"

    say "-- symbol parity (nm -D)"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/r_syms.$$"
    missing=$(comm -23 "${TMPDIR:-/tmp}/c_syms.$$" "${TMPDIR:-/tmp}/r_syms.$$")
    if [ -n "$missing" ]; then
      say "   MISSING FROM RUST .so:"; printf '     %s\n' $missing; rc=1
    else
      say "   ok - 0 missing symbols"
    fi
    if ldd "$R_SO" | grep -q 'not found'; then
      say "   UNRESOLVED shared-object dependency"; ldd "$R_SO" | grep 'not found'; rc=1
    fi

    say "-- differential tests (Phase B + C)"
    # NOTE: `cargo test` does not rebuild a cdylib-only lib target, so the build
    # above is what produced the .so under test; tests/common asserts freshness.
    if timeout 600 cargo test --offline $PFLAG $cb --tests -- --test-threads=1 >>"$LOG" 2>&1; then
      say "   ok"
    else
      say "   TESTS FAILED (see $LOG)"; rc=1
    fi
  done
done

rm -f "${TMPDIR:-/tmp}/c_syms.$$" "${TMPDIR:-/tmp}/r_syms.$$"

hdr "summary"
if [ $rc -eq 0 ]; then
  say "ALL FEATURE COMBINATIONS PASSED"
else
  say "FAILURES DETECTED - full log: $LOG"
fi
exit $rc
