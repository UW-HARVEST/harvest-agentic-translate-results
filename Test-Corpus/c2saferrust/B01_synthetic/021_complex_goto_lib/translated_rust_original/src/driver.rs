
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn driver(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");
        if x == 1 && y == 4 {
            while y != 0 {
                println!("y");
                y -= 1;
                if x < 3 {
                    if x > 0 {
                        println!("x");
                        x -= 1;
                    }
                }
            }
        } else {
            loop {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }

                if y == 0 {
                    break;
                }

                println!("y");
                y -= 1;

                if x >= 3 {
                    break;
                }
            }
        }
    }
}

