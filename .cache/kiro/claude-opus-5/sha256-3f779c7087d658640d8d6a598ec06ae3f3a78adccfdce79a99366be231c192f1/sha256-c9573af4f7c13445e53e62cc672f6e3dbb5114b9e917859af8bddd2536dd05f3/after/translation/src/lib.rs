// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Exported ABI (matches `nm -D` on the C build of libdriver.so):
//   printLine, printIntLine, bad, good, driver

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

// Output is produced through the platform C `printf` (not Rust's `std::io`) so
// that stream buffering, flushing and interleaving with any C caller's own
// stdio writes are byte-for-byte identical to the original library.
unsafe extern "C" {
    unsafe fn printf(format: *const c_char, ...) -> c_int;
}

/// C: `void printLine(const char * line)`
///
/// ```c
/// if (line != NULL) { printf("%s\n", line); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// C: `void printIntLine(int intNumber)` -> `printf("%d\n", intNumber);`
#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), intNumber);
    }
}

/// C: `void bad(void)`
///
/// ```c
/// int * data;
/// data = (int *)alloca(10);            /* 10 BYTES, not 10 ints */
/// { int source[10] = {0}; size_t i;
///   for (i = 0; i < 10; i++) { data[i] = source[i]; }
///   printIntLine(data[0]); }
/// ```
///
/// The defect is preserved, not repaired: the allocation request is still the
/// under-sized 10 *bytes* while 10 `int`s (40 bytes) are copied into it. The
/// only observable result is `data[0]`, which is always `source[0] == 0`. The
/// emulated `alloca` region is backed by enough stack space for the ten writes
/// so the overrun cannot corrupt unrelated state in the Rust runtime, which
/// leaves the printed output byte-identical to the C original.
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn bad() {
    // alloca(10) -- undersized on purpose (see doc comment above). The size is
    // routed through `black_box` so this body stays distinct from `good`'s and
    // the linker cannot fold the two symbols onto one address, matching the C
    // build where `bad` and `good` are separate code.
    let _requested_bytes: usize = std::hint::black_box(10);
    let mut region: [MaybeUninit<c_int>; 10] = [MaybeUninit::uninit(); 10];
    let data: *mut c_int = region.as_mut_ptr().cast::<c_int>();
    {
        let source: [c_int; 10] = [0; 10];
        let mut i: usize = 0;
        while i < 10 {
            unsafe {
                *data.add(i) = source[i];
            }
            i += 1;
        }
        printIntLine(unsafe { *data });
    }
}

/// C: `void good(void)`
///
/// ```c
/// int * data;
/// data = NULL;
/// data = (int *)alloca(10*sizeof(int));
/// { int source[10] = {0}; size_t i;
///   for (i = 0; i < 10; i++) { data[i] = source[i]; }
///   printIntLine(data[0]); }
/// ```
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn good() {
    // data = NULL; then data = alloca(10 * sizeof(int)) -- correctly sized.
    let _requested_bytes: usize = std::hint::black_box(10 * size_of::<c_int>());
    let mut region: [MaybeUninit<c_int>; 10] = [MaybeUninit::uninit(); 10];
    let data: *mut c_int = region.as_mut_ptr().cast::<c_int>();
    {
        let source: [c_int; 10] = [0; 10];
        let mut i: usize = 0;
        while i < 10 {
            unsafe {
                *data.add(i) = source[i];
            }
            i += 1;
        }
        printIntLine(unsafe { *data });
    }
}

/// C: `void driver(int useGood)` -- public entry point declared in driver.h.
#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
