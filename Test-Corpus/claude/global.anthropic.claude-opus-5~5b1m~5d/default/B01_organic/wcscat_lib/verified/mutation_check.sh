#!/usr/bin/env bash
# Sensitivity check for the differential suite.
#
# Deliberately breaks the Rust translation in ways a sloppy translator plausibly
# would, and proves the suite CATCHES each one. Every mutation must make at least
# one test fail. src/lib.rs is restored after each mutation (and on exit).
#
#   ./mutation_check.sh
#
# Exit status = number of mutations that escaped detection (0 is the goal, apart
# from the mutations explicitly listed as observationally equivalent).
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap 'restore; rm -f "$BAK"' EXIT

FAILS=0
ESCAPED=()

# run_mutation <description> <perl-expr> [expect_equivalent]
run_mutation() {
    desc=$1; expr=$2; equiv=${3:-no}
    restore
    perl -0pi -e "$expr" "$SRC"
    if diff -q "$BAK" "$SRC" >/dev/null; then
        echo "ERROR: mutation did not apply: $desc"
        FAILS=$((FAILS+1)); return
    fi
    out=$(timeout 600 cargo test --release 2>&1)
    if echo "$out" | grep -qE '^test result: FAILED|^error: test failed|SIGSEGV|signal'; then
        n=$(echo "$out" | grep -cE '^test .* FAILED$')
        echo "CAUGHT   (${n} failing tests): $desc"
    elif [ "$equiv" = equivalent ]; then
        echo "EQUIV    (no observable difference, by design): $desc"
    else
        echo "SURVIVED (suite still green!): $desc"
        ESCAPED+=("$desc")
        FAILS=$((FAILS+1))
    fi
}

echo "=== mutation sensitivity check ==="

run_mutation 'EINVAL 22 -> 21 on the !dst / numElem==0 branch' \
  's/if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;/if dst.is_null() || num_elem == 0 {\n        return 21;/'

run_mutation 'EINVAL 22 -> 21 on the !src branch' \
  's/if src\.is_null\(\) \{\n        unsafe \{ \*dst = 0 \};\n        return 22;/if src.is_null() {\n        unsafe { *dst = 0 };\n        return 21;/'

run_mutation 'ERANGE 34 -> 33 on truncation' \
  's/unsafe \{ \*dst = 0 \};\n    34\n\}/unsafe { *dst = 0 };\n    33\n}/'

run_mutation 'numElem==0 also zeroes dst[0] (C leaves dst completely untouched)' \
  's/if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;/if dst.is_null() || num_elem == 0 {\n        if !dst.is_null() { unsafe { *dst = 0 }; }\n        return 22;/'

run_mutation 'check !src BEFORE numElem==0 (wrong short-circuit order)' \
  's/if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;\n    \}/if dst.is_null() {\n        return 22;\n    }/; s/(if src\.is_null\(\) \{\n        unsafe \{ \*dst = 0 \};\n        return 22;\n    \})/$1\n    if num_elem == 0 { return 22; }/'

run_mutation 'copy-loop bound  ptr < end  ->  ptr <= end' \
  's/    while ptr < end \{\n        let c/    while ptr <= end {\n        let c/'

run_mutation 'scan-loop bound  ptr < end  ->  ptr <= end  (extra OOB read only)' \
  's/while ptr < end && unsafe \{ \*ptr \} != 0/while ptr <= end \&\& unsafe { *ptr } != 0/' \
  equivalent

run_mutation 'NUL-terminate on truncation at dst[numElem-1] instead of dst[0]' \
  's/unsafe \{ \*dst = 0 \};\n    34\n\}/unsafe { *dst.wrapping_add(num_elem - 1) = 0 };\n    34\n}/'

run_mutation 'no write at all on truncation (drop the dst[0]=0)' \
  's/unsafe \{ \*dst = 0 \};\n    34\n\}/34\n}/'

run_mutation 'saturating instead of wrapping dst+numElem (misses the overflow case)' \
  's/dst\.wrapping_add\(num_elem\)/((dst as usize).saturating_add(num_elem.saturating_mul(4))) as *mut wchar_t/'

run_mutation 'off-by-one: end = dst + numElem - 1' \
  's/let end: \*mut wchar_t = dst\.wrapping_add\(num_elem\);/let end: *mut wchar_t = dst.wrapping_add(num_elem).wrapping_sub(1);/'

run_mutation 'terminator consumed but not written into dst' \
  's/        unsafe \{ \*ptr = c \};\n        ptr = ptr\.wrapping_add\(1\);\n        if c == 0 \{\n            return 0;\n        \}/        if c == 0 {\n            return 0;\n        }\n        unsafe { *ptr = c };\n        ptr = ptr.wrapping_add(1);/'

run_mutation 'scan stops on negative wchar_t (signedness bug)' \
  's/while ptr < end && unsafe \{ \*ptr \} != 0/while ptr < end \&\& unsafe { *ptr } > 0/'

run_mutation 'wrong wchar_t width (i16 instead of i32)' \
  's/pub type wchar_t = i32;/pub type wchar_t = i16;/'

run_mutation 'src pointer not advanced (copies src[0] forever)' \
  's/        src_ptr = src_ptr\.wrapping_add\(1\);\n//'

run_mutation 'skip the scan loop entirely (always overwrite from dst[0])' \
  's/    while ptr < end && unsafe \{ \*ptr \} != 0 \{\n        ptr = ptr\.wrapping_add\(1\);\n    \}//'

restore
echo "=== done: $FAILS mutation(s) escaped detection ==="
if [ ${#ESCAPED[@]} -gt 0 ]; then
    printf '  escaped: %s\n' "${ESCAPED[@]}"
fi
exit "$FAILS"
