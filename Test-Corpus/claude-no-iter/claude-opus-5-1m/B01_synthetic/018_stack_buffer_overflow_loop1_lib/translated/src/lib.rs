// Translation of c_src/src/driver.c
//
// The C library exposes a single public function `driver(int useGood)`
// (declared in include/driver.h). It dispatches to either `good()` or
// `bad()`, both of which print the value of `data[0]` (which is 0).
//
// The C `bad()` function calls `alloca(10)` (10 BYTES) and then writes
// 10 ints into that allocation -- this is a CWE-121-style stack buffer
// overflow. The original (buggy) behavior is preserved here: we
// allocate a 10-byte stack buffer, treat it as `*mut c_int`, and write
// 10 ints through it. This is undefined behavior, exactly mirroring the
// C source. Observably, the printed output is the same as `good()`:
// the integer 0 followed by a newline.

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

/// Print a C string followed by a newline, if non-null.
///
/// Mirrors the C `printLine` function defined in driver.c.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Walk the C string until we hit a NUL byte, then write the
        // bytes followed by a newline. Using stdout().write_all matches
        // the C `printf("%s\n", line)` byte-for-byte.
        let mut len = 0usize;
        while unsafe { *line.add(len) } != 0 {
            len += 1;
        }
        let bytes = unsafe { std::slice::from_raw_parts(line as *const u8, len) };
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(bytes);
        let _ = handle.write_all(b"\n");
        let _ = handle.flush();
    }
}

/// Print an integer followed by a newline.
///
/// Mirrors the C `printIntLine` function defined in driver.c.
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    let s = format!("{}\n", int_number);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

/// CWE-121 bad: allocates only 10 BYTES (not 10 ints) and then writes
/// 10 ints through it. We faithfully reproduce that buffer overflow.
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // 10 bytes of "stack" storage -- matches `alloca(10)` in C.
    let mut buf: [u8; 10] = [0; 10];
    let data: *mut c_int = buf.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    // The loop writes 10 ints (40 bytes) into a 10-byte buffer:
    // this is a deliberate out-of-bounds write that mirrors the C bug.
    for i in 0..10usize {
        unsafe {
            *data.add(i) = source[i];
        }
    }
    let first = unsafe { *data };
    printIntLine(first);
}

/// The "good" version: properly allocates 10*sizeof(int) bytes.
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let mut storage: [c_int; 10] = [0; 10];
    let data: *mut c_int = storage.as_mut_ptr();
    let source: [c_int; 10] = [0; 10];
    for i in 0..10usize {
        unsafe {
            *data.add(i) = source[i];
        }
    }
    let first = unsafe { *data };
    printIntLine(first);
}

/// Public entry point declared in driver.h.
#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
