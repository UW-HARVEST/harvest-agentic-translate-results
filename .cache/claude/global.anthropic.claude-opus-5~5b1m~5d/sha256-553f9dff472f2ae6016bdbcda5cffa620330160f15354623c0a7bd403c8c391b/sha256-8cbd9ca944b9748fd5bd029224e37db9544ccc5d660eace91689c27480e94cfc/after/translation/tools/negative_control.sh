#!/usr/bin/env bash
# Negative control for the differential test suite.
#
# Builds deliberately WRONG copies of src/lib.rs as standalone cdylibs (the real
# src/lib.rs is never touched), points the test harness at each one via
# HARVEST_RUST_SO, and asserts the suite FAILS. If a mutant survives, the tests
# are not actually checking that behaviour.
set -uo pipefail

CRATE="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${TMPDIR:-/tmp}/harvest_mutants"
rm -rf "$WORK"; mkdir -p "$WORK"

# name|sed-expression
MUTANTS=(
  'flips_off_by_one|s|let flips: c_int = h.wrapping_div(2);|let flips: c_int = h.wrapping_div(2) + 1;|'
  'flips_round_up|s|let flips: c_int = h.wrapping_div(2);|let flips: c_int = (h + 1).wrapping_div(2);|'
  'wrong_source_row|s|h.wrapping_sub(i).wrapping_sub(1)|h.wrapping_sub(i)|'
  # `*b = t` then restoring b's *own* previous alpha: rgb is swapped, a is not.
  'skip_alpha_channel|s|\*b = t;|let b_a = (*b).a; *b = t; (*b).a = b_a;|'
  'inner_loop_off_by_one|s|while j < w {|while j < w - 1 {|'
  'no_op_stub|s|let mut i: c_int = 0;|let mut i: c_int = flips;|'
  'signed_h_ignored|s|while i < flips {|while i < flips.abs() {|'
  'dest_row_off_by_one|s|let off_a = w.wrapping_mul(i) as isize;|let off_a = w.wrapping_mul(i.wrapping_add(1)) as isize;|'
  'columns_reversed|s|a = a.offset(1);|a = a.offset(-1);|'
  'stride_uses_height|s|let off_a = w.wrapping_mul(i) as isize;|let off_a = h.wrapping_mul(i) as isize;|'
)

# NOTE on an intentionally omitted mutant: replacing the row-offset arithmetic
#   w.wrapping_mul(h - i - 1) as isize   ->   (w as i64 * (h as i64 - i - 1))
# is a *semantically equivalent* mutant for every observable input, because the
# two differ only when the `int` product overflows, and reaching an overflowing
# product with a dereference requires w >= 1 together with h/2 * w > 2^31 pixel
# copies (> 8 GiB of writes). See ERRORS.md row E10. Instead of an untestable
# mutant, the 32-bit-multiply-then-sign-extend semantics are confirmed
# statically: both .so files use a 32-bit `imul` followed by a sign extension
# (`cltq` in C, `movslq` in Rust) before scaling by sizeof(cp_pixel_t).

fail=0
for entry in "${MUTANTS[@]}"; do
  name="${entry%%|*}"; expr="${entry#*|}"
  d="$WORK/$name"; mkdir -p "$d/src"
  cat > "$d/Cargo.toml" <<EOF
[package]
name = "mutant_$name"
version = "0.1.0"
edition = "2024"
[lib]
name = "flip_horizontal_lib"
path = "src/lib.rs"
crate-type = ["cdylib"]
[workspace]
EOF
  sed "$expr" "$CRATE/src/lib.rs" > "$d/src/lib.rs"

  if cmp -s "$CRATE/src/lib.rs" "$d/src/lib.rs"; then
    echo "MUTANT $name: SED DID NOT APPLY (source unchanged) -- fix the script"
    fail=1; continue
  fi

  if ! (cd "$d" && cargo build --release --offline >"$d/build.log" 2>&1); then
    echo "MUTANT $name: did not compile (skipped)"; tail -3 "$d/build.log"
    fail=1; continue
  fi

  so="$d/target/release/libflip_horizontal_lib.so"
  if HARVEST_RUST_SO="$so" cargo test --offline --manifest-path "$CRATE/Cargo.toml" \
        -- --test-threads=4 >"$d/test.log" 2>&1; then
    echo "MUTANT $name: SURVIVED -- the test suite passed a known-wrong library!"
    fail=1
  else
    n=$(grep -cE '^test .* (FAILED|panicked)' "$d/test.log" 2>/dev/null)
    n=${n:-0}
    echo "MUTANT $name: killed (${n} reported test failures; suite exited non-zero)"
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "NEGATIVE CONTROL: FAILED"; exit 1
fi
echo "NEGATIVE CONTROL: all mutants killed"
