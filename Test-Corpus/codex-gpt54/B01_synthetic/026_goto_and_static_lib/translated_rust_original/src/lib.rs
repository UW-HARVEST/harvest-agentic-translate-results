use std::ffi::c_int;

static mut Y: c_int = 123;

unsafe extern "C" {
    fn printf(format: *const i8, ...) -> c_int;
}

const ERROR_X_NE_1: &[u8] = b"Error: x != 1\n\0";
const ERROR_Y_NE_2: &[u8] = b"Error: x == 1 but y != 2\n\0";
const ERROR_Z_NE_3: &[u8] = b"Error: x == 1 and y == 2, but z != 3\n\0";
const OK: &[u8] = b"Ok!\n\0";
const OPERATION_FAILED: &[u8] = b"Operation failed\n\0";
const RESULT: &[u8] = b"Result: %d\n\0";

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result = 0;

    if x != 1 {
        unsafe {
            printf(ERROR_X_NE_1.as_ptr().cast());
        }
        result = 1;
        unsafe {
            printf(OPERATION_FAILED.as_ptr().cast());
        }
        return result;
    }

    if unsafe { Y } != 2 {
        unsafe {
            printf(ERROR_Y_NE_2.as_ptr().cast());
        }
        result = 2;
        unsafe {
            printf(OPERATION_FAILED.as_ptr().cast());
        }
        return result;
    }

    if z != 3 {
        unsafe {
            printf(ERROR_Z_NE_3.as_ptr().cast());
        }
        result = 3;
        unsafe {
            printf(OPERATION_FAILED.as_ptr().cast());
        }
        return result;
    }

    unsafe {
        printf(OK.as_ptr().cast());
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
        printf(RESULT.as_ptr().cast(), result);
    }
}
