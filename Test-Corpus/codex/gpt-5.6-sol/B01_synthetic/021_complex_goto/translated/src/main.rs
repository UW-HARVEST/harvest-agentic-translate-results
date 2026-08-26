use std::ffi::{c_char, c_int};
use std::io::{self, Write};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn foo(mut x: c_int, mut y: c_int, output: &mut impl Write) {
    while x > 0 || y > 0 {
        let _ = output.write_all(b"loop\n");

        let mut at_label2 = x == 1 && y == 4;
        loop {
            if !at_label2 {
                if x > 0 {
                    let _ = output.write_all(b"x\n");
                    x -= 1;
                }
            }

            at_label2 = false;
            if y == 0 {
                break;
            }

            let _ = output.write_all(b"y\n");
            y -= 1;
            if x >= 3 {
                break;
            }
        }
    }
}

fn main() {
    let mut x: c_int = 0;
    let mut y: c_int = 0;
    const FORMAT: &[u8] = b"%d %d\0";

    // The source ignores scanf's return value and retains each initialized value
    // when its corresponding conversion does not complete.
    let _ = unsafe { scanf(FORMAT.as_ptr().cast(), &mut x, &mut y) };

    let stdout = io::stdout();
    let mut output = stdout.lock();
    foo(x, y, &mut output);
}
