use std::ptr::addr_of_mut;

use libc::c_int;

static mut Y: c_int = 123;

fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    if x != 1 {
        print!("Error: x != 1\n");
        result = 1;
        print!("Operation failed\n");
        return result;
    }

    if unsafe { Y } != 2 {
        print!("Error: x == 1 but y != 2\n");
        result = 2;
        print!("Operation failed\n");
        return result;
    }

    if z != 3 {
        print!("Error: x == 1 and y == 2, but z != 3\n");
        result = 3;
        print!("Operation failed\n");
        return result;
    }

    print!("Ok!\n");
    result
}

fn main() {
    let mut x: c_int = 0;
    let mut z: c_int = 0;

    unsafe {
        libc::scanf(
            b"%d %d %d\0".as_ptr().cast(),
            &mut x,
            addr_of_mut!(Y),
            &mut z,
        );
    }

    let result = multi_stage(x, z);
    print!("Result: {}\n", result);
}
