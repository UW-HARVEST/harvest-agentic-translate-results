// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::io::{self, Read, Write};

const UINT16_MAX: u32 = 65535;

type OperationFunc = fn(&mut i32, i32) -> i32;

fn increment_counter(counter: &mut i32, value: i32) -> i32 {
    *counter = counter.wrapping_add(value);
    *counter
}

fn decrement_counter(counter: &mut i32, value: i32) -> i32 {
    *counter = counter.wrapping_sub(value);
    *counter
}

fn multiply_counter(counter: &mut i32, value: i32) -> i32 {
    *counter = counter.wrapping_mul(value);
    *counter
}

fn reset_counter(counter: &mut i32, value: i32) -> i32 {
    *counter = value;
    *counter
}

fn is_string_empty(s: &str) -> i32 {
    // mirrors `if (!str) return 1; if (*str) return 0; return 1;`
    if s.is_empty() {
        return 1;
    }
    0
}

fn find_char_in_buffer(buffer: &[u8], target: u8) -> Option<usize> {
    buffer.iter().position(|&b| b == target)
}

fn create_buffer(initial: &str) -> Option<Vec<u8>> {
    // mirrors malloc + strcpy: allocate len+1 bytes including NUL
    let mut v = Vec::with_capacity(initial.len() + 1);
    v.extend_from_slice(initial.as_bytes());
    v.push(0);
    Some(v)
}

fn validate_uint16_range(value: i32) -> i32 {
    if value < 0 {
        return 0;
    }
    if (value as i64) > (UINT16_MAX as i64) {
        return 0;
    }
    1
}

fn apply_operation(op: Option<OperationFunc>, counter: &mut i32, value: i32) -> i32 {
    match op {
        None => -1,
        Some(f) => f(counter, value),
    }
}

fn charinbuf(mode: i32, value: i32, opt1: i32, opt2: i32) -> i32 {
    let mut result: i32 = 0;
    let test_string: &str = "";
    let non_empty_string: &str = "Hello, World!";

    let mut counter: i32 = 0;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    match mode {
        0 => {
            let _ = writeln!(out, "Mode 0: UINT16_MAX validation");
            let _ = writeln!(
                out,
                "Checking if value {} is within uint16_t range...",
                value
            );

            if validate_uint16_range(value) != 0 {
                let _ = writeln!(
                    out,
                    "Value {} is valid (0 <= value <= {})",
                    value, UINT16_MAX
                );
                result = value;
            } else {
                let _ = writeln!(out, "Value {} is out of range for uint16_t", value);
                result = -1;
            }

            let _ = writeln!(out, "UINT16_MAX constant value: {}", UINT16_MAX);
        }
        1 => {
            let _ = writeln!(out, "Mode 1: String empty check by dereference");

            if is_string_empty(test_string) != 0 {
                let _ = writeln!(out, "Test string is empty (checked with *string)");
                result = 0;
            } else {
                let _ = writeln!(out, "Test string is not empty");
                result = 1;
            }

            if is_string_empty(non_empty_string) != 0 {
                let _ = writeln!(out, "Non-empty string check failed!");
            } else {
                let _ = writeln!(out, "Non-empty string correctly identified");
                result += 10;
            }
        }
        2 => {
            let _ = writeln!(out, "Mode 2: Dynamic memory allocation and free");

            let buffer = create_buffer("Testing malloc and free");

            if let Some(buf) = buffer {
                // strlen excludes the trailing NUL
                let len = buf.len() - 1;
                // Print contents up to (but not including) the NUL
                let s = std::str::from_utf8(&buf[..len]).unwrap_or("");
                let _ = writeln!(out, "Buffer allocated: '{}'", s);
                let _ = writeln!(out, "Buffer length: {}", len);
                result = len as i32;

                // drop(buf) acts as free
                drop(buf);
                let _ = writeln!(out, "Buffer freed successfully");
            } else {
                let _ = writeln!(out, "Failed to allocate buffer");
                result = -1;
            }
        }
        3 => {
            let _ = writeln!(out, "Mode 3: Function pointers with static counter");

            let mut current_op: Option<OperationFunc> = Some(reset_counter);
            result = apply_operation(current_op, &mut counter, value);
            let _ = writeln!(out, "Counter reset to: {}", result);

            current_op = Some(increment_counter);
            result = apply_operation(current_op, &mut counter, opt1);
            let _ = writeln!(out, "Counter after increment by {}: {}", opt1, result);

            current_op = Some(multiply_counter);
            result = apply_operation(current_op, &mut counter, opt2);
            let _ = writeln!(out, "Counter after multiply by {}: {}", opt2, result);

            current_op = Some(decrement_counter);
            result = apply_operation(current_op, &mut counter, 5);
            let _ = writeln!(out, "Counter after decrement by 5: {}", result);

            let _ = writeln!(out, "Final static counter value: {}", counter);
        }
        4 => {
            let _ = writeln!(out, "Mode 4: Using memchr to find character");

            let buffer = create_buffer("Search for character X in this buffer");

            if let Some(buf) = buffer {
                let buf_size = buf.len() - 1; // strlen
                let search_char: u8 = b'X';

                let s = std::str::from_utf8(&buf[..buf_size]).unwrap_or("");
                let _ = writeln!(
                    out,
                    "Searching for '{}' in: '{}'",
                    search_char as char, s
                );

                let found_pos = find_char_in_buffer(&buf[..buf_size], search_char);

                if let Some(pos) = found_pos {
                    result = pos as i32;
                    let _ = writeln!(
                        out,
                        "Found '{}' at position: {}",
                        search_char as char, result
                    );
                } else {
                    let _ = writeln!(out, "Character '{}' not found", search_char as char);
                    result = -1;
                }

                drop(buf);
            }
        }
        _ => {
            let _ = writeln!(out, "Invalid mode: {}", mode);
            result = -1;
        }
    }

    result
}

/// Read whitespace-separated integers from stdin, mimicking C's scanf("%d %d %d %d", ...)
/// which reads across newlines.
fn read_four_ints() -> Option<(i32, i32, i32, i32)> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok()?;
    let mut iter = input.split_ascii_whitespace();
    let a: i32 = iter.next()?.parse().ok()?;
    let b: i32 = iter.next()?.parse().ok()?;
    let c: i32 = iter.next()?.parse().ok()?;
    let d: i32 = iter.next()?.parse().ok()?;
    Some((a, b, c, d))
}

fn main() {
    if let Some((mode, value, opt1, opt2)) = read_four_ints() {
        let _ = charinbuf(mode, value, opt1, opt2);
    }
}
