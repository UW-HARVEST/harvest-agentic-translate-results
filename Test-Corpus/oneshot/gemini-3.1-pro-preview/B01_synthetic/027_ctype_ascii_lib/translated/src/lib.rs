use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let u = c as u8;
    
    println!("alphanumeric: {}", u.is_ascii_alphanumeric() as i32);
    println!("alphabetic: {}", u.is_ascii_alphabetic() as i32);
    println!("lowercase: {}", u.is_ascii_lowercase() as i32);
    println!("uppercase: {}", u.is_ascii_uppercase() as i32);
    println!("digit: {}", u.is_ascii_digit() as i32);
    println!("hexadecimal: {}", u.is_ascii_hexdigit() as i32);
    println!("control: {}", u.is_ascii_control() as i32);
    println!("graphical: {}", (u >= 0x21 && u <= 0x7E) as i32);
    println!("space: {}", u.is_ascii_whitespace() as i32);
    println!("blank: {}", (u == b' ' || u == b'\t') as i32);
    println!("printing: {}", (u >= 0x20 && u <= 0x7E) as i32);
    println!("punctuation: {}", u.is_ascii_punctuation() as i32);
    println!("to lower: {}", u.to_ascii_lowercase() as char);
    println!("to upper: {}", u.to_ascii_uppercase() as char);
}
