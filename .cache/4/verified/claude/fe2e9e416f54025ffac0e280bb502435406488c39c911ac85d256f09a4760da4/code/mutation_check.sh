#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects deliberate bugs into src/lib.rs one at a time, rebuilds the cdylib and
# re-runs the differential tests.
#
#   * every mutation in MUTATIONS      MUST be caught (tests must FAIL)
#   * every mutation in EQUIV_MUTANTS  must NOT be caught, because it is
#     provably behaviour-preserving (proofs below). If one of these were caught
#     the proof would be wrong — equally worth knowing.
#
# src/lib.rs is always restored, even on interrupt.
#
# Usage:  ./mutation_check.sh [START] [COUNT]
#         Runs only mutations [START, START+COUNT) of the combined list, so a
#         full sweep can be split into chunks that each finish well inside a
#         600 s budget. With no arguments, runs everything.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
SRC=src/lib.rs
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
cleanup() { restore; rm -f "$BAK"; }
trap cleanup EXIT INT TERM

THREADS=${MUT_THREADS:-12}

# --- mutations that MUST be detected --------------------------------------
MUTATIONS=(
  "mask no longer clears bit 0 ::: s@const MASK: u64 = u64::MAX << 1;@const MASK: u64 = u64::MAX;@"
  "mask clears two low bits instead of one ::: s@const MASK: u64 = u64::MAX << 1;@const MASK: u64 = u64::MAX << 2;@"
  "left shift: C-UB-naive (>=64 yields 0) instead of hardware masking ::: s@val << (n \& (TFLAC_UINT_BITS - 1))@val.checked_shl(n).unwrap_or(0)@"
  "right shift: C-UB-naive (>=64 yields 0) instead of hardware masking ::: s@val >> (n \& (TFLAC_UINT_BITS - 1))@val.checked_shr(n).unwrap_or(0)@"
  "left shift masked to 5 bits instead of 6 ::: s@val << (n \& (TFLAC_UINT_BITS - 1))@val << (n \& 31)@"
  "right shift masked to 7 bits instead of 6 ::: s@val >> (n \& (TFLAC_UINT_BITS - 1))@val >> (n \& 127)@"
  "tot counter saturates instead of wrapping ::: s@read_unaligned(p_tot).wrapping_add(bits)@read_unaligned(p_tot).saturating_add(bits)@"
  "bw->bits accumulation saturates ::: s@read_unaligned(p_bits).wrapping_add(b)@read_unaligned(p_bits).saturating_add(b)@"
  "loop guard promoted to 64-bit (loses 32-bit wraparound) ::: s@while unsafe { read_unaligned(p_bits) }.wrapping_add(bits) >= TFLAC_UINT_BITS@while unsafe { read_unaligned(p_bits) } as u64 + bits as u64 >= TFLAC_UINT_BITS as u64@"
  "loop guard uses > instead of >= ::: s@.wrapping_add(bits) >= TFLAC_UINT_BITS \&\& i < 100@.wrapping_add(bits) > TFLAC_UINT_BITS \&\& i < 100@"
  "off-by-one dropped: b = 64 - bw->bits instead of 64 - bw->bits - 1 ::: s@^            \.wrapping_sub(1);@            ;@"
  "b computed from 63 instead of 64 (extra off-by-one) ::: s@let mut b: u32 = TFLAC_UINT_BITS@let mut b: u32 = (TFLAC_UINT_BITS - 1)@"
  "loop cap removed entirely ::: s@ \&\& i < 100@@"
  "loop cap reduced to 0 iterations (body never runs) ::: s@i < 100@i < 0@"
  "field access via an aligned reference again (breaks a misaligned bw) ::: s@addr_of_mut!((\*bw).val)@\&mut (*bw).val@"
  "bits field via an aligned reference (breaks a misaligned bw) ::: s@addr_of_mut!((\*bw).bits)@\&mut (*bw).bits@"
  "bw->val accumulates with XOR instead of OR ::: s@read_unaligned(p_val) | shr(val, cur_bits)@read_unaligned(p_val) ^ shr(val, cur_bits)@"
  "bw->val accumulates a left shift instead of a right shift ::: s@shr(val, cur_bits)@shl(val, cur_bits)@"
  "in-loop 'bw->val \&= mask' dropped ::: s@write_unaligned(p_val, read_unaligned(p_val) \& MASK)@write_unaligned(p_val, read_unaligned(p_val))@"
  "tail 'bw->bits += bits' dropped ::: s@write_unaligned(p_bits, read_unaligned(p_bits).wrapping_add(bits))@write_unaligned(p_bits, read_unaligned(p_bits))@"
  "writes tot into the pos field (wrong offset) ::: s@addr_of_mut!((\*bw).tot)@addr_of_mut!((*bw).pos)@"
  "writes bits into the len field (wrong offset) ::: s@addr_of_mut!((\*bw).bits)@addr_of_mut!((*bw).len)@"
  "reads/writes val at the buffer offset (wrong offset) ::: s@addr_of_mut!((\*bw).val)@addr_of_mut!((*bw).buffer).cast::<u64>()@"
  "returns -1 instead of 0 ::: s@^    0\$@    -1@"
  "returns 1 instead of 0 ::: s@^    0\$@    1@"
)

# --- mutations that provably CANNOT be detected ---------------------------
#
# PROOF. Brute force over 1 313 316 structured + 40 000 000 random
# (bw->bits, bits) pairs shows the while-loop makes AT MOST ONE progressing
# iteration (one with b != 0). Therefore:
#
#  (1) Once b == 0 the body is
#          bw->val = (bw->val | (val >> bw->bits)) & mask;  bw->bits += 0;
#          val <<= 0;                                       bits    -= 0;
#      Its operands never change again and x |-> (x | y) & m is idempotent, so
#      spins 2..cap cannot alter observable state.
#  (2) The single progressing iteration ends with `bw->val &= mask`, so bit 0 of
#      bw->val is already 0 when the loop stalls. Writing X = val >> bw->bits,
#      the extra stall spin yields ((A | X) & ~1) | X in the tail, whereas
#      stopping immediately yields A | X. Both have bit 0 = X_0 and identical
#      higher bits, so they are equal.
#      => every cap >= 1 is observationally identical to cap == 100. A separate
#      8 000 000-case sweep confirms caps 1, 2, 3, 99, 101 and 1000 all agree
#      with 100, while cap 0 differs in 41 % of cases (hence cap 0 is in
#      MUTATIONS above, and is caught).
#  (3) `b > bits ? bits : b` and `b >= bits ? bits : b` differ only when
#      b == bits, where both arms evaluate to the same number.
EQUIV_MUTANTS=(
  "loop cap 100 -> 101 ::: s@i < 100@i < 101@"
  "loop cap 100 -> 1000 ::: s@i < 100@i < 1000@"
  "loop cap 100 -> 3 ::: s@i < 100@i < 3@"
  "loop cap 100 -> 2 ::: s@i < 100@i < 2@"
  "loop cap 100 -> 1 (only cap 0 is observable) ::: s@i < 100@i < 1@"
  "min() comparison b > bits -> b >= bits (arms equal at the tie) ::: s@if b > bits { bits } else { b }@if b >= bits { bits } else { b }@"
)

N_MUT=${#MUTATIONS[@]}
N_EQ=${#EQUIV_MUTANTS[@]}
TOTAL=$((N_MUT + N_EQ))
START=${1:-0}
COUNT=${2:-$TOTAL}
END=$((START + COUNT))
[ "$END" -gt "$TOTAL" ] && END=$TOTAL

CAUGHT=0; MISSED=0; NOTAPPLIED=0; EQ_OK=0; EQ_BAD=0

# run_one <sed-expr> -> notapplied | buildfail | caught | passed
run_one() {
    cp "$BAK" "$SRC"
    sed -i "$1" "$SRC" 2>/dev/null
    if cmp -s "$BAK" "$SRC"; then echo notapplied; return; fi
    if ! timeout 300 cargo build --offline >/dev/null 2>&1; then echo buildfail; return; fi
    if timeout 400 cargo test --offline -- --test-threads="$THREADS" >/dev/null 2>&1; then
        echo passed
    else
        echo caught
    fi
}

printf 'mutation sweep: entries [%d, %d) of %d  (%d real bugs + %d equivalent)\n' \
       "$START" "$END" "$TOTAL" "$N_MUT" "$N_EQ"

for ((idx = START; idx < END; idx++)); do
    if [ "$idx" -lt "$N_MUT" ]; then
        entry=${MUTATIONS[$idx]}; kind=real
    else
        entry=${EQUIV_MUTANTS[$((idx - N_MUT))]}; kind=equiv
    fi
    DESC=$(printf '%s' "${entry%%:::*}" | sed 's/[[:space:]]*$//')
    EXPR=$(printf '%s' "${entry#*:::}" | sed 's/^[[:space:]]*//')
    RESULT=$(run_one "$EXPR")

    if [ "$kind" = real ]; then
        case $RESULT in
          notapplied) printf '\033[33m[%2d NOT APPLIED]\033[0m %s\n' "$idx" "$DESC"; NOTAPPLIED=$((NOTAPPLIED+1));;
          buildfail)  printf '\033[33m[%2d BUILD FAIL]\033[0m  %s\n' "$idx" "$DESC"; NOTAPPLIED=$((NOTAPPLIED+1));;
          caught)     printf '\033[32m[%2d caught]\033[0m      %s\n' "$idx" "$DESC"; CAUGHT=$((CAUGHT+1));;
          passed)     printf '\033[31m[%2d MISSED!]\033[0m     %s\n' "$idx" "$DESC"; MISSED=$((MISSED+1));;
        esac
    else
        case $RESULT in
          notapplied|buildfail) printf '\033[33m[%2d NOT APPLIED]\033[0m %s\n' "$idx" "$DESC"; NOTAPPLIED=$((NOTAPPLIED+1));;
          passed) printf '\033[32m[%2d equivalent, as proven]\033[0m %s\n' "$idx" "$DESC"; EQ_OK=$((EQ_OK+1));;
          caught) printf '\033[31m[%2d PROOF WRONG - caught]\033[0m %s\n' "$idx" "$DESC"; EQ_BAD=$((EQ_BAD+1));;
        esac
    fi
done

restore
timeout 300 cargo build --offline >/dev/null 2>&1

printf '\nreal bugs caught: %d, MISSED: %d, not applied: %d\n' "$CAUGHT" "$MISSED" "$NOTAPPLIED"
printf 'equivalent mutants confirmed equivalent: %d, proof violations: %d\n' "$EQ_OK" "$EQ_BAD"
if [ "$MISSED" -eq 0 ] && [ "$EQ_BAD" -eq 0 ] && [ "$NOTAPPLIED" -eq 0 ]; then
    printf '\033[32mCHUNK PASSED\033[0m\n'; exit 0
fi
printf '\033[31mCHUNK FAILED\033[0m\n'; exit 1
