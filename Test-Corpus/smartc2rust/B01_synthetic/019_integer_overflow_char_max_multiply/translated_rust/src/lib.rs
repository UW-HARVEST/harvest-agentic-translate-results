#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, BufRead, Write};

fn print_hex_char_line(char_hex: i8) {
    // Match C's printf("%02x\n", char) behavior: char is promoted to int with
    // sign extension, then interpreted as unsigned for %x formatting.
    let val = (char_hex as i32) as u32;
    println!("{:02x}", val);
}

fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = i8::MAX;
    let _ = data;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            print_line(Some("data value is too large to perform arithmetic safely."));
        }
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn read_int_from_stdin() -> i32 {
    let stdin = io::stdin();
    let mut line = String::new();
    let _ = io::stdout().flush();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => line.trim().parse::<i32>().unwrap_or(0),
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let x = read_int_from_stdin();

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}