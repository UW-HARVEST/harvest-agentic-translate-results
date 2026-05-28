// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical behavior.

use std::ffi::c_int;
use std::io::Write;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

struct Result {
    value: c_int,
    operation: [u8; 32],
    permissions: c_int,
}

/// Print to stdout with a flush so behavior matches C's printf line-buffered output.
fn cprint(s: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

/// Replicates C's strcpy into a fixed buffer of size 32.
/// Writes bytes from src and a trailing NUL.
fn strcpy_into(dest: &mut [u8; 32], src: &[u8]) {
    // Mirror C strcpy: copy bytes including final NUL. If src is too long this
    // would be UB in C, but inputs here are short string literals.
    let n = src.len().min(31);
    dest[..n].copy_from_slice(&src[..n]);
    dest[n] = 0;
    // Zero the rest for cleanliness (not strictly required by C semantics).
    for b in dest[n + 1..].iter_mut() {
        *b = 0;
    }
}

/// Compares the C-string content of buffer with a given byte literal (without NUL).
fn cstr_eq(buf: &[u8; 32], s: &[u8]) -> bool {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..len] == s
}

/// Mirrors create_result_string: produces a heap string up to 63 chars + NUL.
/// Returns Some(String) — None branch is unreachable here since allocation cannot fail in safe Rust.
fn create_result_string(op: &str, val: c_int) -> Option<String> {
    // snprintf(str, 64, "Operation: %s, Value: %d", op, val);
    // produces at most 63 chars (then NUL). Truncate accordingly.
    let formatted = format!("Operation: {}, Value: {}", op, val);
    let truncated: String = formatted.chars().scan(0usize, |len, c| {
        let clen = c.len_utf8();
        if *len + clen > 63 {
            None
        } else {
            *len += clen;
            Some(c)
        }
    }).collect();
    Some(truncated)
}

fn check_permissions(perms: c_int, required: c_int) -> c_int {
    if (perms & required) == required { 1 } else { 0 }
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        cprint("Insufficient permissions for addition\n");
        return 0;
    }
    a.wrapping_add(b)
}

fn multiply_with_log(a: c_int, b: c_int, log_msg: &mut Option<String>) -> c_int {
    *log_msg = create_result_string("multiply", a.wrapping_mul(b));
    if log_msg.is_none() {
        return 0;
    }
    a.wrapping_mul(b)
}

fn copy_and_sum(src: Option<&[c_int]>, count: c_int) -> c_int {
    let src = match src {
        None => {
            cprint("Source pointer is NULL\n");
            return -1;
        }
        Some(s) => s,
    };

    // Mirror malloc(count * sizeof(int)). In Rust we just use Vec.
    let n = count as usize;
    let mut dest: Vec<c_int> = vec![0; n];
    // memcpy
    dest[..n].copy_from_slice(&src[..n]);

    let mut sum: c_int = 0;
    for i in 0..n {
        sum = sum.wrapping_add(dest[i]);
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: Option<String> = None;

    let permissions: c_int = 0o644; // rw-r--r--

    let mut res_tracker = Result {
        value: 0,
        operation: [0u8; 32],
        permissions,
    };
    res_tracker.value = 0;
    res_tracker.permissions = permissions;
    strcpy_into(&mut res_tracker.operation, b"none");

    match mode {
        1 => {
            strcpy_into(&mut res_tracker.operation, b"addition");
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;

            cprint("Mode 1: Addition\n");
            cprint(&format!("Result: {}\n", result));
        }
        2 => {
            strcpy_into(&mut res_tracker.operation, b"multiplication");
            result = multiply_with_log(value1, value2, &mut log_message);
            res_tracker.value = result;

            match &log_message {
                None => {
                    cprint("Log message creation failed\n");
                }
                Some(msg) if msg.is_empty() => {
                    cprint("Log message creation failed\n");
                }
                Some(msg) => {
                    cprint(&format!("Mode 2: {}\n", msg));
                    // free(log_message) — drop happens automatically.
                }
            }
        }
        3 => {
            strcpy_into(&mut res_tracker.operation, b"array_sum");
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(Some(&values), 3);
            res_tracker.value = result;

            cprint("Mode 3: Array Sum\n");
            cprint(&format!("Result: {}\n", result));
        }
        4 => {
            strcpy_into(&mut res_tracker.operation, b"complex");

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            res_tracker.value = result;
            cprint("Mode 4: Complex Calculation\n");
            cprint(&format!("Result: {}\n", result));
        }
        _ => {
            cprint("Invalid mode\n");
            result = -1;
        }
    }

    if !cstr_eq(&res_tracker.operation, b"none") {
        // Convert operation cstr to &str for printing.
        let len = res_tracker
            .operation
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(res_tracker.operation.len());
        let op_str = std::str::from_utf8(&res_tracker.operation[..len]).unwrap_or("");
        cprint(&format!("Operation performed: {}\n", op_str));
    }

    // res_tracker is dropped automatically (replaces free()).
    result
}

// Suppress unused warnings for helpers that might be optimized out.
#[allow(dead_code)]
fn _used() {
    let _ = compare_operations;
}

#[allow(dead_code)]
fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> c_int {
    if op1.is_none() || op2.is_none() {
        cprint("One or both operation strings are NULL\n");
        return -1;
    }
    let a = op1.unwrap().as_bytes();
    let b = op2.unwrap().as_bytes();
    // Mirror strcmp behavior
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return (a[i] as c_int) - (b[i] as c_int);
        }
    }
    (a.len() as c_int) - (b.len() as c_int)
}
