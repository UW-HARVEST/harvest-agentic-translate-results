// Rust translation of c_src/src/main.c
//
// Takes two arguments, a base and an exponent, and prints base^exponent.
//
// The original C program relies on the platform C library for three
// behaviours that have no exact equivalent in the Rust standard library:
//
//   * `strtod` parsing (leading whitespace, hex floats, `inf`/`nan`, the
//     empty string converting to 0.0, and `ERANGE` for values that
//     overflow or lose precision in the subnormal range),
//   * `pow` reporting `EDOM`/`ERANGE` through `errno`,
//   * `printf("%.2f")` rounding and its `inf`/`nan` spellings.
//
// To stay byte-identical those three primitives are called directly in the
// C library; everything else is plain safe Rust.

use std::ffi::{c_int, CString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

const EDOM: c_int = 33;
const ERANGE: c_int = 34;

mod libc_shim {
    use std::ffi::{c_char, c_int, CStr};

    extern "C" {
        fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
        fn pow(x: f64, y: f64) -> f64;
        fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
        fn __errno_location() -> *mut c_int;
    }

    fn errno() -> c_int {
        unsafe { *__errno_location() }
    }

    fn set_errno(value: c_int) {
        unsafe { *__errno_location() = value }
    }

    /// `errno = 0; v = strtod(s, &end);` returning the parsed value, the
    /// number of bytes consumed, and the resulting `errno`.
    pub fn strtod_checked(s: &CStr) -> (f64, usize, c_int) {
        let start = s.as_ptr();
        let mut end: *mut c_char = std::ptr::null_mut();
        set_errno(0);
        let value = unsafe { strtod(start, &mut end) };
        let err = errno();
        let consumed = (end as usize) - (start as usize);
        (value, consumed, err)
    }

    /// `errno = 0; r = pow(x, y);` returning the result and `errno`.
    pub fn pow_checked(x: f64, y: f64) -> (f64, c_int) {
        set_errno(0);
        let result = unsafe { pow(x, y) };
        (result, errno())
    }

    /// Formats `value` exactly the way C's `printf("%.2f", value)` would.
    pub fn format_fixed2(value: f64) -> Vec<u8> {
        const FMT: *const c_char = c"%.2f".as_ptr();

        let needed = unsafe { snprintf(std::ptr::null_mut(), 0, FMT, value) };
        if needed < 0 {
            return Vec::new();
        }
        let needed = needed as usize;
        let mut buf = vec![0u8; needed + 1];
        unsafe {
            snprintf(buf.as_mut_ptr() as *mut c_char, buf.len(), FMT, value);
        }
        buf.truncate(needed);
        buf
    }
}

use libc_shim::{format_fixed2, pow_checked, strtod_checked};

fn stderr_write(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

fn stdout_write(bytes: &[u8]) {
    let mut out = std::io::stdout();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

fn main() -> ExitCode {
    // Raw argv bytes: the C program passes them straight to strtod and to
    // printf's "%s", so they must not go through UTF-8 validation.
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|arg| arg.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    if argc != 3 {
        // glibc prints "(null)" for a NULL "%s" argument, which is what an
        // empty argv would produce in the C original.
        let program: &[u8] = argv.first().map_or(b"(null)", |a| a.as_slice());
        let mut msg = b"Usage: ".to_vec();
        msg.extend_from_slice(program);
        msg.extend_from_slice(b" base exponent\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    }

    // Convert base
    let arg1 = CString::new(argv[1].clone()).expect("argv cannot contain NUL");
    let (base, consumed1, errno1) = strtod_checked(&arg1);
    if errno1 == ERANGE {
        let mut msg = b"Range error while converting base '".to_vec();
        msg.extend_from_slice(&argv[1]);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    } else if consumed1 != arg1.as_bytes().len() {
        // i.e. *endptr != '\0'
        let mut msg = b"Invalid numeric input for base: '".to_vec();
        msg.extend_from_slice(&argv[1]);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    }

    // Convert exponent
    let arg2 = CString::new(argv[2].clone()).expect("argv cannot contain NUL");
    let (exponent, consumed2, errno2) = strtod_checked(&arg2);
    if errno2 == ERANGE {
        let mut msg = b"Range error while converting exponent '".to_vec();
        msg.extend_from_slice(&argv[2]);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    } else if consumed2 != arg2.as_bytes().len() {
        let mut msg = b"Invalid numeric input for exponent: '".to_vec();
        msg.extend_from_slice(&argv[2]);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    }

    // Calculate power
    let (result, errno3) = pow_checked(base, exponent);
    if errno3 == EDOM {
        let mut msg = b"Domain error: pow(".to_vec();
        msg.extend_from_slice(&format_fixed2(base));
        msg.extend_from_slice(b", ");
        msg.extend_from_slice(&format_fixed2(exponent));
        msg.extend_from_slice(b") is undefined in the real number domain.\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    } else if errno3 == ERANGE {
        let mut msg = b"Range error: pow(".to_vec();
        msg.extend_from_slice(&format_fixed2(base));
        msg.extend_from_slice(b", ");
        msg.extend_from_slice(&format_fixed2(exponent));
        msg.extend_from_slice(b") caused overflow or underflow.\n");
        stderr_write(&msg);
        return ExitCode::from(1);
    }

    let mut out = b"Result: ".to_vec();
    out.extend_from_slice(&format_fixed2(result));
    out.push(b'\n');
    stdout_write(&out);
    ExitCode::SUCCESS
}
