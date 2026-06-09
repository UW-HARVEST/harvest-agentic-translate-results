// Translation of c_src/src/lib.c to Rust.
//
// The original C code is a shared library (no `main`). It exposes a
// `cleanup` function that performs validation, runs a switch over four
// integers, allocates a small buffer, prints a formatted message, and
// returns an integer result. Because the C source has no `main`, the
// translated executable produces no output by default; the `cleanup`
// function is preserved here with the same observable behavior.

use std::io::Write;

#[allow(dead_code)]
fn cleanup_resources(_dynamic_str: Option<String>) {
    // In C, free() is called on the dynamic_str pointer if non-null.
    // In Rust, ownership moving into this function means the buffer is
    // dropped here, mirroring the free() call.
}

fn cleanup(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let numbers: [i32; 4] = [a, b, c, d];
    let mut dynamic_str: Option<String> = None;
    let mut result: i32 = 0;

    let expected_str = "VALID";
    let input_str = "VALID";
    // strncmp(input_str, expected_str, strlen(expected_str)) != 0
    let n = expected_str.len();
    let cmp = {
        let a_bytes = input_str.as_bytes();
        let b_bytes = expected_str.as_bytes();
        let mut differs = false;
        for i in 0..n {
            let ai = if i < a_bytes.len() { a_bytes[i] } else { 0 };
            let bi = if i < b_bytes.len() { b_bytes[i] } else { 0 };
            if ai != bi {
                differs = true;
                break;
            }
            if ai == 0 {
                break;
            }
        }
        differs
    };

    'cleanup: {
        if cmp {
            print!("Input string validation failed.\n");
            let _ = std::io::stdout().flush();
            break 'cleanup;
        }

        // Match C's switch fall-through behavior exactly.
        for i in 0..4 {
            match numbers[i] {
                10 => {
                    result += 10;
                    // fall-through to case 20
                    result += 20;
                }
                20 => {
                    result += 20;
                }
                30 => {
                    result += 30;
                    // fall-through to case 40
                    result += 40;
                }
                40 => {
                    result += 40;
                }
                _ => {
                    result += numbers[i];
                }
            }
        }

        // dynamic_str = malloc(50). In Rust, allocate a String buffer.
        // Allocation will not fail in normal operation; mirror the C
        // path where allocation succeeds.
        let mut buf = String::with_capacity(50);

        // snprintf(dynamic_str, 50, "Processed numbers: %s", TO_STRING(numbers));
        // TO_STRING(numbers) expands to the literal string "numbers".
        let formatted = format!("Processed numbers: {}", "numbers");
        // snprintf truncates to 49 bytes (plus null terminator).
        let truncated: String = formatted.chars().take(49).collect();
        // The actual byte limit in C is 49 bytes, but the produced string
        // here is well within that limit.
        let _ = truncated.len();
        buf.push_str(&truncated);

        print!("{}\n", buf);
        let _ = std::io::stdout().flush();
        dynamic_str = Some(buf);
    }

    cleanup_resources(dynamic_str);
    result
}

#[allow(dead_code)]
fn print_result(label: &str, result: i32) {
    print!("{}: {}\n", label, result);
    let _ = std::io::stdout().flush();
}

fn main() {
    // The original C source defines a shared library with no `main`,
    // so the executable produces no output.
    let _ = cleanup;
    let _ = print_result;
}
