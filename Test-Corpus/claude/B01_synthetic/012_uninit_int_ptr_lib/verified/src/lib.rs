// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/driver.c

use std::ffi::c_int;
use std::mem::MaybeUninit;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(int_number: *const c_int) {
    // Mimic: printf("%d\n", *intNumber);
    let value = unsafe { *int_number };
    let fmt = b"%d\n\0".as_ptr() as *const libc::c_char;
    unsafe {
        libc::printf(fmt, value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // Original C:
    //   int *data;          // uninitialized pointer
    //   printIntPtrLine(data);
    // This is undefined behavior (reading uninitialized pointer).
    // We reproduce the exact same UB pattern here.
    let data: *const c_int = unsafe { MaybeUninit::uninit().assume_init() };
    unsafe { printIntPtrLine(data) };
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    // Original C:
    //   int data;
    //   data = 5;
    //   int *data_addr;
    //   data_addr = &data;
    //   printIntPtrLine(data_addr);
    let data: c_int = 5;
    let data_addr: *const c_int = &data as *const c_int;
    unsafe { printIntPtrLine(data_addr) };
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
