// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of staticalias.c

#![allow(non_snake_case)]

use std::ffi::c_int;

// Function-local static `inner` from C, hoisted to a module-level static mut
// to preserve the "shared across calls" semantics.
static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    unsafe {
        if *outer >= INNER {
            INNER += *outer;
            &raw mut INNER
        } else {
            *outer += INNER;
            outer
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // Mirror the C: `int *running_sum = &initial_value;`
    // `initial_value` is a parameter on the stack — take a pointer to it.
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value;
    let mut i: c_int = 0;
    while i < iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            // Use libc::printf to match the C output byte-for-byte (including buffering).
            libc::printf(b"%d\n\0".as_ptr() as *const _, *running_sum);
        }
        i += 1;
    }
}
