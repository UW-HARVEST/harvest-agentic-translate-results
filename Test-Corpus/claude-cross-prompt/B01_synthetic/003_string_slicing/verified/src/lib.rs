// Translated from c_src/src/main.c
// Preserves the exact behavior (and bugs) of the original C code.

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

/// Index into a passed string and print the substring indexed by [start, stop).
/// If there is no start, use 0. If there is no stop, use the end of the string.
#[unsafe(no_mangle)]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    // Helper closures for IO. We write directly to stdout for byte-identical output.
    fn write_stdout(bytes: &[u8]) {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(bytes);
        let _ = handle.flush();
    }

    if (argc > 4) || (argc == 1) {
        write_stdout(b"Error: there should be one to three arguments passed:\n");
        write_stdout(b"<string> [start] [stop]\n");
        return 1;
    }

    // argv[1] -- the string
    // SAFETY: argc >= 2 here per the check above (argc != 1 and argc <= 4)
    let argv_slice = unsafe { std::slice::from_raw_parts(argv, argc as usize) };

    let arg1_ptr = argv_slice[1];
    // strlen on argv[1]
    let len: usize = unsafe { libc_strlen(arg1_ptr) };

    // In C, `start` and `stop` are declared `int` (uninitialized).
    let start: c_int;
    let stop: c_int;

    // Mirrors C's `char *end;` declaration. Tracks whether we successfully set it.
    // In C, `end` is left uninitialized if argc < 3, but the bug at line 63 references
    // it later only when argc == 4 (which implies argc >= 3, so end was set).
    let mut end_matched_arg2: bool = false;
    // Save pointer comparison result for the bug-preservation case.
    // We track the literal pointer end == argv[3] check (which is buggy, since
    // `end` is set from strtol on argv[2], not argv[3]).
    let mut end_ptr_after_arg2: *const c_char = std::ptr::null();

    if argc >= 3 {
        let arg2_ptr = argv_slice[2];
        let (val, end_ptr) = c_strtol(arg2_ptr, 10);
        end_ptr_after_arg2 = end_ptr;
        if end_ptr == arg2_ptr {
            // Note: C's printf has no trailing newline here; preserve that.
            write_stdout(b"Second argument must be an integer!");
            return 1;
        }
        end_matched_arg2 = true;
        start = val as c_int;
        // C compares signed `int start` with `size_t len`. With usual arithmetic
        // conversions, `start` is converted to size_t. Negative `start` becomes
        // a very large unsigned and compares > len. Preserve that behavior.
        if (start as i64) < 0 || (start as u64) > (len as u64) {
            write_stdout(b"Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let arg3_ptr = argv_slice[3];
        let (val, _end_ptr) = c_strtol(arg3_ptr, 10);
        stop = val as c_int;
        // BUG PRESERVATION: C code checks `if (end == argv[3])`, but `end` was
        // last assigned from the strtol on argv[2]. This compares the end-of-parse
        // pointer from argv[2] against argv[3], which will essentially never match.
        // Reproduce that exact (buggy) check.
        let _ = end_matched_arg2;
        if end_ptr_after_arg2 == arg3_ptr {
            write_stdout(b"Third argument must be an integer!");
            return 1;
        }

        if (stop as i64) < 0 || (stop as u64) > (len as u64) {
            write_stdout(b"Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            write_stdout(b"Error: stop must come after start!\n");
            return 1;
        }
    } else {
        stop = len as c_int;
    }

    // printf("%.*s\n", stop - start, argv[1] + start);
    let count = (stop - start) as usize;
    // SAFETY: We've validated start <= len and stop <= len, so the slice [start, stop)
    // is within the C string's bytes (excluding the null terminator).
    let bytes = unsafe {
        std::slice::from_raw_parts((arg1_ptr as *const u8).add(start as usize), count)
    };
    {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(bytes);
        let _ = handle.write_all(b"\n");
        let _ = handle.flush();
    }

    0
}

/// Compute strlen on a NUL-terminated C string.
unsafe fn libc_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    let mut p = s;
    while unsafe { *p } != 0 {
        n += 1;
        p = unsafe { p.add(1) };
    }
    n
}

/// Mimic C's strtol: parse a (possibly signed) integer in the given base from a
/// NUL-terminated C string. Returns (value, end_ptr).
/// Skips leading whitespace, then optional +/-, then digits in the given base.
/// Stops at the first invalid character, returning a pointer to it.
/// If no digits are consumed, end_ptr == input pointer.
fn c_strtol(s: *const c_char, base: u32) -> (i64, *const c_char) {
    let mut p = s;
    // Skip leading whitespace (matches C isspace for ASCII ws).
    unsafe {
        while is_c_space(*p) {
            p = p.add(1);
        }
    }
    let start_of_number = p;
    let mut negative = false;
    unsafe {
        let c = *p;
        if c == b'+' as c_char {
            p = p.add(1);
        } else if c == b'-' as c_char {
            negative = true;
            p = p.add(1);
        }
    }

    let digits_start = p;
    let mut acc: i64 = 0;
    let mut any = false;
    unsafe {
        loop {
            let c = *p as u8;
            if c == 0 {
                break;
            }
            let digit = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'z' => c - b'a' + 10,
                b'A'..=b'Z' => c - b'A' + 10,
                _ => break,
            };
            if (digit as u32) >= base {
                break;
            }
            acc = acc.wrapping_mul(base as i64).wrapping_add(digit as i64);
            any = true;
            p = p.add(1);
        }
    }

    if !any {
        // No digits parsed. C's strtol sets endptr to the original input.
        return (0, start_of_number_or_input(s, start_of_number, digits_start, false));
    }

    let val = if negative { acc.wrapping_neg() } else { acc };
    (val, p)
}

fn start_of_number_or_input(
    input: *const c_char,
    _after_ws: *const c_char,
    _digits_start: *const c_char,
    _had_sign: bool,
) -> *const c_char {
    // C's strtol: if no conversion is performed, endptr is set to nptr (input).
    input
}

fn is_c_space(c: c_char) -> bool {
    matches!(c as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}
