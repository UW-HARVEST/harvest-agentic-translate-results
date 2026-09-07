#!/usr/bin/env bash
# Phase D sweep: symbol parity + every feature combination, both profiles.
set -uo pipefail

cd "$(dirname "$0")"
fail=0

C_SO=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
if [[ -z "${C_SO}" ]]; then
  echo "FATAL: C .so not built"; exit 1
fi

# ---- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' Cargo.toml)

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# combos: always the default build and the no-default-features build; plus the
# power set of declared features (there are none in this crate).
COMBOS=("" "--no-default-features")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
fi

for profile in debug release; do
  relflag=""
  [[ $profile == release ]] && relflag="--release"
  for combo in "${COMBOS[@]}"; do
    flags="${combo}"
    echo
    echo "===== profile=${profile} combo='${flags:-default}' ====="

    if ! timeout 600 cargo build $relflag $flags >/tmp/build.$$.log 2>&1; then
      echo "BUILD FAILED"; tail -20 /tmp/build.$$.log; fail=1; continue
    fi

    R_SO="target/${profile}/libpow43_lib.so"
    if [[ ! -f $R_SO ]]; then echo "MISSING $R_SO"; fail=1; continue; fi

    # ---- symbol parity ----
    nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u > /tmp/csyms.$$
    nm -D --defined-only "$R_SO"  | awk '{print $NF}' | sort -u > /tmp/rsyms.$$
    missing=$(comm -23 /tmp/csyms.$$ /tmp/rsyms.$$)
    if [[ -n $missing ]]; then
      echo "SYMBOL DIFF (in C, missing from Rust):"; echo "$missing"; fail=1
    else
      echo "symbol diff: EMPTY ($(wc -l < /tmp/csyms.$$) C symbol(s) all present)"
    fi
    # undefined non-libc symbols in the Rust .so
    undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
      | grep -v '@GLIBC' | grep -v '@GCC' \
      | grep -vE '^(_ITM_|__cxa_|__gmon_start__|__tls_get_addr|gettid|_Unwind_)' || true)
    if [[ -n $undef ]]; then
      echo "UNDEFINED NON-LIBC SYMBOLS:"; echo "$undef"; fail=1
    else
      echo "undefined non-libc symbols: 0"
    fi

    # ---- differential tests ----
    if timeout 600 cargo test $relflag $flags >/tmp/test.$$.log 2>&1; then
      grep -E '^test result:' /tmp/test.$$.log | sed 's/^/  /'
    else
      echo "TESTS FAILED"; grep -E '^(test result:|---- |thread)' /tmp/test.$$.log | head -40; fail=1
    fi
  done
done

rm -f /tmp/csyms.$$ /tmp/rsyms.$$ /tmp/build.$$.log /tmp/test.$$.log
echo
if ((fail)); then echo "SWEEP: FAILURES PRESENT"; exit 1; else echo "SWEEP: ALL GREEN"; fi
