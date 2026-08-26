use std::ffi::c_int;
use std::io::{self, Write};

static mut Y: c_int = 123;

fn print_str(s: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    'fail: {
        if x != 1 {
            print_str("Error: x != 1\n");
            result = 1;
            break 'fail;
        }

        // SAFETY: single-threaded use as in original C code
        let y_val = unsafe { Y };
        if y_val != 2 {
            print_str("Error: x == 1 but y != 2\n");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            print_str("Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break 'fail;
        }

        print_str("Ok!\n");
        return result;
    }

    print_str("Operation failed\n");
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    // SAFETY: single-threaded use as in original C code
    unsafe {
        Y = local_y;
    }
    let result = multi_stage(x, z);
    print_str(&format!("Result: {}\n", result));
}
