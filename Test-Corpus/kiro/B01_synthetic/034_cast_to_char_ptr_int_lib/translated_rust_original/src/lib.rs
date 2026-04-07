use std::ffi::c_int;

fn print_hex(p: &[u8]) {
    for &b in p {
        print!("{:02x}", b);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    print_hex(&x.to_ne_bytes());
}
