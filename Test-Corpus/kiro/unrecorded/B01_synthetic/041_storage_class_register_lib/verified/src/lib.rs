use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let y = (2 as c_int).wrapping_mul(x).wrapping_add(300);
    println!("{}", y);
}
