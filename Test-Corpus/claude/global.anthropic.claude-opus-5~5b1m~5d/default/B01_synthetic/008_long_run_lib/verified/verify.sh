#!/usr/bin/env bash
# One-shot verification runner for the C -> Rust translation of `liblong`.
#
#   ./verify.sh          fast suite + extended soak (about 3 minutes)
#   ./verify.sh --full   additionally the full `long_exec` end-to-end runs
#                        (about 8 more minutes, 5.24e10 step() evaluations each)
set -uo pipefail
cd "$(dirname "$0")"
FAIL=0
step() { echo; echo "######## $* ########"; }

step "1/6  build the C shared library"
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l ../c_src/build/liblong.so

step "2/6  cargo check"
cargo check --offline --all-targets 2>&1 | tail -5 || FAIL=1

step "3/6  build both Rust profiles"
# debug  : overflow-checks = true  -> any non-wrapping arithmetic aborts
# release: opt-level 3 + lto       -> the vectorised code path
cargo build --offline           2>&1 | tail -2 || FAIL=1
cargo build --offline --release 2>&1 | tail -2 || FAIL=1

step "4/6  symbol parity (nm -D on both .so files)"
diff <(nm -D --defined-only ../c_src/build/liblong.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/liblong.so | awk '{print $NF}' | sort \
        | grep -vE '^(_init|_fini|_edata|_end|__bss_start|_IO_stdin_used|__rust|rust_|_ZN)') \
  && echo "symbol diff: EMPTY (C \\ Rust == {})" \
  || { echo "SYMBOL PARITY FAILURE (lines starting with '<' are missing from Rust)"; FAIL=1; }

step "5/6  differential test suite (all feature combinations)"
# Cargo.toml declares no [features], so the default, --no-default-features and
# --all-features combinations are the complete matrix. Enumerated anyway so a
# future [features] table is picked up here.
COMBOS=("" "--no-default-features" "--all-features")
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo "---- cargo test $label ----"
  log="target/test_${label//[^a-zA-Z0-9]/_}.log"
  timeout 600 cargo test --offline $combo >"$log" 2>&1
  rc=$?
  grep -E '^test result|^error|FAILED|panicked' "$log" || true
  [ $rc -ne 0 ] && { echo "FAILED for '$label' (rc=$rc, see $log)"; FAIL=1; }
done

step "6/6  extended randomised soak (31.4M distinct starting values)"
timeout 600 cargo test --offline --test valid_paths -- --ignored --nocapture soak 2>&1 \
  | grep -E '^soak|^test result' || FAIL=1

if [ "${1:-}" == "--full" ]; then
  step "EXTRA  full long_exec end-to-end differential (seeds 1 2 12345)"
  ./run_full_long_exec.sh 1 2 12345 || FAIL=1
else
  echo
  echo "NOTE: the full long_exec end-to-end differential was skipped."
  echo "      Run './verify.sh --full' or './run_full_long_exec.sh' for it."
fi

echo
if [ $FAIL -eq 0 ]; then echo "=== VERIFICATION PASSED ==="; else echo "=== VERIFICATION FAILED ==="; fi
exit $FAIL
