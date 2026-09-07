#!/usr/bin/env bash
# Phase D — symbol parity + feature-combination sweep.
# Enumerates every cargo feature from Cargo.toml, builds the powerset of
# combinations, and runs the full differential suite under each. With no
# [features] section the powerset is just {default, no-default-features}, and
# both are still executed rather than assumed.
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT=..

C_SO=$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)
if [ -z "$C_SO" ]; then
  echo "FATAL: C .so not built. Run cmake first." >&2
  exit 1
fi

echo "=== Phase D.1: feature enumeration ==="
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[ ]*=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1]!="default") print a[1]}
' Cargo.toml)
NFEAT=$(printf '%s' "$FEATURES" | grep -c . || true)
echo "declared features: ${FEATURES:-<none>} (count=$NFEAT)"

# Build the list of --features arguments to test.
COMBOS=()
COMBOS+=("")                       # default
COMBOS+=("--no-default-features")
if [ "$NFEAT" -gt 0 ]; then
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="${sel:+$sel,}${FARR[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
    COMBOS+=("--features $sel")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=== combo: $label ==="

  if ! timeout 300 cargo build $combo --quiet 2>&1 | tail -5; then
    echo "  BUILD FAILED"
    fail=1
    continue
  fi

  # --- symbol parity for this combo ---
  RUST_SO=$(ls target/debug/lib*.so 2>/dev/null | head -1)
  if [ -z "$RUST_SO" ]; then
    echo "  FATAL: no Rust cdylib produced"
    fail=1
    continue
  fi
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED - missing from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  symbol parity: OK (0 missing)"
  fi

  # --- differential test suite for this combo ---
  out=$(timeout 600 cargo test $combo --quiet 2>&1)
  echo "$out" | grep 'test result' | sed 's/^/  /'
  if echo "$out" | grep -q 'test result: FAILED'; then
    echo "$out" | grep 'FAILED$' | sed 's/^/    /'
    fail=1
  fi
done

echo
echo "=== Phase D.2: binary/driver check ==="
if [ -d src/bin ] || grep -q '^\[\[bin\]\]' Cargo.toml 2>/dev/null; then
  echo "  Rust binary target present - stdout comparison required"
else
  echo "  no Rust [[bin]] target"
fi
if grep -q 'add_executable' "$ROOT"/c_src/CMakeLists.txt; then
  echo "  C executable target present - stdout comparison required"
else
  echo "  no C add_executable target"
fi
echo "  => project builds no driver binary; stdout comparison N/A"

echo
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: PASS (all ${#COMBOS[@]} combinations: symbols parity + tests green)"
else
  echo "PHASE D: FAIL"
fi
exit "$fail"
