use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

static mut Y: i32 = 123;

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;
    loop {
        if x != 1 {
            unsafe { printf(b"Error: x != 1\n\0".as_ptr()); }
            result = 1;
            break;
        }
        if unsafe { Y } != 2 {
            unsafe { printf(b"Error: x == 1 but y != 2\n\0".as_ptr()); }
            result = 2;
            break;
        }
        if z != 3 {
            unsafe { printf(b"Error: x == 1 and y == 2, but z != 3\n\0".as_ptr()); }
            result = 3;
            break;
        }
        unsafe { printf(b"Ok!\n\0".as_ptr()); }
        return result;
    }
    // fail:
    unsafe { printf(b"Operation failed\n\0".as_ptr()); }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    unsafe { Y = local_y; }
    let result = multi_stage(x, z);
    unsafe { printf(b"Result: %d\n\0".as_ptr(), result); }
}
