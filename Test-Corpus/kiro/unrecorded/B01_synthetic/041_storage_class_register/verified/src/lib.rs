use std::os::raw::c_int;

fn driver_impl(x: i32) {
    let y = x.wrapping_mul(2).wrapping_add(300);
    println!("{}", y);
}

#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    driver_impl(x);
}
