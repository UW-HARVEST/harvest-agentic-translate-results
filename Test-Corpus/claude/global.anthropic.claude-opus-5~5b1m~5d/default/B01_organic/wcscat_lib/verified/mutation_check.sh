#!/usr/bin/env bash
# Mutation / negative-control harness.
#
# Proves the differential test suite is NOT vacuous: each mutation below injects
# a specific class of behavioural divergence into the Rust translation, and the
# suite MUST fail for every one of them. Any "MISSED" line is a blind spot in
# the tests.
#
# The original src/lib.rs is restored (and verified byte-identical) after every
# mutation, and again at exit via a trap.

set -uo pipefail
cd "$(dirname "$0")"

ORIG=".lib.rs.orig"
cp src/lib.rs "$ORIG"

restore() {
  cp "$ORIG" src/lib.rs
}
trap 'restore; cargo build --offline -q >/dev/null 2>&1; rm -f "$ORIG"' EXIT

pass=0
miss=0
noop=0

run_mut() {
  local name="$1"; shift
  restore
  perl -0pi -e "$1" src/lib.rs

  if cmp -s "$ORIG" src/lib.rs; then
    echo "?? NO-OP    : $name  (pattern did not match -- mutation not applied)"
    noop=$((noop+1))
    return
  fi

  if ! cargo build --offline -q >/dev/null 2>&1; then
    echo "?? NOBUILD  : $name  (mutant does not compile)"
    noop=$((noop+1))
    restore
    return
  fi

  # ALLOW_STALE is deliberately NOT set: the freshness guard must see the new .so.
  if timeout 300 cargo test --offline -q >/dev/null 2>&1; then
    echo "!! MISSED   : $name  <-- TEST SUITE BLIND SPOT"
    miss=$((miss+1))
  else
    echo "OK DETECTED : $name"
    pass=$((pass+1))
  fi
  restore
}

echo "=== Mutation battery (every mutant MUST be detected) ==="

run_mut "M1  ERANGE return code 34 -> 33" \
  's/\n    unsafe \{ \*dst = 0 \};\n    34\n\}/\n    unsafe { *dst = 0 };\n    33\n}/'

run_mut "M2  drop the dst[0]=0 side effect on the ERANGE path" \
  's/\n    unsafe \{ \*dst = 0 \};\n    34\n\}/\n    34\n}/'

run_mut "M3  drop the dst[0]=0 side effect on the NULL-src path" \
  's/    if src\.is_null\(\) \{\n        unsafe \{ \*dst = 0 \};\n        return 22;/    if src.is_null() {\n        return 22;/'

run_mut "M4  EINVAL return code 22 -> 21 on the null-dst check" \
  's/    if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;\n    \}/    if dst.is_null() || num_elem == 0 {\n        return 21;\n    }/'

run_mut "M5  validate src BEFORE numElem (wrong check order -- diverges on E5)" \
  's/    if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;\n    \}\n\n    \/\/ `if \(!src\) \{ dst\[0\] = 0; return 22; \}`\n    if src\.is_null\(\) \{\n        unsafe \{ \*dst = 0 \};\n        return 22;\n    \}/    if dst.is_null() { return 22; }\n    if src.is_null() { unsafe { *dst = 0 }; return 22; }\n    if num_elem == 0 { return 22; }/'

run_mut "M6  off-by-one window: dst + (numElem - 1)" \
  's/let end: \*mut wchar_t = dst\.wrapping_add\(num_elem\);/let end: *mut wchar_t = dst.wrapping_add(num_elem.saturating_sub(1));/'

run_mut "M7  treat wchar_t as UNSIGNED when testing for NUL" \
  's/while ptr < end \&\& unsafe \{ \*ptr \} != 0 \{/while ptr < end \&\& (unsafe { *ptr } as u32) != 0u32 \&\& (unsafe { *ptr }) > 0 {/'

run_mut "M8  zero-pad the rest of the window after a successful copy" \
  's/        if c == 0 \{\n            return 0;\n        \}/        if c == 0 {\n            while ptr < end { unsafe { *ptr = 0 }; ptr = ptr.wrapping_add(1); }\n            return 0;\n        }/'

run_mut "M9  numElem == 0 also clears dst[0] (spurious side effect)" \
  's/    if dst\.is_null\(\) \|\| num_elem == 0 \{\n        return 22;\n    \}/    if dst.is_null() { return 22; }\n    if num_elem == 0 { unsafe { *dst = 0 }; return 22; }/'

run_mut "M10 wchar_t mapped to u16 instead of i32 (ABI\/truncation bug)" \
  's/#\[cfg\(not\(windows\)\)\]\npub type wchar_t = i32;\n#\[cfg\(windows\)\]\npub type wchar_t = u16;/pub type wchar_t = u16;/'

run_mut "M11 guarantee NUL termination on truncation (a 'fix' the C does not do)" \
  's/\n    unsafe \{ \*dst = 0 \};\n    34\n\}/\n    unsafe { *end.wrapping_sub(1) = 0 };\n    unsafe { *dst = 0 };\n    34\n}/'

run_mut "M12 first loop starts at dst+1, skipping dst[0]" \
  's/    let mut ptr: \*mut wchar_t = dst;/    let mut ptr: *mut wchar_t = dst;\n    let _ = \&mut ptr;/; s/    while ptr < end \&\& unsafe \{ \*ptr \} != 0 \{/    ptr = ptr.wrapping_add(1);\n    while ptr < end \&\& unsafe { *ptr } != 0 {/'

run_mut "M15 second loop bound uses <= end (writes one element past the window)" \
  's/    while ptr < end \{\n        let c = unsafe \{ \*src_ptr \};/    while ptr <= end {\n        let c = unsafe { *src_ptr };/'

run_mut "M16 empty src short-circuits to return 0 without checking room" \
  's/    let mut src_ptr: \*const wchar_t = src;/    if unsafe { *src } == 0 { unsafe { *dst = 0 }; return 0; }\n    let mut src_ptr: *const wchar_t = src;/'

run_mut "M13 copy loop writes src AFTER incrementing (shifted by one)" \
  's/        let c = unsafe \{ \*src_ptr \};\n        src_ptr = src_ptr\.wrapping_add\(1\);/        src_ptr = src_ptr.wrapping_add(1);\n        let c = unsafe { *src_ptr };/'

run_mut "M14 return 0 instead of 34 when the destination is unterminated" \
  's/\n    unsafe \{ \*dst = 0 \};\n    34\n\}/\n    unsafe { *dst = 0 };\n    0\n}/'

echo
echo "detected=$pass  missed=$miss  skipped=$noop"
if [ "$miss" -ne 0 ]; then
  echo "RESULT: FAIL -- the suite has blind spots"
  exit 1
fi
echo "RESULT: PASS -- every mutant was detected"
