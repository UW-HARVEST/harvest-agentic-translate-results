use std::ffi::{c_char, c_int};
use std::process::ExitCode;

const MAX_BUFFER_SIZE: usize = 1024;

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn c_len(value: &[u8]) -> usize {
    value
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(value.len())
}

fn c_str(value: &[u8]) -> &[u8] {
    &value[..c_len(value)]
}

fn c_eq(left: &[u8], right: &[u8]) -> bool {
    c_str(left) == c_str(right)
}

fn c_n_eq(left: &[u8], right: &[u8], count: usize) -> bool {
    for index in 0..count {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        if left_byte != right_byte {
            return false;
        }
        if left_byte == 0 {
            return true;
        }
    }
    true
}

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_eq(token, expected) {
        return 1;
    }

    if c_eq(token, b"VALID\0") || c_eq(token, b"OK\0") {
        return 1;
    }

    0
}

fn parse_command(buffer: &[u8], buf_size: usize) -> i32 {
    const COMMANDS: [&[u8]; 5] = [b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0"];

    for (index, command) in COMMANDS.iter().enumerate() {
        let command_len = c_len(command);

        if buf_size >= command_len
            && c_n_eq(buffer, command, command_len)
            && matches!(buffer.get(command_len), Some(0 | b' '))
        {
            return index as i32;
        }

        if c_eq(buffer, command) {
            return index as i32;
        }
    }

    if c_eq(buffer, b"ADMIN\0") {
        return 99;
    }

    -1
}

fn compare_prefix(value: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = c_len(prefix);

    if exact_match {
        if c_eq(value, prefix) {
            return 1;
        }

        const VARIATIONS: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (index, variation) in VARIATIONS.iter().enumerate() {
            let mut expected = Vec::with_capacity(64);
            expected.extend_from_slice(&c_str(prefix)[..prefix_len.min(63)]);
            let remaining = 63 - expected.len();
            expected.extend_from_slice(&variation[..variation.len().min(remaining)]);
            expected.push(0);

            if c_eq(value, &expected) {
                return 2 + index as i32;
            }
        }

        0
    } else if c_n_eq(value, prefix, prefix_len) {
        1
    } else {
        0
    }
}

fn find_delimiter(data: &[u8], len: usize, delimiter: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    for (index, &byte) in data[..len].iter().enumerate() {
        if byte == delimiter {
            return index as i32;
        }
        if byte == 0 {
            break;
        }
    }

    if delimiter == b'|' && c_eq(data, b"NONE\0") {
        return -2;
    }

    if delimiter == b':' && c_eq(data, b"EMPTY\0") {
        return -3;
    }

    -1
}

fn truncated_pattern(prefix: &[u8], pattern: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(64);
    for byte in prefix.iter().chain(c_str(pattern)).chain(suffix).take(63) {
        result.push(*byte);
    }
    result.push(0);
    result
}

fn wrapped_pattern_search(text: &[u8], pattern: &[u8], pattern_len: usize) -> i32 {
    // The release C layout places reference_buffer 1024 bytes after input_buffer.
    for start in 0..=MAX_BUFFER_SIZE {
        let matches = (0..pattern_len).all(|offset| {
            let position = start + offset;
            let byte = if position < MAX_BUFFER_SIZE {
                text[position]
            } else {
                pattern[position - MAX_BUFFER_SIZE]
            };
            byte == pattern[offset]
        });
        if matches {
            return (10usize + start) as i32;
        }
    }
    unreachable!()
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_eq(text, pattern) {
            return 1;
        }

        let wildcard_patterns = [
            truncated_pattern(b"*", pattern, b"*"),
            truncated_pattern(b"", pattern, b"*"),
            truncated_pattern(b"*", pattern, b""),
        ];

        for (index, wildcard) in wildcard_patterns.iter().enumerate() {
            if c_eq(text, wildcard) {
                return 2 + index as i32;
            }
        }

        let text_len = c_len(text);
        let pattern_len = c_len(pattern);
        if pattern_len > text_len {
            return wrapped_pattern_search(text, pattern, pattern_len);
        }
        for index in 0..=text_len - pattern_len {
            if c_n_eq(&text[index..], pattern, pattern_len) {
                return (10usize + index) as i32;
            }
        }
    } else {
        if c_eq(text, pattern) {
            return 1;
        }

        let pattern_len = c_len(pattern);
        let text_len = c_len(text);

        if text_len != pattern_len && c_n_eq(text, pattern, pattern_len) {
            return 5;
        }

        if text_len == pattern_len {
            let mut matches = true;
            for index in 0..pattern_len {
                let mut left = text[index];
                let mut right = pattern[index];

                if left.is_ascii_uppercase() {
                    left += 32;
                }
                if right.is_ascii_uppercase() {
                    right += 32;
                }

                if left != right {
                    matches = false;
                    break;
                }
            }
            if matches {
                return 6;
            }
        }
    }

    0
}

fn process_strings(
    input: &[u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    match operation {
        0 => validate_token(input, reference),
        1 => parse_command(input, input_len),
        2 => compare_prefix(input, reference, flags & 0x01 != 0),
        3 => {
            let delimiter = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delimiter)
        }
        4 => match_pattern(input, reference, flags & 0x02 != 0),
        _ => -3,
    }
}

fn scan_i32(value: &mut i32) -> bool {
    unsafe { scanf(c"%d".as_ptr(), value) == 1 }
}

fn scan_u32(value: &mut u32) -> bool {
    unsafe { scanf(c"%u".as_ptr(), value) == 1 }
}

fn scan_usize(value: &mut usize) -> bool {
    unsafe { scanf(c"%zu".as_ptr(), value) == 1 }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut operation = 0;
    let mut flags = 0;
    let mut input_len = 0;
    let mut ref_len = 0;
    let mut input_buffer = [0u8; MAX_BUFFER_SIZE + 1];
    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE + 1];

    if !scan_i32(&mut operation) {
        return fail("Error reading operation");
    }

    if !scan_u32(&mut flags) {
        return fail("Error reading flags");
    }

    if !scan_usize(&mut input_len) {
        return fail("Error reading input length");
    }

    if input_len > MAX_BUFFER_SIZE {
        eprintln!("Error: input length {input_len} exceeds maximum {MAX_BUFFER_SIZE}");
        return ExitCode::FAILURE;
    }

    for (index, byte) in input_buffer[..input_len].iter_mut().enumerate() {
        let mut value = 0u32;
        if !scan_u32(&mut value) {
            eprintln!("Error reading input byte {index}");
            return ExitCode::FAILURE;
        }
        *byte = value as u8;
    }

    if !scan_usize(&mut ref_len) {
        return fail("Error reading reference length");
    }

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!("Error: reference length {ref_len} exceeds maximum {MAX_BUFFER_SIZE}");
        return ExitCode::FAILURE;
    }

    for (index, byte) in ref_buffer[..ref_len].iter_mut().enumerate() {
        let mut value = 0u32;
        if !scan_u32(&mut value) {
            eprintln!("Error reading reference byte {index}");
            return ExitCode::FAILURE;
        }
        *byte = value as u8;
    }

    let result = process_strings(
        &input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }

    ExitCode::SUCCESS
}
