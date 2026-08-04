use std::ffi::c_float;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let bytes = x.to_ne_bytes();
    for byte in &bytes {
        print!("{:02x}", byte);
    }
    println!();
}