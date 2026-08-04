use std::ffi::c_int;
use std::os::raw::c_int as RawCInt;
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result = 0;
    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        return fail(result);
    }

    if Y.load(Ordering::SeqCst) != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
        return fail(result);
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        return fail(result);
    }

    println!("Ok!");
    result
}

fn fail(result: c_int) -> c_int {
    println!("Operation failed");
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: RawCInt, local_y: RawCInt, z: RawCInt) {
    Y.store(local_y, Ordering::SeqCst);
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}