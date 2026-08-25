use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

static mut Y: c_int = 123;

unsafe fn multi_stage(x: c_int, z: c_int) -> c_int {
    if x != 1 {
        printf(b"Error: x != 1\n\0".as_ptr().cast());
        printf(b"Operation failed\n\0".as_ptr().cast());
        return 1;
    }

    if Y != 2 {
        printf(b"Error: x == 1 but y != 2\n\0".as_ptr().cast());
        printf(b"Operation failed\n\0".as_ptr().cast());
        return 2;
    }

    if z != 3 {
        printf(b"Error: x == 1 and y == 2, but z != 3\n\0".as_ptr().cast());
        printf(b"Operation failed\n\0".as_ptr().cast());
        return 3;
    }

    printf(b"Ok!\n\0".as_ptr().cast());
    0
}

#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let mut z: c_int = 0;

    scanf(
        b"%d %d %d\0".as_ptr().cast(),
        &mut x as *mut c_int,
        std::ptr::addr_of_mut!(Y),
        &mut z as *mut c_int,
    );

    let result = multi_stage(x, z);
    printf(b"Result: %d\n\0".as_ptr().cast(), result);
    0
}
