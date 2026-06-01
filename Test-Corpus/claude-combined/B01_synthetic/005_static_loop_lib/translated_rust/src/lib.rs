// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from staticloop.c

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Module-level state for the static sum variable inside static_sum.
// Use AtomicI32 to provide safe, interior-mutable global state with the same
// semantics as a C function-local `static int sum = 0;` (initialized once,
// preserved across calls, and using non-atomic-per-thread but acceptable
// here since the C version is also not thread-safe).
static SUM: AtomicI32 = AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    // Mirror C semantics: sum += update; return sum;
    // Use Relaxed because the C code uses no memory ordering at all.
    let prev = SUM.load(Ordering::Relaxed);
    let new = prev.wrapping_add(update);
    SUM.store(new, Ordering::Relaxed);
    new
}

extern "C" {
    fn printf(format: *const libc::c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    // Maintain a running total using a static variable
    // for (int i = 0; i < 10; i++) { printf("%d\n", static_sum(i * stride)); }
    let fmt = b"%d\n\0".as_ptr() as *const libc::c_char;
    for i in 0i32..10 {
        let val = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(fmt, val);
        }
    }
}
