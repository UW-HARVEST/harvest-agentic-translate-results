use std::os::raw::c_float;

fn print_hex(p: &[u8]) {
    for &b in p {
        print!("{:02x}", b);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
