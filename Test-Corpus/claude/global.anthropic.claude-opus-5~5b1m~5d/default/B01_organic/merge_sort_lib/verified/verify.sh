#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff their exported symbols, and run
# the full differential suite across every feature combination AND against both
# the release and debug Rust cdylib.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
FAILED=0
step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf '!! FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Build C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -f "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate cargo feature combinations"
# Mechanically derive the feature list from cargo metadata rather than assuming.
FEATURES=$(cargo metadata --no-deps --format-version 1 --manifest-path "$CRATE_DIR/Cargo.toml" 2>/dev/null \
  | tr '{},' '\n\n\n' | sed -n 's/.*"features":\[\(.*\)\].*/\1/p' | head -1)
echo "declared features: [${FEATURES:-<none>}]"
# Combos: default, no-default-features, all-features. With zero declared
# features these are all the same build, but they are run explicitly so a future
# feature addition is covered automatically.
COMBOS=( "" "--no-default-features" "--all-features" )

# ---------------------------------------------------------------------------
for COMBO in "${COMBOS[@]}"; do
  for PROFILE in release debug; do
    LABEL="features='${COMBO:-<default>}' profile=$PROFILE"
    step "Build + test: $LABEL"

    if [ "$PROFILE" = release ]; then
      ( cd "$CRATE_DIR" && cargo build --release $COMBO ) >/dev/null 2>&1 \
        || { fail "cargo build --release $COMBO"; continue; }
      RUST_SO="$CRATE_DIR/target/release/libmerge_sort_lib.so"
    else
      ( cd "$CRATE_DIR" && cargo build $COMBO ) >/dev/null 2>&1 \
        || { fail "cargo build $COMBO"; continue; }
      RUST_SO="$CRATE_DIR/target/debug/libmerge_sort_lib.so"
    fi
    [ -f "$RUST_SO" ] || { fail "no Rust .so at $RUST_SO"; continue; }

    # --- symbol parity -----------------------------------------------------
    DIFF=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$DIFF" ]; then
      fail "symbols exported by C but missing from Rust ($LABEL):"; echo "$DIFF"
    else
      echo "symbol parity: OK (0 missing)"
    fi

    # Undefined non-libc symbols in the Rust .so (must be none).
    UNDEF=$(nm -D -u "$RUST_SO" | awk '{print $NF}' \
      | grep -vE '@(GLIBC|GCC)|^_ITM_|^__gmon_start__|^__cxa_|^gettid$|^statx$' || true)
    if [ -n "$UNDEF" ]; then
      fail "undefined non-libc symbols in Rust .so ($LABEL):"; echo "$UNDEF"
    else
      echo "undefined non-libc symbols: none"
    fi

    # --- differential suite ------------------------------------------------
    ( cd "$CRATE_DIR" && C_LIB_PATH="$C_SO" RUST_LIB_PATH="$RUST_SO" \
        timeout 600 cargo test $COMBO --tests -- --test-threads=4 ) 2>&1 \
      | grep -E '^(test result|error|warning: unused)' \
      || fail "test run produced no result line ($LABEL)"
    # shellcheck disable=SC2181
    if ! ( cd "$CRATE_DIR" && C_LIB_PATH="$C_SO" RUST_LIB_PATH="$RUST_SO" \
            timeout 600 cargo test $COMBO --tests >/dev/null 2>&1 ); then
      fail "differential tests ($LABEL)"
    fi
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "THERE WERE FAILURES"; fi
exit "$FAILED"
