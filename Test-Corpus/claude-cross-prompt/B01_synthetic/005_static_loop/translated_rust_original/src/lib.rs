// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Use the libc crate for printf and strtol so output is byte-identical
// to the original C program (writes via libc's stdout buffering).
extern crate libc;

// `sum` is the function-local `static int sum = 0;` from the original C code.
// Translated to a module-level atomic for thread-safety; the original C
// program is single-threaded, so this matches semantics exactly.
static STATIC_SUM_TOTAL: AtomicI32 = AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    // Wrapping add to mirror C's signed overflow behavior on `int`.
    let prev = STATIC_SUM_TOTAL.load(Ordering::Relaxed);
    let new = prev.wrapping_add(update);
    STATIC_SUM_TOTAL.store(new, Ordering::Relaxed);
    new
}

/// C-style entrypoint: int main(int argc, char **argv)
///
/// Reproduces the original program's behavior, including all error
/// messages and ordering.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 2 {
        let msg = b"Error: should only be a single (integer) argument!\n\0";
        unsafe {
            libc::printf(msg.as_ptr() as *const c_char);
        }
        return 1;
    }

    // Replicate: char *end; int stride = strtol(argv[1], &end, 10);
    let arg1: *mut c_char = unsafe { *argv.add(1) };
    let mut end: *mut c_char = std::ptr::null_mut();
    let stride_long: libc::c_long =
        unsafe { libc::strtol(arg1, &mut end as *mut *mut c_char, 10) };
    // Original C narrows the long return value to int via assignment.
    let stride: c_int = stride_long as c_int;

    if end == arg1 {
        let msg = b"Error: first argument must be an integer!\n\0";
        unsafe {
            libc::printf(msg.as_ptr() as *const c_char);
        }
        return 1;
    }

    let fmt = b"%d\n\0";
    for i in 0..10i32 {
        // Match C semantics: i * stride is computed as `int` multiplication.
        let arg = i.wrapping_mul(stride);
        let total = static_sum(arg);
        unsafe {
            libc::printf(fmt.as_ptr() as *const c_char, total as c_int);
        }
    }

    0
}
