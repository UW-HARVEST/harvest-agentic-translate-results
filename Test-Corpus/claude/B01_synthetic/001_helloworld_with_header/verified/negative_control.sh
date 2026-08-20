#!/usr/bin/env bash
# Harness self-check ("does this suite have teeth?").
#
# Builds deliberately WRONG translations into $TMPDIR, points the suite at each
# via DRIVER_RUST_SO, and requires the suite to FAIL on every one of them. Also
# re-runs the suite against the real library and requires it to PASS.
#
# Exits 0 when every mutant was caught, non-zero otherwise.
set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}/driver-negctl.$$"
mkdir -p "$TMP" || exit 1
trap 'rm -rf "$TMP"' EXIT
CAUGHT=0
MISSED=0

mutant() { # name, sillymain-body
  local name="$1" body="$2"
  mkdir -p "$TMP/$name"
  cat > "$TMP/$name/sillymain.rs" <<EOF
$body
EOF
  cat > "$TMP/$name/lib.rs" <<'EOF'
pub mod sillymain;
#[no_mangle] pub extern "C" fn helloworld() -> std::os::raw::c_int { sillymain::helloworld() }
#[no_mangle] pub extern "C" fn main() -> std::os::raw::c_int { sillymain::helloworld() }
EOF
  if ! rustc --edition 2021 --crate-type cdylib --crate-name driver \
        "$TMP/$name/lib.rs" -o "$TMP/$name/libdriver.so" 2>"$TMP/$name/build.log"; then
    echo "   ?? mutant '$name' failed to build:"; tail -5 "$TMP/$name/build.log"; MISSED=$((MISSED+1)); return
  fi
  if DRIVER_RUST_SO="$TMP/$name/libdriver.so" cargo --offline test \
       -- --test-threads=1 >"$TMP/$name/test.log" 2>&1; then
    echo "   MISSED  mutant '$name' was NOT detected by the suite"
    MISSED=$((MISSED + 1))
  else
    echo "   caught  mutant '$name'  ($(grep -c 'FAILED$' "$TMP/$name/test.log") test(s) failed:" \
         "$(grep '^test .*FAILED$' "$TMP/$name/test.log" | sed 's/^test //;s/ \.\.\..*//' | paste -sd' ')"")"
    CAUGHT=$((CAUGHT + 1))
  fi
}

echo "-- mutants (each must be rejected) --"

# 1. The original translation: correct bytes, but written through Rust's own
#    line-buffered stdout instead of C's FILE *stdout -> different flush timing.
mutant std_stdout '
use std::io::Write;
pub fn helloworld() -> i32 {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"Hello World!\n");
    let _ = out.flush();
    0
}'

# 2. Raw write(2) to fd 1: bypasses the stream entirely.
mutant raw_write '
pub fn helloworld() -> i32 {
    extern "C" { fn write(fd: i32, buf: *const u8, n: usize) -> isize; }
    unsafe { write(1, b"Hello World!\n".as_ptr(), 13); }
    0
}'

# 3. Wrong text (missing "!").
mutant wrong_text '
pub fn helloworld() -> i32 {
    extern "C" { fn printf(f: *const std::os::raw::c_char, ...) -> i32; }
    unsafe { printf(b"Hello World\n\0".as_ptr() as *const _); }
    0
}'

# 4. Missing trailing newline.
mutant no_newline '
pub fn helloworld() -> i32 {
    extern "C" { fn printf(f: *const std::os::raw::c_char, ...) -> i32; }
    unsafe { printf(b"Hello World!\0".as_ptr() as *const _); }
    0
}'

# 5. Propagates printf's result instead of discarding it (returns non-zero on a
#    write failure, which C never does).
mutant propagates_error '
pub fn helloworld() -> i32 {
    extern "C" { fn printf(f: *const std::os::raw::c_char, ...) -> i32; }
    let r = unsafe { printf(b"Hello World!\n\0".as_ptr() as *const _) };
    if r < 0 { -1 } else { 0 }
}'

# 6. Prints twice.
mutant printed_twice '
pub fn helloworld() -> i32 {
    extern "C" { fn printf(f: *const std::os::raw::c_char, ...) -> i32; }
    unsafe { printf(b"Hello World!\n\0".as_ptr() as *const _); }
    unsafe { printf(b"Hello World!\n\0".as_ptr() as *const _); }
    0
}'

# 7. Flushes eagerly (right bytes, wrong buffering behaviour).
mutant eager_flush '
pub fn helloworld() -> i32 {
    extern "C" {
        fn printf(f: *const std::os::raw::c_char, ...) -> i32;
        fn fflush(s: *mut std::os::raw::c_void) -> i32;
    }
    unsafe { printf(b"Hello World!\n\0".as_ptr() as *const _); fflush(std::ptr::null_mut()); }
    0
}'

echo "-- control (the real library must still pass) --"
if cargo --offline test -- --test-threads=1 >"$TMP/real.log" 2>&1; then
  echo "   ok      the real translation passes"
else
  echo "   BROKEN  the real translation FAILS:"
  grep -E "^test .*FAILED|assertion|test result" "$TMP/real.log" | head -20
  MISSED=$((MISSED + 1))
fi

echo "-- result: $CAUGHT mutant(s) caught, $MISSED problem(s) --"
[ "$MISSED" = 0 ]
