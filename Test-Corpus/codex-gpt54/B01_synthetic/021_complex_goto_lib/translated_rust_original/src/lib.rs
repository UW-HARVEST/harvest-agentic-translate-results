use std::ffi::c_int;

const LOOP_MSG: &[u8] = b"loop\n\0";
const X_MSG: &[u8] = b"x\n\0";
const Y_MSG: &[u8] = b"y\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;

    'outer: while x > 0 || y > 0 {
        unsafe {
            libc::printf(LOOP_MSG.as_ptr().cast());
        }

        if x == 1 && y == 4 {
            if y == 0 {
                continue;
            }

            unsafe {
                libc::printf(Y_MSG.as_ptr().cast());
            }
            y -= 1;

            while x < 3 {
                if x > 0 {
                    unsafe {
                        libc::printf(X_MSG.as_ptr().cast());
                    }
                    x -= 1;
                }

                if y == 0 {
                    continue 'outer;
                }

                unsafe {
                    libc::printf(Y_MSG.as_ptr().cast());
                }
                y -= 1;
            }

            continue;
        }

        loop {
            if x > 0 {
                unsafe {
                    libc::printf(X_MSG.as_ptr().cast());
                }
                x -= 1;
            }

            if y == 0 {
                continue 'outer;
            }

            unsafe {
                libc::printf(Y_MSG.as_ptr().cast());
            }
            y -= 1;

            if x >= 3 {
                break;
            }
        }
    }
}
