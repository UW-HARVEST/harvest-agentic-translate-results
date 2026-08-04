
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::Read;

fn driver(c: char) {
    let cc = c as u32;
    let is_space = matches!(cc, 0x20 | 0x09 | 0x0A | 0x0B | 0x0C | 0x0D);
    let is_blank = cc == 0x20 || cc == 0x09;
    let is_print = c.is_ascii() && !c.is_ascii_control();

    println!("alphanumeric: {}", c.is_ascii_alphanumeric() as i32);
    println!("alphabetic: {}", c.is_ascii_alphabetic() as i32);
    println!("lowercase: {}", c.is_ascii_lowercase() as i32);
    println!("uppercase: {}", c.is_ascii_uppercase() as i32);
    println!("digit: {}", c.is_ascii_digit() as i32);
    println!("hexadecimal: {}", c.is_ascii_hexdigit() as i32);
    println!("control: {}", c.is_ascii_control() as i32);
    println!("graphical: {}", c.is_ascii_graphic() as i32);
    println!("space: {}", is_space as i32);
    println!("blank: {}", is_blank as i32);
    println!("printing: {}", is_print as i32);
    println!("punctuation: {}", c.is_ascii_punctuation() as i32);
    println!("to lower: {}", c.to_ascii_lowercase());
    println!("to upper: {}", c.to_ascii_uppercase());
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let mut buf = [0u8; 1];
    if let Ok(1) = std::io::stdin().read(&mut buf) {
        driver(buf[0] as char);
    }
    0
}