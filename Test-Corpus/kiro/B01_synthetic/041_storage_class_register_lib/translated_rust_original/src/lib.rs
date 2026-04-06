use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let y: c_int = 2i32.wrapping_mul(x).wrapping_add(300);
    println!("{}", y);
}
