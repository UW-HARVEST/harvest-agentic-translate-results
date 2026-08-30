use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

#[inline]
fn print_line(line: &'static [u8]) {
    // Each caller supplies a static, NUL-terminated C string.
    unsafe {
        puts(line.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    'outer: while x > 0 || y > 0 {
        print_line(b"loop\0");

        if x != 1 || y != 4 {
            if x > 0 {
                print_line(b"x\0");
                x -= 1;
            }
        }

        loop {
            if y == 0 {
                continue 'outer;
            }

            print_line(b"y\0");
            y = y.wrapping_sub(1);

            if x >= 3 {
                break;
            }

            if x > 0 {
                print_line(b"x\0");
                x -= 1;
            }
        }
    }
}
