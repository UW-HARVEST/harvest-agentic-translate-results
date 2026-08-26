#!/usr/bin/env bash
# Full verification run: build C + Rust, check symbol parity, then run the
# differential test suites for EVERY feature combination x EVERY Rust artifact.
set -uo pipefail
cd "$(dirname "$0")"
rc=0

echo "###### 1. build the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libdriver.so

echo
echo "###### 2. feature combinations (from Cargo.toml [features])"
# No optional features exist, so the powerset is: {} (== default).
COMBOS=("")            # "" means --no-default-features with no features
echo "combinations: 1  -> [<none>]  (+ the default feature set, which is identical)"

echo
echo "###### 3. cargo check for every combination"
for f in "${COMBOS[@]}"; do
  echo "--- cargo check --no-default-features --features '$f'"
  timeout 300 cargo check --offline --no-default-features --features "$f" || rc=1
done
echo "--- cargo check (default features)"
timeout 300 cargo check --offline || rc=1

echo
echo "###### 4. build both Rust artifacts for every combination"
for f in "${COMBOS[@]}"; do
  timeout 300 cargo build --offline --no-default-features --features "$f" || rc=1
  timeout 300 cargo build --offline --release --no-default-features --features "$f" || rc=1
done

echo
echo "###### 5. symbol parity (SYMBOLS.md gate)"
./check_symbols.sh || rc=1

echo
echo "###### 6. differential tests: every combination x {debug,release} Rust .so"
for f in "${COMBOS[@]}"; do
  for so in target/debug/libdriver.so target/release/libdriver.so; do
    echo "=============================================================="
    echo "features='${f:-<none>}'   RUST_DRIVER_SO=$so"
    echo "=============================================================="
    RUST_DRIVER_SO="$PWD/$so" timeout 600 cargo test --offline \
        --no-default-features --features "$f" -- --test-threads=4 || rc=1
  done
done

echo
if [ $rc -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES (rc=$rc)"; fi
exit $rc
