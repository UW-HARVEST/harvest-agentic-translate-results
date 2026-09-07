#!/bin/bash
# Phase D driver: build both libraries, diff their exported symbols, and run the
# fast differential suite under EVERY feature combination.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
CSO="$W/c_src/build/liblong.so"
RSO="$W/translation/target/release/liblong.so"
fail=0

echo "=== build C ==="
mkdir -p "$W/c_src/build"
( cd "$W/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }

echo "=== build Rust ==="
( cd "$W/translation" && cargo build --offline --release >/dev/null 2>&1 ) || { echo "RUST BUILD FAILED"; exit 1; }

echo "=== symbol parity (nm -D) ==="
defs() { nm -D "$1" | awk '$2 ~ /^[ABDGRSTVWi]$/ {print $3}' | grep -v -E '^(_ITM_|__gmon_start__|__cxa_finalize|_init|_fini|_edata|_end|__bss_start)' | sort -u; }
undefs() { nm -D "$1" | awk '$1=="U" {print $2}' | sort -u; }
diff <(defs "$CSO") <(defs "$RSO") && echo "OK: defined-symbol diff is EMPTY" || { echo "FAIL: symbol diff non-empty"; fail=1; }
echo "-- C imports:";    undefs "$CSO"
echo "-- Rust imports:"; undefs "$RSO"
missing=$(comm -23 <(undefs "$CSO") <(undefs "$RSO"))
[ -z "$missing" ] && echo "OK: every libc import of the C .so is also imported by Rust" || { echo "FAIL: Rust does not import: $missing"; fail=1; }

echo "=== feature combinations ==="
combos=$(cd "$W/translation" && python3 - <<'EOF'
import re,sys
try:
    import tomllib
    d=tomllib.load(open('Cargo.toml','rb'))
except Exception:
    d={}
f=d.get('features',{})
print('\n'.join(k for k in f if k!='default') if f else '')
EOF
)
if [ -z "$combos" ]; then
  echo "no [features] declared -> single configuration; running default + --no-default-features"
  sets=("" "--no-default-features")
else
  sets=("" "--no-default-features")
  for c in $combos; do sets+=("--no-default-features --features $c"); done
fi

for s in "${sets[@]}"; do
  echo "--- cargo test --release $s --test differential"
  ( cd "$W/translation" && timeout 590 cargo test --offline --release $s --test differential 2>&1 | tail -4 ) || fail=1
done

echo
echo "=== cheap long_exec cross-checks (uses cached expensive results) ==="
( cd "$W/translation" && cargo test --offline --release --test long_exec_full -- --nocapture 2>&1 | tail -12 ) || fail=1

echo
[ $fail -eq 0 ] && echo "ALL GREEN" || echo "FAILURES PRESENT"
exit $fail
