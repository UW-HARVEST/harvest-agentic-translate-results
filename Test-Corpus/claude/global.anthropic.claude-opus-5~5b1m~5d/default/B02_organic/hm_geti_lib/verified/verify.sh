#!/bin/sh
# Full verification driver: builds both libraries, checks symbol parity, and
# runs every differential test under every feature combination.
set -e
cd "$(dirname "$0")"
ROOT="$(pwd)/.."

echo "=== building C shared object ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
CSO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "  $CSO"

echo "=== enumerating feature combinations ==="
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATURES" ]; then
  COMBOS="__default__"
  echo "  no [features] table -> single configuration (default)"
else
  COMBOS="__default__ __none__"
  for f in $FEATURES; do COMBOS="$COMBOS $f"; done
  echo "  features: $FEATURES"
fi

for combo in $COMBOS; do
  case "$combo" in
    __default__) ARGS="" ; LABEL="default" ;;
    __none__)    ARGS="--no-default-features" ; LABEL="no-default-features" ;;
    *)           ARGS="--no-default-features --features $combo" ; LABEL="$combo" ;;
  esac
  echo
  echo "=== configuration: $LABEL ==="
  cargo build --offline --release $ARGS
  RSO=target/release/libhm_geti_lib.so
  echo "--- symbol parity ---"
  diff <(nm -D --defined-only "$CSO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$RSO" | awk '{print $3}' | sort) \
    && echo "  OK: symbol diff empty ($(nm -D --defined-only "$CSO" | wc -l) symbols)"
  echo "--- differential tests ---"
  cargo test --offline --release $ARGS -- --test-threads=1
done
echo
echo "ALL CONFIGURATIONS PASSED"
