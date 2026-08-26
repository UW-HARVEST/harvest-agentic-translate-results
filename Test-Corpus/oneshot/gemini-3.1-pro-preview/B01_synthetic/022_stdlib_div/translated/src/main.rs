use std::io::{self, Read};

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        let mut tokens = input.split_whitespace();
        if let Some(token_x) = tokens.next() {
            if let Ok(parsed_x) = token_x.parse() {
                x = parsed_x;
                if let Some(token_y) = tokens.next() {
                    if let Ok(parsed_y) = token_y.parse() {
                        y = parsed_y;
                    }
                }
            }
        }
    }

    let quot = x / y;
    let rem = x % y;

    println!("quotient: {}, remainder: {}", quot, rem);
}