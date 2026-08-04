use std::io::{self, Read, Write};

fn is_scanf_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | 0x0b | 0x0c)
}

fn scan_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < input.len() && is_scanf_space(input[*pos]) {
        *pos += 1;
    }

    if *pos >= input.len() {
        return None;
    }

    let mut index = *pos;
    let mut negative = false;

    match input[index] {
        b'+' => {
            index += 1;
        }
        b'-' => {
            negative = true;
            index += 1;
        }
        _ => {}
    }

    if index >= input.len() || !input[index].is_ascii_digit() {
        return None;
    }

    let mut value = 0_i32;
    while index < input.len() && input[index].is_ascii_digit() {
        let digit = (input[index] - b'0') as i32;
        value = if negative {
            value.wrapping_mul(10).wrapping_sub(digit)
        } else {
            value.wrapping_mul(10).wrapping_add(digit)
        };
        index += 1;
    }

    *pos = index;
    Some(value)
}

fn fma_array(data: &mut [i32], len: usize) {
    for item in &mut data[..len] {
        let value = *item;
        *item = value.wrapping_mul(value).wrapping_add(value);
    }
}

fn driver(data: &mut [i32], len: usize) {
    fma_array(data, len);

    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for value in &data[..len] {
        writeln!(stdout, "{value}").expect("stdout write failed");
    }
}

fn main() {
    let mut input = Vec::new();
    io::stdin()
        .lock()
        .read_to_end(&mut input)
        .expect("stdin read failed");

    let mut data = [0_i32; 100];
    let mut pos = 0_usize;
    let mut len = 0_usize;

    while len < 100 {
        match scan_int(&input, &mut pos) {
            Some(value) => {
                data[len] = value;
                len += 1;
            }
            None => break,
        }
    }

    driver(&mut data, len);
}
