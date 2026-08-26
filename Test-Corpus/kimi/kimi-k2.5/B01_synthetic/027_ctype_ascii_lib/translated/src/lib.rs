use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let c = c as u8 as char;
    
    println!("alphanumeric: {}", c.is_alphanumeric() as i32);
    println!("alphabetic: {}", c.is_alphabetic() as i32);
    println!("lowercase: {}", c.is_lowercase() as i32);
    println!("uppercase: {}", c.is_uppercase() as i32);
    println!("digit: {}", c.is_ascii_digit() as i32);
    println!("hexadecimal: {}", c.is_ascii_hexdigit() as i32);
    println!("control: {}", c.is_ascii_control() as i32);
    println!("graphical: {}", (c.is_ascii_graphic()) as i32);
    println!("space: {}", c.is_ascii_whitespace() as i32);
    println!("blank: {}", (c == ' ' || c == '\t') as i32);
    println!("printing: {}", c.is_ascii() as i32);
    println!("punctuation: {}", c.is_ascii_punctuation() as i32);
    println!("to lower: {}", c.to_ascii_lowercase());
    println!("to upper: {}", c.to_ascii_uppercase());
}