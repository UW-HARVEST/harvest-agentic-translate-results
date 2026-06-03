// Translation of c_src/src/driver.c to Rust.
// Preserves the exact control flow (including the C `goto` labels) and
// produces byte-identical output by routing the prints through C's
// `printf`, so stdout buffering matches the original C library.

use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    // C-string literals that exactly match the original `printf` format
    // strings (including the trailing newline and null terminator).
    let loop_str = b"loop\n\0".as_ptr() as *const c_char;
    let x_str = b"x\n\0".as_ptr() as *const c_char;
    let y_str = b"y\n\0".as_ptr() as *const c_char;

    'outer: while x > 0 || y > 0 {
        unsafe { printf(loop_str); }

        // `goto label2` from the original C — skip the `label1` block on
        // the first pass through the inner loop only.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    unsafe { printf(x_str); }
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            unsafe { printf(y_str); }
            y -= 1;
            if x < 3 {
                // `goto label1` in the original C — loop back to label1
                // within the same outer iteration.
                continue;
            }
            break;
        }
    }
}
