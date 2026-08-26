#!/usr/bin/env bash
# Harness self-check (guards against a vacuous test suite).
#
#   * mutants marked "catch"      MUST be detected by the differential suite;
#   * mutants marked "equivalent" MUST survive — they are provably unobservable
#     through the C API, so detecting them would mean the tests assert something
#     the C does not actually specify.
#
# The pristine src/lib.rs is never modified: mutants are built out of $TMPDIR and
# injected into the test run through RUST_CDYLIB.
set -u
cd "$(dirname "$0")" || exit 1
WORK=${TMPDIR:-/tmp}/bin2hex-mutants
mkdir -p "$WORK"

fails=0

run_mutant() {
    local expect="$1" name="$2" sedexpr="$3"
    local src="$WORK/$name.rs" so="$WORK/lib$name.so"
    sed "$sedexpr" src/lib.rs > "$src"
    if cmp -s src/lib.rs "$src"; then
        echo "MUT $name: SED DID NOT APPLY (harness bug)"; fails=$((fails+1)); return
    fi
    if ! rustc --edition=2021 --crate-type=cdylib --crate-name bin2hex_lib \
              -Cdebug-assertions=off -Coverflow-checks=off -Cdebuginfo=0 \
              "$src" -o "$so" 2>"$WORK/$name.build.log"; then
        echo "MUT $name: DID NOT COMPILE (harness bug)"; tail -3 "$WORK/$name.build.log"
        fails=$((fails+1)); return
    fi
    if RUST_CDYLIB="$so" timeout 600 cargo test --no-default-features -q \
           >"$WORK/$name.test.log" 2>&1; then
        if [ "$expect" = equivalent ]; then
            echo "MUT $name: survived (expected: provably unobservable) OK"
        else
            echo "MUT $name: *** SURVIVED *** (tests are blind to this change)"
            fails=$((fails+1))
        fi
    else
        if [ "$expect" = equivalent ]; then
            echo "MUT $name: *** CAUGHT *** but it is unobservable -> over-strict test"
            fails=$((fails+1))
        else
            local n
            n=$(grep -c 'panicked at' "$WORK/$name.test.log")
            printf 'MUT %-18s caught (%s failing assertions) %s\n' "$name:" "$n" \
                "$(grep -m1 -o '\[[CEG][0-9a-zA-Z/_.]*\][^"]*' "$WORK/$name.test.log" | head -1 | cut -c1-90)"
        fi
    fi
}

# --- observable mutations: must be caught -----------------------------------
run_mutant catch swap_nibbles    's/(lo_ch << 8) | hi_ch/(hi_ch << 8) | lo_ch/'
run_mutant catch maxlen_lt       's/hex_maxlen <= bin_len.wrapping_mul(2)/hex_maxlen < bin_len.wrapping_mul(2)/'
run_mutant catch limit_gt        's|bin_len >= (18446744073709551615u64 as usize) / 2|bin_len > (18446744073709551615u64 as usize) / 2|'
run_mutant catch limit_half      's|18446744073709551615u64 as usize) / 2|18446744073709551615u64 as usize) / 4|'
run_mutant catch no_nul          's|^        \*hex.add(i.wrapping_mul(2)) = 0;|        // removed|'
run_mutant catch off_by_one_nul  's|\*hex.add(i.wrapping_mul(2)) = 0;|*hex.add(i.wrapping_mul(2).wrapping_add(1)) = 0;|'
run_mutant catch mask_not        's/& !38u32/\& 38u32/g'
run_mutant catch mask_39         's/& !38u32/\& !39u32/g'
run_mutant catch base_88         's/(87u32/(88u32/g'
run_mutant catch bias_11         's/c.wrapping_sub(10)/c.wrapping_sub(11)/'
run_mutant catch nibble_swap_src 's/let c: u32 = (byte \& 0xf) as u32;/let c: u32 = (byte >> 4) as u32;/'
run_mutant catch loop_bound      's|while i < bin_len|while i <= bin_len|'
run_mutant catch mask_0xf_0x7    's/(byte \& 0xf)/(byte \& 0x7)/'
run_mutant catch signed_shift    's/(byte >> 4) as u32/(byte as i8 >> 4) as u32/'

# --- provably unobservable mutations: must survive ---------------------------
# The `(unsigned char)` truncation discards everything above bit 7, and for
# c < 10 every right shift in 1..=24 leaves the low byte at 0xFF, so the shift
# amount cannot be observed through the API (verified for all 16 nibble values).
run_mutant equivalent shift7     's/>> 8) \& !38u32/>> 7) \& !38u32/g'
run_mutant equivalent shift9     's/>> 8) \& !38u32/>> 9) \& !38u32/g'
# Only the low 8 bits of `lo_ch` (via `x >>= 8`) and of `hi_ch` (via `(char)x`)
# are ever stored, and the untruncated values have zero bits in positions 8..15,
# so dropping the `as u8` casts cannot change either output byte (verified
# exhaustively for all 256 input bytes and all 4 truncate/keep combinations).
run_mutant equivalent no_trunc_lo 's/(c.wrapping_sub(10) >> 8) \& !38u32)) as u8 as u32/(c.wrapping_sub(10) >> 8) \& !38u32)) as u32/'
run_mutant equivalent no_trunc_hi 's/(b.wrapping_sub(10) >> 8) \& !38u32)) as u8 as u32/(b.wrapping_sub(10) >> 8) \& !38u32)) as u32/'

echo
if [ "$fails" -eq 0 ]; then
    echo "MUTATION CHECK PASSED — every observable mutation is caught, every"
    echo "unobservable one survives: the differential suite is neither vacuous"
    echo "nor over-strict."
else
    echo "MUTATION CHECK FAILED: $fails problem(s)."
fi
exit "$fails"
