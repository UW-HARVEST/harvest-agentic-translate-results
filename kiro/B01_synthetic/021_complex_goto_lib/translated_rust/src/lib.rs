use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        print!("loop\n");

        let mut skip_label1 = x == 1 && y == 4;

        loop {
            // label1:
            if !skip_label1 {
                if x > 0 {
                    print!("x\n");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                break; // continue outer while
            }
            print!("y\n");
            y -= 1;
            if x < 3 {
                continue; // goto label1
            }
            break;
        }
    }
}
