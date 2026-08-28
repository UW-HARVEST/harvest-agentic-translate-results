#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration the crate exposes.
#
#   1. enumerate every valid feature combination from Cargo.toml
#   2. cargo check each combination
#   3. build the C .so and compare exported symbols against the Rust .so
#   4. run the differential test suite (debug + release) for each combination
#
# Usage: ./verify_all.sh        (run from translation/)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="$(cd "$ROOT/.." && pwd)"
C_SRC="$WORK/c_src"
C_BUILD="$C_SRC/build"
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations
# ---------------------------------------------------------------------------
note "enumerating feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "non-default features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Every subset of the declared features, plus the crate default.
COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  sel=""
  for ((i = 0; i < n; i++)); do
    if (( mask & (1 << i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
  done
  COMBOS+=("$sel")
done
echo "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check every combination
# ---------------------------------------------------------------------------
note "cargo check (all combinations, incl. default and --all-targets)"
if ! timeout 600 cargo check --all-targets > /tmp/chk_default.log 2>&1; then
  fail "cargo check (default features)"; tail -30 /tmp/chk_default.log
fi
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  if ! timeout 600 cargo check --all-targets --no-default-features \
       ${combo:+--features "$combo"} > /tmp/chk.log 2>&1; then
    fail "cargo check --no-default-features --features '$label'"
    tail -30 /tmp/chk.log
  else
    echo "ok: cargo check --no-default-features --features '$label'"
  fi
done

# ---------------------------------------------------------------------------
# 3. build the C .so and diff exported symbols
# ---------------------------------------------------------------------------
note "building C shared library"
mkdir -p "$C_BUILD"
(
  cd "$C_BUILD" \
    && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && timeout 300 cmake --build .
) > /tmp/cbuild.log 2>&1 || { fail "C build"; tail -30 /tmp/cbuild.log; }

C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C .so: $C_SO"

check_symbols() {
  local rust_so="$1"
  nm -D --defined-only "$C_SO"   | awk '$2 ~ /^[A-Za-z]$/ {print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$rust_so" | awk '$2 ~ /^[A-Za-z]$/ {print $3}' | sort -u > /tmp/r_syms.txt
  local missing
  missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then
    fail "symbols exported by C but missing from $rust_so:"
    echo "$missing"
  else
    echo "ok: all $(wc -l < /tmp/c_syms.txt) C symbols present in $(basename "$rust_so")"
  fi
}

# ---------------------------------------------------------------------------
# 4. per-combination: build cdylib, diff symbols, run differential tests
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    note "combo '$label' / $profile"

    if ! timeout 600 cargo build $relflag --no-default-features \
         ${combo:+--features "$combo"} > /tmp/build.log 2>&1; then
      fail "cargo build $profile, features '$label'"; tail -30 /tmp/build.log; continue
    fi
    RUST_SO="$ROOT/target/$profile/libspec_ray_lib.so"
    [ -f "$RUST_SO" ] || { fail "missing $RUST_SO"; continue; }
    check_symbols "$RUST_SO"

    if ! C_SO="$C_SO" RUST_SO="$RUST_SO" timeout 600 cargo test $relflag \
         --no-default-features ${combo:+--features "$combo"} \
         > /tmp/test.log 2>&1; then
      fail "cargo test $profile, features '$label'"
      grep -E "^(test |error|thread|---- )" /tmp/test.log | head -40
    else
      grep -E "test result" /tmp/test.log
    fi
  done
done

# also the plain default configuration
for profile in debug release; do
  relflag=""; [ "$profile" = release ] && relflag="--release"
  note "default features / $profile"
  timeout 600 cargo build $relflag > /tmp/build.log 2>&1 || { fail "default build $profile"; tail -30 /tmp/build.log; continue; }
  RUST_SO="$ROOT/target/$profile/libspec_ray_lib.so"
  check_symbols "$RUST_SO"
  if ! C_SO="$C_SO" RUST_SO="$RUST_SO" timeout 600 cargo test $relflag > /tmp/test.log 2>&1; then
    fail "default test $profile"
    grep -E "^(test |error|thread|---- )" /tmp/test.log | head -40
  else
    grep -E "test result" /tmp/test.log
  fi
done

note "summary"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
