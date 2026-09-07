#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination x profile.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/lib*.so | head -1)
export C_SO_PATH="$(readlink -f "$C_SO")"

echo "=== Feature enumeration from Cargo.toml ==="
# Collect declared features (excluding "default"); build the full power set.
mapfile -t FEATS < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+ *=/ {gsub(/ *=.*/,""); if ($0 != "default") print}
' Cargo.toml)
echo "declared features: ${#FEATS[@]} (${FEATS[*]:-none})"

COMBOS=()
n=${#FEATS[@]}
if [ "$n" -eq 0 ]; then
  COMBOS+=("--no-default-features")   # no features exist -> only the empty config
  COMBOS+=("")                        # default
else
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((b=0; b<n; b++)); do (( mask & (1<<b) )) && sel+=("${FEATS[$b]}"); done
    if [ ${#sel[@]} -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("")                        # default feature set
fi
echo "combinations to verify: ${#COMBOS[@]}"

FAIL=0
for profile in debug release; do
  PFLAG=""; [ "$profile" = release ] && PFLAG="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"
    echo
    echo "################ $label ################"

    if ! timeout 600 cargo build $PFLAG $combo >/tmp/pd_build.log 2>&1; then
      echo "BUILD FAILED: $label"; tail -20 /tmp/pd_build.log; FAIL=1; continue
    fi
    export RUST_SO_PATH="$(readlink -f "target/$profile/libhdr_bitrate_lib.so")"

    # --- symbol parity for this configuration ---
    diff <(nm -D --defined-only "$C_SO_PATH" | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$RUST_SO_PATH" | awk '{print $NF}' | sort -u \
            | grep -vE '^(_ZN|rust_|__rust|_ITM_|__cxa|_Unwind|__tls|_edata|_end|__bss_start)') \
         > /tmp/pd_symdiff.txt
    if [ -s /tmp/pd_symdiff.txt ]; then
      echo "SYMBOL DIFF NOT EMPTY ($label):"; cat /tmp/pd_symdiff.txt; FAIL=1
    else
      echo "symbol parity: OK (diff empty)"
    fi

    # --- Phases B + C for this configuration ---
    if timeout 600 cargo test $PFLAG $combo >/tmp/pd_test.log 2>&1; then
      grep -E '^test result' /tmp/pd_test.log | sed 's/^/  /'
    else
      echo "TESTS FAILED: $label"; grep -E 'FAILED|DIVERGENCE|panicked' /tmp/pd_test.log | head -20; FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then echo "PHASE D: ALL CONFIGURATIONS PASSED"; else echo "PHASE D: FAILURES PRESENT"; fi
exit $FAIL
