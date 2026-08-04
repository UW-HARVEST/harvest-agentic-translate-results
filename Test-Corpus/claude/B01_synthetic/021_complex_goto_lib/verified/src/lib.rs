// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;

unsafe extern "C" {
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
}

fn print_line(s: &[u8]) {
    // Write directly to fd 1 (stdout) to avoid Rust's stdio buffering,
    // which may not flush when called from a C harness.
    let mut remaining = s.len();
    let mut ptr = s.as_ptr();
    while remaining > 0 {
        let n = unsafe { write(1, ptr, remaining) };
        if n <= 0 {
            break;
        }
        let n = n as usize;
        remaining -= n;
        unsafe { ptr = ptr.add(n) };
    }
}

fn driver_impl(mut x: c_int, mut y: c_int) {
    'outer: while x > 0 || y > 0 {
        print_line(b"loop\n");

        // The C code uses `if (x == 1 && y == 4) goto label2;` to skip
        // the label1 block on the FIRST pass of this outer iteration.
        // Subsequent `goto label1` from `x < 3` must NOT re-trigger this skip.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    print_line(b"x\n");
                    x -= 1;
                }
            }
            // After the first iteration, any goto label1 must hit label1.
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            print_line(b"y\n");
            y -= 1;
            if x < 3 {
                // goto label1 -- continue inner loop without printing "loop"
                continue;
            }
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    driver_impl(x, y);
}
