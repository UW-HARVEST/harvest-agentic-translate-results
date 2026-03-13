use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let bytes = x.to_ne_bytes();
    for b in &bytes {
        print!("{:02x}", b);
    }
    println!();
}
