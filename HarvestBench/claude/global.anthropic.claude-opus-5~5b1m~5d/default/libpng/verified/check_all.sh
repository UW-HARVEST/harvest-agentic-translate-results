#!/bin/bash
# Full verification driver: rebuild both shared objects, diff their exported
# symbols, then run every differential test under every Cargo feature
# combination (this crate declares no [features], so the default is the only
# combination -- the loop is generated from Cargo.toml so it stays correct if
# features are ever added).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/translation"

echo "=== 1. build the reference C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || exit 1
ls -l "$ROOT/c_src/build/libpng.so"

echo
echo "=== 2. build the Rust cdylib ==="
cargo build --offline --release || exit 1
ls -l target/release/liblibpng.so

echo
echo "=== 3. exported-symbol parity (SYMBOLS.md) ==="
W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT
nm -D --defined-only "$ROOT/c_src/build/libpng.so"      | awk '$2!="U"{print $3}' | sort -u > "$W/c.txt"
nm -D --defined-only target/release/liblibpng.so        | awk '$2!="U"{print $3}' | sort -u > "$W/rs.txt"
echo "C exports:    $(wc -l < "$W/c.txt")"
echo "Rust exports: $(wc -l < "$W/rs.txt")"
echo "--- in C but MISSING from Rust (must be empty) ---"
comm -23 "$W/c.txt" "$W/rs.txt" | tee "$W/missing.txt"
echo "--- in Rust but not in C (must be empty) ---"
comm -13 "$W/c.txt" "$W/rs.txt" | tee "$W/extra.txt"
MISSING=$(wc -l < "$W/missing.txt"); EXTRA_SYMS=$(wc -l < "$W/extra.txt")
echo "--- undefined non-libc symbols in the Rust .so (must be empty) ---"
nm -D --undefined-only target/release/liblibpng.so | awk '{print $2}' | sed 's/@.*//' | sort -u \
  | grep -E '^png_' || echo "(none)"

echo
echo "=== 4. feature combinations ==="
FEATURES=$(python3 - <<'PY'
import re,sys
txt=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*(.*?)(^\[|\Z)', txt, re.S|re.M)
names=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            n=line.split('=')[0].strip()
            if n!='default': names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the default build is the only combination."
  COMBOS=("default")
else
  COMBOS=("default")
  for f in $FEATURES; do COMBOS+=("$f"); done
  COMBOS+=("$(echo $FEATURES | tr ' ' ',')")
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "--- testing combination: $combo ---"
  if [ "$combo" = "default" ]; then
    EXTRA=""
  else
    EXTRA="--no-default-features --features $combo"
    cargo build --offline --release $EXTRA >/dev/null 2>&1 || FAIL=1
  fi
  # deterministic order first, then the default parallel order (which exercises
  # a different heap layout and catches buffer overruns the serial run hides)
  for mode in "-- --test-threads=1" ""; do
    LOG="$W/test$$.log"
    # shellcheck disable=SC2086
    cargo test --offline --release $EXTRA $mode > "$LOG" 2>&1 || FAIL=1
    grep -E '^(running|test result)|Running |FAILED|SIGSEGV|SIGABRT|^error' "$LOG" || true
    grep -E 'test result: FAILED|SIGSEGV|SIGABRT|malloc\(\)' "$LOG" && FAIL=1
    rm -f "$LOG"
  done
done

echo
echo "=== SUMMARY ==="
echo "symbols missing from Rust .so : $MISSING"
echo "symbols extra in Rust .so     : $EXTRA_SYMS"
echo "test failures                 : $FAIL"
[ "$MISSING" -eq 0 ] && [ "$EXTRA_SYMS" -eq 0 ] && [ "$FAIL" -eq 0 ] && echo "ALL GREEN" || echo "NOT GREEN"
