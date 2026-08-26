#!/bin/sh
# Anti-vacuity check: prove the differential tests would actually catch a wrong
# translation.
#
# Each mutation deliberately breaks one behaviour of the Rust translation, then
# the test that is supposed to notice is run and MUST fail. A mutation that the
# suite still passes means that test is vacuous.
#
# The sources are restored after every mutation (and on interrupt).

set -eu
cd "$(dirname "$0")"

BACKUP="${TMPDIR:-/tmp}/container_of_src_backup.$$"
mkdir -p "$BACKUP"
cp src/container_of.rs src/lib.rs src/main.rs "$BACKUP/"

# Restoring the sources is not enough: the tests load target/<profile>/libdriver.so,
# which only `cargo build` refreshes, so a mutated .so would otherwise be left
# behind for the next `cargo test` to pick up. Always rebuild after restoring.
restore() {
  cp "$BACKUP/container_of.rs" src/container_of.rs
  cp "$BACKUP/lib.rs" src/lib.rs
  cp "$BACKUP/main.rs" src/main.rs
  cargo build --offline >/dev/null 2>&1 || true
}
trap 'restore; rm -rf "$BACKUP"' EXIT INT TERM

status=0

# mutate <file> <python-replacement-expr> ; uses python for exact string edits
mutate() {
  file="$1"; from="$2"; to="$3"
  python3 - "$file" "$from" "$to" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(f"mutation target not found in {path}: {frm!r}")
open(path, 'w').write(s.replace(frm, to, 1))
PY
}

check() {
  label="$1"; test_target="$2"; filter="$3"
  printf '\n---- mutation: %s ----\n' "$label"
  if ! cargo build --offline >/dev/null 2>&1; then
    echo "UNEXPECTED: the mutated source did not compile"
    status=1
    restore
    return
  fi
  if cargo test --offline --test "$test_target" "$filter" >/dev/null 2>&1; then
    echo "VACUOUS: $test_target/$filter still passes with '$label' broken"
    status=1
  else
    echo "CAUGHT: $test_target/$filter fails, as it must"
  fi
  restore
}

# 1. offsetof(struct test, b) — the whole point of container_of.
mutate src/container_of.rs \
  'pub const OFFSET_OF_B: usize = core::mem::offset_of!(Test, b);' \
  'pub const OFFSET_OF_B: usize = 0;'
check "offsetof(struct test, b) = 0" error_paths row17

# 2. strtol saturation on overflow (should clamp to LONG_MAX/LONG_MIN).
mutate src/container_of.rs \
  'None => saturated = true,' \
  'None => { acc = acc.wrapping_mul(10).wrapping_add(digit); },'
check "strtol overflow saturation" error_paths row09

# 3. the C-locale isspace set.
mutate src/container_of.rs \
  "matches!(b, b' ' | b'\\t' | b'\\n' | 0x0b | 0x0c | b'\\r')" \
  "matches!(b, b' ' | b'\\t' | b'\\n' | 0x0c | b'\\r')"
check "isspace includes \\v" error_paths row06

# 4. wrapping int addition.
mutate src/container_of.rs \
  'let sum = (*pa).a.wrapping_add((*pb).b);' \
  'let sum = (*pa).a.saturating_add((*pb).b);'
check "int addition wraps" error_paths row15

# 5. the printf format (trailing newline).
mutate src/container_of.rs \
  "    pos -= 1;
    buf[pos] = b'\\n';" \
  "    pos -= 1;
    buf[pos] = b' ';"
check "printf trailing newline" differential row08

# 6. strtol accepts a leading '+' as well as '-'.
mutate src/container_of.rs \
  "    let negative = if cur == b'+' || cur == b'-' {" \
  "    let negative = if cur == b'-' {"
check "strtol accepts a leading '+'" differential row10

# 7. the (int) truncation of the long result.
mutate src/container_of.rs \
  '    value as c_int' \
  '    value.clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int'
check "(int) truncation of strtol result" error_paths row11

# 8. argc must be ignored, not validated.
mutate src/container_of.rs \
  'pub unsafe fn c_main(_argc: c_int, argv: *mut *mut c_char) -> c_int {' \
  'pub unsafe fn c_main(_argc: c_int, argv: *mut *mut c_char) -> c_int {
    if _argc < 3 { return 1; }'
check "argc is never validated" error_paths row12

# 9. the null-pointer dereference must really happen.
mutate src/container_of.rs \
  '    let mut cur = core::ptr::read_volatile(p);' \
  '    if p.is_null() { return 0; }
    let mut cur = core::ptr::read_volatile(p);'
check "atoi(NULL) faults" error_paths row01

printf '\n==== mutation check: %s ====\n' "$([ $status -eq 0 ] && echo 'all mutations caught' || echo 'VACUOUS TESTS FOUND')"
exit $status
