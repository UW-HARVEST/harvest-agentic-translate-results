use std::io::{self, Read, Write};

fn driver(x: i32, y: i32) {
    let result = x | !y;
    print!("{}", result);
    println!();
}

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read input");

    let mut iter = input.split_ascii_whitespace();
    let x: i32 = iter
        .next()
        .expect("missing x")
        .parse()
        .expect("invalid x");
    let y: i32 = iter
        .next()
        .expect("missing y")
        .parse()
        .expect("invalid y");

    driver(x, y);
    io::stdout().flush().ok();
}
