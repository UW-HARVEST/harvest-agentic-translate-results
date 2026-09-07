// Rust translation of c_src/src/driver.c (CWE-190 style integer-overflow driver).
//
// Public ABI reproduced exactly (as exported by the C shared library):
//   printLine, printHexCharLine, bad, good, driver
// `goodG2B` and `goodB2G` are `static` in the C source and therefore remain
// private here.
//
// Output is produced through the C library's own `printf` so that the byte
// stream and the stdio buffering behaviour are identical to the C build.

#![allow(non_snake_case, unused_assignments)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `CHAR_MAX` from <limits.h> for the platform's (signed) `char`.
const CHAR_MAX: c_char = c_char::MAX;

/// void printLine(const char * line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// void printHexCharLine(char charHex)
///
/// The C code passes a `char` through varargs, where it is promoted to `int`
/// and then formatted with `%02x` (i.e. reinterpreted as `unsigned int`), so a
/// negative value prints as eight hex digits.
///
/// ABI note: gcc compiles the `char` parameter as a *narrowing* read of the
/// argument register — `mov %edi,%eax; mov %al,-0x4(%rbp); movsbl -0x4(%rbp),%eax`
/// — so a caller that leaves garbage in the upper 24 bits of `edi` (e.g. one
/// using a mismatched `void(int)` prototype) still only observes the low byte.
/// Declaring the parameter as `c_char` here is NOT equivalent: rustc/LLVM tags
/// it `signext i8`, assumes the caller already extended it, and forwards the
/// whole register (`mov %edi,%esi`), which prints `100` where C prints `00`.
/// The parameter is therefore taken as `c_int` (identical register on the
/// SysV x86-64 ABI, which is what the C library is built for) and truncated
/// explicitly, reproducing gcc's observable behaviour for every input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printHexCharLine(charHex: c_int) {
    print_hex_char_line(charHex as c_char);
}

/// Internal, correctly-typed body of `printHexCharLine`, used by the callers
/// inside this translation unit (where the argument really is a `char`).
#[inline]
unsafe fn print_hex_char_line(charHex: c_char) {
    printf(b"%02x\n\0".as_ptr() as *const c_char, charHex as c_int);
}

/// void bad()
///
/// Deliberately overflows: `CHAR_MAX * 2` truncated back into a `char`.
/// Reproduced as-is; not "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: c_char;
    data = CHAR_MAX;
    if data > 0 {
        let result: c_char = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

/// static void goodG2B()
unsafe fn goodG2B() {
    let data: c_char;
    data = 2;
    if data > 0 {
        let result: c_char = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

/// static void goodB2G()
unsafe fn goodB2G() {
    let mut data: c_char;
    data = b' ' as c_char;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: c_char = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            printLine(
                b"data value is too large to perform arithmetic safely.\0".as_ptr()
                    as *const c_char,
            );
        }
    }
}

/// void good()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}

/// void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
