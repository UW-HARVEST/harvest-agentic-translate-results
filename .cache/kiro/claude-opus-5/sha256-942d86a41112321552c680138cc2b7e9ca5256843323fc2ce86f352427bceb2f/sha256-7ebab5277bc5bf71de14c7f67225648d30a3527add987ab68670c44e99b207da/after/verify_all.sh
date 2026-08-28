#!/usr/bin/env bash
# Verify the translation across every declared feature combination:
#   * cargo check
#   * exported-symbol parity with the C shared object (nm -D)
#   * the differential test suite
#
# Usage: ./verify_all.sh
set -uo pipefail
cd "$(dirname "$0")"

ROOT="$PWD"
TRANS="$ROOT/translation"
CSO="$ROOT/c_src/build/libharvest-work-89nuEu.so"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$TRANS/Cargo.toml"
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  note "Cargo.toml declares no [features]; the only configuration is the default one"
  COMBOS=("")
else
  # Power set of the declared features.
  n=${#FEATURES[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("$(IFS=,; echo "${sel[*]}")")
  done
fi

note "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  [${c:-<none>}]"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared object
# ---------------------------------------------------------------------------
note "building the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/verify-cmake.log 2>&1 \
    && cmake --build . >>/tmp/verify-cmake.log 2>&1 ) \
  || { echo "C build FAILED, see /tmp/verify-cmake.log"; exit 1; }
CSO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "C .so: $CSO"

defined_syms() { nm -D --defined-only "$1" | awk '{print $3}' | grep -v '^$' | sort -u; }
defined_syms "$CSO" >/tmp/verify-c-syms.txt
echo "C exports $(wc -l </tmp/verify-c-syms.txt) symbols"

# ---------------------------------------------------------------------------
# 3. Per-combination: check, symbol parity, tests
# ---------------------------------------------------------------------------
cd "$TRANS"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  if [ -n "$combo" ]; then
    FARGS=(--no-default-features --features "$combo")
  else
    FARGS=(--no-default-features)
  fi

  note "cargo check [$label]"
  if ! timeout 600 cargo check "${FARGS[@]}" >/tmp/verify-check.log 2>&1; then
    echo "FAIL: cargo check [$label]"; tail -40 /tmp/verify-check.log; fail=1; continue
  fi
  echo "ok"

  note "symbol parity [$label]"
  for profile in debug release; do
    if [ "$profile" = release ]; then
      timeout 600 cargo build --release "${FARGS[@]}" >/tmp/verify-build.log 2>&1
    else
      timeout 600 cargo build "${FARGS[@]}" >/tmp/verify-build.log 2>&1
    fi
    if [ $? -ne 0 ]; then
      echo "FAIL: cargo build --$profile [$label]"; tail -40 /tmp/verify-build.log; fail=1; continue
    fi
    RSO="$TRANS/target/$profile/libconfusion_lib.so"
    defined_syms "$RSO" >/tmp/verify-r-syms.txt
    missing=$(comm -23 /tmp/verify-c-syms.txt /tmp/verify-r-syms.txt)
    if [ -n "$missing" ]; then
      echo "FAIL: $profile is missing symbols exported by the C .so:"
      echo "$missing" | sed 's/^/  /'
      fail=1
    else
      echo "$profile: all $(wc -l </tmp/verify-c-syms.txt) C symbols present"
    fi
  done

  note "cargo test [$label]"
  if ! timeout 600 cargo test "${FARGS[@]}" >/tmp/verify-test.log 2>&1; then
    echo "FAIL: cargo test [$label]"; tail -60 /tmp/verify-test.log; fail=1; continue
  fi
  grep -aE "test result:" /tmp/verify-test.log | tail -2
done

# ---------------------------------------------------------------------------
# 4. Re-run the suite against C built by other compilers / optimisation levels
# ---------------------------------------------------------------------------
note "differential suite against alternative C builds"
for spec in "gcc:Debug" "gcc:Release" "gcc:MinSizeRel" "clang:Debug" "clang:Release"; do
  cc=${spec%%:*}; bt=${spec##*:}
  command -v "$cc" >/dev/null || { echo "skip $spec ($cc not installed)"; continue; }
  d="/tmp/verify-cbuild-$cc-$bt"
  rm -rf "$d"
  cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_C_COMPILER="$cc" -DCMAKE_BUILD_TYPE="$bt" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/verify-altc.log 2>&1 \
    && cmake --build "$d" >>/tmp/verify-altc.log 2>&1 \
    || { echo "skip $spec (build failed)"; continue; }
  so=$(ls "$d"/*.so | head -1)
  if C2RUST_C_SO="$so" timeout 600 cargo test >/tmp/verify-test-alt.log 2>&1; then
    echo "$spec: $(grep -aE 'test result: ok' /tmp/verify-test-alt.log | tail -1)"
  else
    echo "FAIL: differential suite against $spec"
    grep -aE "FAILED|mismatch" /tmp/verify-test-alt.log | head -20
    fail=1
  fi
done

note "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$fail"
