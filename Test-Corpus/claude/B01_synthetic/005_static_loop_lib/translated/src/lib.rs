// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Mirror the C `static int sum = 0;` inside `static_sum`.
// The C original is not thread-safe; using an atomic here preserves the
// observable behavior in single-threaded use while remaining safe Rust.
static SUM: AtomicI32 = AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    // sum += update; return sum;
    let prev = SUM.fetch_add(update, Ordering::SeqCst);
    prev.wrapping_add(update)
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    // for (int i = 0; i < 10; i++) {
    //   printf("%d\n", static_sum(i * stride));
    // }
    let fmt = b"%d\n\0".as_ptr() as *const std::ffi::c_char;
    for i in 0..10i32 {
        let val = static_sum(i.wrapping_mul(stride));
        unsafe {
            libc::printf(fmt, val);
        }
    }
}
