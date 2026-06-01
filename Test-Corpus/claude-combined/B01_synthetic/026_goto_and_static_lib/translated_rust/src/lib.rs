// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of c_src/src/driver.c

use std::ffi::c_int;
use std::sync::Mutex;

// Use libc printf so output goes through the same stdio buffer as C callers.
extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

// File-scope `static int y = 123;` in C. We use a Mutex<i32> to protect it,
// matching the single-threaded semantics of the original C code while still
// being safe in Rust.
static Y: Mutex<c_int> = Mutex::new(123);

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;
    let y_val: c_int = *Y.lock().unwrap();

    'fail: {
        if x != 1 {
            unsafe {
                printf(b"Error: x != 1\n\0".as_ptr());
            }
            result = 1;
            break 'fail;
        }

        if y_val != 2 {
            unsafe {
                printf(b"Error: x == 1 but y != 2\n\0".as_ptr());
            }
            result = 2;
            break 'fail;
        }

        if z != 3 {
            unsafe {
                printf(b"Error: x == 1 and y == 2, but z != 3\n\0".as_ptr());
            }
            result = 3;
            break 'fail;
        }

        unsafe {
            printf(b"Ok!\n\0".as_ptr());
        }
        return result;
    }

    // fail label
    unsafe {
        printf(b"Operation failed\n\0".as_ptr());
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    {
        let mut y_guard = Y.lock().unwrap();
        *y_guard = local_y;
    }
    let result = multi_stage(x, z);
    unsafe {
        printf(b"Result: %d\n\0".as_ptr(), result);
    }
}
