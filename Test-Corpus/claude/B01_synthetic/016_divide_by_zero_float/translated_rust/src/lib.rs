// Translated from c_src/src/main.c
// Reproduces C behavior, including buggy behavior in bad().
//
// This crate is built as both an `rlib` (so the binary can use it) and a
// `cdylib` so an external test harness can load it via libloading and call
// the same exported C ABI symbols that the C shared library exports.

use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int};

pub const CHAR_ARRAY_SIZE: usize = 20;

// ---------------------------------------------------------------------------
// Internal helpers used by both the binary and the C ABI wrappers.
// ---------------------------------------------------------------------------

/// Print a line plus newline (only when the pointer/string is not NULL).
fn print_line_str(line: &str) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(line.as_bytes());
    let _ = h.write_all(b"\n");
    let _ = h.flush();
}

/// Print an integer plus newline.
fn print_int_line_i(n: c_int) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = write!(h, "{}\n", n);
    let _ = h.flush();
}

/// Read up to `buf_size - 1` bytes from `reader`, stopping after a newline
/// (newline included) or at EOF. Returns `None` if no bytes were read at EOF
/// (mimicking C's fgets returning NULL).
pub fn fgets<R: Read>(reader: &mut R, buf_size: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    if buf_size == 0 {
        return None;
    }
    while buf.len() + 1 < buf_size {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimic C's `atof`: skip leading whitespace, accept an optional sign,
/// digits with optional fractional part, and an optional exponent.
/// Returns 0.0 on parse failure.
pub fn c_atof(bytes: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < bytes.len()
        && matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
        )
    {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digits = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digits = true;
            i += 1;
        }
    }
    if !has_digits {
        return 0.0;
    }
    let mut end = i;
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let mut exp_has_digits = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            exp_has_digits = true;
            j += 1;
        }
        if exp_has_digits {
            end = j;
        }
    }
    let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
    s.parse::<f64>().unwrap_or(0.0)
}

/// Mimic the x86 `cvttsd2si` semantics used by typical C compilers when
/// casting a double to int: NaN or out-of-range values yield INT_MIN
/// (the "indefinite integer value"). Otherwise truncate toward zero.
pub fn double_to_int_c(x: f64) -> c_int {
    if x.is_nan() {
        return c_int::MIN;
    }
    let t = x.trunc();
    if t > 2_147_483_647.0_f64 || t < -2_147_483_648.0_f64 {
        return c_int::MIN;
    }
    t as c_int
}

// ---------------------------------------------------------------------------
// Internal high-level helpers shared between bin and FFI.
// ---------------------------------------------------------------------------

pub fn bad_impl<R: Read>(reader: &mut R) {
    let mut data: f32 = 0.0;
    match fgets(reader, CHAR_ARRAY_SIZE) {
        Some(input_buffer) => {
            data = c_atof(&input_buffer) as f32;
        }
        None => {
            print_line_str("fgets() failed.");
        }
    }
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line_i(result);
}

pub fn good_g2b_impl() {
    let data: f32 = 2.0;
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line_i(result);
}

pub fn good_b2g_impl<R: Read>(reader: &mut R) {
    let mut data: f32 = 0.0;
    match fgets(reader, CHAR_ARRAY_SIZE) {
        Some(input_buffer) => {
            data = c_atof(&input_buffer) as f32;
        }
        None => {
            print_line_str("fgets() failed.");
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line_i(result);
    } else {
        print_line_str("This would result in a divide by zero");
    }
}

pub fn good_impl<R: Read>(reader: &mut R) {
    good_g2b_impl();
    good_b2g_impl(reader);
}

// ---------------------------------------------------------------------------
// C ABI exports — match the C source's external symbols exactly.
// ---------------------------------------------------------------------------

/// Mirror of `void printLine(const char * line)`.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        return;
    }
    // C uses printf("%s\n", line); replicate by writing the bytes then '\n'.
    let cstr = CStr::from_ptr(line);
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(cstr.to_bytes());
    let _ = h.write_all(b"\n");
    let _ = h.flush();
}

/// Mirror of `void printIntLine(int intNumber)`.
#[no_mangle]
pub extern "C" fn printIntLine(int_number: c_int) {
    print_int_line_i(int_number);
}

/// Mirror of `void bad()`.
#[no_mangle]
pub extern "C" fn bad() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    bad_impl(&mut handle);
}

/// Mirror of `void good()`.
#[no_mangle]
pub extern "C" fn good() {
    good_g2b_impl();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    good_b2g_impl(&mut handle);
}

/// Mirror of `int main(int argc, char *argv[])`.
/// The C source's `main` is exported by the shared library; we replicate the
/// same symbol so the Rust `.so` exports the full surface area of the C `.so`.
///
/// We only emit this when *not* compiling the `cargo test` harness, because
/// the test harness generates its own `main` and would clash with this one.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let stdout_msg_calling_good = b"Calling good()...\n";
    let stdout_msg_finished_good = b"Finished good()\n";
    let stdout_msg_calling_bad = b"Calling bad()...\n";
    let stdout_msg_finished_bad = b"Finished bad()\n";
    let stdout = io::stdout();
    {
        let mut h = stdout.lock();
        let _ = h.write_all(stdout_msg_calling_good);
        let _ = h.flush();
    }
    good();
    {
        let mut h = stdout.lock();
        let _ = h.write_all(stdout_msg_finished_good);
        let _ = h.write_all(stdout_msg_calling_bad);
        let _ = h.flush();
    }
    bad();
    {
        let mut h = stdout.lock();
        let _ = h.write_all(stdout_msg_finished_bad);
        let _ = h.flush();
    }
    0
}
