use std::io::{self, Read};

fn fma_array(out: &mut [i32], len: usize) {
    for i in 0..len {
        let value = out[i];
        out[i] = value.wrapping_mul(value).wrapping_add(value);
    }
}

fn driver(out: &mut [i32], len: usize) {
    fma_array(out, len);
    for value in out.iter().take(len) {
        println!("{}", value);
    }
}

fn is_scanf_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\t' | b'\r' | 0x0b | 0x0c)
}

fn scan_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < input.len() && is_scanf_whitespace(input[*pos]) {
        *pos += 1;
    }

    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    let mut sign = 1i64;
    if input[*pos] == b'+' || input[*pos] == b'-' {
        if input[*pos] == b'-' {
            sign = -1;
        }
        *pos += 1;
    }

    if *pos >= input.len() || !input[*pos].is_ascii_digit() {
        *pos = start;
        return None;
    }

    let mut value = 0i64;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((input[*pos] - b'0') as i64);
        *pos += 1;
    }

    Some(value.wrapping_mul(sign) as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut pos = 0usize;
    let mut i = 0usize;

    while i < 100 {
        match scan_int(&input, &mut pos) {
            Some(value) => {
                data[i] = value;
                i += 1;
            }
            None => break,
        }
    }

    driver(&mut data, i);
}
