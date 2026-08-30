// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The original C library is a CWE-121/CWE-787 style "stack based buffer
// overflow" demonstration.  Its public ABI consists of five symbols:
//
//     printLine, printIntLine, bad, good, driver
//
// The two helpers `goodG2B` and `goodB2G` are `static` in the C source and are
// therefore *not* exported; they are private here as well.
//
// All output goes through the C runtime's `printf` so that stream buffering,
// flushing behaviour, and interleaving with any output produced by a C caller
// are byte-for-byte identical to the original library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Number of elements in the fixed-size buffers used below (the C source
/// hard-codes `10`).
const BUFFER_LEN: usize = 10;

/// `void printLine(const char * line)`
///
/// Prints `line` followed by a newline, but only if the pointer is non-NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(b"%d\n\0".as_ptr() as *const c_char, intNumber);
}

/// Backing storage for the `int buffer[10]` locals of `bad`, `goodG2B` and
/// `goodB2G`.
///
/// The C code performs an unchecked `buffer[data] = 1` in `bad()`, which for
/// `data >= 10` writes past the end of the array and into the rest of the
/// function's stack frame.  A plain `[c_int; 10]` here would either panic
/// (safe indexing) or immediately smash Rust's own frame in ways unrelated to
/// the C layout, so the array is embedded in a struct that reserves trailing
/// stack space, mirroring how the C compiler's frame absorbs modest
/// overflows.  Only the first `BUFFER_LEN` elements are ever printed, exactly
/// as in C.
#[repr(C)]
struct Frame {
    buffer: [c_int; BUFFER_LEN],
    /// Trailing slack that stands in for the remainder of the C stack frame.
    _slack: [c_int; 118],
}

impl Frame {
    fn new() -> Self {
        // `int buffer[10] = { 0 };`
        Frame {
            buffer: [0; BUFFER_LEN],
            _slack: [0; 118],
        }
    }
}

/// `void bad(int data)`
///
/// Reproduced verbatim, including the missing upper-bound check: a `data`
/// value of 10 or more writes outside `buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut frame = Frame::new();
    if data >= 0 {
        // buffer[data] = 1;  -- deliberately unchecked, as in the C original.
        let base: *mut c_int = frame.buffer.as_mut_ptr();
        base.offset(data as isize).write(1);
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(*base.add(i));
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

/// `static void goodG2B(void)` -- the fixed data source: `data` is always 7,
/// so the write is in bounds.
unsafe fn goodG2B() {
    let data: c_int = 7;
    let mut frame = Frame::new();
    if data >= 0 {
        let base: *mut c_int = frame.buffer.as_mut_ptr();
        base.offset(data as isize).write(1);
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(*base.add(i));
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

/// `static void goodB2G(int data)` -- the fixed sink: the index is fully
/// range-checked before use.
unsafe fn goodB2G(data: c_int) {
    let mut frame = Frame::new();
    if data >= 0 && data < (BUFFER_LEN as c_int) {
        let base: *mut c_int = frame.buffer.as_mut_ptr();
        base.offset(data as isize).write(1);
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(*base.add(i));
        }
    } else {
        printLine(b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char);
    }
}

/// `void good(int data)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    goodG2B();
    goodB2G(data);
}

/// `void driver(int goodData, int badData)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_int, badData: c_int) {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good(goodData);
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad(badData);
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
