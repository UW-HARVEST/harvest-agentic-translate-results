use std::ffi::c_float;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}

fn print_hex(p: &[u8]) {
    for byte in p {
        print!("{:02x}", byte);
    }
    println!();
}