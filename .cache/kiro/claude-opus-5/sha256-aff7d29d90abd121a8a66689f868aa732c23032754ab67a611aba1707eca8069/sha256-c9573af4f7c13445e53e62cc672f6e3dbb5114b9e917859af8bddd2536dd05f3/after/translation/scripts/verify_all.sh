#!/usr/bin/env bash
# Phase D — run the full differential suite under every feature combination.
#
# Features are extracted from Cargo.toml rather than hard-coded, so this keeps
# working if features are added later.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"

if [[ ! -f "$C_SO" ]]; then
  echo "Building the C reference library..."
  (cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || exit 1
fi

# Every declared feature name in [features].
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

echo "Declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# The empty-string entry means "default features".
COMBOS=("" "--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("--no-default-features --features $combo")
    COMBOS+=("--features $combo")
  done
elif (( n > 12 )); then
  echo "More than 12 features; testing each individually plus all-features."
  for f in "${FEATURES[@]}"; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

fail=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    flags="$combo"

    label="profile='${profile:-dev}' features='${flags:-default}'"
    echo "=============================================================="
    echo ">>> cargo check   $label"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check --tests $profile $flags >/tmp/vc.log 2>&1; then
      echo "!!! CHECK FAILED: $label"; tail -20 /tmp/vc.log; fail=1; continue
    fi
    echo ">>> cargo build   $label"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $profile $flags >/tmp/vb.log 2>&1; then
      echo "!!! BUILD FAILED: $label"; tail -20 /tmp/vb.log; fail=1; continue
    fi
    echo ">>> cargo test    $label"
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile $flags -- --test-threads=1 >/tmp/vt.log 2>&1; then
      grep -E "^test result" /tmp/vt.log | sed 's/^/    /'
    else
      echo "!!! TEST FAILED: $label"
      grep -E "^test result|FAILED|DIVERGENCE|panicked" /tmp/vt.log | head -30
      fail=1
    fi
  done
done

echo "=============================================================="
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: all feature combinations and profiles passed"
