#!/usr/bin/env bash
# Negative control for the differential suite.
#
# A green test run only means something if the suite can actually FAIL. Each
# mutant below injects a plausible, subtle translation bug into src/lib.rs,
# rebuilds the Rust .so, and checks that the differential suite catches it --
# and, crucially, that the ERRORS.md/CONFIGS.md row that is *supposed* to guard
# that behaviour is among the rows that fail.
#
# src/lib.rs is restored from .verify/lib.rs.pristine after every mutant.
set -u
cd "$(dirname "$0")"

PRISTINE=.verify/lib.rs.pristine
[[ -f $PRISTINE ]] || { echo "missing $PRISTINE"; exit 1; }

restore() { cp "$PRISTINE" src/lib.rs; }
trap restore EXIT

pass=0; fail=0

# run_mutant <name> <perl-expr> <row-that-must-fail>
run_mutant() {
  local name="$1" expr="$2" want="$3"
  restore
  perl -0pi -e "$expr" src/lib.rs
  if ! cmp -s "$PRISTINE" src/lib.rs; then :; else
    echo "MUTANT '$name': perl expr did not change the file -- bad mutant"
    fail=$((fail+1)); return
  fi
  cargo build --release --offline >/dev/null 2>&1 || {
    echo "MUTANT '$name': did not compile -- bad mutant"; fail=$((fail+1)); return; }

  local out
  out=$(cargo test --release --offline --test differential 2>&1)
  if grep -q 'result: ok' <<<"$out"; then
    echo "NOT DETECTED  '$name'  <-- suite is blind to this bug!"
    fail=$((fail+1))
  elif grep -qE "^  (${want})[^ ]* +\.\.\. FAILED" <<<"$out"; then
    echo "detected      '$name'  (guard row $want failed, as intended)"
    pass=$((pass+1))
  else
    echo "detected      '$name'  BUT guard row $want passed -- that row is too weak"
    grep -E '\.\.\. FAILED' <<<"$out" | head -3 | sed 's/^/                /'
    fail=$((fail+1))
  fi
}

echo "=== mutation / negative-control run ==="

# 1. Wrong fixed-point precision.
run_mutant "precision %.4f -> %.5f" \
  's/const FMT: &\[u8\] = b"\%llx \%a \%\.4f/const FMT: &[u8] = b"%llx %a %.5f/' \
  "C1"

# 2. NaNs canonicalised in transit -- the classic sNaN/payload-quieting bug.
#    NB: the obvious `let f = f * 1.0;` mutant is USELESS here: LLVM folds
#    `fmul x, 1.0` to `x`, so the .so comes out byte-identical (confirmed by
#    objdump: no mulsd emitted). An explicit rewrite is needed instead.
run_mutant "NaN payload canonicalised" \
  's/(let u = raw_double_t::from_f64\(f\);)/let f = if f.is_nan() { f64::NAN } else { f };\n    $1/' \
  "E8|E9"

# 3. Sign of negative zero dropped -- invisible unless -0.0 is tested.
run_mutant "negative zero sign dropped" \
  's/(let u = raw_double_t::from_f64\(f\);)/let f = if f == 0.0 { 0.0 } else { f };\n    $1/' \
  "E2"

# 4. Formatting reimplemented with Rust's own formatter instead of libc printf:
#    breaks %a, and also breaks stdout interleaving (separate buffer).
run_mutant "Rust println! instead of libc printf" \
  's/        printf\(FMT\.as_ptr\(\) as \*const c_char, bits, f, f\);/        println!("{:x} {:?} {:.4}", bits, f, f);/' \
  "C1"

# 5. Bit pattern byte-swapped -- %llx field wrong, other two fields fine.
# NB: guard cannot be E1 -- swap_bytes(0) == 0, so the all-zero row is blind
# to this one by construction.
run_mutant "bit pattern byte-swapped" \
  's/(let bits = u\.bits\(\);)/$1\n    let bits = bits.swap_bytes();/' \
  "C1"

# 6. %llx fed a 32-bit-truncated value: only wrong for large bit patterns.
run_mutant "bits truncated to u32" \
  's/(let bits = u\.bits\(\);)/$1\n    let bits = bits as u32 as u64;/' \
  "C1"

# 7. Subnormals flushed to zero -- only the subnormal rows can see this.
run_mutant "subnormals flushed to zero" \
  's/(let u = raw_double_t::from_f64\(f\);)/let f = if f != 0.0 \&\& f.abs() < f64::MIN_POSITIVE { 0.0 * f.signum() } else { f };\n    $1/' \
  "E10"

# 8. Trailing newline dropped.
run_mutant "trailing newline dropped" \
  's/b"\%llx \%a \%\.4f\\n\\0"/b"%llx %a %.4f\\0"/' \
  "C1"

restore
cargo build --release --offline >/dev/null 2>&1

echo
echo "mutants detected by the right guard row: $pass ; problems: $fail"
[[ $fail -eq 0 ]] || exit 1
