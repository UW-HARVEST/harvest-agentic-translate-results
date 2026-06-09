use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn scanf(fmt: *const u8, ...) -> c_int;
}

static mut Y: c_int = 123;

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;
    unsafe {
        if x != 1 {
            printf(b"Error: x != 1\n\0".as_ptr());
            result = 1;
            // goto fail
            printf(b"Operation failed\n\0".as_ptr());
            return result;
        }

        if Y != 2 {
            printf(b"Error: x == 1 but y != 2\n\0".as_ptr());
            result = 2;
            // goto fail
            printf(b"Operation failed\n\0".as_ptr());
            return result;
        }

        if z != 3 {
            printf(b"Error: x == 1 and y == 2, but z != 3\n\0".as_ptr());
            result = 3;
            // goto fail
            printf(b"Operation failed\n\0".as_ptr());
            return result;
        }

        printf(b"Ok!\n\0".as_ptr());
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let mut z: c_int = 0;
    unsafe {
        scanf(
            b"%d %d %d\0".as_ptr(),
            &mut x as *mut c_int,
            &raw mut Y,
            &mut z as *mut c_int,
        );
        let result = multi_stage(x, z);
        printf(b"Result: %d\n\0".as_ptr(), result);
    }
    0
}
