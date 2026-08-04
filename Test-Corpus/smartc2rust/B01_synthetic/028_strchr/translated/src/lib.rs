
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn foo(input: &[u8], c: u8) -> i32 {
    input.iter()
        .take_while(|&&b| b != 0)
        .filter(|&&b| b == c)
        .count() as i32
}

fn driver(input: &[u8]) {
    println!("A: {}", foo(input, b'A'));
    println!("x: {}", foo(input, b'x'));
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    use std::io::Read;
    let mut input = [0u8; 1000];
    let mut filled = 0usize;
    let mut stdin = std::io::stdin();
    while filled < input.len() {
        match stdin.read(&mut input[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => break,
        }
    }
    driver(&input);
    0
}