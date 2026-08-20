#!/bin/bash
# Phase D driver: for every (OP, REPEAT) configuration
#   1. build the C .so + C executable and the Rust .so + Rust executable
#   2. diff `nm -D` between the two shared libraries (symbol parity)
#   3. run the whole differential test suite (Phases B and C)
#
# usage: ./run_all.sh [feature-spelling]      # "digit" (default) or "alias"
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
SPELLING=${1:-digit}
cd "$ROOT"

pass=0
fail=0
failed_configs=()

for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    if [ "$SPELLING" = alias ]; then
      feat="$op,repeat_$r"
    else
      feat="$op,$r"
    fi

    printf '=== %-12s ' "$feat"

    if ! ./build_all.sh "$op" "$r" >/dev/null 2>"$ROOT/artifacts/build-$op-$r.err"; then
      echo "BUILD FAILED"; cat "$ROOT/artifacts/build-$op-$r.err"; fail=$((fail+1)); failed_configs+=("$feat/build"); continue
    fi
    # the alias spelling needs its own Rust .so
    if [ "$SPELLING" = alias ]; then
      cargo build --quiet --no-default-features --features "$feat" || { echo "BUILD FAILED"; fail=$((fail+1)); continue; }
      cp target/debug/libdriver.so "artifacts/${op}_${r}/librdriver.so"
      cp target/debug/driver       "artifacts/${op}_${r}/rdriver"
    fi

    # --- symbol parity ---
    csyms=$(nm -D --defined-only "artifacts/${op}_${r}/libcdriver.so" | awk '{print $NF}' | sort)
    rsyms=$(nm -D --defined-only "artifacts/${op}_${r}/librdriver.so" | awk '{print $NF}' | sort)
    missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
    if [ -n "$missing" ]; then
      echo "SYMBOL MISMATCH: missing from Rust: $missing"
      fail=$((fail+1)); failed_configs+=("$feat/symbols"); continue
    fi

    # --- differential tests ---
    log="$ROOT/artifacts/test-$op-$r.log"
    if HARVEST_OP="$op" HARVEST_REPEAT="$r" \
       timeout 600 cargo test --no-default-features --features "$feat" \
       -- --test-threads=1 >"$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      if [ "$n" -lt 36 ]; then
        echo "TOO FEW TESTS RAN ($n) -- refusing to count this as a pass"
        fail=$((fail+1)); failed_configs+=("$feat/testcount"); continue
      fi
      echo "OK  (symbols match, $n tests passed)"
      pass=$((pass+1))
    else
      echo "TEST FAILED (see $log)"
      grep -E '^(test .* FAILED|failures:|---- )' "$log" | head -20
      fail=$((fail+1)); failed_configs+=("$feat/tests")
    fi
  done
done

echo
echo "configurations passed: $pass   failed: $fail"
if [ "$fail" -ne 0 ]; then
  printf 'failed: %s\n' "${failed_configs[@]}"
  exit 1
fi
