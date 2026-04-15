use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result = 0;

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
    } else if Y.load(Ordering::SeqCst) as c_int != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
    } else {
        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y as i32, Ordering::SeqCst);
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
