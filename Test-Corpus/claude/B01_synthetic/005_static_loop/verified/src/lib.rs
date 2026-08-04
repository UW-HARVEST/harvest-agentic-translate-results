// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to reproduce identical behavior.
//
// Library form of the program: exposes the same C symbols
// (`static_sum`, `main`) as the C build of the program, so the
// Rust .cdylib has byte-identical externally observable behavior
// when loaded as a shared library.

use std::cell::Cell;
use std::os::raw::{c_char, c_int};

// `static_sum` — equivalent to the C function:
//
//   int static_sum(int update) {
//     static int sum = 0;
//     sum += update;
//     return sum;
//   }
//
// We model the per-process static state with a thread-local cell.
// The reference program is single-threaded, so this matches C's
// behavior exactly for the program's actual usage.
thread_local! {
    static SUM: Cell<i32> = const { Cell::new(0) };
}

#[no_mangle]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    SUM.with(|s| {
        let new_val = s.get().wrapping_add(update);
        s.set(new_val);
        new_val
    })
}

/// Replicates C's `strtol(s, &end, 10)` for base 10:
///   * Skips leading whitespace.
///   * Accepts an optional '+' or '-' sign.
///   * Parses as many decimal digits as possible.
///   * Returns the parsed value (saturating to LONG_MIN/LONG_MAX on overflow,
///     same as glibc's strtol), and the number of bytes consumed from `s`.
///     If no digits were consumed (after sign), `consumed` will be 0,
///     mimicking C's behavior of leaving `*end == s`.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    let mut negative = false;
    if i < s.len() {
        match s[i] {
            b'+' => i += 1,
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }

    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => value = v,
                None => {
                    overflow = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if i == digits_start {
        return (0, 0);
    }

    (value, i)
}

/// FFI-callable `main` mirroring the C signature `int main(int argc, char **argv)`.
///
/// This intentionally matches the C program's behavior:
///   * Validates exactly one argument.
///   * Parses the argument with strtol-like semantics.
///   * Iterates 0..10 calling `static_sum(i * stride)` and printing the result.
///   * Prints to stdout via libc::printf so output ordering is identical
///     when the C and Rust shared libraries are loaded into the same process.
// `main` is only exported as a C symbol when the crate is built as a
// cdylib (used for the externally-loaded shared library). When the crate
// is consumed as an `rlib` (e.g. by cargo's test harness), exporting a
// `main` symbol would clash with the harness's own `main`, so we
// suppress the no_mangle export there.
#[cfg(feature = "export_main")]
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    driver_main(argc, argv)
}

pub unsafe fn driver_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    use std::ffi::CStr;

    if argc != 2 {
        let msg = b"Error: should only be a single (integer) argument!\n\0";
        libc_printf(msg.as_ptr() as *const c_char);
        return 1;
    }

    let arg_ptr = *argv.offset(1);
    let arg_cstr = CStr::from_ptr(arg_ptr);
    let arg_bytes = arg_cstr.to_bytes();

    let (parsed, consumed) = strtol_base10(arg_bytes);
    if consumed == 0 {
        let msg = b"Error: first argument must be an integer!\n\0";
        libc_printf(msg.as_ptr() as *const c_char);
        return 1;
    }

    let stride: i32 = parsed as i32;

    for i in 0..10i32 {
        let v = static_sum(i.wrapping_mul(stride));
        let fmt = b"%d\n\0";
        libc_printf(fmt.as_ptr() as *const c_char, v);
    }

    0
}

extern "C" {
    #[link_name = "printf"]
    fn libc_printf(fmt: *const c_char, ...) -> c_int;
}
