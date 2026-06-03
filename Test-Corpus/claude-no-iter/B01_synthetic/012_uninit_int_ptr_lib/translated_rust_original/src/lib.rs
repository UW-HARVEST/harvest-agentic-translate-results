// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to preserve byte-identical output.

#![allow(unused_assignments)]
#![allow(invalid_value)]

use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn print_int_ptr_line(int_number: *const c_int) {
    // Match the C printf("%d\n", *intNumber) exactly.
    unsafe {
        printf(b"%d\n\0".as_ptr(), *int_number);
    }
}

fn bad() {
    // Reproduce the original C bug: an uninitialized pointer is dereferenced.
    // The original C declares `int *data;` without initializing it, then
    // passes that uninitialized pointer to printIntPtrLine which dereferences
    // it. This is undefined behavior in C; we replicate the same UB here.
    let data: *const c_int = unsafe { MaybeUninit::<*const c_int>::uninit().assume_init() };
    print_int_ptr_line(data);
}

fn good() {
    let mut data: c_int = 0;
    data = 5;
    let data_addr: *const c_int = &data;
    print_int_ptr_line(data_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
