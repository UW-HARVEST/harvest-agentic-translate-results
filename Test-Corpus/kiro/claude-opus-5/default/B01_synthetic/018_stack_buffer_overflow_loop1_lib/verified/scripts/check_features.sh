#!/usr/bin/env bash
# Phase D: enumerate every feature combination declared in Cargo.toml and run
# `cargo check` + the full differential suite for each. Also runs the two build
# profiles, since `panic = "abort"` only applies to release.
#
# Usage: scripts/check_features.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE="$PWD"
C_BUILD="$CRATE/../c_src/build"

fail=0

# --- 1. Ensure the C reference library exists -------------------------------
if [[ ! -f "$C_BUILD/libdriver.so" ]]; then
  echo "== building C reference library =="
  (mkdir -p "$C_BUILD" && cd "$C_BUILD" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "FAIL: C build"; exit 1; }
fi

# --- 2. Extract feature names from Cargo.toml ------------------------------
# Everything between a [features] header and the next [section] header.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 3. Build the combination list ------------------------------------------
# Always: default, no-default-features, all-features. Plus the power set of the
# declared features (capped so the loop stays bounded).
COMBOS=()
COMBOS+=("--")                       # default features
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo+="${FEATURES[$i]},"; fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
elif (( n > 12 )); then
  echo "WARN: $n features -> power set too large; testing each feature singly"
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

# --- 4. Run every combination against both profiles ------------------------
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    if [[ "$combo" == "--" ]]; then flags=""; else flags="$combo"; fi
    label="profile='${profile_flag:-debug}' features='${flags:-default}'"

    echo "== cargo check   $label =="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $profile_flag $flags >/dev/null 2>&1; then
      echo "FAIL: cargo check ($label)"; fail=1; continue
    fi

    # Both cdylibs must exist so the suite compares every profile's artifact.
    timeout 600 cargo build >/dev/null 2>&1
    timeout 600 cargo build --release >/dev/null 2>&1

    echo "== cargo test    $label =="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $profile_flag $flags 2>&1 | tee /tmp/ft.log | tail -3; then
      echo "FAIL: cargo test ($label)"; fail=1; continue
    fi
    if grep -qE '^test result: FAILED|panicked' /tmp/ft.log; then
      echo "FAIL: test failures ($label)"; fail=1
    fi
  done
done

# --- 5. Symbol parity for both profiles ------------------------------------
for p in debug release; do
  so="target/$p/libdriver.so"
  [[ -f "$so" ]] || continue
  echo "== symbol diff: C .so vs $so =="
  nm -D --defined-only "$C_BUILD/libdriver.so" | awk 'NF==3{print $3}' | sort -u > /tmp/csym
  nm -D --defined-only "$so"                   | awk 'NF==3{print $3}' | sort -u > /tmp/rsym
  missing=$(comm -23 /tmp/csym /tmp/rsym)
  if [[ -n "$missing" ]]; then
    echo "FAIL: missing from $so:"; echo "$missing"; fail=1
  else
    echo "OK: 0 missing symbols ($(wc -l < /tmp/csym) exported by C)"
  fi
done

if (( fail )); then echo; echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo; echo "RESULT: all feature combinations and profiles pass"
