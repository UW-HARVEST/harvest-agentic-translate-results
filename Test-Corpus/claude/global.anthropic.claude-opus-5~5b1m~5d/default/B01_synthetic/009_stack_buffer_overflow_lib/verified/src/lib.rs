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

/// Backing storage for the `int buffer[10]` local of `bad`.
///
/// The C code performs an unchecked `buffer[data] = 1` in `bad()`, which for
/// `data >= 10` writes past the end of the array and into the rest of the
/// function's stack frame.  Reproducing that faithfully for the indices where
/// the C behaviour is still *deterministic* requires somewhere for the stray
/// store to land, so the array is embedded in a struct that reserves trailing
/// stack space, mirroring how the C compiler's frame absorbs modest overflows.
/// Only the first `BUFFER_LEN` elements are ever printed, exactly as in C.
///
/// Measured against the C build (`objdump -d libdriver.so`, frame base
/// `-0x30(%rbp)`, `sub $0x40,%rsp`):
///
/// | `data` | C target slot                | C observable                     |
/// |--------|------------------------------|----------------------------------|
/// | 0..=9  | `buffer[data]`               | `1` printed on line `data + 1`   |
/// | 10     | frame padding `-0x8(%rbp)`   | ten zeros                        |
/// | 11     | loop counter `i` `-0x4(%rbp)`, then overwritten by `i = 0` | ten zeros |
/// | >= 12  | saved `%rbp` / return address | **undefined behaviour**         |
///
/// The Rust version reproduces rows 1-3 exactly.  For `data >= 12` the C
/// program has no defined behaviour at all, so nothing can be required of the
/// translation; the store is simply absorbed by the slack (and suppressed
/// entirely beyond the slack, so that the Rust side never itself commits
/// undefined behaviour).
#[repr(C)]
struct Frame {
    buffer: [c_int; BUFFER_LEN],
    /// Trailing slack that stands in for the remainder of the C stack frame.
    _slack: [c_int; 118],
}

/// Total number of `int`-sized slots in a `Frame` (`BUFFER_LEN` + slack).
const FRAME_SLOTS: usize = BUFFER_LEN + 118;

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
    // Take the base pointer from the *whole* `Frame`, not from the `buffer`
    // field, so that its provenance legitimately spans the slack as well; a
    // pointer derived from `frame.buffer` would only be valid for indices
    // 0..BUFFER_LEN and the stray store would be undefined behaviour on the
    // Rust side (and thus liable to be optimised away).
    let base: *mut c_int = (&raw mut frame) as *mut c_int;
    if data >= 0 {
        // buffer[data] = 1;  -- deliberately *not* upper-bounded, as in C.
        if (data as usize) < FRAME_SLOTS {
            base.add(data as usize).write(1);
        }
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(*base.add(i));
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

/// `static void goodG2B(void)` -- the fixed data source: `data` is always 7,
/// so the write is always in bounds and no frame slack is needed.
unsafe fn goodG2B() {
    let data: c_int = 7;
    let mut buffer: [c_int; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(buffer[i]);
        }
    } else {
        // Dead code in C too (`data` is the constant 7), kept for fidelity.
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

/// `static void goodB2G(int data)` -- the fixed sink: the index is fully
/// range-checked before use, so plain checked indexing matches C exactly.
unsafe fn goodB2G(data: c_int) {
    let mut buffer: [c_int; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 && data < (BUFFER_LEN as c_int) {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            printIntLine(buffer[i]);
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
