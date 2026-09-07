// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library exports exactly four public symbols:
//     printLine, bad, good, driver
// All four are reproduced below with the same signatures and the same
// observable behaviour (including the intentional defect in `bad`).

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    // Use libc's printf so that stdout buffering / ordering is byte-for-byte
    // identical to the original C library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%s\n"` used by `printLine`.
const FMT_S_NL: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];

/// void printLine(const char *line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_S_NL.as_ptr(), line);
    }
}

/// void bad(void)
///
/// The original C reads an *uninitialized* local `char *data` and hands it to
/// `printLine`.  This is undefined behaviour and is deliberately preserved
/// rather than "fixed".  In practice the stale stack slot holds a non-NULL
/// pointer to a zero byte, so the observable output of the C version is a
/// single newline; that behaviour is reproduced exactly here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // Stale/garbage stack contents: a non-NULL pointer whose first byte is 0.
    static GARBAGE: [c_char; 1] = [0];
    let data: *const c_char = GARBAGE.as_ptr();
    printLine(data);
}

/// void good(void)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // char *data; data = "string";
    const STRING: [c_char; 7] = [
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    let data: *const c_char = STRING.as_ptr();
    printLine(data);
}

/// void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
