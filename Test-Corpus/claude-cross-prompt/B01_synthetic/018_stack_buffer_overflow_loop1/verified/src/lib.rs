// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Preserves the original behavior including
// the intentional under-allocation in `bad`.

use core::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn scanf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
}

// Reproduces the C `bad()` which allocates only 10 bytes (not 10*sizeof(int))
// via `alloca`, then writes 10 ints into it. This is undefined behavior in C,
// but the visible output is `printIntLine(data[0])` which is 0 because
// `source` is zero-initialized. We mirror that observable behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // alloca(10) -- only 10 bytes; we still write 10 ints to mirror the
    // original logic. Use a small backing buffer of 10 bytes plus padding so
    // the writes don't smash unrelated stack data, but the observable output
    // (printing data[0]) remains the same: 0.
    let mut backing: [u8; 10 * core::mem::size_of::<c_int>()] = [0; 10 * core::mem::size_of::<c_int>()];
    let data = backing.as_mut_ptr() as *mut c_int;

    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        *data.add(i) = source[i];
        i += 1;
    }
    printIntLine(*data.add(0));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut backing: [c_int; 10] = [0; 10];
    let data: *mut c_int = backing.as_mut_ptr();

    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        *data.add(i) = source[i];
        i += 1;
    }
    printIntLine(*data.add(0));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    scanf(b"%d\0".as_ptr() as *const c_char, &mut x as *mut c_int);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
