#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml,
# then run cargo check + the full differential suite for each, in both the
# debug and release profiles.
set -u
cd "$(dirname "$0")/.."

# --- enumerate features mechanically from Cargo.toml -----------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{sub(/ *=.*/,"");print}' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table: the only two possible configurations.
  COMBOS+=("")                        # default
  COMBOS+=("--no-default-features")   # identical, since no defaults exist
else
  n=${#FEATURES[@]}
  for ((m=0; m<(1<<n); m++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( m & (1<<i) )) && sel+=("${FEATURES[$i]}"); done
    if [ ${#sel[@]} -eq 0 ]; then COMBOS+=("--no-default-features")
    else COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")"); fi
  done
  COMBOS+=("")  # plus the plain default
fi

rc=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile:-debug}' combo='${combo:-<default>}'"
    echo "=============================================================="
    echo "### $label"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check --offline $profile $combo >/dev/null 2>&1; then
      echo "CHECK FAIL: $label"; rc=1; continue
    fi
    # `cargo test` does NOT build the cdylib artifact, so build it explicitly
    # for THIS profile/combo before the tests try to dlopen it.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build --offline $profile $combo >/dev/null 2>&1; then
      echo "BUILD FAIL: $label"; rc=1; continue
    fi
    # shellcheck disable=SC2086
    out=$(timeout 600 cargo test --offline $profile $combo -- --test-threads=1 2>&1)
    echo "$out" | grep -E '^test result:|^error' || true
    if echo "$out" | grep -q 'FAILED\|^error'; then
      echo "TEST FAIL: $label"; echo "$out" | tail -40; rc=1
    else
      echo "OK: $label"
    fi
  done
done

# --- symbol parity diff ---------------------------------------------------
echo "=============================================================="
echo "### symbol parity (nm -D --defined-only)"
C_SO=../c_src/build/libdriver.so
for profile in debug release; do
  R_SO=target/$profile/libdriver.so
  [ -f "$R_SO" ] || { echo "SYMBOL CHECK FAIL: $R_SO was never built"; rc=1; continue; }
  diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$R_SO"   | awk '{print $NF}' | sort) \
       && echo "OK: $profile symbol sets identical" \
       || { echo "SYMBOL DIFF in $profile"; rc=1; }
done

echo "=============================================================="
[ $rc -eq 0 ] && echo "ALL CONFIGURATIONS PASS" || echo "FAILURES PRESENT"
exit $rc
