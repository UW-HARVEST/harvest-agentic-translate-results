use std::ffi::{c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

fn print_line(message: &'static [u8]) {
    unsafe {
        puts(message.as_ptr().cast());
    }
}

fn fail(result: c_int) -> c_int {
    print_line(b"Operation failed\0");
    result
}

fn multi_stage(x: c_int, z: c_int) -> c_int {
    if x != 1 {
        print_line(b"Error: x != 1\0");
        return fail(1);
    }

    if Y.load(Ordering::Relaxed) != 2 {
        print_line(b"Error: x == 1 but y != 2\0");
        return fail(2);
    }

    if z != 3 {
        print_line(b"Error: x == 1 and y == 2, but z != 3\0");
        return fail(3);
    }

    print_line(b"Ok!\0");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int, z: c_int) {
    Y.store(y, Ordering::Relaxed);
    let result = multi_stage(x, z);

    unsafe {
        printf(c"Result: %d\n".as_ptr(), result);
    }
}
