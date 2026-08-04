use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        println!("loop");

        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            if y == 0 {
                break;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                continue;
            } else {
                break;
            }
        }
    }
}
