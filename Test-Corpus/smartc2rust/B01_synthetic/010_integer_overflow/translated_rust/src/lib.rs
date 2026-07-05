
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    // Match C's printf("%02x\n", charHex) behavior:
    // In C, `char` is promoted to `int` with sign extension, then `%x` prints
    // it as an unsigned int. Replicate this by sign-extending to i32 and
    // reinterpreting as u32.
    let val = char_hex as i32 as u32;
    println!("{:02x}", val);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_c_main() -> i32 {
    // Equivalent of: char data = ' '; fscanf(stdin, "%c", &data);
    // If reading fails or EOF is hit, `data` keeps its initial value (space).
    let data: i8 = std::io::stdin()
        .bytes()
        .next()
        .and_then(|res| res.ok())
        .map(|b| b as i8)
        .unwrap_or(b' ' as i8);

    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);

    0
}