use std::ffi::c_char;

fn driver(x: i32) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    println!("{y}");
}

fn main() {
    let mut x: i32 = 0;

    unsafe {
        libc::scanf(c"%d".as_ptr() as *const c_char, &mut x);
    }

    driver(x);
}
