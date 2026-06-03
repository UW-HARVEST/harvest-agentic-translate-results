// Translated from C to Rust.
// Original: MIT Lincoln Laboratory, 2025

use std::sync::Mutex;

// Static counter, equivalent to `static int counter = 0;` in C.
// Uses a Mutex for safe interior mutability across function calls.
static COUNTER: Mutex<i32> = Mutex::new(0);

type OperationFunc = fn(i32) -> i32;

const UINT16_MAX: u32 = 65535;

pub fn increment_counter(value: i32) -> i32 {
    let mut c = COUNTER.lock().unwrap();
    *c = c.wrapping_add(value);
    *c
}

pub fn decrement_counter(value: i32) -> i32 {
    let mut c = COUNTER.lock().unwrap();
    *c = c.wrapping_sub(value);
    *c
}

pub fn multiply_counter(value: i32) -> i32 {
    let mut c = COUNTER.lock().unwrap();
    *c = c.wrapping_mul(value);
    *c
}

pub fn reset_counter(value: i32) -> i32 {
    let mut c = COUNTER.lock().unwrap();
    *c = value;
    *c
}

/// Returns 1 if str is None or points to an empty string, 0 otherwise.
pub fn is_string_empty(s: Option<&str>) -> i32 {
    match s {
        None => 1,
        Some(string) => {
            if string.is_empty() {
                1
            } else {
                // Check if first byte is non-zero (matching C `*str` semantics)
                if string.as_bytes()[0] != 0 {
                    0
                } else {
                    1
                }
            }
        }
    }
}

/// Find the first occurrence of `target` byte in the first `size` bytes of `buffer`.
/// Returns the index where it was found, or None.
pub fn find_char_in_buffer(buffer: Option<&[u8]>, size: usize, target: u8) -> Option<usize> {
    match buffer {
        None => None,
        Some(buf) => {
            let limit = size.min(buf.len());
            buf[..limit].iter().position(|&b| b == target)
        }
    }
}

/// Allocate a buffer that is a copy of `initial` (NUL-terminated semantics).
pub fn create_buffer(initial: Option<&str>) -> Option<Vec<u8>> {
    match initial {
        None => None,
        Some(s) => {
            let mut v = Vec::with_capacity(s.len() + 1);
            v.extend_from_slice(s.as_bytes());
            v.push(0);
            Some(v)
        }
    }
}

pub fn validate_uint16_range(value: i32) -> i32 {
    if value < 0 {
        return 0;
    }
    if (value as i64) > UINT16_MAX as i64 {
        return 0;
    }
    1
}

pub fn apply_operation(op: Option<OperationFunc>, value: i32) -> i32 {
    match op {
        None => -1,
        Some(f) => f(value),
    }
}

/// Helper to compute the strlen-equivalent of a NUL-terminated buffer.
fn strlen_of(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

pub fn charinbuf(mode: i32, value: i32, opt1: i32, opt2: i32) -> i32 {
    let mut result: i32;
    let test_string: &str = "";
    let non_empty_string: &str = "Hello, World!";

    // Reset static counter
    {
        let mut c = COUNTER.lock().unwrap();
        *c = 0;
    }

    match mode {
        0 => {
            println!("Mode 0: UINT16_MAX validation");
            println!("Checking if value {} is within uint16_t range...", value);

            if validate_uint16_range(value) != 0 {
                println!(
                    "Value {} is valid (0 <= value <= {})",
                    value, UINT16_MAX
                );
                result = value;
            } else {
                println!("Value {} is out of range for uint16_t", value);
                result = -1;
            }

            println!("UINT16_MAX constant value: {}", UINT16_MAX);
        }

        1 => {
            println!("Mode 1: String empty check by dereference");

            if is_string_empty(Some(test_string)) != 0 {
                println!("Test string is empty (checked with *string)");
                result = 0;
            } else {
                println!("Test string is not empty");
                result = 1;
            }

            if is_string_empty(Some(non_empty_string)) != 0 {
                println!("Non-empty string check failed!");
            } else {
                println!("Non-empty string correctly identified");
                result += 10;
            }
        }

        2 => {
            println!("Mode 2: Dynamic memory allocation and free");

            let buffer = create_buffer(Some("Testing malloc and free"));

            match buffer {
                Some(buf) => {
                    let len = strlen_of(&buf);
                    let s = std::str::from_utf8(&buf[..len]).unwrap_or("");
                    println!("Buffer allocated: '{}'", s);
                    println!("Buffer length: {}", len);
                    result = len as i32;

                    drop(buf);
                    println!("Buffer freed successfully");
                }
                None => {
                    println!("Failed to allocate buffer");
                    result = -1;
                }
            }
        }

        3 => {
            println!("Mode 3: Function pointers with static counter");

            let mut current_op: OperationFunc;

            current_op = reset_counter;
            result = apply_operation(Some(current_op), value);
            println!("Counter reset to: {}", result);

            current_op = increment_counter;
            result = apply_operation(Some(current_op), opt1);
            println!("Counter after increment by {}: {}", opt1, result);

            current_op = multiply_counter;
            result = apply_operation(Some(current_op), opt2);
            println!("Counter after multiply by {}: {}", opt2, result);

            current_op = decrement_counter;
            result = apply_operation(Some(current_op), 5);
            println!("Counter after decrement by 5: {}", result);

            let final_counter = *COUNTER.lock().unwrap();
            println!("Final static counter value: {}", final_counter);
        }

        4 => {
            println!("Mode 4: Using memchr to find character");

            let buffer = create_buffer(Some("Search for character X in this buffer"));

            result = 0;

            if let Some(buf) = buffer {
                let buf_size = strlen_of(&buf);
                let search_char: u8 = b'X';

                let s = std::str::from_utf8(&buf[..buf_size]).unwrap_or("");
                println!("Searching for '{}' in: '{}'", search_char as char, s);

                let found_pos = find_char_in_buffer(Some(&buf), buf_size, search_char);

                match found_pos {
                    Some(idx) => {
                        result = idx as i32;
                        println!("Found '{}' at position: {}", search_char as char, result);
                    }
                    None => {
                        println!("Character '{}' not found", search_char as char);
                        result = -1;
                    }
                }

                drop(buf);
            }
        }

        _ => {
            println!("Invalid mode: {}", mode);
            result = -1;
        }
    }

    result
}
