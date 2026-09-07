#!/usr/bin/env bash
# Phase D driver: enumerate every build configuration of the crate and run the
# full differential suite (Phases B + C + D) under each one.
set -uo pipefail
cd "$(dirname "$0")"

echo "=== declared features in Cargo.toml ==="
python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
# crude but sufficient TOML section scan
sections = re.split(r'(?m)^\[', src)
feats = []
for s in sections:
    if s.startswith('features]'):
        for line in s.splitlines()[1:]:
            line = line.split('#')[0].strip()
            if not line or line.startswith('['):
                break
            m = re.match(r'([A-Za-z0-9_\-]+)\s*=', line)
            if m:
                feats.append(m.group(1))
print('\n'.join(feats) if feats else '(none)')
PY

FEATS=$(python3 - <<'PY'
import re
src=open('Cargo.toml').read()
out=[]
for s in re.split(r'(?m)^\[', src):
    if s.startswith('features]'):
        for line in s.splitlines()[1:]:
            line=line.split('#')[0].strip()
            if not line or line.startswith('['): break
            m=re.match(r'([A-Za-z0-9_\-]+)\s*=', line)
            if m and m.group(1)!='default': out.append(m.group(1))
print(' '.join(out))
PY
)

status=0

run_suite () {
  local label="$1"; shift
  echo
  echo "########## $label ##########"
  timeout 600 cargo build --release "$@" >/dev/null 2>&1 || { echo "BUILD FAILED"; status=1; return; }
  timeout 600 cargo test --release "$@" 2>&1 | grep -E "^(test result|error|failures:)" || true
  timeout 600 cargo test --release "$@" >/dev/null 2>&1 || { echo "TESTS FAILED"; status=1; }
}

# 1. default features
run_suite "default features"

if [ -n "${FEATS// /}" ]; then
  run_suite "--no-default-features"
  # power set of the declared features
  python3 - "$FEATS" <<'PY' > /tmp/wcscat_combos.txt
import sys, itertools
f=sys.argv[1].split()
for r in range(1,len(f)+1):
    for c in itertools.combinations(f,r):
        print(','.join(c))
PY
  while read -r combo; do
    [ -z "$combo" ] && continue
    run_suite "--no-default-features --features $combo" --no-default-features --features "$combo"
  done < /tmp/wcscat_combos.txt
else
  echo
  echo "no optional features declared -> the default build is the only feature configuration"
fi

# 2. the other build profile (opt-level 0 + panic=unwind) as a distinct
#    codegen configuration of the same source.
echo
echo "########## dev-profile cdylib (opt-level 0, panic=unwind) ##########"
timeout 600 cargo build >/dev/null 2>&1 || { echo "BUILD FAILED"; status=1; }
WCSCAT_RUST_SO="$(pwd)/target/debug/libwcscat_lib.so" \
  timeout 600 cargo test --release 2>&1 | grep -E "^(test result|failures:)" || true
WCSCAT_RUST_SO="$(pwd)/target/debug/libwcscat_lib.so" \
  timeout 600 cargo test --release >/dev/null 2>&1 || { echo "TESTS FAILED"; status=1; }

# 3. binary targets?
echo
echo "=== binary targets ==="
if compgen -G "src/main.rs" > /dev/null || compgen -G "src/bin/*.rs" > /dev/null \
   || grep -q '^\[\[bin\]\]' Cargo.toml; then
  echo "binary target present - stdout comparison required"
  status=1
else
  echo "no [[bin]] / src/main.rs / src/bin -> nothing to compare on stdout"
fi

echo
if [ "$status" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$status"
