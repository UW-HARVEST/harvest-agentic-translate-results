use std::io::{self, Read};

fn foo(input: &str, c: char) -> i32 {
    input.chars().filter(|&ch| ch == c).count() as i32
}

fn driver(input: &str) {
    println!("A: {}", foo(input, 'A'));
    println!("x: {}", foo(input, 'x'));
}

fn main() {
    let mut input = String::new();
    let mut stdin = io::stdin();
    let _ = stdin.take(1000).read_to_string(&mut input);
    driver(&input);
}
