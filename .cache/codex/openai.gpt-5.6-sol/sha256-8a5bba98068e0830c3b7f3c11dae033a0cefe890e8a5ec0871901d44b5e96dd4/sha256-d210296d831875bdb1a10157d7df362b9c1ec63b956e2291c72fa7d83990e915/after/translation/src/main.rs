use std::fmt::Write as _;
use std::io::{self, Read, Write as _};

const BUFFER_LEN: usize = 10;
const FGETS_SIZE: usize = 14;

fn print_line(output: &mut String, line: &str) {
    writeln!(output, "{line}").unwrap();
}

fn print_int_line(output: &mut String, number: i32) {
    writeln!(output, "{number}").unwrap();
}

fn fgets_14(input: &mut impl Read) -> Option<Vec<u8>> {
    let mut buffer = Vec::with_capacity(FGETS_SIZE - 1);

    while buffer.len() < FGETS_SIZE - 1 {
        let mut byte = [0_u8; 1];
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) if buffer.is_empty() => return None,
            Err(_) => break,
        }
    }

    (!buffer.is_empty()).then_some(buffer)
}

fn c_atoi(input: &[u8]) -> i32 {
    let input = &input[..input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(input.len())];
    let mut index = 0;

    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        index += 1;
    }

    let mut sign = 1_i64;
    if index < input.len() {
        match input[index] {
            b'-' => {
                sign = -1;
                index += 1;
            }
            b'+' => index += 1,
            _ => {}
        }
    }

    let mut value = 0_i64;
    while index < input.len() && input[index].is_ascii_digit() {
        value = value * 10 + i64::from(input[index] - b'0');
        index += 1;
    }

    (sign * value) as i32
}

fn bad(input: &mut impl Read, output: &mut String) {
    let mut data = -1;
    if let Some(input_buffer) = fgets_14(input) {
        data = c_atoi(&input_buffer);
    } else {
        print_line(output, "fgets() failed.");
    }

    let mut buffer = [0_i32; BUFFER_LEN];
    if data >= 0 {
        // Preserve the unchecked sink's output without corrupting Rust's stack.
        if let Some(element) = buffer.get_mut(data as usize) {
            *element = 1;
        }
        for number in buffer {
            print_int_line(output, number);
        }
    } else {
        print_line(output, "ERROR: Array index is negative.");
    }
}

fn good_g2b(output: &mut String) {
    let data = 7;
    let mut buffer = [0_i32; BUFFER_LEN];
    if data >= 0 {
        buffer[data as usize] = 1;
        for number in buffer {
            print_int_line(output, number);
        }
    } else {
        print_line(output, "ERROR: Array index is negative.");
    }
}

fn good_b2g(input: &mut impl Read, output: &mut String) {
    let mut data = -1;
    if let Some(input_buffer) = fgets_14(input) {
        data = c_atoi(&input_buffer);
    } else {
        print_line(output, "fgets() failed.");
    }

    let mut buffer = [0_i32; BUFFER_LEN];
    if data >= 0 && data < BUFFER_LEN as i32 {
        buffer[data as usize] = 1;
        for number in buffer {
            print_int_line(output, number);
        }
    } else {
        print_line(output, "ERROR: Array index is out-of-bounds");
    }
}

fn good(input: &mut impl Read, output: &mut String) {
    good_g2b(output);
    good_b2g(input, output);
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = String::new();

    print_line(&mut output, "Calling good()...");
    good(&mut input, &mut output);
    print_line(&mut output, "Finished good()");
    print_line(&mut output, "Calling bad()...");
    bad(&mut input, &mut output);
    print_line(&mut output, "Finished bad()");

    io::stdout().write_all(output.as_bytes()).unwrap();
}
