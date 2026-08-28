#!/usr/bin/env bash
# Test-sensitivity evidence for ERRORS.md / CONFIGS.md.
#
# Injects one single-edit mutation into src/lib.rs at a time, rebuilds the
# cdylibs and re-runs the differential suite. A mutation that is NOT detected is
# a gap in the tests (unless it is provably an equivalent mutant).
#
# src/lib.rs is restored from a pristine copy before every mutation and at exit.
set -uo pipefail
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; rm -rf target/so-dev target/so-rel target/so-ubc; }
trap 'restore; rm -f "$ORIG"' EXIT

# "<sed expression>;#<name>"   (equivalent mutants are marked EQUIV in NOTES)
MUTS=(
  "s/x >> 17/x >> 18/;#shift17"
  "s/x << 23/x << 24/;#shift23"
  "s/y >> 26/y >> 25/;#shift26"
  "s/x.wrapping_add(y)/x.wrapping_sub(y)/;#add_to_sub"
  "s/x.wrapping_add(y)/x + y/;#nonwrapping_add"
  "s/value >> 12/value >> 13/;#shift12"
  "s/exponent: u64 = 1023/exponent: u64 = 1022/;#exponent_bias"
  "s/(exponent << 52)/(exponent << 51)/;#exponent_shift"
  "s/f64::from_bits(result) - 1.0/f64::from_bits(result)/;#drop_minus_one"
  "s/^    x \^= y \^ (y >> 26);/    x ^= y | (y >> 26);/;#xor_to_or"
  "/ptr::write_unaligned(state, y)/d;#drop_state0_write"
  "/ptr::write_unaligned(state.add(1), x)/d;#drop_state1_write"
  "s/ptr::write_unaligned(state, y)/ptr::write_unaligned(state.add(1), y)/;#write_wrong_slot"
  "s/ptr::read_unaligned(state.add(1))/ptr::read_unaligned(state)/;#read_wrong_slot"
  "s/read_unaligned(state) };\$/read_unaligned(state.add(1)) };/;#swap_reads"
  "s/let mantissa: u64 = value >> 12;/let mantissa: u64 = value \& 0x000F_FFFF_FFFF_FFFF;/;#mask_not_shift"
  # --- known EQUIVALENT mutants: expected to survive ---
  "s/f64::from_bits(result) - 1.0/f64::from_bits(result) - 1.0f32 as f64/;#EQUIV_f32_one"
  "s/(exponent << 52) | mantissa/(exponent << 52) ^ mantissa/;#EQUIV_or_to_xor"
)

KILLED=0; SURVIVED=0; SKIPPED=0; UNEXPECTED=0
for m in "${MUTS[@]}"; do
  name="${m##*#}"; expr="${m%%;#*}"
  cp "$ORIG" src/lib.rs
  sed -i "$expr" src/lib.rs
  if diff -q "$ORIG" src/lib.rs >/dev/null; then
    echo "MUTATION $name : SKIPPED (sed pattern did not apply)"
    SKIPPED=$((SKIPPED+1)); continue
  fi
  rm -rf target/so-dev target/so-rel target/so-ubc
  out=$(timeout 300 cargo test --offline -q 2>&1)
  if echo "$out" | grep -qE 'FAILED|panicked|^error'; then
    echo "MUTATION $name : KILLED"
    KILLED=$((KILLED+1))
    case "$name" in EQUIV_*) echo "  !! an EQUIVALENT mutant was killed -- re-check the reasoning"; UNEXPECTED=$((UNEXPECTED+1));; esac
  else
    SURVIVED=$((SURVIVED+1))
    case "$name" in
      EQUIV_*) echo "MUTATION $name : survived (expected -- equivalent mutant)" ;;
      *)       echo "MUTATION $name : *** SURVIVED *** <<< TEST GAP"; UNEXPECTED=$((UNEXPECTED+1)) ;;
    esac
  fi
done

echo "-------------------------------------------------------------"
echo "killed=$KILLED survived=$SURVIVED skipped=$SKIPPED unexpected=$UNEXPECTED"
[ "$UNEXPECTED" -eq 0 ] && [ "$SKIPPED" -eq 0 ] \
  && echo "MUTATION CHECK PASSED (every non-equivalent mutation was detected)" \
  || { echo "MUTATION CHECK FAILED"; exit 1; }
