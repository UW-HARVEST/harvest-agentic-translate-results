use std::io::{self, Read};

fn driver(x: i32, y: i32) {
    let result = x | !y;
    print!("{}", result);
    println!();
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();
    let x: i32 = iter.next().unwrap().parse().unwrap();
    let y: i32 = iter.next().unwrap().parse().unwrap();
    driver(x, y);
}
