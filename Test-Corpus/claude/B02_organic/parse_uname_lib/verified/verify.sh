#!/usr/bin/env bash
# Phase D driver: enumerate every build configuration, check it compiles, prove
# symbol parity against the C shared object, and run Phases B+C in each one.
#
#   ./verify.sh            # everything
#   ./verify.sh symbols    # just the nm -D diff
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
FAIL=0
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { in_f = 1; next }
    /^\[/                { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
step "Feature enumeration"
if [ "$N" -eq 0 ]; then
  echo "  Cargo.toml has no [features] -> exactly one combination (empty/default)"
  COMBOS=("")
else
  echo "  features: ${FEATURES[*]}"
  COMBOS=()
  for ((mask = 0; mask < (1 << N); mask++)); do
    sel=()
    for ((i = 0; i < N; i++)); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("$(IFS=,; echo "${sel[*]}")")
  done
fi
echo "  ${#COMBOS[@]} combination(s): $(printf '[%s] ' "${COMBOS[@]}")"

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check --no-default-features --features <combo>"
for combo in "${COMBOS[@]}"; do
  label=${combo:-<none>}
  if timeout 600 cargo check --offline --no-default-features --features "$combo" \
       --all-targets > "${TMPDIR:-/tmp}/check.log" 2>&1; then
    ok "check [$label]"
  else
    bad "check [$label]"; tail -30 "${TMPDIR:-/tmp}/check.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Build the C shared library
# ---------------------------------------------------------------------------
step "Build the C shared object"
mkdir -p c_src/build
if ( cd c_src/build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
     && cmake --build . ) > "${TMPDIR:-/tmp}/cmake.log" 2>&1; then
  ok "cmake build -> c_src/build/libdriver.so"
else
  bad "cmake build"; tail -30 "${TMPDIR:-/tmp}/cmake.log"
fi
C_SO=$ROOT/c_src/build/libdriver.so

# ---------------------------------------------------------------------------
# 4. Symbol parity, for every combination x profile
# ---------------------------------------------------------------------------
symbols_for_profile() { # $1 = profile flag ("" | --release), $2 = combo
  local flag=$1 combo=$2 dir
  dir=debug; [ -n "$flag" ] && dir=release
  timeout 600 cargo build --offline $flag --no-default-features --features "$combo" \
    > "${TMPDIR:-/tmp}/build.log" 2>&1 || { tail -20 "${TMPDIR:-/tmp}/build.log"; return 1; }
  nm -D --defined-only "$ROOT/target/$dir/libdriver.so" | awk '$2=="T"{print $3}' | sort
}

step "Symbol parity (nm -D)"
nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "${TMPDIR:-/tmp}/c.syms"
echo "  C exports $(wc -l < "${TMPDIR:-/tmp}/c.syms") symbol(s): $(tr '\n' ' ' < "${TMPDIR:-/tmp}/c.syms")"
for combo in "${COMBOS[@]}"; do
  for flag in "" "--release"; do
    label="${combo:-<none>}${flag:+ $flag}"
    if ! symbols_for_profile "$flag" "$combo" > "${TMPDIR:-/tmp}/r.syms"; then
      bad "build [$label]"; continue
    fi
    missing=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
    if [ -z "$missing" ]; then
      ok "symbol diff empty [$label]"
    else
      bad "missing from Rust .so [$label]: $(echo "$missing" | tr '\n' ' ')"
    fi
    # undefined non-libc symbols in the Rust .so
    undef=$(nm -D -u "$ROOT/target/$([ -n "$flag" ] && echo release || echo debug)/libdriver.so" \
            | awk '$1=="U"{print $2}' | sed 's/@.*//' | sort -u \
            | grep -x -F -f "${TMPDIR:-/tmp}/c.syms" || true)
    if [ -z "$undef" ]; then
      ok "no unresolved project symbols [$label]"
    else
      bad "unresolved project symbols [$label]: $undef"
    fi
  done
done

# ---------------------------------------------------------------------------
# 5. Phases B + C in every combination x profile
# ---------------------------------------------------------------------------
if [ "${1:-all}" = symbols ]; then
  exit $FAIL
fi

step "Phases B + C differential tests"
for combo in "${COMBOS[@]}"; do
  for flag in "" "--release"; do
    label="${combo:-<none>}${flag:+ $flag}"
    log=${TMPDIR:-/tmp}/test${flag//-/}.log
    # fork-based and fd-2-based suites must be single threaded
    if timeout 600 cargo test --offline $flag --no-default-features --features "$combo" \
         -- --test-threads=1 > "$log" 2>&1; then
      totals=$(grep '^test result' "$log" \
               | awk -F'[ ;]' '{p+=$4; f+=$6} END {printf "%d passed, %d failed", p, f}')
      ok "cargo test [$label] -- $totals"
      grep -q 'test result: FAILED' "$log" && bad "a suite reported FAILED [$label]"
    else
      bad "cargo test [$label]"
      grep -E '^(test result|failures:|thread .* panicked|error)' -A3 "$log" | tail -40
    fi
  done
done

step "Summary"
[ "$FAIL" -eq 0 ] && echo "  ALL CHECKS PASSED" || echo "  THERE WERE FAILURES"
exit $FAIL
