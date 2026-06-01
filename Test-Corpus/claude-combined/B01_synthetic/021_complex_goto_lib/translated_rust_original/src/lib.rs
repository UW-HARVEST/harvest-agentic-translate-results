// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving exact behavior.

use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    let loop_str = b"loop\n\0".as_ptr() as *const libc::c_char;
    let x_str = b"x\n\0".as_ptr() as *const libc::c_char;
    let y_str = b"y\n\0".as_ptr() as *const libc::c_char;

    'outer: while x > 0 || y > 0 {
        unsafe {
            libc::printf(loop_str);
        }

        // If x == 1 && y == 4, skip label1 (the x>0 block) and jump to label2
        if !(x == 1 && y == 4) {
            // label1:
            if x > 0 {
                unsafe {
                    libc::printf(x_str);
                }
                x -= 1;
            }
        }

        // label2 / inner-loop start:
        loop {
            if y == 0 {
                continue 'outer;
            }
            unsafe {
                libc::printf(y_str);
            }
            y -= 1;
            if x < 3 {
                // goto label1
                if x > 0 {
                    unsafe {
                        libc::printf(x_str);
                    }
                    x -= 1;
                }
                // falls through to label2
                continue;
            }
            break;
        }
    }
}
