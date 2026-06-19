use std::io::{self, Read};

fn is_scanf_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | 0x0b | 0x0c)
}

fn scanf_decimal_int(input: &[u8]) -> Option<i32> {
    let mut index = 0;
    while index < input.len() && is_scanf_whitespace(input[index]) {
        index += 1;
    }

    let mut negative = false;
    if index < input.len() && (input[index] == b'+' || input[index] == b'-') {
        negative = input[index] == b'-';
        index += 1;
    }

    if index >= input.len() || !input[index].is_ascii_digit() {
        return None;
    }

    let mut value: i32 = 0;
    while index < input.len() && input[index].is_ascii_digit() {
        let digit = (input[index] - b'0') as i32;
        value = value.wrapping_mul(10).wrapping_add(digit);
        index += 1;
    }

    if negative {
        Some(value.wrapping_neg())
    } else {
        Some(value)
    }
}

fn driver(x: i32) {
    let y = x.wrapping_mul(2).wrapping_add(300);
    println!("{y}");
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let x = scanf_decimal_int(&input).unwrap_or(0);
    driver(x);
}
