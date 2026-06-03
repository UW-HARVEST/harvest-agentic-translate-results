// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

pub fn cleanup(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let numbers = [a, b, c, d];
    let mut result: i32 = 0;

    let expected_str = "VALID";
    let input_str = "VALID";

    // Equivalent to strncmp(input_str, expected_str, strlen(expected_str)) != 0
    let n = expected_str.len();
    let input_bytes = input_str.as_bytes();
    let expected_bytes = expected_str.as_bytes();
    let cmp_len = n.min(input_bytes.len());
    let mismatch = if cmp_len < n {
        true
    } else {
        input_bytes[..n] != expected_bytes[..n]
    };

    if mismatch {
        println!("Input string validation failed.");
        // Equivalent to "goto cleanup;": fall through to cleanup section
        return cleanup_section(result, None);
    }

    for i in 0..4 {
        // Replicate C switch fall-through behavior precisely.
        match numbers[i] {
            10 => {
                result = result.wrapping_add(10);
                // fall through to case 20
                result = result.wrapping_add(20);
            }
            20 => {
                result = result.wrapping_add(20);
            }
            30 => {
                result = result.wrapping_add(30);
                // fall through to case 40
                result = result.wrapping_add(40);
            }
            40 => {
                result = result.wrapping_add(40);
            }
            _ => {
                result = result.wrapping_add(numbers[i]);
            }
        }
    }

    // Allocate a dynamic string (Vec<u8>) similar to malloc(50)
    let mut dynamic_str: Vec<u8> = vec![0u8; 50];

    // snprintf(dynamic_str, 50, "Processed numbers: %s", TO_STRING(numbers));
    // TO_STRING(numbers) stringizes the macro argument, producing the literal "numbers".
    let formatted = format!("Processed numbers: {}", "numbers");
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min(dynamic_str.len() - 1);
    dynamic_str[..copy_len].copy_from_slice(&bytes[..copy_len]);
    dynamic_str[copy_len] = 0;

    // printf("%s\n", dynamic_str);
    let printable = std::str::from_utf8(&dynamic_str[..copy_len]).unwrap_or("");
    println!("{}", printable);

    cleanup_section(result, Some(dynamic_str))
}

fn cleanup_section(result: i32, dynamic_str: Option<Vec<u8>>) -> i32 {
    cleanup_resources(dynamic_str);
    result
}

pub fn print_result(label: &str, result: i32) {
    println!("{}: {}", label, result);
}

fn cleanup_resources(dynamic_str: Option<Vec<u8>>) {
    if let Some(s) = dynamic_str {
        // Explicitly drop to mirror free(); ownership is consumed here.
        drop(s);
    }
}
