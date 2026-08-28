#!/usr/bin/env bash
# Phase D driver: enumerate every cargo feature combination, and for each one
# build both .so's, diff their exported symbols, and run the full differential
# suite in BOTH the debug and release profiles.
#
# The debug profile matters: it enables Rust's arithmetic overflow checks, so any
# place where the translation used a checked operator instead of reproducing the
# C code's wrapping `int` arithmetic would panic there instead of matching C.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
FAILED=0

# Logs/scratch must go somewhere writable ($TMPDIR is set by the sandbox).
LOG="${TMPDIR:-/tmp}/driver-verify.$$"
mkdir -p "$LOG" || exit 1
trap 'rm -rf "$LOG"' EXIT

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ------------------------------------------------------------------ #
step "Building the C shared library"
( mkdir -p "$ROOT/c_src/build" \
  && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "built $C_SO" || { bad "C build"; exit 1; }

# ------------------------------------------------------------------ #
step "Enumerating cargo feature combinations"
# Read the [features] table out of Cargo.toml. If there is none, the crate has
# exactly one configuration: the default (empty) feature set.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
         sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)
echo "  declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No features declared -> these three invocations are all the same single
  # configuration; we run all three anyway so the claim is actually tested.
  COMBOS+=("__default__" "__none__" "__all__")
else
  COMBOS+=("__default__" "__none__" "__all__")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi
echo "  combinations to verify: ${#COMBOS[@]}"

flags_for() {
  case "$1" in
    __default__) echo "" ;;
    __none__)    echo "--no-default-features" ;;
    __all__)     echo "--all-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# ------------------------------------------------------------------ #
for combo in "${COMBOS[@]}"; do
  FLAGS="$(flags_for "$combo")"
  for PROFILE in debug release; do
    PFLAG=""; [ "$PROFILE" = release ] && PFLAG="--release"
    step "combo=$combo profile=$PROFILE  (cargo $PFLAG $FLAGS)"

    # shellcheck disable=SC2086
    if timeout 600 cargo build --offline $PFLAG $FLAGS > "$LOG/build.log" 2>&1; then
      ok "cargo build"
    else
      bad "cargo build"; tail -20 "$LOG/build.log"; continue
    fi

    RUST_SO="target/$PROFILE/libdriver.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }

    # --- symbol parity -------------------------------------------- #
    nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u > "$LOG/c_syms"
    nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > "$LOG/r_syms"
    MISSING="$(comm -23 "$LOG/c_syms" "$LOG/r_syms")"
    if [ -z "$MISSING" ]; then
      ok "symbol parity ($(wc -l < "$LOG/c_syms") C symbols, 0 missing from Rust)"
    else
      bad "symbols exported by C but MISSING from Rust:"; echo "$MISSING" | sed 's/^/       /'
    fi

    # 'encode' is static in C and must not leak out of either object
    if nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | grep -qx encode; then
      bad "Rust .so wrongly exports the static helper 'encode'"
    else
      ok "static helper 'encode' correctly not exported"
    fi

    # --- the differential suite ----------------------------------- #
    # shellcheck disable=SC2086
    if timeout 600 cargo test --offline $PFLAG $FLAGS > "$LOG/test.log" 2>&1; then
      ok "cargo test ($(grep -c '^test .* ok$' "$LOG/test.log") tests passed)"
    else
      bad "cargo test"; grep -E "FAILED|panicked|^test .* FAILED|test result" "$LOG/test.log" | head -30
    fi
  done
done

# ------------------------------------------------------------------ #
step "RESULT"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL FEATURE COMBINATIONS x PROFILES PASSED\033[0m\n'
else
  printf '\033[31mFAILURES PRESENT\033[0m\n'
fi
exit "$FAILED"
