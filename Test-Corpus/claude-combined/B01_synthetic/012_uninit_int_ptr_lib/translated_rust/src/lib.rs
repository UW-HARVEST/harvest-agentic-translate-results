// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

// Format string "%d\n\0"
const FMT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printIntPtrLine(int_number: *const c_int) {
    unsafe {
        // Dereference the pointer, exactly as the C does: printf("%d\n", *intNumber);
        let value = *int_number;
        printf(FMT.as_ptr(), value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // Reproduce the C undefined behavior: uninitialized int*, then dereference.
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data_ptr: *const c_int = unsafe { data.assume_init() };
    printIntPtrLine(data_ptr);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data: c_int = 5;
    let data_addr: *const c_int = &data;
    printIntPtrLine(data_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
