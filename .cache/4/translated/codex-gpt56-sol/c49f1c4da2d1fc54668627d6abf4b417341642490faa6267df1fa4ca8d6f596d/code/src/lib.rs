use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

#[inline]
fn write_line(line: &'static [u8]) {
    unsafe {
        puts(line.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    const LOOP: &[u8] = b"loop\0";
    const X: &[u8] = b"x\0";
    const Y: &[u8] = b"y\0";

    'outer: while x > 0 || y > 0 {
        write_line(LOOP);

        let mut run_label1 = !(x == 1 && y == 4);
        loop {
            if run_label1 && x > 0 {
                write_line(X);
                x = x.wrapping_sub(1);
            }

            if y == 0 {
                continue 'outer;
            }

            write_line(Y);
            y = y.wrapping_sub(1);
            if x < 3 {
                run_label1 = true;
                continue;
            }
            break;
        }
    }
}
