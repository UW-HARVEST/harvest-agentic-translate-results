use std::io::{self, Read};

pub fn driver(s1: &str, s2: &str) {
    let result = s1.chars().take_while(|c| !s2.contains(*c)).count();
    println!("{}", result);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut lines = input.lines();
    let s1 = lines.next().unwrap_or("");
    let s2 = lines.next().unwrap_or("");
    driver(s1, s2);
}
