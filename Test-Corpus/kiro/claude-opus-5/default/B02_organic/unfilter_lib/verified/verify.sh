#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
#
# Everything here is derived mechanically; nothing is hard-coded per-feature.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO=$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)
R_SO="$ROOT/translation/target/release/libunfilter_lib.so"
fail=0

echo "=============================================================="
echo "Phase D.1 — symbol parity (nm -D)"
echo "=============================================================="
if [ -z "${C_SO:-}" ]; then
  echo "FAIL: no C .so; build it first" >&2
  exit 1
fi
cargo build --release >/dev/null 2>&1 || { echo "FAIL: cargo build --release"; exit 1; }

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
echo "C exports   : $(wc -l < /tmp/pd_c.txt)"
echo "Rust exports: $(wc -l < /tmp/pd_r.txt)"
MISSING=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
if [ -n "$MISSING" ]; then
  echo "FAIL: symbols exported by C but MISSING from Rust:"
  echo "$MISSING"
  fail=1
else
  echo "PASS: symbol diff is empty (every C export is present in the Rust .so)"
fi

echo
echo "Undefined non-libc symbols in the Rust .so:"
UNDEF=$(ldd -r "$R_SO" 2>&1 | grep -i "undefined symbol" || true)
if [ -n "$UNDEF" ]; then
  echo "FAIL: $UNDEF"
  fail=1
else
  echo "PASS: none"
fi

echo
echo "=============================================================="
echo "Phase D.2 — enumerate feature combinations from Cargo.toml"
echo "=============================================================="
# Extract the [features] section, if any.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
     split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
     if (a[1] != "default") print a[1];
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] section."
  echo "=> exactly one configuration exists: the default (no features)."
  COMBOS=("__default__")
else
  echo "Declared features: $FEATURES"
  # power set
  read -r -a FARR <<< "$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then
        combo="${combo:+$combo,}${FARR[b]}"
      fi
    done
    COMBOS+=("${combo:-__none__}")
  done
fi
echo "Combinations to verify: ${#COMBOS[@]}"

echo
echo "=============================================================="
echo "Phase D.3 — cargo check + full test suite per combination"
echo "=============================================================="
TESTS=(t00_smoke t01_unfilter_valid t02_inflate_valid t03_globals t04_errors t05_fuzz t06_lens_overflow)

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) ARGS=() ; label="default" ;;
    __none__)    ARGS=(--no-default-features) ; label="no-default-features" ;;
    *)           ARGS=(--no-default-features --features "$combo") ; label="$combo" ;;
  esac
  echo
  echo "--- combination: $label ---"
  if ! cargo check "${ARGS[@]}" >/tmp/pd_check.log 2>&1; then
    echo "FAIL: cargo check ($label)"; tail -20 /tmp/pd_check.log; fail=1; continue
  fi
  echo "  cargo check: ok"
  if ! cargo build --release "${ARGS[@]}" >/tmp/pd_build.log 2>&1; then
    echo "FAIL: cargo build --release ($label)"; tail -20 /tmp/pd_build.log; fail=1; continue
  fi
  # re-verify symbol parity for THIS build of the .so
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
  M=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
  if [ -n "$M" ]; then
    echo "FAIL: missing symbols under $label: $M"; fail=1
  else
    echo "  symbol parity: ok"
  fi
  for t in "${TESTS[@]}"; do
    printf '  %-22s ' "$t"
    if timeout 1800 cargo test --release "${ARGS[@]}" --test "$t" -- --test-threads=1 \
         >/tmp/pd_test_$t.log 2>&1; then
      grep -h "test result:" /tmp/pd_test_$t.log | tail -1
    else
      echo "FAIL"
      grep -E "DIVERGENCE|panicked|test result:" /tmp/pd_test_$t.log | head -20
      fail=1
    fi
  done
done

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: ALL CHECKS PASSED"
else
  echo "PHASE D: FAILURES PRESENT"
fi
echo "=============================================================="
exit "$fail"
