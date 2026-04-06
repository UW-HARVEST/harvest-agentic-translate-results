use std::io::Read;

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: i32) {
    print_hex(&x.to_ne_bytes());
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
    driver(x);
    0
}
