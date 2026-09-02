#!/usr/bin/env bash
# Negative control for the differential harness.
#
# Builds deliberately-MUTATED copies of the Rust crate in a scratch directory
# (the real translation/src is never touched), points the test suite at each
# mutant via RUST_SO, and asserts the suite FAILS. A mutation that the suite
# does not catch is a blind spot in the tests.
set -u

ROOT="$(cd "$(dirname "$0")" && pwd)"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

pass=0
fail=0

# apply_mutation <name> <sed-expression> <target-file>
run_mutant() {
  local name="$1" expr="$2" file="$3"
  local dir="$SCRATCH/$name"
  rm -rf "$dir"
  mkdir -p "$dir"
  cp -r "$ROOT/src" "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$dir/" 2>/dev/null
  cp -r "$ROOT/tests" "$dir/"

  if ! sed -i "$expr" "$dir/$file"; then
    echo "MUTANT $name: sed failed"; fail=$((fail+1)); return
  fi
  if ! cmp -s "$dir/$file" "$ROOT/$file"; then :; else
    echo "MUTANT $name: sed matched nothing (mutation is a no-op)"; fail=$((fail+1)); return
  fi

  if ! (cd "$dir" && timeout 300 cargo build --release >"$dir/build.log" 2>&1); then
    echo "MUTANT $name: mutant did not compile (see $dir/build.log)"; fail=$((fail+1)); return
  fi

  local out
  out=$(cd "$ROOT" && C_SO="$ROOT/../c_src/build/libdriver.so" \
        RUST_SO="$dir/target/release/libdriver.so" \
        timeout 600 cargo test --release -- --test-threads=1 2>&1)
  if echo "$out" | grep -qE '^test result: FAILED|panicked'; then
    local which
    which=$(echo "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' | head -3 | tr '\n' ' ')
    echo "CAUGHT   $name  (by: ${which:-assertion})"
    pass=$((pass+1))
  else
    echo "MISSED   $name  <-- BLIND SPOT: tests still pass against the mutant"
    fail=$((fail+1))
  fi
}

echo "=== mutation / negative-control run ==="

run_mutant wrong_multiplier   's/x\.wrapping_mul(2)/x.wrapping_mul(3)/'                       src/goto.rs
run_mutant off_by_one_sign    's/if x < 0 {/if x <= 0 {/'                                     src/goto.rs
run_mutant stdout_text        's/Processing: %d\\n/Processing:  %d\\n/'                        src/goto.rs
run_mutant stderr_text        's/Error: negative input/Error: negative Input/'                src/goto.rs
run_mutant cleanup_text       's/Error: opening or processing file/Error: opening file/'       src/goto.rs
run_mutant buffer_len_99      's/const BUFFER_LEN: usize = 100;/const BUFFER_LEN: usize = 99;/' src/goto.rs
run_mutant buffer_len_101     's/const BUFFER_LEN: usize = 100;/const BUFFER_LEN: usize = 101;/' src/goto.rs
run_mutant skip_ferror        's/if unsafe { ferror(fp) } == 0 {/if true {/'                   src/goto.rs
run_mutant driver_wrong_code  's/return -2;/return -3;/'                                      src/goto.rs
run_mutant driver_sentinel    's/if res == -1 {/if res == -2 {/'                              src/goto.rs
run_mutant goto_output_text   's/Goto output: %d\\n/Goto Output: %d\\n/'                       src/goto.rs

# Symbol-parity mutant: keep the implementation but export it under a different
# name, which must be caught by the Phase D symbol diff.
run_mutant_perl() {
  local name="$1" expr="$2" file="$3"
  local dir="$SCRATCH/$name"
  rm -rf "$dir"; mkdir -p "$dir"
  cp -r "$ROOT/src" "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$dir/" 2>/dev/null
  cp -r "$ROOT/tests" "$dir/"
  perl -0777 -pi -e "$expr" "$dir/$file"
  if cmp -s "$dir/$file" "$ROOT/$file"; then
    echo "MUTANT $name: perl matched nothing (no-op)"; fail=$((fail+1)); return
  fi
  if ! (cd "$dir" && timeout 300 cargo build --release >"$dir/build.log" 2>&1); then
    echo "MUTANT $name: mutant did not compile (see $dir/build.log)"; fail=$((fail+1)); return
  fi
  local out
  out=$(cd "$ROOT" && C_SO="$ROOT/../c_src/build/libdriver.so" \
        RUST_SO="$dir/target/release/libdriver.so" \
        timeout 600 cargo test --release -- --test-threads=1 2>&1)
  if echo "$out" | grep -qE '^test result: FAILED|panicked'; then
    echo "CAUGHT   $name"
    pass=$((pass+1))
  else
    echo "MISSED   $name  <-- BLIND SPOT"
    fail=$((fail+1))
  fi
}

run_mutant_perl renamed_export \
  's/#\[unsafe\(no_mangle\)\]\npub unsafe extern "C" fn open_with_cleanup/#[unsafe(export_name = "open_with_cleanup_typo")]\npub unsafe extern "C" fn open_with_cleanup/' \
  src/goto.rs

echo
echo "=== mutants caught: $pass   missed/errored: $fail ==="
[ "$fail" -eq 0 ]
