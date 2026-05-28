// Copyright 2025 MIT Lincoln Laboratory
// (License header preserved from original C source)

use std::io::{self, Read};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;
    let y = unsafe { Y };

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        // goto fail
        println!("Operation failed");
        return result;
    }

    if y != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
        // goto fail
        println!("Operation failed");
        return result;
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        // goto fail
        println!("Operation failed");
        return result;
    }

    println!("Ok!");
    result
}

/// Reads up to 3 integers from stdin, separated by any whitespace (spaces,
/// tabs, newlines), mimicking C's `scanf("%d %d %d", ...)` behavior.
///
/// Returns however many integers it managed to parse. Any tokens that are not
/// valid integers (or EOF) cause parsing to stop, matching scanf semantics
/// where unmatched fields leave the destination variables unchanged.
fn read_three_ints() -> Vec<i32> {
    let mut input = String::new();
    // Read all of stdin so that scanf-style whitespace skipping (including
    // newlines) works correctly.
    let _ = io::stdin().read_to_string(&mut input);

    let mut results: Vec<i32> = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while results.len() < 3 && i < bytes.len() {
        // Skip any whitespace (scanf %d skips leading whitespace).
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Parse optional sign.
        let start = i;
        if bytes[i] == b'+' || bytes[i] == b'-' {
            i += 1;
        }
        let digits_start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        if i == digits_start {
            // No digits parsed -> matching failure, scanf would stop here.
            break;
        }
        let token = &input[start..i];
        match token.parse::<i32>() {
            Ok(v) => results.push(v),
            Err(_) => break,
        }
    }
    results
}

fn main() {
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    let parsed = read_three_ints();
    if parsed.len() >= 1 {
        x = parsed[0];
    }
    if parsed.len() >= 2 {
        unsafe {
            Y = parsed[1];
        }
    }
    if parsed.len() >= 3 {
        z = parsed[2];
    }

    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
