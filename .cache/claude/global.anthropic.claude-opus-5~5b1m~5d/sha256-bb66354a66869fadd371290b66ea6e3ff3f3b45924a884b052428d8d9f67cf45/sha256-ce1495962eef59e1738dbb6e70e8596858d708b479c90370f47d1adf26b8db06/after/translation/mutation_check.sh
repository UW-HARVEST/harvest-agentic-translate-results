#!/usr/bin/env bash
# Mutation sweep: proves the differential test suite can actually DETECT a
# divergence in each distinct code path of the translation.
#
# For each mutation we patch src/lib.rs, rebuild the cdylib, and run the suite.
# Each mutation is tagged with its EXPECTED verdict:
#
#   CAUGHT     - the suite must fail. If it passes, that path is untested (a gap).
#   EQUIVALENT - the mutation is provably semantics-preserving, so the suite
#                MUST still pass. Each of these is justified below and is
#                additionally asserted as a property by
#                `quirk_equivalent_mutant_properties` /
#                `quirk_seed_cancels_out_identically_in_both_libraries`
#                in tests/phase_c_errors.rs.
#
# The script also verifies the rebuilt .so actually differs from the baseline
# .so; if the codegen is byte-identical the mutation is reported as a no-op
# rather than producing a misleading verdict.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

PROFILE_DIR=target/debug
SO="$PROFILE_DIR/libsiphash_lib.so"

BAK="$(mktemp)"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; rm -f "$BAK"; touch src/lib.rs; cargo build -q >/dev/null 2>&1; }
trap restore EXIT

# Baseline build + hash of the pristine .so.
touch src/lib.rs
cargo build -q >/dev/null 2>&1 || { echo "baseline build failed"; exit 1; }
BASE_SHA="$(sha256sum "$SO" | cut -d' ' -f1)"

# desc @@ expected @@ literal-to-replace @@ replacement
MUTATIONS=(
'main-loop low half: sign-extend -> zero-extend@@CAUGHT@@data = lo as usize;@@data = lo as u32 as usize;'
'main-loop high half: shift 32 -> 31@@CAUGHT@@data |= ((hi as usize) << 16) << 16;@@data |= ((hi as usize) << 15) << 16;'
'main-loop high half: byte index 4 -> 5@@CAUGHT@@(*d.add(4) as i32)@@(*d.add(5) as i32)'
'tail case 4: sign-extend -> zero-extend@@CAUGHT@@data |= ((*d.add(3) as i32) << 24) as usize;@@data |= ((*d.add(3) as i32) << 24) as u32 as usize;'
'tail case 7: shift 48 -> 47@@CAUGHT@@data |= ((*d.add(6) as usize) << 24) << 24;@@data |= ((*d.add(6) as usize) << 23) << 24;'
'tail case 6: shift 40 -> 41@@CAUGHT@@data |= ((*d.add(5) as usize) << 20) << 20;@@data |= ((*d.add(5) as usize) << 21) << 20;'
'tail case 5: shift 32 -> 33@@CAUGHT@@data |= ((*d.add(4) as usize) << 16) << 16;@@data |= ((*d.add(4) as usize) << 17) << 16;'
'tail case 3: byte index 2 -> 1@@CAUGHT@@data |= ((*d.add(2) as i32) << 16) as usize;@@data |= ((*d.add(1) as i32) << 16) as usize;'
'tail case 2: shift 8 -> 9@@CAUGHT@@data |= ((*d.add(1) as i32) << 8) as usize;@@data |= ((*d.add(1) as i32) << 9) as usize;'
'tail case 1: byte index 0 -> 1@@CAUGHT@@data |= (*d.add(0) as i32) as usize;@@data |= (*d.add(1) as i32) as usize;'
'tail arm 6 guard: rem>=6 -> rem>=5@@CAUGHT@@if rem >= 6 && rem <= 7 {@@if rem >= 5 && rem <= 7 {'
'tail arm 4 guard: rem>=4 -> rem>=3@@CAUGHT@@if rem >= 4 && rem <= 7 {@@if rem >= 3 && rem <= 7 {'
'length mixin: len << 56 -> len << 55@@CAUGHT@@data = len << (SIZE_T_BITS - 8);@@data = len << (SIZE_T_BITS - 9);'
'length mixin: len -> 0@@CAUGHT@@data = len << (SIZE_T_BITS - 8);@@data = 0 << (SIZE_T_BITS - 8);'
'finalization constant: 0xff -> 0xfe@@CAUGHT@@v2 ^= 0xff;@@v2 ^= 0xfe;'
'finalization: v2 -> v1@@CAUGHT@@v2 ^= 0xff;@@v1 ^= 0xff;'
'sipround rotate 13 -> 14@@CAUGHT@@*v1 = rotate_left(*v1, 13);@@*v1 = rotate_left(*v1, 14);'
'sipround rotate 16 -> 15@@CAUGHT@@*v3 = rotate_left(*v3, 16);@@*v3 = rotate_left(*v3, 15);'
'sipround rotate 17 -> 18@@CAUGHT@@*v1 = rotate_left(*v1, 17);@@*v1 = rotate_left(*v1, 18);'
'sipround rotate 21 -> 22@@CAUGHT@@*v3 = rotate_left(*v3, 21);@@*v3 = rotate_left(*v3, 22);'
'sipround half-rotate: /2 -> /2+1@@CAUGHT@@*v0 = rotate_left(*v0, SIZE_T_BITS / 2);@@*v0 = rotate_left(*v0, SIZE_T_BITS / 2 + 1);'
'sipround: v0 += v1 -> v0 -= v1@@CAUGHT@@*v0 = v0.wrapping_add(*v1);@@*v0 = v0.wrapping_sub(*v1);'
'v0 init constant@@CAUGHT@@.wrapping_add(0x70736575) ^ seed;@@.wrapping_add(0x70736576) ^ seed;'
# Flipping only ONE of the two `~seed` occurrences leaves `seed ^ ~seed` = all-ones,
# which is seed-independent but a DIFFERENT constant -> observable, hence CAUGHT.
'v1 init: ~seed -> seed (one of two)@@CAUGHT@@.wrapping_add(0x6e646f6d) ^ !seed;@@.wrapping_add(0x6e646f6d) ^ seed;'
# The sign-extension of `hi` is shifted entirely out by the total shift of 32.
'main-loop high half: sign-extend -> zero-extend@@EQUIVALENT@@data |= ((hi as usize) << 16) << 16;@@data |= ((hi as u32 as usize) << 16) << 16;'
# The tail residue is always <= 7, so `== 7` and `>= 7` are the same guard.
'tail residue guard: rem==7 -> rem>=7@@EQUIVALENT@@if rem == 7 {@@if rem >= 7 {'
'v2 second xor constant@@CAUGHT@@v2 ^= (0x0706050403020100u64 as usize) ^ seed;@@v2 ^= (0x0706050403020101u64 as usize) ^ seed;'
'final fold: drop v3@@CAUGHT@@v0 ^ v1 ^ v2 ^ v3@@v0 ^ v1 ^ v2'
'main loop: 2 sipround -> 1@@CAUGHT@@        while j < 2 {@@        while j < 1 {'
'final: 4 sipround -> 3@@CAUGHT@@    while j < 4 {@@    while j < 3 {'
'main loop bound: <= len -> < len@@CAUGHT@@while i + sz <= len {@@while i + sz < len {'
'siphash: mem fill truncation removed@@CAUGHT@@mem[i as usize] = z as u8;@@mem[i as usize] = (z & 0x7f) as u8;'
'siphash: z increment removed@@CAUGHT@@z = z.wrapping_add(1);@@z = z.wrapping_add(0);'
'siphash: printf format 0x%02x -> 0x%02X@@CAUGHT@@c_printf(c"0x%02x, ".as_ptr(), byte as c_int);@@c_printf(c"0x%02X, ".as_ptr(), byte as c_int);'
'siphash: prefix two spaces -> one@@CAUGHT@@c_printf(c"  { ".as_ptr());@@c_printf(c" { ".as_ptr());'
'siphash: suffix " }," -> " };"@@CAUGHT@@c_printf(c" },@@c_printf(c" };'
'siphash: byte order j*8 -> (7-j)*8@@CAUGHT@@let byte = ((hash >> (j * 8)) & 255) as u8;@@let byte = ((hash >> ((7 - j) * 8)) & 255) as u8;'
'siphash: mask 255 -> 254@@CAUGHT@@& 255) as u8;@@& 254) as u8;'
'siphash: inner loop 8 -> 7@@CAUGHT@@while j < 8 {@@while j < 7 {'
'siphash: outer loop 64 -> 63@@CAUGHT@@    while i < 64 {\n        let hash@@    while i < 63 {\n        let hash'
'siphash: hash seed 0 -> 1@@EQUIVALENT@@as *mut c_void, i as usize, 0) };@@as *mut c_void, i as usize, 1) };'
'siphash: hash len i -> i+1@@CAUGHT@@as *mut c_void, i as usize, 0) };@@as *mut c_void, i as usize + 1, 0) };'
)

# Justification for every EQUIVALENT tag (printed in the summary).
declare -A WHY=(
  ['siphash: hash seed 0 -> 1']='stbds_hash_bytes is seed-INDEPENDENT in the C: `seed` is XORed into each v-word twice (lib.c:10-13 then :14-17) and cancels. Locked down by quirk_seed_cancels_out_identically_in_both_libraries.'
  ['main-loop high half: sign-extend -> zero-extend']='the value is shifted left by a total of 32, so the sign-extended upper 32 bits are discarded; sign- and zero-extension are indistinguishable. Proven by quirk_equivalent_mutant_properties (a).'
  ['tail residue guard: rem==7 -> rem>=7']='the loop exit condition forces len-i in 0..=7, so rem can never exceed 7. Proven by quirk_equivalent_mutant_properties (b) and err_switch_default_arm_unreachable.'
)

caught=0; ok_equiv=0; gaps=0; noop=0; skipped=0
declare -a GAPS

printf '%-56s %-11s %s\n' "MUTATION" "EXPECTED" "RESULT"
printf '%.0s-' {1..92}; echo

for m in "${MUTATIONS[@]}"; do
    desc="${m%%@@*}";  r1="${m#*@@}"
    exp="${r1%%@@*}";  r2="${r1#*@@}"
    from="${r2%%@@*}"; to="${r2#*@@}"

    cp "$BAK" src/lib.rs
    if ! python3 - "$from" "$to" <<'PY'
import sys
frm, to = sys.argv[1].replace('\\n','\n'), sys.argv[2].replace('\\n','\n')
s = open('src/lib.rs').read()
if frm not in s: sys.exit(3)
open('src/lib.rs','w').write(s.replace(frm, to, 1))
PY
    then
        printf '%-56s %-11s %s\n' "$desc" "$exp" "SKIP (pattern not found)"
        skipped=$((skipped+1)); continue
    fi

    touch src/lib.rs
    if ! cargo build -q >/dev/null 2>&1; then
        printf '%-56s %-11s %s\n' "$desc" "$exp" "SKIP (build failed)"
        skipped=$((skipped+1)); continue
    fi

    # Guard against a mutation that produces byte-identical codegen: the
    # verdict would be meaningless.
    if [ "$(sha256sum "$SO" | cut -d' ' -f1)" = "$BASE_SHA" ]; then
        printf '%-56s %-11s %s\n' "$desc" "$exp" "NO-OP (identical codegen)"
        noop=$((noop+1)); continue
    fi

    if cargo test -q >/dev/null 2>&1; then verdict=survived; else verdict=caught; fi

    case "$exp:$verdict" in
      CAUGHT:caught)         printf '%-56s %-11s %s\n' "$desc" "$exp" "caught  OK";          caught=$((caught+1));;
      CAUGHT:survived)       printf '%-56s %-11s %s\n' "$desc" "$exp" "*** SURVIVED - GAP ***"; gaps=$((gaps+1)); GAPS+=("$desc");;
      EQUIVALENT:survived)   printf '%-56s %-11s %s\n' "$desc" "$exp" "survived  OK (equivalent)"; ok_equiv=$((ok_equiv+1));;
      EQUIVALENT:caught)     printf '%-56s %-11s %s\n' "$desc" "$exp" "*** CAUGHT - reclassify ***"; gaps=$((gaps+1)); GAPS+=("$desc (expected EQUIVALENT but was caught)");;
    esac
done

restore; trap - EXIT

printf '%.0s-' {1..92}; echo
echo "caught-as-expected: $caught   equivalent-as-expected: $ok_equiv   no-op: $noop   skipped: $skipped   GAPS: $gaps"
echo
echo "EQUIVALENT mutations and why they cannot be detected:"
for k in "${!WHY[@]}"; do echo "  - $k"; echo "      ${WHY[$k]}"; done
if [ "$gaps" -ne 0 ]; then
    echo; echo "COVERAGE GAPS:"; printf '  - %s\n' "${GAPS[@]}"; exit 1
fi
echo
echo "OK: every non-equivalent mutation was detected by the differential suite."
