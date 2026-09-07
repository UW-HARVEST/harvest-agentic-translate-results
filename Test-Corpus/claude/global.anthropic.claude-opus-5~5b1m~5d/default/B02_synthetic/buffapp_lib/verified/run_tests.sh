#!/usr/bin/env bash
# Build both shared libraries and run the full differential suite.
#
# `--test-threads=1` is required: the harness forks children to capture stdout
# and fatal signals, which is only safe from a single-threaded parent.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$ROOT/c_src/build" >/dev/null
ls "$ROOT"/c_src/build/*.so

echo "== building Rust cdylib =="
cargo build --release --offline

echo "== symbol parity =="
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
R_SO="$ROOT/translation/target/release/libbuffapp_lib.so"
diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) \
  && echo "symbol diff: EMPTY (OK)"

echo "== feature combinations =="
# Enumerate every feature declared in Cargo.toml and every subset of them.
FEATURES=$(python3 - <<'PY'
import re,sys
txt=open("Cargo.toml").read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M|re.S)
feats=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
print(' '.join(feats))
PY
)
echo "declared features: [${FEATURES:-none}]"

COMBOS=()
COMBOS+=("")                      # default
COMBOS+=("--no-default-features") # nothing enabled
if [ -n "$FEATURES" ]; then
  read -r -a FARR <<< "$FEATURES"
  N=${#FARR[@]}
  for ((mask=0; mask<(1<<N); mask++)); do
    sel=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then sel="$sel,${FARR[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
    COMBOS+=("--features ${sel#,}")
  done
fi

for FEAT_ARGS in "${COMBOS[@]}"; do
  echo "-- combo: [${FEAT_ARGS:-<default>}] --"
  cargo build --release --offline $FEAT_ARGS
  cargo test --release --offline $FEAT_ARGS -- --test-threads=1 -q
done

echo "== dev profile (overflow-checks enabled) =="
cargo build --offline
BUFFAPP_RUST_SO="$ROOT/translation/target/debug/libbuffapp_lib.so" \
  cargo test --release --offline -- --test-threads=1 -q

echo "ALL PHASES PASSED"
