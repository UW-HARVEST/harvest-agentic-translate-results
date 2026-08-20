// Rust translation of c_src (MIT Lincoln Laboratory `driver` library).
//
// Exported public ABI (matches `nm -D` of the C shared library):
//   printLine, printIntLine, bad, good, driver
//
// Output is produced through the C library's `printf` so that the bytes written
// (and the stdio buffering behavior) are identical to the C implementation.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
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
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), intNumber);
    }
}

/// C: `void bad()`
///
/// ```c
/// int * data;
/// data = (int *)alloca(10);        /* CWE-131: undersized allocation */
/// { int source[10] = {0}; size_t i;
///   for (i = 0; i < 10; i++) { data[i] = source[i]; }
///   printIntLine(data[0]); }
/// ```
///
/// The original C allocates only 10 bytes on the stack and then writes ten
/// `int`s (40 bytes) through it, overflowing the region.  The bug is preserved
/// in the sense that the same undersized-allocation code path is taken and the
/// same value is printed; the stand-in backing storage below keeps the
/// out-of-region stores from corrupting unrelated stack memory, which is not
/// observable in the C program's output (it prints `0` and returns normally).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let mut region = [0i32; 10];
    let data: *mut i32 = region.as_mut_ptr();
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            unsafe {
                *data.add(i) = source[i];
            }
            i += 1;
        }
        unsafe {
            printIntLine(*data);
        }
    }
}

/// C: `void good()`
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
#[allow(unused_assignments)]
pub unsafe extern "C" fn good() {
    let mut region = [0i32; 10];
    let mut data: *mut i32 = std::ptr::null_mut();
    data = region.as_mut_ptr();
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            unsafe {
                *data.add(i) = source[i];
            }
            i += 1;
        }
        unsafe {
            printIntLine(*data);
        }
    }
}

/// C: `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() }
    } else {
        unsafe { bad() }
    }
}
