#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every cargo feature
# combination and under both test profiles, plus the symbol-parity diff.
#
#   ./verify.sh            # everything
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.
set -uo pipefail

cd "$(dirname "$0")"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# --------------------------------------------------------------------------
# 0. Build the C shared library (idempotent) and locate both .so files
# --------------------------------------------------------------------------
step "building the C shared library"
mkdir -p ../c_src/build
( cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls ../c_src/build/lib*.so | head -1)
note "C   .so: $C_SO"

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
step "enumerating cargo feature combinations"
FEATURES=$(python3 - <<'PY'
import re, sys, itertools
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
# Print one combination per line; empty line = no features at all.
combos = []
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        combos.append(",".join(c))
print("\n".join(combos))
PY
)
if [ -z "${FEATURES//[$'\n']/}" ]; then
  note "Cargo.toml declares no [features] section"
  note "=> the only combination is the empty one (default == --no-default-features)"
  COMBOS=("")
else
  mapfile -t COMBOS <<<"$FEATURES"
fi
note "combinations to test: ${#COMBOS[@]}"

# --------------------------------------------------------------------------
# 2. cargo check / clippy-clean compile for each combination
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  step "cargo check --no-default-features --features '$label'"
  if [ -z "$combo" ]; then
    cargo check --offline --no-default-features --all-targets 2>&1 | tail -3
    rc=${PIPESTATUS[0]}
  else
    cargo check --offline --no-default-features --features "$combo" --all-targets 2>&1 | tail -3
    rc=${PIPESTATUS[0]}
  fi
  [ "$rc" -eq 0 ] || { echo "  CHECK FAILED for '$label'"; FAIL=1; }
done

# --------------------------------------------------------------------------
# 3. The differential suite, per feature combination, in both test profiles
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  for profile in release debug; do
    step "cargo test ($profile) --no-default-features --features '$label'"
    args=(test --offline --no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ "$profile" = release ] && args+=(--release)
    # HSL_TEST_FEATURES makes the harness build the *cdylib under test* with the
    # same feature selection (see tests/common/mod.rs).
    HSL_TEST_FEATURES="$combo" timeout 900 cargo "${args[@]}" 2>&1 \
      | grep -E 'test result|FAILED|panicked|DIVERGENCE|^error' | head -30
    rc=${PIPESTATUS[0]}
    [ "$rc" -eq 0 ] || { echo "  TEST FAILED for '$label' ($profile)"; FAIL=1; }
  done
done

# --------------------------------------------------------------------------
# 4. Symbol-parity diff (must be empty), per feature combination
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  step "symbol diff, features '$label'"
  args=(build --offline --lib --release --no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")
  cargo "${args[@]}" >/dev/null 2>&1 || { echo "  build FAILED"; FAIL=1; continue; }
  R_SO=target/release/libhsl_to_rgb_lib.so
  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
  note "C exports   : $(printf '%s' "$c_syms" | tr '\n' ' ')"
  note "Rust exports: $(printf '%s' "$r_syms" | tr '\n' ' ')"
  if [ -n "$missing" ]; then
    echo "  MISSING FROM RUST: $missing"
    FAIL=1
  else
    note "missing symbols: NONE"
  fi
done

# --------------------------------------------------------------------------
# 5. Negative control: the suite must actually catch injected defects
# --------------------------------------------------------------------------
step "negative control (mutation testing)"
if [ -f mutate.py ] && [ -f .mutbak/lib.rs.orig ]; then
  timeout 900 python3 mutate.py configs errors fenv 2>&1 | tail -32
  rc=${PIPESTATUS[0]}
  [ "$rc" -eq 0 ] || { echo "  MUTATION CONTROL FAILED"; FAIL=1; }
else
  note "skipped (mutate.py or baseline snapshot missing)"
fi

step "summary"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
