#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination x every profile.
# Usage:  cd translation && ./verify.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$CRATE_DIR"
C_BUILD="$CRATE_DIR/../c_src/build"
RUST_SO_NAME="libbitwriter_add_lib.so"
FAIL=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- build the C
step "Building C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so: $C_SO"

# ------------------------------------------- enumerate feature combinations
# Parse the [features] table from Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("--offline")                        # default features
COMBOS+=("--offline --no-default-features")  # nothing enabled
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--offline --no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
step "Feature combinations to verify: ${#COMBOS[@]}"
printf '  declared features: %s\n' "${FEATURES[*]:-<none declared in Cargo.toml>}"
for c in "${COMBOS[@]}"; do printf '   - cargo test %s\n' "$c"; done

# ------------------------------------- per combo: build, symbol-diff, test
for PROFILE in dev release; do
  case "$PROFILE" in
    dev)     PROF_FLAG="";          OUT_DIR="target/debug"   ;;
    release) PROF_FLAG="--release"; OUT_DIR="target/release" ;;
  esac

  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE  $COMBO"
    step "$LABEL"

    # shellcheck disable=SC2086
    if ! cargo build $COMBO $PROF_FLAG >/dev/null 2>&1; then
      bad "cargo build ($LABEL)"; continue
    fi
    RUST_SO="$OUT_DIR/$RUST_SO_NAME"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }

    # --- symbol parity: every C dynamic symbol must exist in the Rust .so
    C_SYMS=$(nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort -u)
    R_SYMS=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
    MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
    if [ -z "$MISSING" ]; then
      ok "symbol parity ($(echo "$C_SYMS" | grep -c . ) C symbols, 0 missing)"
    else
      bad "symbols missing from Rust .so:"; echo "$MISSING" | sed 's/^/        /'
    fi

    # --- undefined symbols in the Rust .so must be libc/runtime only
    UNDEF=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' | sort -u \
            | grep -vE '^(_|__)' | grep -vxF 'bitwriter_add')
    if [ -z "$UNDEF" ]; then
      ok "no unresolved project symbols"
    else
      printf '  \033[33mNOTE\033[0m non-underscore undefined syms (expected libc): %s\n' \
        "$(echo "$UNDEF" | tr '\n' ' ')"
    fi

    # --- run both differential suites
    # shellcheck disable=SC2086
    if cargo test $COMBO $PROF_FLAG --test phase_b_configs --test phase_c_errors \
         >"$CRATE_DIR/.verify_out" 2>&1; then
      ok "$(grep -hE '^test result' "$CRATE_DIR/.verify_out" | tr '\n' ' ')"
    else
      bad "differential tests failed ($LABEL)"
      tail -40 "$CRATE_DIR/.verify_out" | sed 's/^/        /'
    fi
    rm -f "$CRATE_DIR/.verify_out"
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL PHASE D CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
