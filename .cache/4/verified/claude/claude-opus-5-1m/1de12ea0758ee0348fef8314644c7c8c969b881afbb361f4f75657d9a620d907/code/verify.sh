#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration and run the full
# differential suite under each one.
#
# Usage:  ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAIL=0

echo "############ Phase A: enumerate feature combinations ############"

# Extract feature names from the [features] table in Cargo.toml (ignoring the
# implicit "default" key). If there is no [features] section this yields
# nothing, and the only valid combination is the empty set.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] -> exactly 1 valid combination (empty set)."
  COMBOS=("")
else
  echo "Declared features: $(echo "$FEATURES" | tr '\n' ' ')"
  # Full power set.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FARR[$b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Total combinations to verify: ${#COMBOS[@]}"

run() { # run <label> <cargo-args...>
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  if timeout 600 "$@" >/tmp_out 2>&1; then :; else :; fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FLAGS=(--no-default-features)
    LABEL="<no features>"
  else
    FLAGS=(--no-default-features --features "$combo")
    LABEL="$combo"
  fi

  echo
  echo "############ combination: $LABEL ############"

  for phase in check build test; do
    printf '  %-6s ... ' "$phase"
    if out=$(timeout 600 cargo "$phase" --offline "${FLAGS[@]}" 2>&1); then
      if [ "$phase" = test ]; then
        echo "OK ($(echo "$out" | grep -c '^test .* ok$') tests passed)"
      else
        echo OK
      fi
    else
      echo "FAILED"
      echo "$out" | tail -25 | sed 's/^/      /'
      FAIL=1
    fi
  done
done

echo
echo "############ default / all-features sanity ############"
for extra in "" "--all-features"; do
  printf '  cargo test %-14s ... ' "${extra:-<default>}"
  if out=$(timeout 600 cargo test --offline $extra 2>&1); then
    echo "OK ($(echo "$out" | grep -c '^test .* ok$') tests passed)"
  else
    echo FAILED
    echo "$out" | tail -25 | sed 's/^/      /'
    FAIL=1
  fi
done

echo
echo "############ release profile (optimized cdylib) ############"
printf '  cargo build --release ... '
if timeout 600 cargo build --offline --release --no-default-features >/dev/null 2>&1; then echo OK; else echo FAILED; FAIL=1; fi
printf '  cargo test  --release ... '
if out=$(timeout 600 cargo test --offline --release --no-default-features 2>&1); then
  echo "OK ($(echo "$out" | grep -c '^test .* ok$') tests passed)"
else
  echo FAILED
  echo "$out" | tail -30 | sed 's/^/      /'
  FAIL=1
fi

echo
echo "############ symbol parity (nm -D) ############"
C_SO=c_src/build/libdriver.so
R_SO=target/debug/libdriver.so
if [ -f "$C_SO" ] && [ -f "$R_SO" ]; then
  csyms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)
  rsyms=$(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  echo "  C exports   : $(echo "$csyms" | tr '\n' ' ')"
  echo "  Rust exports: $(echo "$rsyms" | tr '\n' ' ')"
  if [ -z "$missing" ]; then
    echo "  MISSING FROM RUST: (none) -- symbol diff is EMPTY"
  else
    echo "  MISSING FROM RUST: $missing"
    FAIL=1
  fi
else
  echo "  SKIP: build both .so files first"
  FAIL=1
fi

echo
if [ "$FAIL" -eq 0 ]; then echo "RESULT: ALL CONFIGURATIONS PASS"; else echo "RESULT: FAILURES PRESENT"; fi
exit "$FAIL"
