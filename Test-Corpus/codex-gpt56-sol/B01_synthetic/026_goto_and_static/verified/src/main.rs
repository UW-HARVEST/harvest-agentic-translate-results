use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn multi_stage(x: c_int, y: c_int, z: c_int) -> c_int {
    if x != 1 {
        print!("Error: x != 1\n");
        print!("Operation failed\n");
        return 1;
    }

    if y != 2 {
        print!("Error: x == 1 but y != 2\n");
        print!("Operation failed\n");
        return 2;
    }

    if z != 3 {
        print!("Error: x == 1 and y == 2, but z != 3\n");
        print!("Operation failed\n");
        return 3;
    }

    print!("Ok!\n");
    0
}

fn main() {
    let mut x: c_int = 0;
    let mut y: c_int = 123;
    let mut z: c_int = 0;

    unsafe {
        scanf(
            b"%d %d %d\0".as_ptr().cast(),
            &mut x as *mut c_int,
            &mut y as *mut c_int,
            &mut z as *mut c_int,
        );
    }

    let result = multi_stage(x, y, z);
    print!("Result: {result}\n");
}
