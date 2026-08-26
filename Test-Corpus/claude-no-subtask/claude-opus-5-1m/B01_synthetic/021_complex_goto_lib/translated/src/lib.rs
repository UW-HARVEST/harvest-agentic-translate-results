// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    // Format strings as C-style null-terminated literals.
    let fmt_loop = b"loop\n\0".as_ptr() as *const c_char;
    let fmt_x = b"x\n\0".as_ptr() as *const c_char;
    let fmt_y = b"y\n\0".as_ptr() as *const c_char;

    while x > 0 || y > 0 {
        unsafe {
            printf(fmt_loop);
        }

        // Track whether we are entering from the top of the iteration
        // (in which case we should evaluate the `if (x == 1 && y == 4)` check
        // that controls `goto label2`) or whether we re-entered via
        // `goto label1` (in which case that check must NOT re-execute).
        let mut from_top = true;

        'iter: loop {
            // Top-of-iteration goto check, equivalent to:
            //     if (x == 1 && y == 4) { goto label2; }
            // Only evaluated on the initial entry from the while-loop top.
            let skip_label1 = from_top && x == 1 && y == 4;
            from_top = false;

            if !skip_label1 {
                // label1:
                if x > 0 {
                    unsafe {
                        printf(fmt_x);
                    }
                    x -= 1;
                }
            }

            // label2:
            if y == 0 {
                // C `continue;` -- jump back to the while-loop condition.
                break 'iter;
            }
            unsafe {
                printf(fmt_y);
            }
            y -= 1;
            if x < 3 {
                // goto label1;
                continue 'iter;
            }
            // Fall through past the closing brace of the while body.
            break 'iter;
        }
    }
}
