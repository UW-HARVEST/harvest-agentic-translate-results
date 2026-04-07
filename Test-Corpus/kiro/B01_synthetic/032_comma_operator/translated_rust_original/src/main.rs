use std::io::{self, Read};

fn driver(x: i32) {
    let mut j = 0i32;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    driver(x);
}
