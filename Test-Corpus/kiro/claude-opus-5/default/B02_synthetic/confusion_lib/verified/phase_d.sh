#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination x profile.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/*.so | head -1)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (the [features] section only).
# ---------------------------------------------------------------------------
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");gsub(/ /,"");if($0!="default")print}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "No [features] in Cargo.toml -> the only combination is the default one."
  COMBOS=("--no-default-features" "" "--all-features")
else
  COMBOS=("--no-default-features" "" "--all-features")
  n=$(echo "$FEATS" | wc -l)
  # Full power set of the declared features.
  mapfile -t ARR <<< "$FEATS"
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( m & (1<<i) )); then sel="$sel,${ARR[i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
fi

rc=0
for PROFILE in release dev; do
  for COMBO in "${COMBOS[@]}"; do
    echo
    echo "=============================================================="
    echo "profile=$PROFILE  combo='${COMBO:-<default>}'"
    echo "=============================================================="
    if [ "$PROFILE" = release ]; then PFLAG="--release"; DIR=release; else PFLAG=""; DIR=debug; fi

    # shellcheck disable=SC2086
    timeout 300 cargo build $PFLAG $COMBO >/dev/null 2>&1 || { echo "BUILD FAILED"; rc=1; continue; }
    R_SO="target/$DIR/libconfusion_lib.so"

    # --- symbol parity ---
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
    MISSING=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED, missing from Rust .so:"; echo "$MISSING"; rc=1
    else
      echo "symbol parity: OK ($(wc -l < /tmp/pd_c.txt) C symbols, 0 missing)"
    fi

    # --- differential tests ---
    # shellcheck disable=SC2086
    C_SO="$C_SO" RUST_SO="$R_SO" timeout 600 cargo test $PFLAG $COMBO -- --test-threads=1 2>&1 \
      | grep -E "^test |test result:|panicked|FAILED" | tail -80
    # shellcheck disable=SC2086
    C_SO="$C_SO" RUST_SO="$R_SO" timeout 600 cargo test $PFLAG $COMBO -- --test-threads=1 >/dev/null 2>&1 \
      || { echo "TESTS FAILED for profile=$PROFILE combo='$COMBO'"; rc=1; }
  done
done

echo
if [ $rc -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $rc
