// Library wrapper exposing the same C-ABI symbols as the C source.
//
// The C source defines:
//   void fma_array(int *restrict out, const int *mul1, const int *mul2,
//                  const int *add, int len);
//   int  call_fma(const int *data, int len);
//
// We match those exact signatures here.

use std::os::raw::c_int;
use std::slice;

pub fn fma_array_safe(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

pub fn call_fma_safe(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array_safe(&mut out, &ones, &data[..len], &zeros, len);
    out[len - 1]
}

/// # Safety
/// `out`, `mul1`, `mul2`, `add` must each point to at least `len` valid `c_int` elements.
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    if len <= 0 {
        return;
    }
    let n = len as usize;
    let out_s = slice::from_raw_parts_mut(out, n);
    let mul1_s = slice::from_raw_parts(mul1, n);
    let mul2_s = slice::from_raw_parts(mul2, n);
    let add_s = slice::from_raw_parts(add, n);
    fma_array_safe(out_s, mul1_s, mul2_s, add_s, n);
}

/// # Safety
/// `data` must point to at least `len` valid `c_int` elements (when `len > 0`).
#[no_mangle]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len <= 0 {
        return 0;
    }
    let n = len as usize;
    let data_s = slice::from_raw_parts(data, n);
    call_fma_safe(data_s, n)
}

// The C source defines `main`, so the C-built shared library exports `main`.
// Mirror that export here so the Rust `cdylib` exposes the same symbol set.
// Reads integers from stdin (mimicking scanf("%d", ...)) and writes the
// `call_fma` result followed by a newline to stdout.
//
// We must NOT define `#[no_mangle] fn main` when compiling for tests — the
// Rust test harness already provides its own `main` and would conflict.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    use std::io::{Read, Write};

    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);

    let mut data = [0i32; 100];
    let mut i: usize = 0;
    let mut pos: usize = 0;
    while i < 100 {
        // Skip whitespace
        while pos < buf.len() && matches!(buf[pos], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            pos += 1;
        }
        if pos >= buf.len() {
            break;
        }
        let start = pos;
        let mut sign: i64 = 1;
        if buf[pos] == b'+' {
            pos += 1;
        } else if buf[pos] == b'-' {
            sign = -1;
            pos += 1;
        }
        let digits_start = pos;
        while pos < buf.len() && buf[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == digits_start {
            // Match failure; mimic scanf returning != 1 (we just stop).
            let _ = start; // quiet "unused" if optimized; preserves intent.
            break;
        }
        let mut v: i64 = 0;
        for &b in &buf[digits_start..pos] {
            v = v.wrapping_mul(10).wrapping_add((b - b'0') as i64);
        }
        v = v.wrapping_mul(sign);
        data[i] = v as i32;
        i += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), i as c_int) };
    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    let _ = writeln!(h, "{}", result);
    0
}
