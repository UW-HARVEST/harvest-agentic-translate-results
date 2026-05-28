// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical behavior.

use std::ffi::{c_char, c_int};
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

/// Length of a NUL-terminated C string. Caller must ensure the pointer is valid.
unsafe fn cstrlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    n
}

/// Mirrors create_result_string: produces a malloc'd 64-byte buffer with snprintf-formatted text.
/// The returned pointer is allocated with libc::malloc and must be freed with libc::free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let buf = unsafe { libc::malloc(64) } as *mut c_char;
    if buf.is_null() {
        return std::ptr::null_mut();
    }
    // Build "Operation: %s, Value: %d" using snprintf via libc to match exactly.
    let fmt = b"Operation: %s, Value: %d\0";
    unsafe {
        libc::snprintf(buf as *mut _, 64, fmt.as_ptr() as *const _, op, val);
    }
    buf
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    if (perms & required) == required { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        cprint("Insufficient permissions for addition\n");
        return 0;
    }
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    let op = b"multiply\0";
    let s = unsafe { create_result_string(op.as_ptr() as *const c_char, a.wrapping_mul(b)) };
    unsafe {
        *log_msg = s;
    }
    if s.is_null() {
        return 0;
    }
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        cprint("Source pointer is NULL\n");
        return -1;
    }

    let n = count as usize;
    let bytes = n.checked_mul(std::mem::size_of::<c_int>()).unwrap_or(0);
    let dest = unsafe { libc::malloc(bytes) } as *mut c_int;
    if dest.is_null() {
        cprint("Memory allocation failed\n");
        return -1;
    }

    unsafe {
        libc::memcpy(dest as *mut _, src as *const _, bytes);
    }

    let mut sum: c_int = 0;
    for i in 0..n {
        sum = sum.wrapping_add(unsafe { *dest.add(i) });
    }

    unsafe { libc::free(dest as *mut _) };
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        cprint("One or both operation strings are NULL\n");
        return -1;
    }
    unsafe { libc::strcmp(op1, op2) }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result: c_int;
    let mut log_message: *mut c_char = std::ptr::null_mut();

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
            result = unsafe { multiply_with_log(value1, value2, &mut log_message) };
            res_tracker.value = result;

            if log_message.is_null() {
                cprint("Log message creation failed\n");
            } else {
                let len = unsafe { cstrlen(log_message) };
                if len == 0 {
                    cprint("Log message creation failed\n");
                } else {
                    let bytes = unsafe { std::slice::from_raw_parts(log_message as *const u8, len) };
                    let s = std::str::from_utf8(bytes).unwrap_or("");
                    cprint(&format!("Mode 2: {}\n", s));
                    unsafe { libc::free(log_message as *mut _) };
                }
            }
        }
        3 => {
            strcpy_into(&mut res_tracker.operation, b"array_sum");
            let mut values: [c_int; 3] = [value1, value2, value3];
            result = unsafe { copy_and_sum(values.as_mut_ptr(), 3) };
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
        let len = res_tracker
            .operation
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(res_tracker.operation.len());
        let op_str = std::str::from_utf8(&res_tracker.operation[..len]).unwrap_or("");
        cprint(&format!("Operation performed: {}\n", op_str));
    }

    result
}
