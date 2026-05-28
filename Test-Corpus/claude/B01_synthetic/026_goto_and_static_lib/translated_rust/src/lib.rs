// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the original C behavior, including
// using libc::printf so output goes through the same C stdio buffers.

use std::ffi::c_int;

static mut Y: c_int = 123;

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    // Replicates C control flow with `goto fail;` using a labeled block.
    'fail: {
        if x != 1 {
            unsafe {
                libc::printf(c"Error: x != 1\n".as_ptr());
            }
            result = 1;
            break 'fail;
        }

        // SAFETY: Y mirrors the C file-scope `static int y`.
        let y_val = unsafe { Y };
        if y_val != 2 {
            unsafe {
                libc::printf(c"Error: x == 1 but y != 2\n".as_ptr());
            }
            result = 2;
            break 'fail;
        }

        if z != 3 {
            unsafe {
                libc::printf(c"Error: x == 1 and y == 2, but z != 3\n".as_ptr());
            }
            result = 3;
            break 'fail;
        }

        unsafe {
            libc::printf(c"Ok!\n".as_ptr());
        }
        return result;
    }

    unsafe {
        libc::printf(c"Operation failed\n".as_ptr());
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    // SAFETY: matches the C semantics of writing to file-scope `static int y`.
    unsafe {
        Y = local_y;
    }
    let result = multi_stage(x, z);
    unsafe {
        libc::printf(c"Result: %d\n".as_ptr(), result);
    }
}
