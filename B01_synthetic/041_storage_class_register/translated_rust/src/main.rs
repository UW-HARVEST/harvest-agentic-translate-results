use std::io::{self, Read};

fn driver(x: i32) {
    let y = 2 * x + 300;
    println!("{}", y);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().unwrap().parse().unwrap();
    driver(x);
}
