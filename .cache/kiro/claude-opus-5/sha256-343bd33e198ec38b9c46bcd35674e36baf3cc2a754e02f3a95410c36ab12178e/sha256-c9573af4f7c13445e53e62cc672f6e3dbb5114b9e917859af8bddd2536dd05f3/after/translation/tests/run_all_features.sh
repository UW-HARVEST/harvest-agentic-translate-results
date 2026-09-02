#!/usr/bin/env bash
# Phase D — run the whole differential suite under every feature combination
# and every build profile, without repeating commands by hand.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a [features] entry automatically widens the matrix.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"

TIMEOUT=${TIMEOUT:-600}
fail=0

echo "=== rebuilding the C shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { echo "FAIL: C build"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "C .so: $C_SO"

# --- enumerate feature combinations from Cargo.toml -------------------------
# Every key of the [features] table, plus the always-available meta flags.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml
)

COMBOS=("" "--no-default-features" "--all-features")
if [ ${#FEATURES[@]} -gt 0 ]; then
  echo "declared features: ${FEATURES[*]}"
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--features $f")
  done
  # Full powerset of the declared features (guard against a huge matrix).
  n=${#FEATURES[@]}
  if [ "$n" -le 8 ]; then
    for ((mask = 1; mask < (1 << n); mask++)); do
      sel=()
      for ((i = 0; i < n; i++)); do
        (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
      done
      [ ${#sel[@]} -gt 1 ] && COMBOS+=("--no-default-features --features $(
        IFS=,
        echo "${sel[*]}"
      )")
    done
  fi
else
  echo "declared features: (none — Cargo.toml has no [features] table)"
fi

# Deduplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo
echo "=== matrix: ${#COMBOS[@]} feature combination(s) x 2 profile(s) ==="

for profile in release debug; do
  prof_flag=""
  [ "$profile" = release ] && prof_flag="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"
    echo
    echo "--- cargo check $prof_flag $combo ---"
    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo check $prof_flag $combo --all-targets >/tmp/pc.log 2>&1; then
      echo "FAIL (check): $label"; tail -30 /tmp/pc.log; fail=1; continue
    fi

    echo "--- cargo build $prof_flag $combo ---"
    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build $prof_flag $combo >/tmp/pb.log 2>&1; then
      echo "FAIL (build): $label"; tail -30 /tmp/pb.log; fail=1; continue
    fi

    RUST_SO="target/$profile/libgotomach_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      echo "FAIL: $RUST_SO not produced ($label)"; fail=1; continue
    fi

    # Symbol diff, independently of the test suite.
    miss=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
    if [ -n "$miss" ]; then
      echo "FAIL: symbols missing from $RUST_SO ($label):"; echo "$miss"; fail=1
    else
      echo "symbol diff: empty ($label)"
    fi

    echo "--- cargo test $prof_flag $combo ---"
    # shellcheck disable=SC2086
    if timeout "$TIMEOUT" cargo test $prof_flag $combo >/tmp/pt.log 2>&1; then
      grep -h "^test result:" /tmp/pt.log | sed "s/^/    /"
      echo "PASS: $label"
    else
      echo "FAIL (test): $label"
      grep -vE "^\[(INFO|ERROR|WARNING)\]" /tmp/pt.log | tail -40
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
