use std::io::{self, Read};

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

#[cfg(not(test))]
#[export_name = "main"]
pub extern "C" fn c_main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().unwrap().parse().unwrap();
    driver(x);
    0
}
