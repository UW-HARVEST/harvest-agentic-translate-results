// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust. Reproduces byte-identical output with the original C library.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Function-local `static int inner = 1;` from the C source. Stored at module
// scope here to preserve the single-instance, persistent semantics.
static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    if *outer >= INNER {
        INNER += *outer;
        &raw mut INNER
    } else {
        *outer += INNER;
        outer
    }
}

/*
  Maintain a sum leveraging multiple references to a static variable
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // The C code takes the address of the parameter `initial_value`. We
    // replicate this by binding the parameter to a mutable local that lives
    // for the duration of the function so its address is stable.
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &raw mut initial_value;
    let mut i: c_int = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        printf(b"%d\n\0".as_ptr() as *const c_char, *running_sum);
        i += 1;
    }
}
