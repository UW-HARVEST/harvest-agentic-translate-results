use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static mut Y: c_int = 123;

fn print_line(message: &'static [u8]) {
    unsafe {
        printf(message.as_ptr().cast());
    }
}

fn multi_stage(x: c_int, z: c_int) -> c_int {
    if x != 1 {
        print_line(b"Error: x != 1\n\0");
        print_line(b"Operation failed\n\0");
        return 1;
    }

    if unsafe { Y } != 2 {
        print_line(b"Error: x == 1 but y != 2\n\0");
        print_line(b"Operation failed\n\0");
        return 2;
    }

    if z != 3 {
        print_line(b"Error: x == 1 and y == 2, but z != 3\n\0");
        print_line(b"Operation failed\n\0");
        return 3;
    }

    print_line(b"Ok!\n\0");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    unsafe {
        Y = local_y;
    }

    let result = multi_stage(x, z);
    unsafe {
        printf(b"Result: %d\n\0".as_ptr().cast(), result);
    }
}
