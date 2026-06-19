use std::io::{self, Read, Write};

fn c_atoi(input: &[u8]) -> i32 {
    let mut index = 0;
    while index < input.len()
        && matches!(
            input[index],
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c
        )
    {
        index += 1;
    }

    let mut sign = 1i32;
    if index < input.len() {
        if input[index] == b'-' {
            sign = -1;
            index += 1;
        } else if input[index] == b'+' {
            index += 1;
        }
    }

    let mut value = 0i32;
    while index < input.len() && input[index].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((input[index] - b'0') as i32);
        index += 1;
    }

    value.wrapping_mul(sign)
}

fn fgets_14_stdin() -> Option<Vec<u8>> {
    let mut stdin = io::stdin();
    let mut all_input = Vec::new();
    stdin.read_to_end(&mut all_input).ok()?;

    if all_input.is_empty() {
        return None;
    }

    let mut input_buffer = Vec::new();
    for &byte in all_input.iter().take(13) {
        input_buffer.push(byte);
        if byte == b'\n' {
            break;
        }
    }

    Some(input_buffer)
}

fn crash_like_c() -> ! {
    std::process::exit(139);
}

fn main() {
    let mut data = -1i32;
    let mut out = io::BufWriter::new(io::stdout());

    if let Some(input_buffer) = fgets_14_stdin() {
        data = c_atoi(&input_buffer);
    } else {
        let _ = writeln!(out, "fgets() failed.");
    }

    let mut dest = Vec::new();
    if data < 100 {
        if data < 0 {
            crash_like_c();
        }
        dest.resize(data as usize, b'A');
    }

    let _ = out.write_all(&dest);
    let _ = out.write_all(b"\n");
}
