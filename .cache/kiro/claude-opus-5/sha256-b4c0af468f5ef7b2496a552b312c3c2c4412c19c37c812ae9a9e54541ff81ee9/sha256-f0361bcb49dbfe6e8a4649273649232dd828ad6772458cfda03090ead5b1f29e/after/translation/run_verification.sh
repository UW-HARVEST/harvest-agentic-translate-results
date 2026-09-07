#!/usr/bin/env bash
# Rebuild the C shared objects and the Rust cdylib, then run the differential
# test-suite across every cargo feature combination declared in Cargo.toml.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== building C shared libraries ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# Feature combinations: powerset of the [features] table (empty table -> just
# the default configuration).
FEATURES=$(python3 - <<'PY'
import re,sys,itertools
s=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
feats=[]
if m:
    for line in m.group(1).splitlines():
        line=line.strip()
        if not line or line.startswith('#'): continue
        k=line.split('=')[0].strip()
        if k and k!='default': feats.append(k)
combos=[]
for n in range(len(feats)+1):
    for c in itertools.combinations(feats,n):
        combos.append(','.join(c))
print('\n'.join(combos) if combos else '')
PY
)

run_suite() {
  local label="$1"; shift
  echo "=== [$label] cargo build $* ==="
  timeout 600 cargo build "$@" >/dev/null 2>&1 || { echo "  build FAILED"; return 1; }
  timeout 600 cargo build --release "$@" >/dev/null 2>&1 || { echo "  release build FAILED"; return 1; }
  for build in debug release; do
    echo "=== [$label] cargo test $* (against target/$build cdylib) ==="
    CJSON_RUST_SO="$PWD/target/$build/libcJSON_test.so" \
      timeout 600 cargo test "$@" -- --test-threads=4 2>&1 \
      | grep -E 'test result|panicked|^error|FAILED|^test .*FAILED'
  done
}

if [ -z "$FEATURES" ]; then
  run_suite "default (no [features] table)"
else
  while IFS= read -r combo; do
    if [ -z "$combo" ]; then
      run_suite "no-default-features" --no-default-features
    else
      run_suite "$combo" --no-default-features --features "$combo"
    fi
  done <<< "$FEATURES"
fi

echo "=== symbol parity ==="
nm -D --defined-only "$ROOT/c_src/build/libcjson.so.1.7.19" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort >/tmp/c_syms.txt
nm -D --defined-only "$ROOT/c_src/build/libcJSON_test.so"   | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort >>/tmp/c_syms.txt
sort -u -o /tmp/c_syms.txt /tmp/c_syms.txt
nm -D --defined-only target/release/libcJSON_test.so | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort >/tmp/r_syms.txt
missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
if [ -n "$missing" ]; then echo "MISSING FROM RUST:"; echo "$missing"; else echo "0 missing symbols"; fi
