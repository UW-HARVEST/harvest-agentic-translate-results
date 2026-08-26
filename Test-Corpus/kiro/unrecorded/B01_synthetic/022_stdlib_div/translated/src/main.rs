use std::io::Read;

fn main() {
    let (mut x, mut y) = (1i32, 1i32);
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let mut iter = input.split_whitespace();
    if let Some(s) = iter.next() {
        if let Ok(v) = s.parse::<i32>() {
            x = v;
            if let Some(s2) = iter.next() {
                if let Ok(v2) = s2.parse::<i32>() {
                    y = v2;
                }
            }
        }
    }
    let quot = x / y;
    let rem = x % y;
    println!("quotient: {}, remainder: {}", quot, rem);
}
