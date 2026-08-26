use std::io::{self, BufRead};

fn driver(s1: &str, s2: &str) {
    let result = s1.chars().position(|c| s2.contains(c)).unwrap_or(s1.len());
    println!("{}", result);
}

fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    let s1 = lines.next().unwrap().unwrap();
    let s2 = lines.next().unwrap().unwrap();
    
    driver(&s1, &s2);
}
