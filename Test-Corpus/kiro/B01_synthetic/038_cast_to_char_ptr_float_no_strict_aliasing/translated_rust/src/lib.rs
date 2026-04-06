use std::io::{self, Read};

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let mut x: f32 = 0.0;
    if let Some(token) = input.split_whitespace().next() {
        if let Ok(v) = token.parse::<f32>() {
            x = v;
        }
    }
    driver(x);
    0
}
