// Translated from c_src/src/lib.c — preserves byte-identical stdout output.

use std::ffi::c_int;
use std::io::Write;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

#[repr(C)]
struct Result {
    value: c_int,
    operation: [u8; 32],
    permissions: c_int,
}

/// Print exactly like C printf to stdout (no extra buffering layer beyond
/// what stdout already gives us). We use write! on stdout directly so that
/// flushing semantics match a typical C program.
fn cprint(s: &str) {
    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(s.as_bytes());
    let _ = h.flush();
}

fn write_to_buf(buf: &mut [u8], s: &str) -> usize {
    // Mimic snprintf: write up to buf.len() - 1 bytes, NUL-terminate.
    if buf.is_empty() {
        return 0;
    }
    let max = buf.len() - 1;
    let bytes = s.as_bytes();
    let n = bytes.len().min(max);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
    n
}

fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn create_result_string(op: &str, val: c_int) -> Option<Vec<u8>> {
    // Allocate a 64-byte buffer and snprintf into it, mirroring C semantics.
    let formatted = format!("Operation: {}, Value: {}", op, val);
    let mut buf = vec![0u8; 64];
    write_to_buf(&mut buf, &formatted);
    Some(buf)
}

fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        cprint("Insufficient permissions for addition\n");
        return 0;
    }
    a.wrapping_add(b)
}

fn multiply_with_log(a: c_int, b: c_int, log_msg: &mut Option<Vec<u8>>) -> c_int {
    let product = a.wrapping_mul(b);
    *log_msg = create_result_string("multiply", product);
    if log_msg.is_none() {
        return 0;
    }
    product
}

fn copy_and_sum(src: &[c_int]) -> c_int {
    // src is guaranteed non-null and length matches `count` from the caller;
    // we replicate the C behaviour of allocating a temporary buffer, copying,
    // summing, and freeing it. Sum uses wrapping addition to mirror C int.
    let dest: Vec<c_int> = src.to_vec();
    let mut sum: c_int = 0;
    for &v in &dest {
        sum = sum.wrapping_add(v);
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    let result: c_int;
    let mut log_message: Option<Vec<u8>> = None;

    let permissions: c_int = 0o644; // rw-r--r--

    let mut res_tracker = Box::new(Result {
        value: 0,
        operation: [0u8; 32],
        permissions: 0,
    });

    res_tracker.value = 0;
    res_tracker.permissions = permissions;
    set_operation(&mut res_tracker.operation, "none");

    match mode {
        1 => {
            set_operation(&mut res_tracker.operation, "addition");
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;

            cprint("Mode 1: Addition\n");
            cprint(&format!("Result: {}\n", result));
        }
        2 => {
            set_operation(&mut res_tracker.operation, "multiplication");
            result = multiply_with_log(value1, value2, &mut log_message);
            res_tracker.value = result;

            match &log_message {
                None => {
                    cprint("Log message creation failed\n");
                }
                Some(buf) => {
                    let len = cstr_len(buf);
                    if len == 0 {
                        cprint("Log message creation failed\n");
                    } else {
                        let s = std::str::from_utf8(&buf[..len]).unwrap_or("");
                        cprint(&format!("Mode 2: {}\n", s));
                    }
                }
            }
        }
        3 => {
            set_operation(&mut res_tracker.operation, "array_sum");
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(&values);
            res_tracker.value = result;

            cprint("Mode 3: Array Sum\n");
            cprint(&format!("Result: {}\n", result));
        }
        4 => {
            set_operation(&mut res_tracker.operation, "complex");

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

    let op_len = cstr_len(&res_tracker.operation);
    let op_str = std::str::from_utf8(&res_tracker.operation[..op_len]).unwrap_or("");
    if op_str != "none" {
        cprint(&format!("Operation performed: {}\n", op_str));
    }

    // Box drop frees the tracker, mirroring free(res_tracker).
    drop(res_tracker);

    result
}

fn set_operation(buf: &mut [u8; 32], s: &str) {
    // Mirror strcpy: copy bytes followed by NUL terminator. Caller guarantees
    // s.len() < 32 — all literal strings here ("none", "addition", etc.) fit.
    for b in buf.iter_mut() {
        *b = 0;
    }
    let bytes = s.as_bytes();
    let n = bytes.len().min(31);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
}
