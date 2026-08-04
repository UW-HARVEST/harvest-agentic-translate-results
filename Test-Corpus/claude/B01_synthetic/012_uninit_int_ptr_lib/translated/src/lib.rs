// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/driver.c

use std::ffi::c_int;
use std::mem::MaybeUninit;

fn print_int_ptr_line(int_number: *const c_int) {
    // Mimic: printf("%d\n", *intNumber);
    unsafe {
        let value = *int_number;
        // Use println! formatting which matches printf("%d\n", value)
        // printf "%d" prints a signed int in decimal followed by newline.
        println!("{}", value);
    }
}

fn bad() {
    // Original C:
    //   int *data;          // uninitialized pointer
    //   printIntPtrLine(data);
    // This is undefined behavior (reading uninitialized pointer).
    // We reproduce the exact same UB pattern here.
    let data: *const c_int = unsafe { MaybeUninit::uninit().assume_init() };
    print_int_ptr_line(data);
}

fn good() {
    // Original C:
    //   int data;
    //   data = 5;
    //   int *data_addr;
    //   data_addr = &data;
    //   printIntPtrLine(data_addr);
    let data: c_int = 5;
    let data_addr: *const c_int = &data as *const c_int;
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
