#!/usr/bin/env bash
# Mutation check: prove the differential test-suite actually detects divergence.
#
# For each mutation we inject a deliberate bug into src/lib.rs, rebuild the
# cdylib, run the full suite, and REQUIRE it to fail. A mutation that survives
# means the suite has a blind spot.
#
# src/lib.rs is restored from a backup after every mutation (and on exit).
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp "$SRC" "$BAK"
trap 'cp "$BAK" "$SRC"; rm -f "$BAK"' EXIT

pass=0
fail=0

# run_mutation <expect> <name> <from> <to>
#   expect=caught      -> the suite MUST fail on this mutant
#   expect=equivalent  -> the mutant is provably semantics-preserving, so the
#                         suite MUST still pass (see the notes at each use)
run_mutation() {
    local expect="$1" name="$2" from="$3" to="$4"
    cp "$BAK" "$SRC"
    if ! grep -qF -- "$from" "$SRC"; then
        echo "!! SKIP  $name -- pattern not found: $from"
        fail=$((fail + 1))
        return
    fi
    python3 - "$SRC" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert a in s, a
open(p, 'w').write(s.replace(a, b, 1))
PY
    if ! cargo build --release >/dev/null 2>&1; then
        echo "!! SKIP  $name -- mutant did not compile"
        fail=$((fail + 1))
        cp "$BAK" "$SRC"
        return
    fi
    local survived=0
    if timeout 600 cargo test --release >/dev/null 2>&1; then
        survived=1
    fi
    if [ "$expect" = caught ]; then
        if [ "$survived" -eq 1 ]; then
            echo "XX SURVIVED   $name   <-- BLIND SPOT in the test suite"
            fail=$((fail + 1))
        else
            echo "ok CAUGHT     $name"
            pass=$((pass + 1))
        fi
    else
        if [ "$survived" -eq 1 ]; then
            echo "ok EQUIVALENT $name (semantics-preserving, correctly still passes)"
            pass=$((pass + 1))
        else
            echo "XX FLAGGED    $name   <-- suite rejects a semantics-PRESERVING change"
            fail=$((fail + 1))
        fi
    fi
    cp "$BAK" "$SRC"
}

echo "=== mutation check: every mutant MUST be caught ==="

run_mutation caught "decode: fall-through 63 -> 62" \
    "    63
}" "    62
}"

run_mutation caught "decode: 'A'..'Z' upper bound off-by-one" \
    "c <= b'Z' as c_char" "c < b'Z' as c_char"

run_mutation caught "decode: 'a' offset 26 -> 25" \
    "+ 26) as u8" "+ 25) as u8"

run_mutation caught "decode: digit offset 52 -> 53" \
    "+ 52) as u8" "+ 53) as u8"

run_mutation caught "decode: '+' 62 -> 61" \
    "return 62;" "return 61;"

run_mutation caught "is_base64: drop '+'" \
    "|| (c == b'+' as c_char)" "|| (c == b'+' as c_char && false)"

run_mutation caught "is_base64: drop '='" \
    "|| (c == b'=' as c_char)" "|| (c == b'=' as c_char && false)"

# EQUIVALENT: for signed char, 0x80..0xFF are negative and fail `>= 'a'`; read as
# u8 they are 128..255 and fail `<= 'z'`. Both forms reject exactly the same set.
run_mutation equivalent "is_base64: unsigned compare of the 'a'..'z' range" \
    "(c >= b'a' as c_char && c <= b'z' as c_char)" \
    "((c as u8) >= b'a' && (c as u8) <= b'z')"

# NON-equivalent sign-extension bug: makes every negative (0x80..0xFF) byte pass
# the base64 filter, which the C rejects. Must be caught.
run_mutation caught "is_base64: accept negative/high bytes (real sign bug)" \
    "|| (c == b'/' as c_char)" "|| (c == b'/' as c_char) || (c as i32) < 0"

run_mutation caught "alloc: calloc l+13 -> l+12" \
    "l.wrapping_add(13)" "l.wrapping_add(12)"

run_mutation caught "alloc: strlen+1 -> strlen" \
    "strlen(src).wrapping_add(1) as c_int" "strlen(src) as c_int"

run_mutation caught "alloc: malloc(l) -> malloc(l+1)" \
    "buf = malloc(l as isize as usize)" "buf = malloc((l + 1) as isize as usize)"

run_mutation caught "leak: drop free(buf)" \
    "free(buf as *mut c_void);" "let _ = buf;"

run_mutation caught "leak: drop free(dest) on malloc failure" \
    "                free(dest as *mut c_void);
                return ptr::null_mut();" \
    "                return ptr::null_mut();"

run_mutation caught "byte 1: b2 >> 4 -> b2 >> 3" \
    "(b2 as u32) >> 4" "(b2 as u32) >> 3"

run_mutation caught "byte 2: b2 & 0xf -> b2 & 0x7" \
    "((b2 as u32) & 0xf)" "((b2 as u32) & 0x7)"

# EQUIVALENT: the extra bit of `& 0x7` lands at 1<<8 after `<< 6` and is discarded
# by the store into a byte, so the observable result is unchanged.
run_mutation equivalent "byte 3: b3 & 0x3 -> b3 & 0x7 (extra bit truncated away)" \
    "((b3 as u32) & 0x3)" "((b3 as u32) & 0x7)"

# NON-equivalent: `& 0x1` DROPS bit 1, whose 1<<7 contribution survives the store.
run_mutation caught "byte 3: b3 & 0x3 -> b3 & 0x1 (loses a live bit)" \
    "((b3 as u32) & 0x3)" "((b3 as u32) & 0x1)"

run_mutation caught "byte 1: b1 << 2 -> b1 << 1" \
    "(b1 as u32) << 2" "(b1 as u32) << 1"

run_mutation caught "byte 3: | b4 -> | (b4 >> 1)" \
    "<< 6 | (b4 as u32)" "<< 6 | ((b4 as u32) >> 1)"

run_mutation caught "pad: c3 != '=' -> c3 != '/'" \
    "if c3 != b'=' as c_char" "if c3 != b'/' as c_char"

run_mutation caught "pad: c4 != '=' -> always emit" \
    "if c4 != b'=' as c_char" "if true"

run_mutation caught "loop: k < l -> k + 3 < l (drops tail quartet)" \
    "while k < l {" "while k + 3 < l {"

run_mutation caught "loop: k += 4 -> k += 3" \
    "k += 4;" "k += 3;"

run_mutation caught "tail: c2 default 'A' -> '='" \
    "let mut c2: c_char = b'A' as c_char;" "let mut c2: c_char = b'=' as c_char;"

run_mutation caught "tail: c3 default 'A' -> '='" \
    "let mut c3: c_char = b'A' as c_char;" "let mut c3: c_char = b'=' as c_char;"

run_mutation caught "tail: k+2 < l -> k+2 <= l" \
    "if k + 2 < l {" "if k + 2 <= l {"

run_mutation caught "guard: accept empty string" \
    "if !src.is_null() && *src != 0 {" "if !src.is_null() {"

run_mutation caught "guard: accept NULL" \
    "if !src.is_null() && *src != 0 {" "if src.is_null() || *src != 0 {"

echo
echo "=== mutants caught: $pass ; survived/skipped: $fail ==="
[ "$fail" -eq 0 ] || exit 1
