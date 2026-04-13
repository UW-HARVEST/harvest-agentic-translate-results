use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;
    
    while x > 0 || y > 0 {
        println!("loop");
        
        if x == 1 && y == 4 {
            if y == 0 {
                continue;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            continue;
        }
        
        'label1: {
            if x > 0 {
                println!("x");
                x -= 1;
            }
        }
        
        if y == 0 {
            continue;
        }
        println!("y");
        y -= 1;
        if x < 3 {
            if x > 0 {
                println!("x");
                x -= 1;
            }
        }
    }
}