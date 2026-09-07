#!/usr/bin/env bash
# Differential test driver.
#
#   scripts/run_all.sh                 # all 24 OP x REPEAT configurations
#   scripts/run_all.sh add 5           # a single configuration
#   scripts/run_all.sh add 5 -- <args> # extra args forwarded to `cargo test`
#
# For each configuration it
#   1. builds the C shared object + C `driver` executable with -DOP/-DREPEAT,
#   2. builds the Rust cdylib + Rust `driver` with the matching cargo features,
#   3. runs `cargo test` with those features, pointing the tests at both
#      artifacts through DIFF_C_SO / DIFF_RUST_SO / DIFF_C_BIN / DIFF_RUST_BIN.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(dirname "$HERE")"
ROOT="$(dirname "$CRATE")"
CSRC="$ROOT/c_src/src"
OUT="$ROOT/cbuild"
RUSTOUT="$ROOT/rbuild"
mkdir -p "$OUT" "$RUSTOUT"

OPS=(add sub mul)
REPS=(0 1 2 3 4 5 6 7)
EXTRA=()
if [[ $# -ge 2 ]]; then
  OPS=("$1"); REPS=("$2"); shift 2
  [[ "${1:-}" == "--" ]] && shift && EXTRA=("$@")
fi

pass=0; fail=0; failed_cfgs=()

for op in "${OPS[@]}"; do
  for rep in "${REPS[@]}"; do
    cfg="${op}_${rep}"
    echo "==================== OP=$op REPEAT=$rep ===================="

    # --- 1. C artifacts -------------------------------------------------
    gcc -O2 -shared -fPIC -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/libc_$cfg.so" "$CSRC/mdcore.c" || { echo "C .so build FAILED"; fail=$((fail+1)); failed_cfgs+=("$cfg:cbuild"); continue; }
    gcc -O2 -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/cdriver_$cfg" "$CSRC/mdcore.c" "$CSRC/mdmain.c" || { echo "C bin build FAILED"; fail=$((fail+1)); failed_cfgs+=("$cfg:cbin"); continue; }

    # --- 2. Rust artifacts ---------------------------------------------
    ( cd "$CRATE" && cargo build --offline --release --no-default-features --features "$op,$rep" ) \
      > "$OUT/rustbuild_$cfg.log" 2>&1 \
      || { echo "Rust build FAILED (see $OUT/rustbuild_$cfg.log)"; tail -20 "$OUT/rustbuild_$cfg.log"; fail=$((fail+1)); failed_cfgs+=("$cfg:rustbuild"); continue; }
    cp "$CRATE/target/release/libdriver.so" "$RUSTOUT/librust_$cfg.so"
    cp "$CRATE/target/release/driver"       "$RUSTOUT/rustdriver_$cfg"

    # --- 3. differential tests -----------------------------------------
    ( cd "$CRATE" && \
      DIFF_C_SO="$OUT/libc_$cfg.so" \
      DIFF_RUST_SO="$RUSTOUT/librust_$cfg.so" \
      DIFF_C_BIN="$OUT/cdriver_$cfg" \
      DIFF_RUST_BIN="$RUSTOUT/rustdriver_$cfg" \
      timeout 600 cargo test --offline --release --no-default-features --features "$op,$rep" \
        "${EXTRA[@]}" 2>&1 ) | tee "$OUT/test_$cfg.log" | grep -E "^(test result|error|warning: unused|---- |thread )|panicked|FAILED"
    st=${PIPESTATUS[0]}
    if grep -qE "FAILED|error\[|^error" "$OUT/test_$cfg.log"; then st=1; fi
    if [[ $st -eq 0 ]]; then
      echo "RESULT $cfg: PASS"; pass=$((pass+1))
    else
      echo "RESULT $cfg: FAIL (log: $OUT/test_$cfg.log)"; fail=$((fail+1)); failed_cfgs+=("$cfg")
    fi
  done
done

echo
echo "############ SUMMARY: $pass passed, $fail failed ############"
if [[ $fail -gt 0 ]]; then
  printf 'failed configurations: %s\n' "${failed_cfgs[*]}"
  exit 1
fi
