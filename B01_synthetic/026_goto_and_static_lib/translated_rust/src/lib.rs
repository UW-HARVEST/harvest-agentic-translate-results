use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let result;

    loop {
        if x != 1 {
            print!("Error: x != 1\n");
            result = 1;
            break;
        }

        if Y.load(Ordering::SeqCst) != 2 {
            print!("Error: x == 1 but y != 2\n");
            result = 2;
            break;
        }

        if z != 3 {
            print!("Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break;
        }

        print!("Ok!\n");
        return 0;
    }

    // fail:
    print!("Operation failed\n");
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::SeqCst);
    let result = multi_stage(x, z);
    print!("Result: {}\n", result);
}
