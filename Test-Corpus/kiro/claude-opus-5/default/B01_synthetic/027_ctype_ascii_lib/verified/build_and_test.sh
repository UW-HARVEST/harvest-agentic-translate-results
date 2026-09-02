#!/usr/bin/env bash
# Full verification driver: build the C ground truth, then check symbol parity
# and run every differential suite under every feature combination and both
# build profiles.
#
# Usage: ./build_and_test.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_DIR="$ROOT/c_src"
C_SO="$C_DIR/build/libdriver.so"
TIMEOUT=600
FAILURES=0
# Every configuration must run at least this many tests; a lower count means a
# suite vanished (renamed file, compile error swallowed, filter left in place).
MIN_EXPECTED_TESTS=60

hr() { printf '%s\n' "------------------------------------------------------------"; }
note() { printf '\n== %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth). c_src is never modified.
# ---------------------------------------------------------------------------
note "Building the C ground truth"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout $TIMEOUT cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
[[ -f "$C_SO" ]] || { fail "missing $C_SO"; exit 1; }
printf 'ok: %s\n' "$C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml.
#    Every subset of the declared features is checked, plus the default set.
# ---------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)
printf 'declared non-default features: %d %s\n' "${#FEATURES[@]}" "${FEATURES[*]:-(none)}"

# COMBOS holds the cargo flag string for each configuration to verify.
COMBOS=()
COMBOS+=("")                        # default features
COMBOS+=("--no-default-features")   # nothing enabled
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
fi
COMBOS+=("--all-features")
printf 'configurations to verify: %d\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check every configuration first (fast failure).
# ---------------------------------------------------------------------------
note "cargo check across all configurations"
for combo in "${COMBOS[@]}"; do
  if timeout $TIMEOUT cargo check --quiet $combo >/dev/null 2>&1; then
    printf 'ok    check  [%s]\n' "${combo:-default}"
  else
    fail "cargo check [${combo:-default}]"
  fi
done

# ---------------------------------------------------------------------------
# 4. Per configuration and profile: build the cdylib, diff symbols, run tests.
# ---------------------------------------------------------------------------
for profile in release debug; do
  if [[ $profile == release ]]; then PFLAG="--release"; else PFLAG=""; fi
  for combo in "${COMBOS[@]}"; do
    label="${combo:-default} / $profile"
    hr
    note "Configuration: $label"

    if ! timeout $TIMEOUT cargo build --quiet $PFLAG $combo >/dev/null 2>&1; then
      fail "cargo build [$label]"
      continue
    fi
    RUST_SO="$CRATE_DIR/target/$profile/libdriver.so"
    if [[ ! -f $RUST_SO ]]; then
      fail "missing $RUST_SO [$label]"
      continue
    fi

    # --- Phase D symbol parity -------------------------------------------
    c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
    if [[ -n $missing ]]; then
      fail "symbols missing from the Rust .so [$label]: $(tr '\n' ' ' <<<"$missing")"
    else
      printf 'ok    symbol parity (%s C-defined symbol(s), 0 missing)\n' \
        "$(wc -l <<<"$c_syms")"
    fi

    # Unresolved non-libc imports would mean an untranslated module.
    unresolved=$(nm -D -u "$RUST_SO" | awk '{print $NF}' \
      | grep -v '@GLIBC' | grep -v '@GCC' \
      | grep -vE '^(_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable|__gmon_start__)$')
    if [[ -n $unresolved ]]; then
      fail "unresolved non-libc symbols [$label]: $(tr '\n' ' ' <<<"$unresolved")"
    else
      printf 'ok    no unresolved non-libc imports\n'
    fi

    # --- Phases B and C: every differential suite ------------------------
    # --test-threads=1 is mandatory: the suites redirect fd 1 to capture
    # stdout, and libtest's own progress output would otherwise interleave.
    log=/tmp/driver_test_$$.log
    if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$RUST_SO" \
       timeout $TIMEOUT cargo test $PFLAG $combo -- --test-threads=1 \
       > "$log" 2>&1; then
      passed=$(grep -cE '^test .+ \.\.\. ok$' "$log")
      suites=$(grep -c '^     Running ' "$log")
      if ((passed == 0)); then
        fail "no tests actually ran [$label] — check the log format"
        tail -n 20 "$log"
      else
        printf 'ok    differential suites: %d test(s) passed across %d binary(ies)\n' \
          "$passed" "$suites"
        # Guard against a suite silently disappearing.
        if ((passed < MIN_EXPECTED_TESTS)); then
          fail "only $passed tests ran, expected >= $MIN_EXPECTED_TESTS [$label]"
        fi
      fi
    else
      fail "differential suites [$label]"
      tail -n 40 "$log"
    fi
    rm -f "$log"
  done
done

hr
if ((FAILURES == 0)); then
  printf '\nALL CONFIGURATIONS PASSED (%d configuration(s) x 2 profile(s))\n' "${#COMBOS[@]}"
  exit 0
fi
printf '\n%d FAILURE(S)\n' "$FAILURES"
exit 1
