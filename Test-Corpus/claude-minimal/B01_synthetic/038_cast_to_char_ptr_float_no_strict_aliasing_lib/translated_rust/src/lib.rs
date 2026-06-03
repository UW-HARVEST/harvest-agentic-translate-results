// Translation of c_src/src/driver.c to Rust.

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: f32) {
    let raw: [u8; std::mem::size_of::<f32>()] = x.to_ne_bytes();
    print_hex(&raw);
}
