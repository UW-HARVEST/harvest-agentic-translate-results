#!/usr/bin/env bash
# Full verification matrix: rebuild both libraries, then run every test under
# every feature combination and every cargo profile.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== 1. rebuild the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/cbuild.log 2>&1 || { echo "C BUILD FAILED"; tail -20 /tmp/cbuild.log; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

# Feature combinations. Cargo.toml declares no [features], so the complete set
# is the default one plus the explicit --no-default-features form. This is
# derived, not hardcoded: if features ever appear, they are enumerated here.
FEATS="$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -oP '^\s*\K[A-Za-z0-9_-]+(?=\s*=)' | grep -v '^default$' || true)"
COMBOS=("")
COMBOS+=("--no-default-features")
if [ -n "$FEATS" ]; then
  for f in $FEATS; do
    COMBOS+=("--no-default-features --features $f")
  done
  ALL="$(echo "$FEATS" | tr '\n' ',' | sed 's/,$//')"
  COMBOS+=("--no-default-features --features $ALL")
  COMBOS+=("--all-features")
fi
echo "feature combinations: ${#COMBOS[@]}"

for PROFILE in release debug; do
  PROFFLAG=""
  [ "$PROFILE" = "release" ] && PROFFLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features=[${COMBO:-default}]"
    echo
    echo "=== cargo check   | $LABEL ==="
    # shellcheck disable=SC2086
    timeout 300 cargo check $PROFFLAG $COMBO --all-targets >/tmp/check.log 2>&1 \
      || { echo "CHECK FAILED   | $LABEL"; tail -30 /tmp/check.log; FAIL=1; continue; }

    echo "=== cargo build   | $LABEL ==="
    # shellcheck disable=SC2086
    timeout 300 cargo build $PROFFLAG $COMBO >/tmp/build.log 2>&1 \
      || { echo "BUILD FAILED   | $LABEL"; tail -30 /tmp/build.log; FAIL=1; continue; }

    R_SO="target/$PROFILE/libgen_ray_lib.so"
    echo "--- symbol diff (C -> Rust), $LABEL ---"
    MISSING="$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))"
    if [ -n "$MISSING" ]; then
      echo "MISSING SYMBOLS | $LABEL:"; echo "$MISSING"; FAIL=1
    else
      echo "symbol diff empty ($(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u | wc -l) exported)"
    fi

    echo "=== cargo test    | $LABEL ==="
    # shellcheck disable=SC2086
    timeout 600 cargo test $PROFFLAG $COMBO -- --test-threads=4 >/tmp/test.log 2>&1
    RC=$?
    grep -E '^test result:' /tmp/test.log | sed "s/^/  /"
    if [ $RC -ne 0 ]; then
      echo "TESTS FAILED    | $LABEL"
      grep -E 'FAILED|panicked|DIVERGENCE' /tmp/test.log | head -40
      FAIL=1
    fi
  done
done

echo
if [ $FAIL -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
