#!/usr/bin/env bash
# Phase D driver: enumerate every Cargo feature combination and every Rust
# build artifact, and run the full differential suite against each.
#
# Usage: ./verify_all.sh          (run from translation/)
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
fail=0

# --- 1. Build the C ground truth ------------------------------------------
echo "== building C ground truth =="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(readlink -f ../c_src/build/libdriver.so)
echo "   $C_SO"

# --- 2. Enumerate declared features ---------------------------------------
# Read the [features] table out of Cargo.toml. Anything other than `default`
# is an optional feature that must be verified on its own.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "== declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)} =="

# Combination list. With no declared features the only reachable
# configuration is the default one; still exercise --no-default-features
# explicitly so an implicit default cannot hide a code path.
COMBOS=("" "--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  # every individual feature, plus the all-features combination
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--all-features")
  # pairwise combinations
  n=${#FEATURES[@]}
  for ((i=0; i<n; i++)); do
    for ((j=i+1; j<n; j++)); do
      COMBOS+=("--no-default-features --features ${FEATURES[i]},${FEATURES[j]}")
    done
  done
fi

# --- 3. cargo check every combination -------------------------------------
echo
echo "== cargo check across ${#COMBOS[@]} combination(s) =="
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default)"}
  if timeout "$TIMEOUT" cargo check $combo >/tmp/chk.log 2>&1; then
    echo "  check  $label ... ok"
  else
    echo "  check  $label ... FAILED"; tail -20 /tmp/chk.log; fail=1
  fi
done

# --- 4. differential suite: every combination x every build profile -------
echo
echo "== differential suite across combination(s) x build profile(s) =="
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default)"}
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"

    # Build the cdylib for this combo/profile and point the tests at it.
    if ! timeout "$TIMEOUT" cargo build --lib $relflag $combo >/tmp/build.log 2>&1; then
      echo "  build  $label / $profile ... FAILED"; tail -20 /tmp/build.log; fail=1; continue
    fi
    RUST_SO=$(readlink -f "target/$profile/libdriver.so")
    if [ ! -f "$RUST_SO" ]; then
      echo "  build  $label / $profile ... no libdriver.so produced"; fail=1; continue
    fi

    # Tests themselves always build unoptimised (the release profile sets
    # panic="abort", which the runner's catch_unwind needs to avoid); only the
    # library under test changes profile, via DRIVER_RUST_SO.
    out=$(DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$RUST_SO" \
          timeout "$TIMEOUT" cargo test --test differential $combo 2>&1)
    if grep -q '^test result: ok' <<<"$out" && ! grep -q 'FAILED' <<<"$out"; then
      passed=$(grep -oP '^test result: ok\. \K[0-9]+' <<<"$out" | tail -1)
      echo "  test   $label / $profile ... ok ($passed cases)"
    else
      echo "  test   $label / $profile ... FAILED"
      grep -E 'FAILED|^---- |^test result' <<<"$out" | head -30
      fail=1
    fi
  done
done

# --- 5. symbol parity -----------------------------------------------------
echo
echo "== symbol parity (nm -D, defined-only) =="
filter() {
  nm -D --defined-only "$1" | awk '{print $NF}' \
    | grep -vE '^(_ITM_(de)?registerTMCloneTable|__gmon_start__|__cxa_.*|_edata|_end|_fini|_init)$' \
    | sort -u
}
for profile in debug release; do
  RUST_SO="target/$profile/libdriver.so"
  [ -f "$RUST_SO" ] || continue
  missing=$(comm -23 <(filter "$C_SO") <(filter "$RUST_SO"))
  if [ -z "$missing" ]; then
    echo "  $profile: 0 missing symbols ok"
  else
    echo "  $profile: MISSING: $missing"; fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASE D CHECKS PASSED"
else
  echo "PHASE D FAILURES PRESENT"
fi
exit "$fail"
