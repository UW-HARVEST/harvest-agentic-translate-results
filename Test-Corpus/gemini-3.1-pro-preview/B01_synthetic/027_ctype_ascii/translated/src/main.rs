use std::io::Read;

fn driver(c: u8) {
    println!("alphanumeric: {}", c.is_ascii_alphanumeric() as i32);
    println!("alphabetic: {}", c.is_ascii_alphabetic() as i32);
    println!("lowercase: {}", c.is_ascii_lowercase() as i32);
    println!("uppercase: {}", c.is_ascii_uppercase() as i32);
    println!("digit: {}", c.is_ascii_digit() as i32);
    println!("hexadecimal: {}", c.is_ascii_hexdigit() as i32);
    println!("control: {}", c.is_ascii_control() as i32);
    println!("graphical: {}", c.is_ascii_graphic() as i32);
    println!("space: {}", c.is_ascii_whitespace() as i32);
    println!("blank: {}", (c == b' ' || c == b'\t') as i32);
    println!("printing: {}", (c.is_ascii_graphic() || c == b' ') as i32);
    println!("punctuation: {}", c.is_ascii_punctuation() as i32);
    println!("to lower: {}", c.to_ascii_lowercase() as char);
    println!("to upper: {}", c.to_ascii_uppercase() as char);
}

fn main() {
    let mut buf = [0u8; 1];
    let c = match std::io::stdin().read_exact(&mut buf) {
        Ok(_) => buf[0],
        Err(_) => 255,
    };
    driver(c);
}
