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

// Only export `main` when building the cdylib, not during tests
#[cfg(not(test))]
#[export_name = "main"]
pub extern "C" fn driver_main() -> i32 {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).unwrap_or(0);
    let x: f32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    driver(x);
    0
}
