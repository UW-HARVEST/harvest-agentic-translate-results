#!/usr/bin/env bash
# Phase D — run the differential test suite under EVERY feature combination and
# every cargo profile. The crate declares no [features] table, so the feature
# power set is {default} == {no-default-features}; both are still exercised
# explicitly, in both debug and release profiles.
set -uo pipefail
cd "$(dirname "$0")"

FEATURES=$(python3 - <<'PY'
import re
t = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', t, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

combos=()
if [ -z "$FEATURES" ]; then
  combos+=("DEFAULT" "NONE")
else
  # full power set of declared features, plus default and none
  combos+=("DEFAULT" "NONE")
  read -ra arr <<< "$FEATURES"
  n=${#arr[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${arr[$i]}")
    done
    combos+=("$(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

rc=0
for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    case "$combo" in
    DEFAULT) fargs=() ;;
    NONE) fargs=(--no-default-features) ;;
    *) fargs=(--no-default-features --features "$combo") ;;
    esac
    label="profile=${profile:-debug} features=${combo}"
    echo "=== cargo test $label ==="
    log=$(mktemp)
    timeout 600 cargo test --offline $profile "${fargs[@]}" >"$log" 2>&1
    st=$?
    grep -E 'test result|^error|panicked at|FAILED' "$log" || true
    if [ $st -eq 0 ]; then
      echo "PASS: $label"
    else
      echo "FAIL: $label (exit $st)"
      rc=1
    fi
    rm -f "$log"
  done
done

echo "=== symbol diff (C vs Rust, both profiles, default features) ==="
CSO=$(ls ../c_src/build/lib*.so | head -1)
syms() { nm -D --defined-only --format=posix "$1" | awk '$2 ~ /^[TDBRWVGS]$/ {print $1}' | sort; }

for p in debug release; do
  RSO="target/so/$p/$p/libcall_predict_lib.so"
  if [ ! -f "$RSO" ]; then
    echo "FAIL: symbol parity ($p): $RSO not built"
    rc=1
    continue
  fi
  # Every C symbol must be present in the Rust .so (comm -23 = C-only).
  missing=$(comm -23 <(syms "$CSO") <(syms "$RSO"))
  if [ -z "$missing" ]; then
    echo "PASS: symbol parity ($p) — 0 missing"
  else
    echo "FAIL: symbol parity ($p) — missing: $missing"
    rc=1
  fi
done

echo
if [ $rc -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $rc
