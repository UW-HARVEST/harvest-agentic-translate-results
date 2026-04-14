use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let quotient = x / y;
    let remainder = x % y;
    println!("quotient: {}, remainder: {}", quotient, remainder);
}