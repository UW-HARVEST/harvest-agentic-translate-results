use std::io::{self, Read};

fn foo(in_str: &str, c: char) -> usize {
    in_str.matches(c).count()
}

fn driver(in_str: &str) {
    println!("A: {}", foo(in_str, 'A'));
    println!("x: {}", foo(in_str, 'x'));
}

fn main() {
    let mut in_buf = String::new();
    io::stdin().read_to_string(&mut in_buf).unwrap();
    driver(&in_buf);
}
