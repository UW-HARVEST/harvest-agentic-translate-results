#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination from Cargo.toml and run
# `cargo check` + the full differential test suite for each, in both profiles.
set -uo pipefail
cd "$(dirname "$0")"

# ---- Phase A.1: enumerate feature combinations mechanically ----------------
FEATURES=$(python3 - <<'PY'
import re,sys
txt=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M|re.S)
feats=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            name=line.split('=')[0].strip().strip('"')
            if name and name!='default':
                feats.append(name)
print(' '.join(feats))
PY
)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] -> exactly ONE configuration."
  COMBOS=("")
else
  # full power set
  read -ra F <<< "$FEATURES"
  n=${#F[@]}
  COMBOS=()
  for ((mask=0; mask<(1<<n); mask++)); do
    c=""
    for ((i=0;i<n;i++)); do
      if (( mask & (1<<i) )); then c="${c:+$c,}${F[$i]}"; fi
    done
    COMBOS+=("$c")
  done
fi

echo "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done
echo

fail=0
for combo in "${COMBOS[@]}"; do
  FLAGS=(--offline --no-default-features)
  [ -n "$combo" ] && FLAGS+=(--features "$combo")
  label="${combo:-<none>}"

  echo "=================================================================="
  echo "### cargo check  [features: $label]"
  if ! timeout 600 cargo check "${FLAGS[@]}" --all-targets 2>&1 | tail -5; then
    echo "CHECK FAILED for '$label'"; fail=1
  fi

  for profile in debug release; do
    RFLAG=()
    [ "$profile" = release ] && RFLAG=(--release)
    echo "### cargo test [$profile] [features: $label]"
    out=$(timeout 600 cargo test "${FLAGS[@]}" "${RFLAG[@]}" 2>&1)
    echo "$out" | grep -E "^test result|MISMATCH|^error" | sed 's/^/    /'
    if echo "$out" | grep -qE "FAILED|^error"; then
      echo "    >>> TEST FAILURE for '$label' [$profile]"
      echo "$out" | tail -40
      fail=1
    fi
  done
done

echo "=================================================================="
# ---- Phase D: symbol diff must be empty --------------------------------
echo "### nm -D symbol diff (C vs Rust)"
C_SO=c_src/build/libtranslated_rust.so
R_SO=target/dylib-release/release/libfloat2half_lib.so
if [ -f "$C_SO" ] && [ -f "$R_SO" ]; then
  csyms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)
  rsyms=$(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort)
  echo "  C exports:    $(echo "$csyms" | tr '\n' ' ')"
  echo "  Rust exports: $(echo "$rsyms" | tr '\n' ' ')"
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [ -n "$missing" ]; then
    echo "  MISSING FROM RUST: $missing"; fail=1
  else
    echo "  symbol diff: EMPTY (0 missing)"
  fi
else
  echo "  (shared objects not built yet; run cargo test first)"; fail=1
fi

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
