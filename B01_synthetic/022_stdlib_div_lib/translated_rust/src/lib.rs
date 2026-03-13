use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {quot}, remainder: {rem}");
}
