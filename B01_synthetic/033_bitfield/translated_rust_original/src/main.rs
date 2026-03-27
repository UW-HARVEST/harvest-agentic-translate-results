use std::io::{self, Read};

fn scan_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

fn print_foo(x: u32, y: u32, b: i32, z: i32) {
    println!("{} {} {} {}", x & 0x3, y & 0x7, (b != 0) as i32, z);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let tokens: Vec<&str> = scan_tokens(&input);

    let x: u32 = tokens[0].parse().unwrap();
    let y: u32 = tokens[1].parse().unwrap();
    let b: i32 = tokens[2].parse().unwrap();
    let z: i32 = tokens[3].parse().unwrap();

    print_foo(x, y, b, z);
}
