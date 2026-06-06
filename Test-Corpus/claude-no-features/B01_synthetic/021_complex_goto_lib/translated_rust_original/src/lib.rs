use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        unsafe {
            printf(b"loop\n\0".as_ptr());
        }

        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    unsafe {
                        printf(b"x\n\0".as_ptr());
                    }
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                break;
            }
            unsafe {
                printf(b"y\n\0".as_ptr());
            }
            y -= 1;
            if x < 3 {
                continue; // goto label1
            }
            break; // fall through to next outer iteration
        }
    }
}
