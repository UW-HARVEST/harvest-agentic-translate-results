#!/usr/bin/env bash
# Phases B/C/D across every configuration.
#
# For each (OP, REPEAT) pair:
#   1. build the C sources as a .so with -DOP/-DREPEAT
#   2. build the Rust cdylib with the matching features
#   3. run the whole differential test suite against that pair of .so files
#
# usage: run_all.sh [OP:REPEAT ...]      (default: all 24 combinations)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/translation"
mkdir -p "$ROOT/cbuild/rs" "$ROOT/cbuild/logs"

if [ "$#" -gt 0 ]; then
  CONFIGS=("$@")
else
  CONFIGS=()
  for op in add sub mul; do for rep in 0 1 2 3 4 5 6 7; do CONFIGS+=("$op:$rep"); done; done
fi

pass=0; fail=0; failed=()
for cfg in "${CONFIGS[@]}"; do
  op="${cfg%%:*}"; rep="${cfg##*:}"
  feats="$op,$rep"
  log="$ROOT/cbuild/logs/test_${op}_${rep}.log"

  c_so=$("$ROOT/scripts/build_c_so.sh" "$op" "$rep") || { echo "C BUILD FAIL $cfg"; fail=$((fail+1)); failed+=("$cfg(cbuild)"); continue; }
  c_exe=$("$ROOT/scripts/build_c_exe.sh" "$op" "$rep") || { echo "C EXE BUILD FAIL $cfg"; fail=$((fail+1)); failed+=("$cfg(cexe)"); continue; }

  if ! timeout 300 cargo build --release --offline --no-default-features --features "$feats" >"$log" 2>&1; then
    echo "RUST BUILD FAIL $cfg (see $log)"; tail -n 15 "$log"; fail=$((fail+1)); failed+=("$cfg(rsbuild)"); continue
  fi
  rs_so="$ROOT/cbuild/rs/libmacrodepth_${op}_${rep}.so"
  cp "$ROOT/translation/target/release/libmacrodepth_add_5.so" "$rs_so"

  if MD_OP="$op" MD_REPEAT="$rep" MD_C_SO="$c_so" MD_RUST_SO="$rs_so" MD_C_EXE="$c_exe" \
     timeout 600 cargo test --release --offline --no-default-features --features "$feats" \
       -- --test-threads=1 >>"$log" 2>&1; then
    n=$(grep -c '^test .* ok$' "$log")
    echo "PASS $cfg  ($n tests)"
    pass=$((pass+1))
  else
    echo "FAIL $cfg (see $log)"
    grep -E "^(test .*FAILED|failures:|thread .* panicked)" -A6 "$log" | head -n 40
    fail=$((fail+1)); failed+=("$cfg")
  fi
done

echo "=================================================="
echo "configurations passed: $pass   failed: $fail"
[ "$fail" -eq 0 ] || { printf 'failing configs: %s\n' "${failed[*]}"; exit 1; }
