#!/usr/bin/env bash
# Negative control: inject known bugs into the Rust translation one at a time
# and confirm the differential test suite FAILS on each. A suite that passes a
# deliberately broken translation proves nothing.
set -u
cd "$(dirname "$0")/.." || exit 1

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap 'restore; rm -f "$BAK"' EXIT

# name | sed expression
# NOTE: mutants marked EQUIV below were tried and are semantically identical to
# the original, so the suite correctly does NOT flag them:
#   s/if value < lower_threshold/if value <= lower_threshold/   (64 -> 64)
#   s/value > upper_threshold/value >= upper_threshold/         (511 -> 511)
#   s/... as u8 == target/... == needle as c_char/              (same low 8 bits)
# The mutants listed here are all genuinely behaviour-changing.
MUTANTS=(
  "octal_signedness|s/v as u32/v as i64/"
  "octal_prefix|s/Octal: 0{}/Octal: {}/"
  "validate_lower_const|s/let lower_threshold: c_int = 0o100;/let lower_threshold: c_int = 0o101;/"
  "validate_upper_const|s/let upper_threshold: c_int = 0o777;/let upper_threshold: c_int = 0o776;/"
  "validate_drop_positive_check|s/if is_nonzero != 0 \&\& value > 0/if is_nonzero != 0/"
  "memchr_sign_extend|s/\*haystack.add(i) as u8 == target/*haystack.add(i) as c_int == needle/"
  "replacement_char|s/\*s.add(idx) = b'X' as c_char/*s.add(idx) = b'Y' as c_char/"
  "subtract_threshold|s/if ACCUMULATOR > 0o150/if ACCUMULATOR >= 0o150/"
  "divide_threshold|s/if MULTIPLIER > 0o100/if MULTIPLIER >= 0o100/"
  "memchr_offset|s/result = result.wrapping_add(offset as c_int)/result = result.wrapping_add(offset as c_int + 1)/"
  "sentinel_value|s/result = 0o777;/result = 0o776;/"
  "multiplier_init|s/static mut MULTIPLIER: c_int = 1;/static mut MULTIPLIER: c_int = 2;/"
  "accumulator_init|s/static mut ACCUMULATOR: c_int = 0;/static mut ACCUMULATOR: c_int = 1;/"
  "opcount_scale|s/OPERATION_COUNT.wrapping_mul(0o10)/OPERATION_COUNT.wrapping_mul(0o11)/"
  "subtract_sign|s/ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b))/ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_sub(b))/"
  "divide_no_count|s/^    OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);\n    MULTIPLIER$/    MULTIPLIER/"
  "mode_multiply|s/let mode_multiply: c_int = 0o2;/let mode_multiply: c_int = 0o3;/"
  "search_buffer_text|s/b\"Function pointer example with static vars\"/b\"function pointer example with static vars\"/"
  "octal_literal|s/process_octal_string(message.as_mut_ptr(), 0o123)/process_octal_string(message.as_mut_ptr(), 0o124)/"
  "findrep_needle|s/find_and_replace_char(message.as_mut_ptr(), b'O' as c_int)/find_and_replace_char(message.as_mut_ptr(), b'o' as c_int)/"
)

fails=0
total=0
for entry in "${MUTANTS[@]}"; do
  name="${entry%%|*}"
  expr="${entry#*|}"
  total=$((total + 1))
  restore
  before=$(md5sum "$SRC" | cut -d' ' -f1)
  sed -i "$expr" "$SRC"
  after=$(md5sum "$SRC" | cut -d' ' -f1)
  if [ "$before" = "$after" ]; then
    echo "SKIP  $name (sed matched nothing - update the expression)"
    continue
  fi
  if ! timeout 300 cargo build --quiet 2>/dev/null; then
    echo "SKIP  $name (mutant does not compile)"
    continue
  fi
  out=$(timeout 500 cargo test --quiet 2>&1)
  if echo "$out" | grep -q "test result: FAILED"; then
    n=$(echo "$out" | grep -c 'FAILED$')
    echo "CAUGHT  $name  ($n failing tests)"
    fails=$((fails + 1))
  else
    echo "MISSED  $name  <-- the suite does not detect this bug"
  fi
done

restore
cargo build --quiet 2>/dev/null
echo "-----"
echo "mutants caught: $fails / $total"
