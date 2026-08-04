// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn print_int_ptr_line(int_number: *const c_int) {
    // Match the C implementation: printf("%d\n", *intNumber);
    unsafe {
        printf(b"%d\n\0".as_ptr(), *int_number);
    }
}

fn bad() {
    // Reproduce the C bug: declare an uninitialized pointer and pass it
    // to printIntPtrLine. This is undefined behavior, matching the C source.
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data_ptr: *const c_int = unsafe { data.assume_init() };
    print_int_ptr_line(data_ptr);
}

fn good() {
    let data: c_int = 5;
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
