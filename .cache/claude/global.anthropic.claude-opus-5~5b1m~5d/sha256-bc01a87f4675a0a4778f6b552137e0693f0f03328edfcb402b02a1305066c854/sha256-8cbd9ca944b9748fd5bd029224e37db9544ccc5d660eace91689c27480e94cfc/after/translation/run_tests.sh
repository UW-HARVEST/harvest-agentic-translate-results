#!/usr/bin/env bash
# Full differential-verification run: builds both libraries, diffs the exported
# symbol sets, and runs every test in every profile x feature combination.
#
# fd 1 is redirected inside the harness, so the test binaries MUST run serially.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_DIR="$(cd .. && pwd)/c_src"
export RUST_TEST_THREADS=1
CARGO="cargo"
OFFLINE="--offline"
FAILED=0

hdr() { printf '\n=========== %s ===========\n' "$*"; }

# --------------------------------------------------------------------------
hdr "build C shared library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$C_DIR"/build/lib*.so | head -n1)
echo "C  .so: $C_SO"

# --------------------------------------------------------------------------
# Feature combinations.  Cargo.toml declares no [features] table, so the only
# configurations are the default build and --no-default-features; both are
# enumerated mechanically so new features are picked up automatically.
FEATURE_SETS=()
FEATS=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -oE '^[A-Za-z0-9_-]+' | grep -v '^default$' || true)
FEATURE_SETS+=("")                      # default features
FEATURE_SETS+=("--no-default-features") # no features
if [ -n "$FEATS" ]; then
  for f in $FEATS; do
    FEATURE_SETS+=("--no-default-features --features $f")
  done
  ALL=$(echo "$FEATS" | paste -sd,)
  FEATURE_SETS+=("--no-default-features --features $ALL")
  FEATURE_SETS+=("--all-features")
fi

for PROFILE_FLAG in "--release" ""; do
  PROFILE_NAME=${PROFILE_FLAG:---dev}
  for FEAT in "${FEATURE_SETS[@]}"; do
    hdr "profile=${PROFILE_NAME} features=[${FEAT:-default}]"

    # shellcheck disable=SC2086
    $CARGO build $OFFLINE $PROFILE_FLAG $FEAT 2>&1 | tail -n 2 || { FAILED=1; continue; }

    if [ -n "$PROFILE_FLAG" ]; then R_SO="target/release/libcheckshift_lib.so";
    else R_SO="target/debug/libcheckshift_lib.so"; fi
    touch "$R_SO"   # make sure the harness picks *this* profile's .so
    echo "RS .so: $R_SO"

    # ---- symbol parity -------------------------------------------------
    C_SYMS=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)
    R_SYMS=$(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u)
    N_C=$(echo "$C_SYMS" | grep -c . )
    if [ "$N_C" -eq 0 ]; then echo "SYMBOL PARITY FAILED - could not read C symbols"; FAILED=1; fi
    MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED - missing from Rust .so:"; echo "$MISSING"; FAILED=1
    else
      echo "symbol parity OK ($N_C C symbols, 0 missing)"
    fi
    # non-libc undefined symbols
    UNDEF=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sort -u)
    echo "undefined (must be libc/loader only): $(echo "$UNDEF" | tr '\n' ' ')"


    # ---- tests ---------------------------------------------------------
    # shellcheck disable=SC2086
    if ! $CARGO test $OFFLINE $PROFILE_FLAG $FEAT 2>&1 | grep -E 'test result|^test |FAILED|panicked' ; then
      FAILED=1
    fi
    # shellcheck disable=SC2086
    $CARGO test $OFFLINE $PROFILE_FLAG $FEAT >/dev/null 2>&1 || { echo "TESTS FAILED"; FAILED=1; }
  done
done

hdr "summary"
if [ "$FAILED" -eq 0 ]; then echo "ALL GREEN"; else echo "FAILURES PRESENT"; fi
exit "$FAILED"
