use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y = 2 * x;
    y += 300;
    println!("{}", y);
}