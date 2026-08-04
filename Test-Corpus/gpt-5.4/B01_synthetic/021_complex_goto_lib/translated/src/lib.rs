use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut x: c_int, mut y: c_int) {
    while x > 0 || y > 0 {
        println!("loop");

        let mut at_label2 = false;
        if x == 1 && y == 4 {
            at_label2 = true;
        }

        loop {
            if !at_label2 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }

            if y == 0 {
                break;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                at_label2 = false;
                continue;
            }
            break;
        }
    }
}