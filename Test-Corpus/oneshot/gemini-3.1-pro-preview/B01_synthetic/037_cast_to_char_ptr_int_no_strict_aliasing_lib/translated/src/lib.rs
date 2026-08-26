use std::os::raw::c_int;

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}