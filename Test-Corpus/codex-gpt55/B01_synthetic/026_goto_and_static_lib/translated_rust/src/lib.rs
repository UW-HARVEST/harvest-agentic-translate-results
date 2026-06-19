use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static mut Y: c_int = 123;

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    if x != 1 {
        unsafe {
            printf(c"Error: x != 1\n".as_ptr());
        }
        result = 1;
    } else if unsafe { Y } != 2 {
        unsafe {
            printf(c"Error: x == 1 but y != 2\n".as_ptr());
        }
        result = 2;
    } else if z != 3 {
        unsafe {
            printf(c"Error: x == 1 and y == 2, but z != 3\n".as_ptr());
        }
        result = 3;
    } else {
        unsafe {
            printf(c"Ok!\n".as_ptr());
        }
        return result;
    }

    unsafe {
        printf(c"Operation failed\n".as_ptr());
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    unsafe {
        Y = local_y;
    }

    let result = multi_stage(x, z);
    unsafe {
        printf(c"Result: %d\n".as_ptr(), result);
    }
}
