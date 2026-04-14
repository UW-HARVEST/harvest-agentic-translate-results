use std::ffi::c_int;

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{b:02x}");
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
