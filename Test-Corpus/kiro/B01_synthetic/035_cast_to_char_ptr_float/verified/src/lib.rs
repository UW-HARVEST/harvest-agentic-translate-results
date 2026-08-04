use std::io::{self, Read};

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: f32 = input.trim().split_whitespace().next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    driver(x);
    0
}
