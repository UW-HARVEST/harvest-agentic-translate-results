// Translated from C library lib.c.
// Original copyright 2025 MIT Lincoln Laboratory.
//
// The original C is a shared library exposing `buffapp(a, b, c, d)`.
// This executable reads four integers from stdin (scanf-style: whitespace
// or newlines separate values) and invokes `buffapp` so that its stdout
// output is byte-identical to a C driver doing the same thing.

use std::io::{self, Read, Write};

// ---------- StringBuffer ----------

struct StringBuffer {
    data: Vec<u8>, // contains a NUL terminator at index `length`
    capacity: i32,
    length: i32,
}

fn create_buffer(initial_capacity: i32) -> Option<Box<StringBuffer>> {
    if initial_capacity <= 0 {
        // C's malloc(0) is implementation-defined; we treat non-positive as failure
        // and treat the rest like the C original (which assumed >0 implicitly).
        return None;
    }
    let cap_usize = initial_capacity as usize;
    let mut data = vec![0u8; cap_usize];
    // mimic `buffer->data[0] = '\0';`
    data[0] = 0;

    Some(Box::new(StringBuffer {
        data,
        capacity: initial_capacity,
        length: 0,
    }))
}

fn append_to_buffer(buffer: &mut StringBuffer, s: &[u8]) -> i32 {
    let str_len = s.len() as i32;
    let required_capacity = buffer.length + str_len + 1;

    if required_capacity > buffer.capacity {
        let new_capacity = required_capacity.wrapping_mul(2);
        if new_capacity <= 0 {
            return -1;
        }
        buffer.data.resize(new_capacity as usize, 0);
        buffer.capacity = new_capacity;
    }

    // strcpy from s into buffer.data[buffer.length..]
    let start = buffer.length as usize;
    let end = start + s.len();
    buffer.data[start..end].copy_from_slice(s);
    // strcpy writes a trailing NUL.
    if end < buffer.data.len() {
        buffer.data[end] = 0;
    }
    buffer.length += str_len;

    0
}

fn buffer_as_cstr<'a>(buffer: &'a StringBuffer) -> &'a [u8] {
    // Up to but not including the NUL terminator at `length`.
    &buffer.data[..buffer.length as usize]
}

// ---------- operation helpers ----------

fn get_operation_name(op_code: i32) -> &'static str {
    match op_code {
        0 => "add",
        1 => "subtract",
        2 => "multiply",
        3 => "divide",
        _ => "unknown",
    }
}

fn perform_operation(a: i32, b: i32, operation: &str) -> i32 {
    match operation {
        "add" => a.wrapping_add(b),
        "subtract" => a.wrapping_sub(b),
        "multiply" => a.wrapping_mul(b),
        "divide" => {
            if b != 0 {
                // C's `/` for ints truncates toward zero; Rust's `/` does too,
                // but it panics on i32::MIN / -1. Use wrapping_div to mimic C
                // (which is UB but typically wraps to i32::MIN on 2's complement).
                a.wrapping_div(b)
            } else {
                0
            }
        }
        _ => 0,
    }
}

// ---------- buffapp ----------

fn buffapp(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut log_buffer = create_buffer(32).expect("create_buffer failed");
    let mut result: i32 = 0;

    log_buffer.length = 0;

    // "Starting computation with %d parameters\n", 4
    let s = format!("Starting computation with {} parameters\n", 4);
    append_to_buffer(&mut log_buffer, s.as_bytes());

    // C uses % which for negative operands returns a value with the sign of
    // the dividend. Rust's % does the same for i32, so this matches.
    let op1 = get_operation_name(param1 % 4);
    let s = format!("Operation 1: {}({}, {})\n", op1, param1, param2);
    append_to_buffer(&mut log_buffer, s.as_bytes());

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3 % 4);
    let s = format!("Operation 2: {}({}, {})\n", op2, param3, param4);
    append_to_buffer(&mut log_buffer, s.as_bytes());

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    let s = format!(
        "Operation 3: {}({}, {})\n",
        op3, intermediate1, intermediate2
    );
    append_to_buffer(&mut log_buffer, s.as_bytes());

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result.wrapping_div(intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    let s = format!("Final result: {}\n", result);
    append_to_buffer(&mut log_buffer, s.as_bytes());

    // printf("Computation Log:\n%s\n", log_buffer->data);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(b"Computation Log:\n").unwrap();
    out.write_all(buffer_as_cstr(&log_buffer)).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();

    // destroy_buffer happens implicitly when log_buffer drops.
    drop(log_buffer);

    result
}

// ---------- scanf-style stdin parsing ----------

/// Read all of stdin and parse 4 whitespace-separated i32 values, matching
/// C's `scanf("%d %d %d %d", ...)`. scanf skips leading whitespace
/// (including newlines), then reads an optional sign and digits.
fn read_four_ints() -> Option<(i32, i32, i32, i32)> {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return None;
    }
    let bytes = input.as_bytes();
    let mut i = 0usize;

    let mut parse_one = || -> Option<i32> {
        // skip whitespace
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        let start = i;
        if bytes[i] == b'+' || bytes[i] == b'-' {
            i += 1;
        }
        let digit_start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        if i == digit_start {
            return None;
        }
        let s = std::str::from_utf8(&bytes[start..i]).ok()?;
        // C scanf with %d on overflow is UB; use wrapping parse via i64 then cast.
        match s.parse::<i64>() {
            Ok(v) => Some(v as i32),
            Err(_) => None,
        }
    };

    let a = parse_one()?;
    let b = parse_one()?;
    let c = parse_one()?;
    let d = parse_one()?;
    Some((a, b, c, d))
}

fn main() {
    let (a, b, c, d) = match read_four_ints() {
        Some(v) => v,
        None => return, // mimic minimal failure path
    };
    let _ = buffapp(a, b, c, d);
}
