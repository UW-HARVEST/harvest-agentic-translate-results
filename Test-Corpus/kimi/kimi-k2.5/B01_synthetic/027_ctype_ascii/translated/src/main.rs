use std::io::{self, Read};

fn driver(c: char) {
    println!("alphanumeric: {}", c.is_alphanumeric() as i32);
    println!("alphabetic: {}", c.is_alphabetic() as i32);
    println!("lowercase: {}", c.is_lowercase() as i32);
    println!("uppercase: {}", c.is_uppercase() as i32);
    println!("digit: {}", c.is_ascii_digit() as i32);
    println!("hexadecimal: {}", c.is_ascii_hexdigit() as i32);
    println!("control: {}", c.is_ascii_control() as i32);
    println!("graphical: {}", c.is_ascii_graphic() as i32);
    println!("space: {}", c.is_ascii_whitespace() as i32);
    println!("blank: {}", (c == ' ' || c == '\t') as i32);
    println!("printing: {}", c.is_ascii() && !c.is_ascii_control() as i32);
    println!("punctuation: {}", c.is_ascii_punctuation() as i32);
    println!("to lower: {}", c.to_lowercase().next().unwrap_or(c));
    println!("to upper: {}", c.to_uppercase().next().unwrap_or(c));
}

fn main() {
    let mut buffer = [0u8; 1];
    io::stdin().read_exact(&mut buffer).ok();
    let c = buffer[0] as char;
    driver(c);
}