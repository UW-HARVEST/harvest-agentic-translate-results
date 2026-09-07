#!/usr/bin/env bash
# Full differential verification: builds the C .so and the Rust .so, then runs
# every test binary under EVERY cargo feature combination declared in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")"

ROOT=$(cd .. && pwd)

echo "=== building the C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "C   .so: $C_SO"

# ---------------------------------------------------------------------------
# enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name not in ('default',):
                feats.append(name)
print(' '.join(feats))
PY
)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "=== no [features] table: a single (default) configuration ==="
  COMBOS+=("__default__")
else
  # powerset of the declared features, plus the plain default build
  COMBOS+=("__default__")
  read -ra F <<<"$FEATURES"
  n=${#F[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then combo+="${F[$i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    BUILD_ARGS=(--release)
    TEST_ARGS=(--release)
    label="default"
  else
    BUILD_ARGS=(--release --no-default-features --features "$combo")
    TEST_ARGS=(--release --no-default-features --features "$combo")
    label="--no-default-features --features $combo"
  fi
  echo
  echo "=================================================================="
  echo "=== configuration: $label"
  echo "=================================================================="
  cargo build --offline "${BUILD_ARGS[@]}"
  ls -l target/release/libarr_ins_lib.so

  echo "--- symbol diff (C -> Rust) ---"
  diff <(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort) \
       <(nm -D --defined-only target/release/libarr_ins_lib.so | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort) \
       > /dev/null && echo "identical exported symbol sets" || {
         echo "SYMBOL DIFFERENCE:"; FAIL=1; }

  cargo test --offline "${TEST_ARGS[@]}" -- --test-threads=1 || FAIL=1
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; exit 1; fi
