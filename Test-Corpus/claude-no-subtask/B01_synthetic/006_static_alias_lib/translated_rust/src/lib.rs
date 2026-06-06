// Translation of MIT Lincoln Laboratory's StaticAlias C code into Rust.
// Reproduces byte-identical output for matching inputs.

#![allow(non_snake_case)]

use std::ffi::c_int;

// File-scope static that mirrors the C function-scope `static int inner = 1;`
// inside `static_alias`. Using `static mut` to preserve the same single-shared
// mutable storage semantics as the C original (non-thread-safe by design).
static mut INNER: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    // Mirrors:
    //   static int inner = 1;
    //   if (*outer >= inner) {
    //     inner += *outer;
    //     return &inner;
    //   } else {
    //     *outer += inner;
    //     return outer;
    //   }
    let inner_ptr: *mut c_int = &raw mut INNER;
    if *outer >= *inner_ptr {
        *inner_ptr += *outer;
        inner_ptr
    } else {
        *outer += *inner_ptr;
        outer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // Mirrors:
    //   int *running_sum = &initial_value;
    //   for (int i = 0; i < iterations; i++) {
    //     running_sum = static_alias(running_sum);
    //     printf("%d\n", *running_sum);
    //   }
    // Note: `initial_value` is a by-value parameter (its own stack slot), so
    // taking its address and letting `static_alias` mutate through that pointer
    // matches the C semantics exactly.
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value;
    let mut i: c_int = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        println!("{}", *running_sum);
        i += 1;
    }
}
