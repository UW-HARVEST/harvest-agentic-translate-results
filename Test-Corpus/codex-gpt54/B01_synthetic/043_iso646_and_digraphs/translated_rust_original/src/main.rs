use std::ffi::c_int;

fn driver(x: c_int, y: c_int) {
    let result = x | !y;

    unsafe {
        libc::printf(c"%d".as_ptr(), result);
        libc::puts(c"".as_ptr());
    }
}

fn main() {
    let mut x: c_int = 0;
    let mut y: c_int = 0;

    unsafe {
        libc::scanf(c"%d".as_ptr(), &mut x);
        libc::scanf(c"%d".as_ptr(), &mut y);
    }

    driver(x, y);
}
