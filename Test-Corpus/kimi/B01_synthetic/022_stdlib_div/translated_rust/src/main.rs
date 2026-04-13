use std::io;

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let mut parts = input.split_whitespace();
    let x: i32 = parts.next().unwrap().parse().unwrap();
    let y: i32 = parts.next().unwrap().parse().unwrap();
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
}