fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
