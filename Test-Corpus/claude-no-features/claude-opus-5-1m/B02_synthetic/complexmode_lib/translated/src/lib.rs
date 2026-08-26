// Translation of c_src/src/lib.c to Rust.
// Preserves exact behavior and output (byte-identical) of the original C code.

use std::ffi::{c_char, c_int};

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

// Mirror of the C `Result` struct (kept for parity, though only used internally).
#[repr(C)]
struct ResultTracker {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

/// Replicates `snprintf(str, 64, "Operation: %s, Value: %d", op, val)`
/// and returns a heap-allocated NUL-terminated buffer of size 64.
fn create_result_string(op: &str, val: c_int) -> Vec<u8> {
    let formatted = format!("Operation: {}, Value: {}", op, val);
    // snprintf with size 64 writes at most 63 bytes plus a terminating NUL.
    let mut buf = vec![0u8; 64];
    let bytes = formatted.as_bytes();
    let n = bytes.len().min(63);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf[n] = 0;
    buf
}

fn check_permissions(perms: c_int, required: c_int) -> bool {
    (perms & required) == required
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        unsafe {
            libc::printf(b"Insufficient permissions for addition\n\0".as_ptr() as *const c_char);
        }
        return 0;
    }
    a.wrapping_add(b)
}

fn multiply_with_log(a: c_int, b: c_int) -> (c_int, Option<Vec<u8>>) {
    let product = a.wrapping_mul(b);
    let buf = create_result_string("multiply", product);
    (product, Some(buf))
}

fn copy_and_sum(src: &[c_int]) -> c_int {
    // src is never NULL here (safe Rust slice). Mirrors the
    // post-NULL-check, post-allocation sum computation.
    let mut sum: c_int = 0;
    for &v in src {
        sum = sum.wrapping_add(v);
    }
    sum
}

#[allow(dead_code)]
fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> c_int {
    match (op1, op2) {
        (Some(a), Some(b)) => {
            // mimic strcmp semantics
            let ab = a.as_bytes();
            let bb = b.as_bytes();
            let len = ab.len().min(bb.len());
            for i in 0..len {
                if ab[i] != bb[i] {
                    return (ab[i] as c_int) - (bb[i] as c_int);
                }
            }
            (ab.len() as c_int) - (bb.len() as c_int)
        }
        _ => {
            unsafe {
                libc::printf(
                    b"One or both operation strings are NULL\n\0".as_ptr() as *const c_char,
                );
            }
            -1
        }
    }
}

/// Sets the `operation` field on the result tracker by copying the
/// given NUL-terminated bytes (must fit in 32 bytes including the NUL).
fn set_operation(tracker: &mut ResultTracker, bytes: &[u8]) {
    // strcpy semantics: copy bytes plus terminating NUL. Source is a
    // string literal so we know it fits.
    for (i, &b) in bytes.iter().enumerate() {
        tracker.operation[i] = b as c_char;
    }
    tracker.operation[bytes.len()] = 0;
}

/// Returns the operation field as a byte slice, up to (but not including)
/// the first NUL terminator.
fn operation_bytes(tracker: &ResultTracker) -> &[u8] {
    let mut len = 0;
    while len < tracker.operation.len() && tracker.operation[len] != 0 {
        len += 1;
    }
    // SAFETY: c_char is i8, safe to reinterpret as u8 for read.
    unsafe { std::slice::from_raw_parts(tracker.operation.as_ptr() as *const u8, len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    #[allow(unused_assignments)]
    let mut result: c_int = 0;

    let permissions: c_int = 0o644; // rw-r--r--

    let mut res_tracker = Box::new(ResultTracker {
        value: 0,
        operation: [0; 32],
        permissions,
    });

    res_tracker.value = 0;
    res_tracker.permissions = permissions;
    set_operation(&mut res_tracker, b"none");

    match mode {
        1 => {
            set_operation(&mut res_tracker, b"addition");
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;

            unsafe {
                libc::printf(b"Mode 1: Addition\n\0".as_ptr() as *const c_char);
                libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }
        2 => {
            set_operation(&mut res_tracker, b"multiplication");
            let (product, log_message) = multiply_with_log(value1, value2);
            result = product;
            res_tracker.value = result;

            match log_message {
                None => unsafe {
                    libc::printf(b"Log message creation failed\n\0".as_ptr() as *const c_char);
                },
                Some(buf) => {
                    // Determine NUL-terminated length to compare with empty string.
                    let mut len = 0usize;
                    while len < buf.len() && buf[len] != 0 {
                        len += 1;
                    }
                    if len == 0 {
                        unsafe {
                            libc::printf(
                                b"Log message creation failed\n\0".as_ptr() as *const c_char,
                            );
                        }
                    } else {
                        unsafe {
                            libc::printf(
                                b"Mode 2: %s\n\0".as_ptr() as *const c_char,
                                buf.as_ptr() as *const c_char,
                            );
                        }
                        // Buf is dropped automatically (mirrors free()).
                    }
                }
            }
        }
        3 => {
            set_operation(&mut res_tracker, b"array_sum");
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(&values);
            res_tracker.value = result;

            unsafe {
                libc::printf(b"Mode 3: Array Sum\n\0".as_ptr() as *const c_char);
                libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }
        4 => {
            set_operation(&mut res_tracker, b"complex");

            if check_permissions(permissions, 0o100) {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            res_tracker.value = result;
            unsafe {
                libc::printf(b"Mode 4: Complex Calculation\n\0".as_ptr() as *const c_char);
                libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }
        _ => {
            unsafe {
                libc::printf(b"Invalid mode\n\0".as_ptr() as *const c_char);
            }
            result = -1;
        }
    }

    let op = operation_bytes(&res_tracker);
    if op != b"none" {
        // Build a NUL-terminated buffer for the operation to pass through %s.
        let mut nul_term = Vec::with_capacity(op.len() + 1);
        nul_term.extend_from_slice(op);
        nul_term.push(0);
        unsafe {
            libc::printf(
                b"Operation performed: %s\n\0".as_ptr() as *const c_char,
                nul_term.as_ptr() as *const c_char,
            );
        }
    }

    // Box drops automatically (mirrors free()).
    drop(res_tracker);

    result
}
