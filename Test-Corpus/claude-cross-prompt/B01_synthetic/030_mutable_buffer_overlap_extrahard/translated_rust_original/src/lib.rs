use std::ffi::c_int;
use std::io::{self, Read, Write};

/// Fused multiply-add over arrays: out[i] = mul1[i] * mul2[i] + add[i]
///
/// # Safety
/// All pointers must be valid for `len` elements. The original C code performs
/// no aliasing checks, so callers may pass overlapping pointers (the driver
/// function does this). The semantics match the C version exactly: each
/// element is read in order and the result written immediately, so callers
/// passing aliased pointers see the same observable behavior as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let n = len as isize;
    let mut i: isize = 0;
    while i < n {
        let m1 = unsafe { *mul1.offset(i) };
        let m2 = unsafe { *mul2.offset(i) };
        let a = unsafe { *add.offset(i) };
        // Wrapping arithmetic to match C signed overflow behavior in practice
        // (C signed overflow is UB; on typical platforms it wraps).
        let v = m1.wrapping_mul(m2).wrapping_add(a);
        unsafe { *out.offset(i) = v };
        i += 1;
    }
}

/// Driver function: applies fma_array with all inputs aliased to `out`,
/// then prints each element followed by a newline.
///
/// # Safety
/// `out` must point to a valid array of `len` `c_int` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(out: *mut c_int, len: c_int) {
    unsafe { fma_array(out, out as *const c_int, out as *const c_int, out as *const c_int, len) };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let n = len as isize;
    let mut i: isize = 0;
    while i < n {
        let v = unsafe { *out.offset(i) };
        // Match printf("%d\n", v) byte-identically.
        let _ = writeln!(handle, "{}", v);
        i += 1;
    }
    let _ = handle.flush();
}

/// Entry point matching the C `main` function. Reads up to 100 integers from
/// stdin (parsed by whitespace-separated `scanf("%d")` semantics), then calls
/// `driver` with the actual count read.
#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut data: [c_int; 100] = [0; 100];
    let mut count: usize = 0;

    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_ok() {
        // scanf("%d", ...) skips leading whitespace and parses an optional
        // sign followed by digits, stopping at the first non-numeric char.
        // We approximate this by tokenizing on ASCII whitespace and parsing
        // each token as a base-10 i32.
        let bytes = buf.as_bytes();
        let mut idx: usize = 0;
        while count < 100 {
            // Skip whitespace
            while idx < bytes.len() && (bytes[idx] as char).is_ascii_whitespace() {
                idx += 1;
            }
            if idx >= bytes.len() {
                break;
            }
            // Parse optional sign
            let start = idx;
            if bytes[idx] == b'+' || bytes[idx] == b'-' {
                idx += 1;
            }
            let digit_start = idx;
            while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
                idx += 1;
            }
            if idx == digit_start {
                // No digits — scanf would fail to match.
                break;
            }
            let token = &buf[start..idx];
            match token.parse::<c_int>() {
                Ok(v) => {
                    data[count] = v;
                    count += 1;
                }
                Err(_) => break,
            }
        }
    }

    unsafe { driver(data.as_mut_ptr(), count as c_int) };
    0
}
