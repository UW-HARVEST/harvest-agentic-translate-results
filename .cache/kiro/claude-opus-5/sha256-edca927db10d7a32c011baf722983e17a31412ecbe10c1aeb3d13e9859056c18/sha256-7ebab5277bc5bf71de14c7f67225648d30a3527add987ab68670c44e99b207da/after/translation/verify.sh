#!/usr/bin/env bash
# Verifies the translation against the C reference for every build
# configuration: each feature combination declared in Cargo.toml, in both the
# dev and release profiles.
#
# Usage: ./verify.sh [--quick]
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
QUICK=${1:-}
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# --- 1. build the C reference -------------------------------------------------
step "building the C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
C_SO=$(ls -1 "$ROOT"/c_src/build/*.so | head -1)
echo "C library: $C_SO"

# --- 2. enumerate every feature combination ----------------------------------
# Feature names come from the [features] table; "default" is not a combination
# of its own, it is the empty set plus whatever it pulls in.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "declared features (${N}): ${FEATURES[*]:-<none>}"

COMBOS=()
COMBOS+=("")                       # --no-default-features
for ((mask = 1; mask < (1 << N); mask++)); do
  combo=""
  for ((b = 0; b < N; b++)); do
    if (( mask & (1 << b) )); then
      combo="${combo:+$combo,}${FEATURES[b]}"
    fi
  done
  COMBOS+=("$combo")
done
echo "feature combinations to verify: ${#COMBOS[@]}"

# --- 3. cargo check for every combination ------------------------------------
step "cargo check, every feature combination"
for combo in "${COMBOS[@]}"; do
  label=${combo:-<none>}
  if timeout 600 cargo check --offline --no-default-features \
       ${combo:+--features "$combo"} --all-targets >/dev/null 2>&1; then
    echo "  ok      --no-default-features --features '$label'"
  else
    fail "cargo check --no-default-features --features '$label'"
    timeout 600 cargo check --offline --no-default-features \
      ${combo:+--features "$combo"} --all-targets 2>&1 | tail -20
  fi
done
# The default feature set as a caller would get it.
if timeout 600 cargo check --offline --all-targets >/dev/null 2>&1; then
  echo "  ok      (default features)"
else
  fail "cargo check (default features)"
fi

# --- 4. exported symbols ------------------------------------------------------
compare_symbols() {
  local rust_so=$1 what=$2
  local missing
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    fail "$what is missing symbols exported by the C library: $(echo "$missing" | tr '\n' ' ')"
  else
    echo "  ok      $what exports every C symbol"
  fi
}

# --- 5. tests for every combination, both profiles ---------------------------
for profile in dev release; do
  flag=""
  outdir="debug"
  if [ "$profile" = release ]; then flag="--release"; outdir="release"; fi
  for combo in "${COMBOS[@]}"; do
    label=${combo:-<none>}
    step "$profile profile, features '$label'"
    if ! timeout 600 cargo build --offline $flag --no-default-features \
           ${combo:+--features "$combo"} >/dev/null 2>&1; then
      fail "cargo build $profile --features '$label'"
      continue
    fi
    compare_symbols "target/$outdir/libpinflate_lib.so" "$profile/'$label'"

    tests=(t00_exports t05_layout t10_stored t20_fixed t30_dynamic t35_overrun)
    if [ "$QUICK" != "--quick" ]; then tests+=(t40_fuzz); fi
    for t in "${tests[@]}"; do
      if PINFLATE_FUZZ_SCALE=${PINFLATE_FUZZ_SCALE:-25} \
         timeout 600 cargo test --offline -q $flag --no-default-features \
           ${combo:+--features "$combo"} --test "$t" >/dev/null 2>&1; then
        echo "  ok      $t"
      else
        fail "$t ($profile, features '$label')"
        PINFLATE_FUZZ_SCALE=${PINFLATE_FUZZ_SCALE:-25} \
        timeout 600 cargo test --offline $flag --no-default-features \
          ${combo:+--features "$combo"} --test "$t" 2>&1 | tail -30
      fi
    done
  done
done

step "summary"
if [ "$FAILED" -eq 0 ]; then
  echo "all configurations match the C reference"
else
  echo "there were failures"
fi
exit "$FAILED"
