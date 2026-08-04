fn foo_impl(mut x: i32, mut y: i32) {
    'outer: loop {
        if !(x > 0 || y > 0) {
            break;
        }
        println!("loop");

        let mut skip_label1 = x == 1 && y == 4;

        loop {
            // label1
            if !skip_label1 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2
            if y == 0 {
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // goto label1
                continue;
            }
            break;
        }
    }
}

#[no_mangle]
pub extern "C" fn foo(x: i32, y: i32) {
    foo_impl(x, y);
}
