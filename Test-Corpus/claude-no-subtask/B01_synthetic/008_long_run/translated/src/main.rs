// Copyright 2025 MIT Lincoln Laboratory
// Translation to Rust producing byte-identical output to the C version.

use std::env;
use std::io::Write;
use std::process::ExitCode;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: i32 = 2000;

extern "C" {
    fn srand(seed: libc_uint);
    fn rand() -> libc_int;
}

#[allow(non_camel_case_types)]
type libc_int = i32;
#[allow(non_camel_case_types)]
type libc_uint = u32;

// Perform expensive arithmetic on each element
fn perform_expensive_operations(array: &mut [i32]) {
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = array[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);  arithmetic shift right on signed
            x ^= x >> 3;
            // x = x - (x << 1); left shift may overflow; use wrapping
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;
            // C integer division truncates toward zero, same as Rust.
            // Need to guard against overflow on x = i32::MIN / -1 etc.,
            // but divisor is constant positive so wrapping ops are unnecessary.
            // However x/2 with x = i32::MIN = -2147483648: -1073741824, fine.
            // x % 7 with positive divisor: result has sign of x in C99/Rust.
            x = x.wrapping_div(2).wrapping_add(x.wrapping_rem(7));
        }
        array[i] = x;
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        let prog = args.get(0).map(|s| s.as_str()).unwrap_or("");
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(handle, "Usage: {} <seed>", prog);
        return ExitCode::from(1);
    }

    // Mimic strtoul(argv[1], &endptr, 10) with checks:
    //   *endptr != '\0' OR errno != 0 OR temp_seed > UINT_MAX
    let arg = &args[1];
    let parsed = parse_strtoul(arg);
    let seed: u32 = match parsed {
        Some(v) => v,
        None => {
            let stderr = std::io::stderr();
            let mut handle = stderr.lock();
            let _ = writeln!(handle, "Invalid seed: '{}'", arg);
            return ExitCode::from(1);
        }
    };

    // Allocate array on the heap, zero-initialized like a C global.
    let mut array: Vec<i32> = vec![0i32; ARRAY_SIZE];

    unsafe {
        srand(seed);
        for i in 0..ARRAY_SIZE {
            array[i] = rand();
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    println!("{}", xor_result);
    ExitCode::from(0)
}

/// Mimic the C strtoul(s, &endptr, 10) followed by:
///   if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX) -> error
///
/// Returns Some(value) if the entire string parses as a non-negative
/// decimal that fits in unsigned int (u32), otherwise None.
///
/// Key C semantics to mirror:
/// - Skips leading whitespace
/// - Accepts optional '+' or '-' sign; '-' is accepted but the value is
///   negated mod ULONG_MAX (very platform-dependent). For our purposes,
///   we match the typical Linux/glibc behavior closely enough: any
///   parse failure or overflow leads to error.
/// - Empty/no-digits case: endptr == s, *endptr is the first char which
///   may or may not be '\0'. We treat that as error if no digits.
fn parse_strtoul(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace (matching isspace in C locale: space, \t, \n, \v, \f, \r)
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => {
                i += 1;
            }
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }

    // Must have at least one digit
    let digits_start = i;
    let mut value: u64 = 0;
    let mut overflow = false;
    while i < bytes.len() {
        let c = bytes[i];
        if !(b'0'..=b'9').contains(&c) {
            break;
        }
        let d = (c - b'0') as u64;
        // u64 will not overflow with at most ~20 digits; but u32 might.
        value = value.wrapping_mul(10).wrapping_add(d);
        if value > u32::MAX as u64 {
            overflow = true;
        }
        i += 1;
    }

    if i == digits_start {
        // No digits parsed -> endptr == start, *endptr likely non-null
        return None;
    }

    // After parsing digits, *endptr must be '\0'
    if i != bytes.len() {
        return None;
    }

    if overflow {
        return None;
    }

    let mut v: u32 = value as u32;
    if negative {
        // strtoul negates: but if value was 0, fine; else this is the
        // 2's-complement negation modulo ULONG_MAX. The C check
        // temp_seed > UINT_MAX would normally trip on platforms where
        // unsigned long is 64-bit. We mirror that: any nonzero negative
        // input would yield a value > UINT_MAX on 64-bit systems, hence
        // error. For value == 0, negation yields 0, which is valid.
        if v != 0 {
            return None;
        }
        v = 0u32.wrapping_sub(v);
    }

    Some(v)
}
