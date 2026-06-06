use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

/// Equivalent of C `printf("%s\n", line);` — writes the C string followed by a
/// newline to stdout, then flushes (so behavior under pipes/redirection
/// matches a typical libc stdout that is line-buffered when attached to a
/// terminal and block-buffered otherwise; for byte-identical comparison we
/// flush eagerly).
fn print_c_string_with_newline(line: *const c_char) {
    // Determine length up to NUL (excluding NUL).
    let mut len: usize = 0;
    // Safety: caller passes a valid NUL-terminated C string, matching the C
    // contract for printf("%s", ...).
    unsafe {
        while *line.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(line as *const u8, len);
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(slice);
        let _ = handle.write_all(b"\n");
        let _ = handle.flush();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        print_c_string_with_newline(line);
    }
}

// Static byte buffer mimicking the C local array `char charString[] =
// "helperBad string";`.  The original C returns a pointer to a stack-allocated
// array, which is undefined behavior; in practice GCC-compiled output prints
// "helperBad string\n", so to obtain byte-identical output we back the buffer
// with static storage.  The function's externally visible behavior (the
// returned pointer dereferences to the same bytes) matches the C in all
// observed test cases.
static HELPER_BAD_STRING: [c_char; 17] = [
    b'h' as c_char,
    b'e' as c_char,
    b'l' as c_char,
    b'p' as c_char,
    b'e' as c_char,
    b'r' as c_char,
    b'B' as c_char,
    b'a' as c_char,
    b'd' as c_char,
    b' ' as c_char,
    b's' as c_char,
    b't' as c_char,
    b'r' as c_char,
    b'i' as c_char,
    b'n' as c_char,
    b'g' as c_char,
    0,
];

fn helper_bad() -> *mut c_char {
    HELPER_BAD_STRING.as_ptr() as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

static HELPER_GOOD1_STRING: [c_char; 19] = [
    b'h' as c_char,
    b'e' as c_char,
    b'l' as c_char,
    b'p' as c_char,
    b'e' as c_char,
    b'r' as c_char,
    b'G' as c_char,
    b'o' as c_char,
    b'o' as c_char,
    b'd' as c_char,
    b'1' as c_char,
    b' ' as c_char,
    b's' as c_char,
    b't' as c_char,
    b'r' as c_char,
    b'i' as c_char,
    b'n' as c_char,
    b'g' as c_char,
    0,
];

fn helper_good1() -> *mut c_char {
    HELPER_GOOD1_STRING.as_ptr() as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
