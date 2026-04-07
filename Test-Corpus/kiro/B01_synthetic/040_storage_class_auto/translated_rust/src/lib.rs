use std::ffi::c_int;

fn driver_impl(x: c_int) {
    let mut y: c_int = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    println!("{}", y);
}

#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    driver_impl(x);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut input = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input);
    let x: c_int = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
    driver_impl(x);
    0
}
