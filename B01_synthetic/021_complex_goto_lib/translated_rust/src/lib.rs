use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        print!("loop\n");

        if x == 1 && y == 4 {
            // goto label2
            if y == 0 {
                continue;
            }
            print!("y\n");
            y -= 1;
            if x < 3 {
                // goto label1
                loop {
                    // label1
                    if x > 0 {
                        print!("x\n");
                        x -= 1;
                    }
                    // label2
                    if y == 0 {
                        break; // continue outer while
                    }
                    print!("y\n");
                    y -= 1;
                    if x >= 3 {
                        break; // fall out of while body, re-check condition
                    }
                    // x < 3: goto label1 again
                }
            }
            continue;
        }

        // label1 through end, with goto label1 loop
        loop {
            // label1
            if x > 0 {
                print!("x\n");
                x -= 1;
            }
            // label2
            if y == 0 {
                break; // continue outer while
            }
            print!("y\n");
            y -= 1;
            if x >= 3 {
                break; // fall out of while body, re-check condition
            }
            // x < 3: goto label1 again
        }
    }
}
