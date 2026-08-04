use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const LOOP: &[u8] = b"loop\n\0";
const X: &[u8] = b"x\n\0";
const Y: &[u8] = b"y\n\0";

unsafe fn c_printf(message: &'static [u8]) {
    unsafe {
        printf(message.as_ptr().cast::<c_char>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        unsafe {
            c_printf(LOOP);
        }

        let mut at_label2 = x == 1 && y == 4;

        loop {
            if !at_label2 && x > 0 {
                unsafe {
                    c_printf(X);
                }
                x -= 1;
            }

            at_label2 = false;

            if y == 0 {
                break;
            }

            unsafe {
                c_printf(Y);
            }
            y -= 1;

            if x < 3 {
                continue;
            }

            break;
        }
    }
}
