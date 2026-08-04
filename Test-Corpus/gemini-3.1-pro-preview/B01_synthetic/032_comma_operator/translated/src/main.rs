use std::io::{self, Read};

fn driver(x: i32) {
    let mut j = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(x) = token.parse::<i32>() {
                driver(x);
            }
        }
    }
}